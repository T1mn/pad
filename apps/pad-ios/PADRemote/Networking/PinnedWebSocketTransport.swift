import CryptoKit
import Foundation
import Security

enum RemoteTransportError: LocalizedError {
    case notConnected
    case nonTextFrame
    case certificatePinMismatch
    case subprotocolMismatch

    var errorDescription: String? {
        switch self {
        case .notConnected: return "尚未连接到 Mac。"
        case .nonTextFrame: return "Mac 返回了不受支持的消息格式。"
        case .certificatePinMismatch: return "Mac 身份校验失败。请重新扫描二维码。"
        case .subprotocolMismatch: return "Mac 未确认 pad.remote.v1 安全子协议。"
        }
    }
}

/// A synchronous producer / single async consumer mailbox. AsyncStream's
/// continuation is thread-safe; the sole consumer awaits each handler before
/// pulling the next item, preserving delivery order across actor hops.
final class OrderedAsyncMailbox<Element: Sendable>: @unchecked Sendable {
    private let continuation: AsyncStream<Element>.Continuation
    private let consumerTask: Task<Void, Never>

    init(consumer: @escaping @Sendable (Element) async -> Void) {
        var installedContinuation: AsyncStream<Element>.Continuation?
        let stream = AsyncStream<Element> { continuation in
            installedContinuation = continuation
        }
        continuation = installedContinuation!
        consumerTask = Task {
            for await element in stream {
                await consumer(element)
            }
        }
    }

    @discardableResult
    func yield(_ element: Element) -> AsyncStream<Element>.Continuation.YieldResult {
        continuation.yield(element)
    }

    func finish() { continuation.finish() }

    func finishAndWait() async {
        continuation.finish()
        await consumerTask.value
    }

    deinit {
        continuation.finish()
        consumerTask.cancel()
    }
}

enum RemoteTransportDelivery: @unchecked Sendable {
    case opened
    case frame(RemoteFrame)
    case failed(Error)
    case closed(Error?)
}

final class PinnedSessionDelegate: NSObject, URLSessionDelegate, URLSessionWebSocketDelegate, @unchecked Sendable {
    let expectedFingerprint: String
    var onOpen: (@Sendable (String?) -> Void)?
    var onClose: (@Sendable (URLSessionWebSocketTask.CloseCode, Data?) -> Void)?
    var onTrustFailure: (@Sendable () -> Void)?

    init(expectedFingerprint: String) {
        self.expectedFingerprint = expectedFingerprint
    }

    func urlSession(
        _: URLSession,
        didReceive challenge: URLAuthenticationChallenge,
        completionHandler: @escaping (URLSession.AuthChallengeDisposition, URLCredential?) -> Void
    ) {
        guard challenge.protectionSpace.authenticationMethod == NSURLAuthenticationMethodServerTrust else {
            completionHandler(.performDefaultHandling, nil)
            return
        }
        guard let trust = challenge.protectionSpace.serverTrust,
              let chain = SecTrustCopyCertificateChain(trust) as? [SecCertificate],
              let leaf = chain.first else {
            onTrustFailure?()
            completionHandler(.cancelAuthenticationChallenge, nil)
            return
        }
        let der = SecCertificateCopyData(leaf) as Data
        let actual = SHA256.hash(data: der).map { String(format: "%02x", $0) }.joined()
        guard actual == expectedFingerprint else {
            onTrustFailure?()
            completionHandler(.cancelAuthenticationChallenge, nil)
            return
        }
        // Exact leaf-DER pinning is the trust anchor, including for the local self-signed certificate.
        completionHandler(.useCredential, URLCredential(trust: trust))
    }

    func urlSession(
        _: URLSession,
        webSocketTask _: URLSessionWebSocketTask,
        didOpenWithProtocol protocolName: String?
    ) {
        onOpen?(protocolName)
    }

    func urlSession(
        _: URLSession,
        webSocketTask _: URLSessionWebSocketTask,
        didCloseWith closeCode: URLSessionWebSocketTask.CloseCode,
        reason: Data?
    ) {
        onClose?(closeCode, reason)
    }
}

final class PinnedWebSocketTransport: @unchecked Sendable {
    typealias DeliveryHandler = @Sendable (UUID, RemoteTransportDelivery) async -> Void

    let generation: UUID
    private let codec = FrameCodec()
    private let delegate: PinnedSessionDelegate
    private let session: URLSession
    private let task: URLSessionWebSocketTask
    private let deliveryMailbox: OrderedAsyncMailbox<RemoteTransportDelivery>
    private var receiveTask: Task<Void, Never>?
    private var heartbeatTask: Task<Void, Never>?

    init(
        endpoint: URL,
        fingerprint: String,
        generation: UUID = UUID(),
        onDelivery: @escaping DeliveryHandler
    ) {
        self.generation = generation
        deliveryMailbox = OrderedAsyncMailbox { delivery in
            await onDelivery(generation, delivery)
        }

        let delegate = PinnedSessionDelegate(expectedFingerprint: fingerprint)
        self.delegate = delegate
        let configuration = URLSessionConfiguration.ephemeral
        configuration.waitsForConnectivity = false
        configuration.timeoutIntervalForRequest = 15
        configuration.urlCredentialStorage = nil
        configuration.httpCookieStorage = nil
        let session = URLSession(configuration: configuration, delegate: delegate, delegateQueue: nil)
        self.session = session

        task = session.webSocketTask(
            with: endpoint,
            protocols: [RemoteProtocolConstants.webSocketSubprotocol]
        )
        delegate.onOpen = { [weak self] negotiated in
            guard let self else { return }
            guard negotiated == RemoteProtocolConstants.webSocketSubprotocol else {
                self.task.cancel(with: .protocolError, reason: nil)
                self.deliveryMailbox.yield(.closed(RemoteTransportError.subprotocolMismatch))
                return
            }
            self.deliveryMailbox.yield(.opened)
        }
        delegate.onClose = { [weak self] _, _ in
            guard let self else { return }
            self.deliveryMailbox.yield(.closed(nil))
        }
        delegate.onTrustFailure = { [weak self] in
            guard let self else { return }
            self.deliveryMailbox.yield(.closed(RemoteTransportError.certificatePinMismatch))
        }
    }

    func connect() {
        task.resume()
        receiveTask = Task { [weak self] in await self?.receiveLoop() }
        heartbeatTask = Task { [weak self] in await self?.heartbeatLoop() }
    }

    func send(_ frame: RemoteFrame) async throws {
        let data = try codec.encode(frame)
        guard let text = String(data: data, encoding: .utf8) else { throw RemoteTransportError.nonTextFrame }
        try await task.send(.string(text))
    }

    func disconnect() {
        receiveTask?.cancel()
        heartbeatTask?.cancel()
        task.cancel(with: .goingAway, reason: nil)
        session.invalidateAndCancel()
        deliveryMailbox.finish()
    }

    private func receiveLoop() async {
        while !Task.isCancelled {
            do {
                let message = try await task.receive()
                let data: Data
                switch message {
                case let .string(text):
                    guard let encoded = text.data(using: .utf8) else { throw RemoteTransportError.nonTextFrame }
                    data = encoded
                case .data:
                    throw RemoteTransportError.nonTextFrame
                @unknown default:
                    throw RemoteTransportError.nonTextFrame
                }
                do {
                    deliveryMailbox.yield(.frame(try codec.decode(data)))
                } catch {
                    deliveryMailbox.yield(.failed(error))
                    return
                }
            } catch {
                guard !Task.isCancelled else { return }
                deliveryMailbox.yield(.closed(error))
                return
            }
        }
    }

    private func heartbeatLoop() async {
        while !Task.isCancelled {
            do {
                try await Task.sleep(for: .seconds(15))
                try await send(RemoteFrame(type: .ping))
                try await sendWebSocketPing()
            } catch {
                guard !Task.isCancelled else { return }
                deliveryMailbox.yield(.closed(error))
                return
            }
        }
    }

    private func sendWebSocketPing() async throws {
        try await withCheckedThrowingContinuation { (continuation: CheckedContinuation<Void, Error>) in
            task.sendPing { error in
                if let error { continuation.resume(throwing: error) }
                else { continuation.resume() }
            }
        }
    }
}
