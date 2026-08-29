import Combine
import Foundation
import Network
import UIKit

enum RemoteConnectionState: Equatable {
    case notPaired
    case connecting
    case pairing
    case online
    case reconnecting(attempt: Int)
    case offline
    case suspended

    var title: String {
        switch self {
        case .notPaired: return "未配对"
        case .connecting: return "正在连接"
        case .pairing: return "正在安全配对"
        case .online: return "实时连接"
        case let .reconnecting(attempt): return "正在恢复连接（第 \(attempt + 1) 次）"
        case .offline: return "离线 · 内容已保存在本机"
        case .suspended: return "已暂停 · 返回前台后恢复"
        }
    }

    var symbol: String {
        switch self {
        case .online: return "bolt.horizontal.circle.fill"
        case .connecting, .pairing, .reconnecting: return "arrow.triangle.2.circlepath"
        case .notPaired: return "iphone.and.arrow.forward"
        case .offline: return "wifi.slash"
        case .suspended: return "pause.circle"
        }
    }
}

enum RemoteTransportFailureDecision: Equatable {
    case retry
    case stop(message: String, offersSettings: Bool)
}

enum RemoteTransportFailurePolicy {
    static func decision(for error: Error?) -> RemoteTransportFailureDecision {
        guard let error else { return .retry }
        if let codecError = error as? FrameCodecError, codecError == .tooLarge {
            return .stop(message: "Mac 返回的数据超过 1 MiB。请在 Mac 缩短任务历史后刷新。", offersSettings: false)
        }
        if let transportError = error as? RemoteTransportError {
            switch transportError {
            case .certificatePinMismatch, .subprotocolMismatch, .nonTextFrame:
                return .stop(message: transportError.localizedDescription, offersSettings: false)
            case .notConnected:
                return .retry
            }
        }
        let nsError = error as NSError
        let underlying = nsError.userInfo[NSUnderlyingErrorKey] as? NSError
        if (nsError.domain == NSPOSIXErrorDomain && [1, 13].contains(nsError.code))
            || (underlying?.domain == NSPOSIXErrorDomain && [1, 13].contains(underlying?.code ?? 0)) {
            return .stop(
                message: "iOS 阻止了局域网连接。请在系统设置中允许 PAD Remote 访问本地网络。",
                offersSettings: true
            )
        }
        guard let urlError = error as? URLError else { return .retry }
        switch urlError.code {
        case .cannotFindHost, .dnsLookupFailed, .badURL, .unsupportedURL:
            return .stop(
                message: "无法解析二维码中的 Mac 地址。请确认处于同一局域网，或重新生成二维码。",
                offersSettings: false
            )
        case .appTransportSecurityRequiresSecureConnection,
             .secureConnectionFailed,
             .serverCertificateHasBadDate,
             .serverCertificateUntrusted,
             .serverCertificateHasUnknownRoot,
             .serverCertificateNotYetValid,
             .clientCertificateRejected,
             .clientCertificateRequired:
            return .stop(message: "安全连接无法建立。请在 Mac 重新生成二维码。", offersSettings: false)
        default:
            return .retry
        }
    }
}

@MainActor
final class RemoteAppModel: ObservableObject {
    @Published private(set) var pairedHost: PairedHost?
    @Published private(set) var connectionState: RemoteConnectionState = .notPaired
    @Published private(set) var cached = CachedRemoteState()
    @Published private(set) var lastError: String?
    @Published private(set) var profileAvailable = true
    @Published private(set) var settingsRecoveryAvailable = false
    @Published private(set) var isSavingPrompt = false
    @Published var composerText = ""

    private let tokenStore: SecureTokenStoring
    private let hostStore: HostMetadataStore
    private let cacheStore: CacheStore
    private let outbox: OutboxStore
    private let monitor = NWPathMonitor()
    private let monitorQueue = DispatchQueue(label: "cn.ghostcloud.pad.remote.network")
    private let reconnectSchedule = ReconnectSchedule()

    private var transport: PinnedWebSocketTransport?
    private var currentGeneration: UUID?
    private var pendingInvitation: PairingInvitation?
    private var queuedInvitationDuringRestore: PairingInvitation?
    private var pairingCompletionGeneration: UUID?
    private var restoreFinished = false
    private var storageReady = false
    private var reconnectTask: Task<Void, Never>?
    private var outboxPumpTask: Task<Void, Never>?
    private var outboxPumpID: UUID?
    private var sentCommandIDs = Set<UUID>()
    private var cacheWriteSequence: UInt64 = 0
    private var pendingAckRevision: Int64?
    private var ackPumpTask: Task<Void, Never>?
    private var ackPumpID: UUID?
    private var highestAckSent: Int64 = 0
    private var handshakeTimeoutTask: Task<Void, Never>?
    private var historyRefreshTasks: [String: Task<Void, Never>] = [:]
    private var sidebarRefreshTask: Task<Void, Never>?
    private var resyncCheckpoint: ResyncCheckpointBuffer?
    private var reconnectAttempt = 0
    private var isForeground = true
    private var pathIsSatisfied = true

    init(
        tokenStore: SecureTokenStoring = KeychainTokenStore(),
        hostStore: HostMetadataStore = HostMetadataStore(),
        cacheStore: CacheStore = CacheStore(),
        outbox: OutboxStore = OutboxStore()
    ) {
        self.tokenStore = tokenStore
        self.hostStore = hostStore
        self.cacheStore = cacheStore
        self.outbox = outbox
        monitor.pathUpdateHandler = { [weak self] path in
            Task { @MainActor [weak self] in
                guard let self else { return }
                let wasSatisfied = self.pathIsSatisfied
                self.pathIsSatisfied = path.status == .satisfied
                if self.pathIsSatisfied, !wasSatisfied, self.isForeground { self.connectImmediately() }
                if !self.pathIsSatisfied { self.markOfflineAndDisconnect() }
            }
        }
        monitor.start(queue: monitorQueue)
        Task { await restore() }
    }

    deinit { monitor.cancel() }

    var tasks: [RemoteTaskSummary] { cached.tasks }
    var selectedTask: RemoteTaskSummary? {
        guard let id = cached.selectedTaskID else { return nil }
        return cached.tasks.first { $0.id == id }
    }
    var selectedMessages: [RemoteMessage] {
        guard let id = cached.selectedTaskID else { return [] }
        return cached.messagesByTask[id] ?? []
    }
    var selectedUIRequests: [RemoteUIRequest] {
        guard let id = cached.selectedTaskID else { return [] }
        return cached.pendingUIRequestsByTask[id] ?? []
    }
    var canSend: Bool {
        pairedHost != nil
            && cached.selectedTaskID != nil
            && !isSavingPrompt
            && !composerText.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
    }

    func pair(uri: String) {
        do {
            let invitation = try PairingInvitation(uri: uri)
            lastError = nil
            if restoreFinished {
                beginPairing(invitation)
            } else {
                queuedInvitationDuringRestore = invitation
                connectionState = .pairing
            }
        } catch {
            lastError = error.localizedDescription
        }
    }

    func selectTask(_ id: String) {
        cached.selectedTaskID = id
        persistCache()
        sendCommand(.history, params: .object(["task_id": .string(id)]))
    }

    func createTask() {
        // A client-generated task id makes a replay after a Mac crash
        // idempotent even if the receipt commit was interrupted.
        enqueueCommands([PendingCommand.createTask()])
    }

    func sendPrompt() {
        let text = composerText.trimmingCharacters(in: .whitespacesAndNewlines)
        guard let taskID = cached.selectedTaskID,
              let identity = pairedHost?.identity,
              !text.isEmpty,
              !isSavingPrompt else { return }
        let draft = composerText
        let localMessageID = "local-\(UUID().uuidString)"
        let commands = PendingCommand.promptTransaction(
            taskID: taskID,
            prompt: text,
            localMessageID: localMessageID
        )
        do {
            for command in commands { _ = try FrameCodec().encode(command.frame) }
        } catch {
            lastError = "消息过长，无法放入 1 MiB 的安全传输帧。请缩短后再发送。"
            return
        }
        let optimistic = RemoteMessage(
            id: localMessageID,
            role: .user,
            text: text,
            createdAt: Date(),
            isStreaming: false
        )
        isSavingPrompt = true
        Task {
            var commandsAreDurable = false
            do {
                _ = try await outbox.enqueue(commands, for: identity)
                commandsAreDurable = true
                guard pairedHost?.identity == identity else {
                    isSavingPrompt = false
                    return
                }
                if composerText == draft { composerText = "" }
                RemoteContentReducer.addOptimisticMessage(optimistic, taskID: taskID, to: &cached)
                cached.updatedAt = Date()
                let cacheSequence = issueCacheWriteSequence()
                _ = try await cacheStore.save(cached, for: identity, sequence: cacheSequence)
                guard pairedHost?.identity == identity else {
                    isSavingPrompt = false
                    return
                }
                isSavingPrompt = false
                pumpOutbox()
            } catch {
                isSavingPrompt = false
                guard pairedHost?.identity == identity else { return }
                if commandsAreDurable {
                    storageReady = false
                    closeTransport()
                    connectionState = .offline
                    lastError = "消息已进入发件箱，但本机缓存写入失败。连接已停止，请释放存储空间后重试。"
                } else if error is FrameCodecError {
                    lastError = "消息过长，未加入发件箱。请缩短后再发送。"
                } else {
                    lastError = "无法安全保存消息，消息尚未发送。请检查本机存储空间后重试。"
                }
            }
        }
    }

    func stopSelectedTask() {
        guard let taskID = cached.selectedTaskID else { return }
        sendCommand(.stopTask, params: .object(["task_id": .string(taskID)]))
    }

    func retrySelectedTask() {
        guard let taskID = cached.selectedTaskID else { return }
        sendCommand(.retryTask, params: .object(["task_id": .string(taskID)]))
    }

    func respondToUI(taskID: String, requestID: String, value: JSONValue) {
        guard let request = cached.pendingUIRequestsByTask[taskID]?.first(where: {
            $0.id == requestID && $0.requiresResponse
        }) else { return }
        enqueueCommands([PendingCommand.respondUI(taskID: taskID, request: request, value: value)])
    }

    func cancelUI(taskID: String, requestID: String) {
        guard let request = cached.pendingUIRequestsByTask[taskID]?.first(where: {
            $0.id == requestID && $0.requiresResponse
        }) else { return }
        enqueueCommands([PendingCommand.cancelUI(taskID: taskID, request: request)])
    }

    func sceneBecameActive() {
        isForeground = true
        connectImmediately()
    }

    func refreshContent() {
        guard connectionState == .online else {
            connectImmediately()
            return
        }
        sendCommand(.bootstrap, params: .object([:]))
        sendCommand(.listSidebar, params: .object([:]))
        if let taskID = cached.selectedTaskID {
            sendCommand(.history, params: .object(["task_id": .string(taskID)]))
        }
    }

    func sceneEnteredBackground() {
        isForeground = false
        reconnectTask?.cancel()
        historyRefreshTasks.values.forEach { $0.cancel() }
        historyRefreshTasks.removeAll()
        sidebarRefreshTask?.cancel()
        sidebarRefreshTask = nil
        cached.updatedAt = Date()
        let snapshot = cached
        let cacheSequence = issueCacheWriteSequence()
        let identity = pairedHost?.identity
        closeTransport()
        connectionState = pairedHost == nil ? .notPaired : .suspended
        guard let identity else { return }
        var taskID: UIBackgroundTaskIdentifier = .invalid
        taskID = UIApplication.shared.beginBackgroundTask(withName: "保存 PAD Remote 状态") {
            if taskID != .invalid { UIApplication.shared.endBackgroundTask(taskID) }
        }
        Task {
            defer {
                if taskID != .invalid { UIApplication.shared.endBackgroundTask(taskID) }
            }
            do {
                _ = try await cacheStore.save(snapshot, for: identity, sequence: cacheSequence)
            } catch {
                guard pairedHost?.identity == identity else { return }
                storageReady = false
                lastError = "进入后台时无法安全保存本机缓存。请检查存储空间后重新打开应用。"
            }
        }
    }

    func clearError() { lastError = nil }

    func openSystemSettings() {
        guard settingsRecoveryAvailable,
              let url = URL(string: UIApplication.openSettingsURLString) else { return }
        UIApplication.shared.open(url)
    }

    func disconnectAndForget() {
        reconnectTask?.cancel()
        cancelScheduledRefreshes()
        closeTransport()
        let oldHost = pairedHost
        if let oldHost {
            do {
                try tokenStore.delete(account: tokenAccount(deviceID: oldHost.deviceID))
            } catch {
                lastError = "配对已取消，但设备凭据无法从钥匙串删除。请在释放存储空间后重试。"
            }
        }
        pairedHost = nil
        pendingInvitation = nil
        queuedInvitationDuringRestore = nil
        storageReady = false
        cached = CachedRemoteState()
        connectionState = .notPaired
        Task {
            guard let identity = oldHost?.identity else { return }
            do {
                try await hostStore.remove(matching: identity)
                try await cacheStore.remove(matching: identity)
                try await outbox.removeAll(matching: identity)
            } catch {
                lastError = "配对已取消，但部分本机缓存无法删除。旧数据不会发送给其他 Mac。"
            }
        }
    }

    private func restore() async {
        defer {
            restoreFinished = true
            if let invitation = queuedInvitationDuringRestore {
                queuedInvitationDuringRestore = nil
                beginPairing(invitation)
            }
        }
        guard let host = await hostStore.load() else {
            cached = CachedRemoteState()
            connectionState = .notPaired
            return
        }
        do {
            let restoredCache = try await cacheStore.activate(for: host.identity)
            try await outbox.activate(for: host.identity)
            pairedHost = host
            cached = restoredCache
            cacheWriteSequence = 0
            storageReady = true
        } catch {
            pairedHost = host
            cached = CachedRemoteState()
            storageReady = false
            lastError = "无法安全绑定这台 Mac 的本机数据，连接已停止。"
            connectionState = .offline
            return
        }
        connectionState = .offline
        if queuedInvitationDuringRestore == nil, isForeground { connectImmediately() }
    }

    private func beginPairing(_ invitation: PairingInvitation) {
        reconnectTask?.cancel()
        cancelScheduledRefreshes()
        closeTransport()
        let oldHost = pairedHost
        if let oldHost {
            do {
                try tokenStore.delete(account: tokenAccount(deviceID: oldHost.deviceID))
            } catch {
                lastError = "无法清除旧设备凭据，已停止新配对。"
                connectionState = .offline
                return
            }
        }
        pairedHost = nil
        cached = CachedRemoteState()
        storageReady = false
        pendingInvitation = invitation
        reconnectAttempt = 0
        connectionState = .pairing
        if let identity = oldHost?.identity {
            Task {
                try? await hostStore.remove(matching: identity)
                try? await cacheStore.remove(matching: identity)
                try? await outbox.removeAll(matching: identity)
            }
        }
        connectImmediately()
    }

    private func connectImmediately() {
        guard RemoteConnectionLifecyclePolicy.shouldOpen(
            state: connectionState,
            hasTransport: transport != nil
        ) else { return }
        reconnectTask?.cancel()
        reconnectTask = nil
        reconnectAttempt = 0
        guard isForeground, pathIsSatisfied else { return }
        guard RemoteStorageReadinessPolicy.canOpen(
            hasPendingInvitation: pendingInvitation != nil,
            hasPairedHost: pairedHost != nil,
            storageReady: storageReady
        ) else { return }
        if let invitation = pendingInvitation {
            open(endpoint: invitation.endpoint, fingerprint: invitation.fingerprint, state: .pairing)
        } else if let host = pairedHost {
            guard storageReady else { return }
            open(endpoint: host.endpoint, fingerprint: host.fingerprint, state: .connecting)
        }
    }

    private func open(endpoint: URL, fingerprint: String, state: RemoteConnectionState) {
        closeTransport()
        let generation = UUID()
        currentGeneration = generation
        connectionState = state
        let newTransport = PinnedWebSocketTransport(
            endpoint: endpoint,
            fingerprint: fingerprint,
            generation: generation,
            onDelivery: { [weak self] generation, delivery in
                await self?.received(delivery, generation: generation)
            }
        )
        transport = newTransport
        newTransport.connect()
        handshakeTimeoutTask?.cancel()
        handshakeTimeoutTask = Task { [weak self] in
            try? await Task.sleep(for: .seconds(12))
            guard !Task.isCancelled, let self, self.currentGeneration == generation,
                  self.connectionState != .online else { return }
            self.lastError = "连接握手超时，正在重试。"
            self.closeTransport()
            self.scheduleReconnect()
        }
    }

    private func opened(generation: UUID) {
        guard generation == currentGeneration else { return }
        settingsRecoveryAvailable = false
        if let invitation = pendingInvitation {
            let frame = RemoteFrame(
                type: .pair,
                device: DeviceDescriptor(displayName: UIDevice.current.name, platform: "ios"),
                pairingID: invitation.pairingID,
                secret: invitation.secret
            )
            sendFrame(frame)
            return
        }
        guard let host = pairedHost else { return }
        let tokenData: Data?
        do { tokenData = try tokenStore.read(account: tokenAccount(deviceID: host.deviceID)) }
        catch { tokenData = nil }
        guard let tokenData,
              let token = String(data: tokenData, encoding: .utf8) else {
            lastError = "设备凭据不可用，请重新配对。"
            disconnectAndForget()
            return
        }
        sendFrame(RemoteFrame(
            type: .resume,
            deviceID: host.deviceID,
            deviceToken: token,
            serverEpoch: cached.cursor.serverEpoch,
            afterRevision: cached.cursor.revision
        ))
    }

    private func received(_ delivery: RemoteTransportDelivery, generation: UUID) async {
        guard generation == currentGeneration else { return }
        switch delivery {
        case .opened:
            opened(generation: generation)
        case let .frame(frame):
            await handle(frame, generation: generation)
        case let .failed(error):
            closed(generation: generation, error: error)
        case let .closed(error):
            closed(generation: generation, error: error)
        }
    }

    private func handle(_ frame: RemoteFrame, generation: UUID) async {
        guard generation == currentGeneration else { return }
        switch frame.type {
        case .paired:
            guard pairingCompletionGeneration != generation else { return }
            guard let epoch = frame.serverEpoch, let latestRevision = frame.latestRevision else {
                handleWireError(RemoteWireError(code: "invalid_paired", message: "Mac 返回的配对恢复点无效。"))
                return
            }
            // This runs synchronously on MainActor before completePairing's first
            // persistence await. The transport mailbox will not consume a later
            // event until completePairing returns, and the checkpoint remains a
            // second line of defence if pairing storage work is refactored later.
            pairingCompletionGeneration = generation
            beginResync(epoch: epoch, latestRevision: latestRevision)
            await completePairing(frame, generation: generation)
        case .welcome:
            guard let epoch = frame.serverEpoch, let latestRevision = frame.latestRevision else {
                handleWireError(RemoteWireError(code: "invalid_welcome", message: "Mac 返回的恢复信息无效。"))
                return
            }
            profileAvailable = frame.profileAvailable ?? true
            if cached.cursor.serverEpoch != epoch {
                beginResync(epoch: epoch, latestRevision: latestRevision)
            } else if latestRevision < cached.cursor.revision {
                beginResync(epoch: epoch, latestRevision: latestRevision)
            }
            reconnectAttempt = 0
            handshakeTimeoutTask?.cancel()
            connectionState = .online
            lastError = nil
            pumpOutbox()
            sendCommand(.bootstrap, params: .object([:]))
        case .commandResult:
            await handleCommandResult(frame, generation: generation)
        case .event:
            handleEvent(frame)
        case .resyncRequired:
            guard let epoch = frame.serverEpoch, let latestRevision = frame.latestRevision else {
                handleWireError(RemoteWireError(code: "invalid_resync", message: "Mac 返回的重新同步信息无效。"))
                return
            }
            beginResync(epoch: epoch, latestRevision: latestRevision)
            sendCommand(.bootstrap, params: .object([:]))
            sendCommand(.listSidebar, params: .object([:]))
            if let taskID = cached.selectedTaskID {
                sendCommand(.history, params: .object(["task_id": .string(taskID)]))
            }
        case .ping:
            sendFrame(RemoteFrame(type: .pong))
        case .error:
            handleWireError(frame.error)
        case .pong, .pair, .resume, .command, .ack:
            break
        }
    }

    private func handleWireError(_ error: RemoteWireError?) {
        if error?.code == "remote_disabled" {
            lastError = error?.message ?? "Mac 已关闭远程连接。请在 Mac 重新开启后点刷新。"
        } else {
            lastError = error?.message ?? "Mac 拒绝了这次连接。"
        }
        reconnectTask?.cancel()
        closeTransport()
        if pendingInvitation != nil {
            pendingInvitation = nil
            connectionState = .notPaired
            return
        }
        switch error?.code {
        case "invalid_device_token", "resume_rejected", "device_revoked", "remote_device_revoked":
            disconnectAndForget()
        case "server_busy", "server_unavailable", "unavailable", "temporarily_unavailable":
            scheduleReconnect()
        default:
            connectionState = pairedHost == nil ? .notPaired : .offline
        }
    }

    private func completePairing(_ frame: RemoteFrame, generation: UUID) async {
        defer {
            if pairingCompletionGeneration == generation {
                pairingCompletionGeneration = nil
            }
        }
        guard generation == currentGeneration else { return }
        guard let invitation = pendingInvitation,
              let deviceID = frame.deviceID,
              let token = frame.deviceToken,
              validateBase64URL32(token),
              let epoch = frame.serverEpoch,
              let latestRevision = frame.latestRevision else {
            lastError = "Mac 返回的配对凭据无效。"
            pendingInvitation = nil
            resyncCheckpoint = nil
            closeTransport()
            connectionState = .notPaired
            return
        }
        let pairingID = invitation.pairingID
        let host = PairedHost(
            endpoint: invitation.endpoint,
            fingerprint: invitation.fingerprint,
            deviceID: deviceID,
            displayName: invitation.endpoint.host ?? "我的 Mac"
        )
        do {
            let resetCache = try await cacheStore.reset(for: host.identity)
            guard isCurrentPairing(generation: generation, pairingID: pairingID) else { return }
            try await outbox.reset(for: host.identity)
            guard isCurrentPairing(generation: generation, pairingID: pairingID) else { return }
            try await hostStore.save(host)
            guard isCurrentPairing(generation: generation, pairingID: pairingID) else { return }

            // Save the credential only after all actor-backed identity stores
            // have committed. There is no actor suspension between this write
            // and publishing the paired host, so an old generation can never
            // delete or overwrite a newer generation's token.
            try tokenStore.save(Data(token.utf8), account: tokenAccount(deviceID: deviceID))
            guard isCurrentPairing(generation: generation, pairingID: pairingID) else { return }

            cached = resetCache
            cacheWriteSequence = 0
            pairedHost = host
            storageReady = true
            pendingInvitation = nil
            cached.cursor.resync(epoch: epoch, latestRevision: latestRevision)
            connectionState = .online
            reconnectAttempt = 0
            handshakeTimeoutTask?.cancel()
            pumpOutbox()
            sendCommand(.bootstrap, params: .object([:]))
        } catch {
            guard isCurrentPairing(generation: generation, pairingID: pairingID) else { return }
            try? tokenStore.delete(account: tokenAccount(deviceID: deviceID))
            pairedHost = nil
            storageReady = false
            pendingInvitation = nil
            cached = CachedRemoteState()
            resyncCheckpoint = nil
            lastError = "无法安全保存配对数据，连接已停止。请重新生成二维码。"
            closeTransport()
            connectionState = .notPaired
        }
    }

    private func isCurrentPairing(generation: UUID, pairingID: String) -> Bool {
        generation == currentGeneration && pendingInvitation?.pairingID == pairingID
    }

    private func handleCommandResult(_ frame: RemoteFrame, generation: UUID) async {
        guard generation == currentGeneration,
              let identity = pairedHost?.identity,
              let commandID = frame.commandID else { return }
        let commands: [PendingCommand]
        do {
            commands = try await outbox.all(for: identity)
        } catch {
            guard generation == currentGeneration, pairedHost?.identity == identity else { return }
            stopForStorageFailure("无法读取当前 Mac 的本机发件箱，连接已停止。")
            return
        }
        guard generation == currentGeneration, pairedHost?.identity == identity else { return }
        guard let command = commands.first(where: { $0.id == commandID }) else { return }
        if frame.ok == true {
                let selectedBeforeResult = cached.selectedTaskID
                if let result = frame.result {
                    RemoteContentReducer.apply(result: result, action: command.action, to: &cached)
                }
                if command.action == .bootstrap { completeResyncCheckpoint() }
                if command.action == .respondUI,
                   let params = command.payload.objectValue,
                   let taskID = params["task_id"]?.stringValue,
                   let requestID = params["request_id"]?.stringValue {
                    cached.pendingUIRequestsByTask[taskID]?.removeAll { $0.id == requestID }
                }
                persistCache()
                do {
                    try await outbox.removeSucceeded(id: commandID, for: identity)
                } catch {
                    guard generation == currentGeneration, pairedHost?.identity == identity else { return }
                    stopForStorageFailure("Mac 已完成操作，但本机发件箱无法更新。连接已停止以避免重复发送。")
                    return
                }
                guard generation == currentGeneration, pairedHost?.identity == identity else { return }
                if command.action == .bootstrap,
                   let taskID = BootstrapFollowupPolicy.historyTaskID(
                       previous: selectedBeforeResult,
                       current: cached.selectedTaskID
                ) {
                    sendCommand(.history, params: .object(["task_id": .string(taskID)]))
                }
                if command.action == .prompt,
                   let taskID = command.payload.objectValue?["task_id"]?.stringValue {
                    scheduleHistoryRefresh(taskID: taskID)
                }
        } else {
                let code = frame.error?.code
                switch RemoteCommandTerminationPolicy.disposition(for: code) {
                case .preservePairingAndStop:
                    lastError = frame.error?.message ?? "Mac 已关闭远程连接。请在 Mac 重新开启后点刷新。"
                    closeTransport()
                    connectionState = pairedHost == nil ? .notPaired : .offline
                    return
                case .forgetPairing:
                    lastError = frame.error?.message ?? "这台 iPhone 的远程授权已撤销，请重新配对。"
                    disconnectAndForget()
                    return
                case .continueHandling:
                    break
                }
                let disposition = RemoteCommandErrorPolicy.disposition(for: code)
                if disposition == .retrySameCommand {
                    lastError = frame.error?.message ?? "Mac 暂时繁忙，正在重试。"
                    if let generation = currentGeneration { closed(generation: generation, error: nil) }
                    return
                }
                do {
                    try await outbox.removeSucceeded(id: commandID, for: identity)
                } catch {
                    guard generation == currentGeneration, pairedHost?.identity == identity else { return }
                    stopForStorageFailure("无法更新本机发件箱，连接已停止以避免重复操作。")
                    return
                }
                guard generation == currentGeneration, pairedHost?.identity == identity else { return }
                if disposition == .resyncWithoutRetry {
                    if command.action == .prompt,
                       let localMessageID = command.localMessageID,
                       let taskID = command.payload.objectValue?["task_id"]?.stringValue {
                        RemoteContentReducer.removeOptimisticMessage(
                            id: localMessageID,
                            taskID: taskID,
                            from: &cached
                        )
                        persistCache()
                    }
                    lastError = frame.error?.message
                        ?? "Mac 无法确认操作结果。已同步最新状态，请确认后再手动重试。"
                    sendCommand(.bootstrap, params: .object([:]))
                    sendCommand(.listSidebar, params: .object([:]))
                    if let taskID = cached.selectedTaskID {
                        sendCommand(.history, params: .object(["task_id": .string(taskID)]))
                    }
                    return
                }
                if code == "response_too_large" {
                    lastError = frame.error?.message
                        ?? "Mac 返回的内容超过 1 MiB。请先在 Mac 上缩短任务历史，再刷新移动端。"
                    if command.action == .bootstrap {
                        resyncCheckpoint = nil
                        closeTransport()
                        connectionState = .offline
                    } else if command.action == .prompt,
                              let taskID = command.payload.objectValue?["task_id"]?.stringValue {
                        sendCommand(.history, params: .object(["task_id": .string(taskID)]))
                    }
                    return
                }
                if code == "profile_unavailable" {
                    profileAvailable = false
                    lastError = frame.error?.message
                        ?? "Mac 上的原工作区已删除或不可用。请在 Mac 处理后点刷新。"
                    resyncCheckpoint = nil
                    closeTransport()
                    connectionState = .offline
                    return
                }
                if command.action == .bootstrap {
                    resyncCheckpoint = nil
                    if let generation = currentGeneration { closed(generation: generation, error: nil) }
                } else if !(command.action == .startTask && ["already_running", "task_already_running"].contains(code)) {
                    if command.action == .prompt,
                       let localMessageID = command.localMessageID,
                       let taskID = command.payload.objectValue?["task_id"]?.stringValue {
                        RemoteContentReducer.removeOptimisticMessage(
                            id: localMessageID,
                            taskID: taskID,
                            from: &cached
                        )
                        persistCache()
                    }
                    lastError = frame.error?.message ?? "Mac 无法完成这个操作。"
                }
        }
    }

    private func handleEvent(_ frame: RemoteFrame) {
        guard let epoch = frame.serverEpoch, let revision = frame.revision else { return }
        if resyncCheckpoint != nil {
            if resyncCheckpoint?.capture(frame) == true {
                resyncCheckpoint = nil
                lastError = "恢复期间事件过多，正在重新建立安全增量同步。"
                if let generation = currentGeneration { closed(generation: generation, error: nil) }
            }
            return
        }
        let decision = cached.cursor.apply(epoch: epoch, revision: revision)
        switch decision {
        case .duplicate:
            sendFrame(RemoteFrame(type: .ack, throughRevision: cached.cursor.revision))
        case .gap:
            lastError = "检测到实时事件缺口，正在安全恢复。"
            if let generation = currentGeneration { closed(generation: generation, error: nil) }
        case .accepted, .newEpoch:
            if let payload = frame.payload {
                if frame.kind == "task_output" {
                    let reduction = RemoteContentReducer.applyTaskOutput(payload, to: &cached)
                    scheduleSidebarRefresh()
                    if reduction.shouldRefreshHistory, let affectedTask = reduction.taskID {
                        scheduleHistoryRefresh(taskID: affectedTask)
                    }
                } else if frame.kind == "invalidated", let payloadObject = payload.objectValue {
                    let affectedTask = payloadObject["task_id"]?.stringValue ?? cached.selectedTaskID
                    scheduleSidebarRefresh()
                    if let affectedTask { scheduleHistoryRefresh(taskID: affectedTask) }
                }
            }
            persistThenAck(revision: cached.cursor.revision)
        }
    }

    private func beginResync(epoch: String, latestRevision: Int64) {
        RemoteContentReducer.discardLiveStreamsForAuthoritativeRecovery(&cached)
        resyncCheckpoint = ResyncCheckpointBuffer(epoch: epoch, revision: max(0, latestRevision))
    }

    private func completeResyncCheckpoint() {
        guard var checkpoint = resyncCheckpoint else { return }
        cached.cursor.resync(epoch: checkpoint.epoch, latestRevision: checkpoint.revision)
        resyncCheckpoint = nil
        let queued = checkpoint.drainInRevisionOrder()
        persistThenAck(revision: checkpoint.revision)
        queued.forEach(handleEvent)
    }

    private func persistThenAck(revision: Int64) {
        cached.updatedAt = Date()
        let snapshot = cached
        guard let identity = pairedHost?.identity else { return }
        let cacheSequence = issueCacheWriteSequence()
        let generation = currentGeneration
        Task {
            do {
                let didWrite = try await cacheStore.save(
                    snapshot,
                    for: identity,
                    sequence: cacheSequence
                )
                guard didWrite, let generation else { return }
                scheduleAck(revision: revision, generation: generation)
            } catch {
                guard pairedHost?.identity == identity else { return }
                stopForStorageFailure("无法持久化实时进度，连接已停止以避免游标回退。")
            }
        }
    }

    private func scheduleAck(revision: Int64, generation: UUID) {
        guard generation == currentGeneration else { return }
        pendingAckRevision = max(pendingAckRevision ?? 0, revision)
        guard ackPumpTask == nil else { return }
        let pumpID = UUID()
        ackPumpID = pumpID
        ackPumpTask = Task { [weak self] in
            guard let self else { return }
            defer {
                if self.ackPumpID == pumpID {
                    self.ackPumpTask = nil
                    self.ackPumpID = nil
                }
            }
            while !Task.isCancelled {
                guard generation == self.currentGeneration,
                      let activeTransport = self.transport,
                      let revision = self.pendingAckRevision else { return }
                self.pendingAckRevision = nil
                guard revision > self.highestAckSent else { continue }
                do {
                    try await activeTransport.send(RemoteFrame(type: .ack, throughRevision: revision))
                } catch {
                    guard generation == self.currentGeneration else { return }
                    self.closed(generation: generation, error: error)
                    return
                }
                guard generation == self.currentGeneration else { return }
                self.highestAckSent = revision
            }
        }
    }

    private func scheduleHistoryRefresh(taskID: String) {
        historyRefreshTasks[taskID]?.cancel()
        historyRefreshTasks[taskID] = Task { [weak self] in
            try? await Task.sleep(for: .milliseconds(100))
            guard !Task.isCancelled, let self else { return }
            self.sendCommand(.history, params: .object(["task_id": .string(taskID)]))
            self.historyRefreshTasks.removeValue(forKey: taskID)
        }
    }

    private func scheduleSidebarRefresh() {
        sidebarRefreshTask?.cancel()
        sidebarRefreshTask = Task { [weak self] in
            try? await Task.sleep(for: .milliseconds(100))
            guard !Task.isCancelled, let self else { return }
            self.sendCommand(.listSidebar, params: .object([:]))
            self.sidebarRefreshTask = nil
        }
    }

    private func cancelScheduledRefreshes() {
        historyRefreshTasks.values.forEach { $0.cancel() }
        historyRefreshTasks.removeAll()
        sidebarRefreshTask?.cancel()
        sidebarRefreshTask = nil
    }

    private func sendCommand(_ action: RemoteAction, params: JSONValue) {
        enqueueCommands([PendingCommand(action: action, payload: params)])
    }

    private func enqueueCommands(_ commands: [PendingCommand]) {
        guard let identity = pairedHost?.identity else {
            lastError = "尚未绑定可接收此操作的 Mac。"
            return
        }
        do {
            for command in commands { _ = try FrameCodec().encode(command.frame) }
        } catch {
            lastError = "操作内容过大，未加入发件箱。"
            return
        }
        Task {
            do {
                _ = try await outbox.enqueue(commands, for: identity)
                guard pairedHost?.identity == identity else { return }
                pumpOutbox()
            } catch {
                guard pairedHost?.identity == identity else { return }
                storageReady = false
                closeTransport()
                connectionState = pairedHost == nil ? .notPaired : .offline
                lastError = "无法安全写入发件箱，操作尚未发送。请检查本机存储空间后重试。"
            }
        }
    }

    /// The sole command sender. It preserves persisted order and sends each
    /// UUID at most once per connection generation. A new generation may
    /// safely replay the same UUID for server-side receipt deduplication.
    private func pumpOutbox() {
        guard connectionState == .online,
              outboxPumpTask == nil,
              let pumpIdentity = pairedHost?.identity else { return }
        let pumpID = UUID()
        outboxPumpID = pumpID
        outboxPumpTask = Task { [weak self] in
            guard let self else { return }
            defer {
                if self.outboxPumpID == pumpID {
                    self.outboxPumpTask = nil
                    self.outboxPumpID = nil
                }
            }
            while !Task.isCancelled {
                guard self.connectionState == .online,
                      let activeTransport = self.transport,
                      let generation = self.currentGeneration,
                      self.pairedHost?.identity == pumpIdentity else { return }
                let pending: [PendingCommand]
                do {
                    pending = try await self.outbox.all(for: pumpIdentity)
                } catch {
                    guard generation == self.currentGeneration,
                          self.pairedHost?.identity == pumpIdentity else { return }
                    self.stopForStorageFailure("无法读取当前 Mac 的本机发件箱，连接已停止。")
                    return
                }
                guard generation == self.currentGeneration,
                      self.pairedHost?.identity == pumpIdentity else { return }
                guard let command = pending.first(where: { !self.sentCommandIDs.contains($0.id) }) else { return }

                do {
                    _ = try FrameCodec().encode(command.frame)
                } catch {
                    do {
                        try await self.outbox.removeSucceeded(id: command.id, for: pumpIdentity)
                        guard generation == self.currentGeneration,
                              self.pairedHost?.identity == pumpIdentity else { return }
                        self.lastError = "已隔离一个损坏或过大的旧发件箱操作，其余操作将继续发送。"
                        continue
                    } catch {
                        guard generation == self.currentGeneration,
                              self.pairedHost?.identity == pumpIdentity else { return }
                        self.storageReady = false
                        self.closeTransport()
                        self.connectionState = .offline
                        self.lastError = "无法修复本机发件箱，连接已停止。请检查存储空间后重试。"
                        return
                    }
                }

                do {
                    try await activeTransport.send(command.frame)
                } catch {
                    guard generation == self.currentGeneration,
                          self.pairedHost?.identity == pumpIdentity else { return }
                    self.closed(generation: generation, error: error)
                    return
                }
                guard generation == self.currentGeneration,
                      self.pairedHost?.identity == pumpIdentity else { return }
                self.sentCommandIDs.insert(command.id)
            }
        }
    }

    private func sendFrame(_ frame: RemoteFrame) {
        guard let activeTransport = transport, let generation = currentGeneration else { return }
        Task {
            do { try await activeTransport.send(frame) }
            catch { closed(generation: generation, error: error) }
        }
    }

    private func closed(generation: UUID, error: Error?) {
        guard generation == currentGeneration else { return }
        if let transportError = error as? RemoteTransportError,
           case .certificatePinMismatch = transportError {
            let message = RemoteTransportError.certificatePinMismatch.localizedDescription
            if pairedHost != nil { disconnectAndForget() }
            else {
                pendingInvitation = nil
                closeTransport()
                connectionState = .notPaired
            }
            lastError = message
            return
        }
        if let error { lastError = error.localizedDescription }
        if case let .stop(message, offersSettings) = RemoteTransportFailurePolicy.decision(for: error) {
            lastError = message
            settingsRecoveryAvailable = offersSettings
            closeTransport()
            connectionState = pairedHost == nil ? .notPaired : .offline
            return
        }
        closeTransport()
        scheduleReconnect()
    }

    private func scheduleReconnect() {
        guard isForeground, pathIsSatisfied else {
            connectionState = pairedHost == nil ? .notPaired : .offline
            return
        }
        guard let target = pendingInvitation.map({ ($0.endpoint, $0.fingerprint) })
            ?? pairedHost.map({ ($0.endpoint, $0.fingerprint) }) else {
            connectionState = .notPaired
            return
        }
        let attempt = reconnectAttempt
        reconnectAttempt += 1
        connectionState = .reconnecting(attempt: attempt)
        reconnectTask?.cancel()
        reconnectTask = Task { [weak self] in
            guard let self else { return }
            let delay = reconnectSchedule.delay(attempt: attempt)
            try? await Task.sleep(for: .seconds(delay))
            guard !Task.isCancelled else { return }
            open(endpoint: target.0, fingerprint: target.1, state: .reconnecting(attempt: attempt))
        }
    }

    private func markOfflineAndDisconnect() {
        reconnectTask?.cancel()
        closeTransport()
        connectionState = pairedHost == nil ? .notPaired : .offline
    }

    private func closeTransport() {
        handshakeTimeoutTask?.cancel()
        handshakeTimeoutTask = nil
        outboxPumpTask?.cancel()
        outboxPumpTask = nil
        outboxPumpID = nil
        sentCommandIDs.removeAll()
        ackPumpTask?.cancel()
        ackPumpTask = nil
        ackPumpID = nil
        pendingAckRevision = nil
        highestAckSent = 0
        currentGeneration = nil
        let old = transport
        transport = nil
        old?.disconnect()
    }

    private func persistCache() {
        cached.updatedAt = Date()
        let snapshot = cached
        guard let identity = pairedHost?.identity else { return }
        let cacheSequence = issueCacheWriteSequence()
        Task {
            do {
                _ = try await cacheStore.save(snapshot, for: identity, sequence: cacheSequence)
            } catch {
                guard pairedHost?.identity == identity else { return }
                stopForStorageFailure("无法安全保存本机缓存，连接已停止。请检查存储空间后重试。")
            }
        }
    }

    private func issueCacheWriteSequence() -> UInt64 {
        cacheWriteSequence &+= 1
        return cacheWriteSequence
    }

    private func stopForStorageFailure(_ message: String) {
        storageReady = false
        closeTransport()
        connectionState = pairedHost == nil ? .notPaired : .offline
        lastError = message
    }

    private func tokenAccount(deviceID: String) -> String { "device:\(deviceID)" }

    private func validateBase64URL32(_ value: String) -> Bool {
        let allowed = CharacterSet(charactersIn: "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_")
        guard value.count == 43,
              !value.contains("="),
              value.unicodeScalars.allSatisfy(allowed.contains) else { return false }
        let normalized = value.replacingOccurrences(of: "-", with: "+")
            .replacingOccurrences(of: "_", with: "/")
        let padding = String(repeating: "=", count: (4 - normalized.count % 4) % 4)
        return Data(base64Encoded: normalized + padding)?.count == 32
    }
}

enum RemoteCommandFailureDisposition: Equatable {
    case retrySameCommand
    case resyncWithoutRetry
    case discard
}

enum RemoteCommandTerminationDisposition: Equatable {
    case preservePairingAndStop
    case forgetPairing
    case continueHandling
}

enum RemoteCommandTerminationPolicy {
    static func disposition(for code: String?) -> RemoteCommandTerminationDisposition {
        switch code {
        case "remote_disabled":
            return .preservePairingAndStop
        case "device_revoked", "remote_device_revoked", "resume_rejected":
            return .forgetPairing
        default:
            return .continueHandling
        }
    }
}

enum RemoteCommandErrorPolicy {
    private static let transientCodes: Set<String> = [
        "server_busy",
        "server_unavailable",
        "command_timeout",
        // Kept for forward/backward-compatible servers; the Rust v1 names are above.
        "unavailable",
        "temporarily_unavailable",
        "timeout",
    ]

    static func shouldRetainOutbox(code: String?) -> Bool {
        disposition(for: code) == .retrySameCommand
    }

    static func disposition(for code: String?) -> RemoteCommandFailureDisposition {
        guard let code else { return .discard }
        if transientCodes.contains(code) { return .retrySameCommand }
        if code == "command_outcome_unknown" { return .resyncWithoutRetry }
        return .discard
    }
}

enum RemoteConnectionLifecyclePolicy {
    static func shouldOpen(state: RemoteConnectionState, hasTransport: Bool) -> Bool {
        guard hasTransport else { return true }
        switch state {
        case .online, .connecting, .pairing, .reconnecting:
            return false
        case .notPaired, .offline, .suspended:
            return true
        }
    }
}

enum BootstrapFollowupPolicy {
    static func historyTaskID(previous: String?, current: String?) -> String? {
        // Bootstrap is authoritative for the sidebar, not necessarily the
        // transcript. Always follow it with the selected task's history.
        current
    }
}

enum RemoteStorageReadinessPolicy {
    static func canOpen(
        hasPendingInvitation: Bool,
        hasPairedHost: Bool,
        storageReady: Bool
    ) -> Bool {
        hasPendingInvitation || (hasPairedHost && storageReady)
    }
}
