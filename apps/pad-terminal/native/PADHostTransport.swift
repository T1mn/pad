#if os(macOS)
import Foundation

/// Minimal JSON tree used on the Swift <-> Node boundary. Kept local to the
/// workbench state layer so the transport has no dependency on any other target.
enum PADJSONValue: Codable, Equatable, Sendable {
    case string(String)
    case number(Double)
    case bool(Bool)
    case object([String: PADJSONValue])
    case array([PADJSONValue])
    case null

    init(from decoder: Decoder) throws {
        let container = try decoder.singleValueContainer()
        if container.decodeNil() {
            self = .null
        } else if let value = try? container.decode(Bool.self) {
            self = .bool(value)
        } else if let value = try? container.decode(Double.self) {
            self = .number(value)
        } else if let value = try? container.decode(String.self) {
            self = .string(value)
        } else if let value = try? container.decode([String: PADJSONValue].self) {
            self = .object(value)
        } else {
            self = .array((try? container.decode([PADJSONValue].self)) ?? [])
        }
    }

    func encode(to encoder: Encoder) throws {
        var container = encoder.singleValueContainer()
        switch self {
        case let .string(value): try container.encode(value)
        case let .number(value): try container.encode(value)
        case let .bool(value): try container.encode(value)
        case let .object(value): try container.encode(value)
        case let .array(value): try container.encode(value)
        case .null: try container.encodeNil()
        }
    }

    var objectValue: [String: PADJSONValue]? {
        guard case let .object(value) = self else { return nil }
        return value
    }

    var stringValue: String? {
        guard case let .string(value) = self else { return nil }
        return value
    }

    var numberValue: Double? {
        guard case let .number(value) = self else { return nil }
        return value
    }

    var boolValue: Bool? {
        guard case let .bool(value) = self else { return nil }
        return value
    }

    var arrayValue: [PADJSONValue]? {
        guard case let .array(value) = self else { return nil }
        return value
    }

    /// Re-decode a subtree into a frozen DTO without leaking the wire shape.
    func decoded<T: Decodable>(_ type: T.Type) -> T? {
        guard let data = try? JSONEncoder().encode(self) else { return nil }
        return try? JSONDecoder().decode(type, from: data)
    }
}

struct PADHostFailure: Error, LocalizedError, Equatable {
    let message: String
    var errorDescription: String? { message }
}

/// Owns the single `workbench-host.mjs` child process and speaks the JSONL
/// protocol from `host/PROTOCOL.md`: LF-delimited UTF-8 frames, unique request
/// ids, asynchronous correlation with timeouts, and stderr kept out of band.
///
/// The class is main-actor isolated: pipe callbacks hop onto the main actor
/// before touching state, so there is no cross-thread mutation. Request and
/// response payloads are never logged.
@MainActor
final class PADHostTransport {
    enum Event {
        case snapshot(PADSnapshot)
        case pi(taskId: String, data: PADJSONValue)
        case auth(PADAuthState)
        case systemProxyNotAdopted
    }

    static let maxFrameBytes = 8 * 1024 * 1024

    /// Called on the main actor for each protocol event frame.
    var onEvent: ((Event) -> Void)?
    /// Called on the main actor after the child process exits.
    var onExit: ((Int32?) -> Void)?

    private struct Pending {
        let command: String
        let completion: (Result<PADJSONValue, Error>) -> Void
        let timeout: Task<Void, Never>
    }

    private var process: Process?
    private var stdinHandle: FileHandle?
    private var stdoutHandle: FileHandle?
    private var stderrHandle: FileHandle?
    private var buffer = Data()
    private var pending: [String: Pending] = [:]
    private var nextRequestId = 0
    private let requestPrefix = String(UUID().uuidString.prefix(8))
    private var stderrTail = ""
    private(set) var isRunning = false

    // MARK: - Launch

    func start() throws {
        guard !isRunning else { return }

        let node = try Self.resolveNode()
        let script = try Self.resolveHostScript()

        let process = Process()
        process.executableURL = URL(fileURLWithPath: node)
        process.arguments = [script]
        process.currentDirectoryURL = URL(fileURLWithPath: NSHomeDirectory(), isDirectory: true)
        process.environment = Self.minimalEnvironment()

        let inputPipe = Pipe()
        let outputPipe = Pipe()
        let errorPipe = Pipe()
        process.standardInput = inputPipe
        process.standardOutput = outputPipe
        process.standardError = errorPipe

        outputPipe.fileHandleForReading.readabilityHandler = { [weak self] handle in
            let data = handle.availableData
            if data.isEmpty {
                handle.readabilityHandler = nil
                return
            }
            Task { @MainActor [weak self] in self?.ingestStdout(data) }
        }
        errorPipe.fileHandleForReading.readabilityHandler = { [weak self] handle in
            let data = handle.availableData
            if data.isEmpty {
                handle.readabilityHandler = nil
                return
            }
            Task { @MainActor [weak self] in self?.ingestStderr(data) }
        }
        process.terminationHandler = { [weak self] finished in
            let status = finished.terminationStatus
            Task { @MainActor [weak self] in self?.handleExit(status) }
        }

        do {
            try process.run()
        } catch {
            outputPipe.fileHandleForReading.readabilityHandler = nil
            errorPipe.fileHandleForReading.readabilityHandler = nil
            throw PADHostFailure(message: "无法启动工作台宿主进程：\(error.localizedDescription)")
        }

        self.process = process
        self.stdinHandle = inputPipe.fileHandleForWriting
        self.stdoutHandle = outputPipe.fileHandleForReading
        self.stderrHandle = errorPipe.fileHandleForReading
        self.isRunning = true
        self.buffer.removeAll(keepingCapacity: true)
    }

    private static func resolveNode() throws -> String {
        let environment = ProcessInfo.processInfo.environment
        if let explicit = environment["PAD_NODE_PATH"], !explicit.isEmpty {
            guard FileManager.default.isExecutableFile(atPath: explicit) else {
                throw PADHostFailure(message: "PAD_NODE_PATH 指向的 Node 不可执行：\(explicit)")
            }
            return explicit
        }
        for candidate in ["/opt/homebrew/bin/node", "/usr/local/bin/node"]
        where FileManager.default.isExecutableFile(atPath: candidate) {
            return candidate
        }
        throw PADHostFailure(
            message: "未找到 Node.js。请安装 Homebrew node，或通过 PAD_NODE_PATH 指定绝对路径。"
        )
    }

    private static func resolveHostScript() throws -> String {
        let environment = ProcessInfo.processInfo.environment
        if let explicit = environment["PAD_TERMINAL_HOST"], !explicit.isEmpty {
            guard FileManager.default.fileExists(atPath: explicit) else {
                throw PADHostFailure(message: "PAD_TERMINAL_HOST 指向的宿主脚本不存在：\(explicit)")
            }
            return explicit
        }
        guard let resourceURL = Bundle.main.resourceURL else {
            throw PADHostFailure(message: "应用资源目录不可用，无法定位 PADHost/host/workbench-host.mjs。")
        }
        let script = resourceURL
            .appendingPathComponent("PADHost", isDirectory: true)
            .appendingPathComponent("host", isDirectory: true)
            .appendingPathComponent("workbench-host.mjs")
            .path
        guard FileManager.default.fileExists(atPath: script) else {
            throw PADHostFailure(
                message: "缺少宿主脚本 PADHost/host/workbench-host.mjs。开发构建可设置 PAD_TERMINAL_HOST。"
            )
        }
        return script
    }

    /// Minimal environment plus explicit developer overrides. The host must not
    /// inherit ambient provider credentials or NODE_OPTIONS.
    private static func minimalEnvironment() -> [String: String] {
        let source = ProcessInfo.processInfo.environment
        let allowedKeys = [
            "HOME", "USER", "LOGNAME", "SHELL", "PATH", "TMPDIR", "LANG", "LC_ALL", "LC_CTYPE",
            "PAD_PI_PACKAGE", "PAD_TERMINAL_DATA_ROOT",
        ]
        var environment: [String: String] = [:]
        for key in allowedKeys {
            if let value = source[key], !value.isEmpty { environment[key] = value }
        }
        if environment["PATH"] == nil {
            environment["PATH"] = "/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin"
        }
        return PADProxyEnvironment.applying(to: environment, source: source)
    }

    // MARK: - Requests

    func send(
        command: String,
        fields: [String: PADJSONValue] = [:],
        timeout: TimeInterval = 30,
        completion: @escaping (Result<PADJSONValue, Error>) -> Void
    ) {
        guard isRunning, let stdinHandle else {
            completion(.failure(PADHostFailure(message: "工作台宿主未运行。")))
            return
        }
        nextRequestId += 1
        let id = "\(requestPrefix)-\(nextRequestId)"
        var payload: [String: PADJSONValue] = ["id": .string(id), "command": .string(command)]
        for (key, value) in fields { payload[key] = value }
        guard var line = try? JSONEncoder().encode(PADJSONValue.object(payload)) else {
            completion(.failure(PADHostFailure(message: "无法编码工作台请求。")))
            return
        }
        line.append(0x0A)
        do {
            try stdinHandle.write(contentsOf: line)
        } catch {
            completion(.failure(PADHostFailure(message: "写入工作台宿主失败。")))
            return
        }
        let timeoutTask = Task { [weak self] in
            let nanoseconds = UInt64(max(1, timeout) * 1_000_000_000)
            try? await Task.sleep(nanoseconds: nanoseconds)
            guard !Task.isCancelled else { return }
            self?.expireRequest(id: id)
        }
        pending[id] = Pending(command: command, completion: completion, timeout: timeoutTask)
    }

    private func expireRequest(id: String) {
        guard let entry = pending.removeValue(forKey: id) else { return }
        entry.timeout.cancel()
        entry.completion(.failure(PADHostFailure(message: "工作台请求超时（\(entry.command)）。")))
    }

    private func failAllPending(_ error: Error) {
        let outstanding = pending
        pending.removeAll()
        for (_, entry) in outstanding {
            entry.timeout.cancel()
            entry.completion(.failure(error))
        }
    }

    // MARK: - Framing

    private func ingestStdout(_ data: Data) {
        guard isRunning else { return }
        buffer.append(data)
        while let newline = buffer.firstIndex(of: 0x0A) {
            let frame = buffer[buffer.startIndex..<newline]
            buffer.removeSubrange(buffer.startIndex...newline)
            guard frame.count <= Self.maxFrameBytes else {
                failAllPending(PADHostFailure(message: "工作台回帧超过 8 MiB 上限。"))
                return
            }
            handleFrame(Data(frame))
        }
        if buffer.count > Self.maxFrameBytes {
            buffer.removeAll(keepingCapacity: false)
            failAllPending(PADHostFailure(message: "工作台回帧超过 8 MiB 上限。"))
        }
    }

    private func ingestStderr(_ data: Data) {
        guard let text = String(data: data, encoding: .utf8) else { return }
        stderrTail.append(text)
        if stderrTail.count > 4_096 { stderrTail = String(stderrTail.suffix(4_096)) }
    }

    private func handleFrame(_ data: Data) {
        var frame = data
        if frame.last == 0x0D { frame.removeLast() }
        guard !frame.isEmpty, let value = try? JSONDecoder().decode(PADJSONValue.self, from: frame),
              let object = value.objectValue else { return }

        switch object["type"]?.stringValue {
        case "response":
            handleResponse(object)
        case "event":
            handleEvent(object)
        default:
            break
        }
    }

    private func handleResponse(_ object: [String: PADJSONValue]) {
        guard let id = object["id"]?.stringValue, let entry = pending.removeValue(forKey: id) else { return }
        entry.timeout.cancel()
        if object["success"]?.boolValue ?? false {
            entry.completion(.success(object["data"] ?? .object([:])))
        } else {
            entry.completion(.failure(PADHostFailure(message: Self.errorMessage(from: object["error"]))))
        }
    }

    private func handleEvent(_ object: [String: PADJSONValue]) {
        switch object["event"]?.stringValue {
        case "snapshot":
            if let snapshot = object["data"]?.decoded(PADSnapshot.self) { onEvent?(.snapshot(snapshot)) }
        case "pi":
            if let taskId = object["taskId"]?.stringValue, let data = object["data"] {
                onEvent?(.pi(taskId: taskId, data: data))
            }
        case "auth":
            if let auth = object["data"]?.decoded(PADAuthState.self) { onEvent?(.auth(auth)) }
        case "host_warning":
            if object["data"]?.objectValue?["code"]?.stringValue == "system_proxy_not_adopted" {
                onEvent?(.systemProxyNotAdopted)
            }
        default:
            break
        }
    }

    private static func errorMessage(from value: PADJSONValue?) -> String {
        switch value {
        case let .string(text):
            return String(text.prefix(1_000))
        case let .object(object):
            if let message = object["message"]?.stringValue { return String(message.prefix(1_000)) }
            return "工作台请求失败。"
        default:
            return "工作台请求失败。"
        }
    }

    // MARK: - Teardown

    /// Best-effort `shutdown`, stdin EOF, then SIGTERM if the child lingers.
    func shutdown() {
        failAllPending(PADHostFailure(message: "工作台宿主已停止。"))
        guard let process, process.isRunning else {
            cleanup()
            return
        }
        if let stdinHandle {
            let line = Data("{\"id\":\"\(requestPrefix)-shutdown\",\"command\":\"shutdown\"}\n".utf8)
            try? stdinHandle.write(contentsOf: line)
            try? stdinHandle.close()
        }
        Task { [weak self] in
            try? await Task.sleep(nanoseconds: 2_000_000_000)
            if process.isRunning { process.terminate() }
            self?.cleanup()
        }
    }

    private func handleExit(_ status: Int32?) {
        isRunning = false
        failAllPending(PADHostFailure(message: "工作台宿主进程已退出。"))
        cleanup()
        onExit?(status)
    }

    private func cleanup() {
        isRunning = false
        stdoutHandle?.readabilityHandler = nil
        stderrHandle?.readabilityHandler = nil
        try? stdinHandle?.close()
        try? stdoutHandle?.close()
        try? stderrHandle?.close()
        stdinHandle = nil
        stdoutHandle = nil
        stderrHandle = nil
        process = nil
        buffer.removeAll(keepingCapacity: false)
    }
}
#endif
