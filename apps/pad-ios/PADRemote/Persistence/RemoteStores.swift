import Foundation

enum RemoteRole: String, Codable, Sendable {
    case user
    case assistant
    case system
}

struct RemoteMessage: Codable, Identifiable, Equatable, Sendable {
    let id: String
    let role: RemoteRole
    var text: String
    let createdAt: Date
    var isStreaming: Bool
}

struct RemoteLiveStreamState: Codable, Equatable, Sendable {
    let messageID: String
    var textBlocks: [Int: String]
    let startedAt: Date
}

enum RemoteTaskStatus: String, Codable, Sendable {
    case idle
    case running
    case attention
    case failed
    case completed

    var localizedTitle: String {
        switch self {
        case .idle: return "空闲"
        case .running: return "运行中"
        case .attention: return "需要处理"
        case .failed: return "失败"
        case .completed: return "已完成"
        }
    }
}

struct RemoteTaskSummary: Codable, Identifiable, Hashable, Sendable {
    let id: String
    var title: String
    var subtitle: String?
    var status: RemoteTaskStatus
    var updatedAt: Date
}

enum RemoteUIRequestKind: String, Codable, Sendable {
    case confirm
    case select
    case input
    case editor
    case unknown
}

struct RemoteUIRequest: Codable, Identifiable, Equatable, Sendable {
    let id: String
    let kind: RemoteUIRequestKind
    let title: String?
    let message: String?
    let options: [String]
    let defaultIndex: Int?
    let defaultValue: String?
    let requiresResponse: Bool
    let placeholder: String?

    init(
        id: String,
        kind: RemoteUIRequestKind,
        title: String?,
        message: String?,
        options: [String],
        defaultIndex: Int?,
        defaultValue: String?,
        requiresResponse: Bool,
        placeholder: String? = nil
    ) {
        self.id = id
        self.kind = kind
        self.title = title
        self.message = message
        self.options = options
        self.defaultIndex = defaultIndex
        self.defaultValue = defaultValue
        self.requiresResponse = requiresResponse
        self.placeholder = placeholder
    }
}

struct CachedRemoteState: Codable, Equatable, Sendable {
    var cursor = RevisionCursor()
    var tasks: [RemoteTaskSummary] = []
    var selectedTaskID: String?
    var messagesByTask: [String: [RemoteMessage]] = [:]
    var liveStreamsByTask: [String: RemoteLiveStreamState] = [:]
    var pendingUIRequestsByTask: [String: [RemoteUIRequest]] = [:]
    var pendingLocalMessageIDsByTask: [String: [String]] = [:]
    var updatedAt = Date.distantPast

    enum CodingKeys: String, CodingKey {
        case cursor, tasks, selectedTaskID, messagesByTask, liveStreamsByTask, pendingUIRequestsByTask
        case pendingLocalMessageIDsByTask, updatedAt
    }

    init(
        cursor: RevisionCursor = RevisionCursor(),
        tasks: [RemoteTaskSummary] = [],
        selectedTaskID: String? = nil,
        messagesByTask: [String: [RemoteMessage]] = [:],
        liveStreamsByTask: [String: RemoteLiveStreamState] = [:],
        pendingUIRequestsByTask: [String: [RemoteUIRequest]] = [:],
        pendingLocalMessageIDsByTask: [String: [String]] = [:],
        updatedAt: Date = .distantPast
    ) {
        self.cursor = cursor
        self.tasks = tasks
        self.selectedTaskID = selectedTaskID
        self.messagesByTask = messagesByTask
        self.liveStreamsByTask = liveStreamsByTask
        self.pendingUIRequestsByTask = pendingUIRequestsByTask
        self.pendingLocalMessageIDsByTask = pendingLocalMessageIDsByTask
        self.updatedAt = updatedAt
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        cursor = try container.decodeIfPresent(RevisionCursor.self, forKey: .cursor) ?? RevisionCursor()
        tasks = try container.decodeIfPresent([RemoteTaskSummary].self, forKey: .tasks) ?? []
        selectedTaskID = try container.decodeIfPresent(String.self, forKey: .selectedTaskID)
        messagesByTask = try container.decodeIfPresent([String: [RemoteMessage]].self, forKey: .messagesByTask) ?? [:]
        liveStreamsByTask = try container.decodeIfPresent([String: RemoteLiveStreamState].self, forKey: .liveStreamsByTask) ?? [:]
        pendingUIRequestsByTask = try container.decodeIfPresent([String: [RemoteUIRequest]].self, forKey: .pendingUIRequestsByTask) ?? [:]
        pendingLocalMessageIDsByTask = try container.decodeIfPresent(
            [String: [String]].self,
            forKey: .pendingLocalMessageIDsByTask
        ) ?? [:]
        updatedAt = try container.decodeIfPresent(Date.self, forKey: .updatedAt) ?? .distantPast
    }
}

enum StorePaths {
    static func applicationSupport() throws -> URL {
        let base = try FileManager.default.url(
            for: .applicationSupportDirectory,
            in: .userDomainMask,
            appropriateFor: nil,
            create: true
        )
        let directory = base.appendingPathComponent("PADRemote", isDirectory: true)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        return directory
    }
}

actor HostMetadataStore {
    private let fileURL: URL

    init(fileURL: URL? = nil) {
        self.fileURL = fileURL ?? (try? StorePaths.applicationSupport().appendingPathComponent("host.json"))
            ?? FileManager.default.temporaryDirectory.appendingPathComponent("pad-remote-host.json")
    }

    func load() -> PairedHost? {
        guard let data = try? Data(contentsOf: fileURL) else { return nil }
        return try? JSONDecoder().decode(PairedHost.self, from: data)
    }

    func save(_ host: PairedHost) throws {
        try JSONEncoder().encode(host).write(to: fileURL, options: [.atomic, .completeFileProtectionUntilFirstUserAuthentication])
    }

    func remove(matching identity: RemoteHostIdentity) throws {
        guard load()?.identity == identity else { return }
        guard FileManager.default.fileExists(atPath: fileURL.path) else { return }
        try FileManager.default.removeItem(at: fileURL)
    }
}

enum BoundStoreError: LocalizedError {
    case notActivated
    case staleIdentity

    var errorDescription: String? {
        switch self {
        case .notActivated: return "远程存储尚未绑定到已配对的 Mac。"
        case .staleIdentity: return "已丢弃来自旧配对的迟到写入。"
        }
    }
}

enum OutboxCapacityError: LocalizedError, Equatable {
    case tooManyCommands
    case storageBudgetExceeded

    var errorDescription: String? {
        switch self {
        case .tooManyCommands: return "发件箱已达到 256 条上限，请先恢复与 Mac 的连接。"
        case .storageBudgetExceeded: return "发件箱已达到 8 MiB 上限，请先恢复与 Mac 的连接。"
        }
    }
}

private struct CacheEnvelope: Codable {
    let version: Int
    let identity: RemoteHostIdentity
    let state: CachedRemoteState
}

enum CacheCapacityError: LocalizedError {
    case storageBudgetExceeded

    var errorDescription: String? {
        "本机缓存超过 8 MiB 安全上限。"
    }
}

enum CachedStateBudget {
    static let maximumTasks = 250
    static let maximumMessagesPerTask = 300
    static let maximumTextCharacters = 1_000_000
    static let maximumEnvelopeBytes = 8 * 1_024 * 1_024

    static func trim(_ input: CachedRemoteState) -> CachedRemoteState {
        var state = input
        var keptTasks = Array(state.tasks.prefix(maximumTasks))
        if let selected = state.selectedTaskID,
           !keptTasks.contains(where: { $0.id == selected }),
           let selectedTask = state.tasks.first(where: { $0.id == selected }) {
            if keptTasks.count == maximumTasks { keptTasks.removeLast() }
            keptTasks.insert(selectedTask, at: 0)
        }
        let validTaskIDs = Set(keptTasks.map(\.id))
        state.tasks = keptTasks
        if let selected = state.selectedTaskID, !validTaskIDs.contains(selected) {
            state.selectedTaskID = keptTasks.first?.id
        }
        state.messagesByTask = state.messagesByTask.filter { validTaskIDs.contains($0.key) }
        state.pendingUIRequestsByTask = state.pendingUIRequestsByTask.filter { validTaskIDs.contains($0.key) }
        state.pendingLocalMessageIDsByTask = state.pendingLocalMessageIDsByTask.filter {
            validTaskIDs.contains($0.key)
        }

        // A persisted partial stream is displayed as a settled cached message;
        // resume/history will authoritatively reconcile it in the foreground.
        state.liveStreamsByTask.removeAll()
        for taskID in Array(state.messagesByTask.keys) {
            state.messagesByTask[taskID] = Array(
                (state.messagesByTask[taskID] ?? []).suffix(maximumMessagesPerTask)
            )
        }

        var remainingCharacters = maximumTextCharacters
        var orderedTaskIDs: [String] = []
        if let selected = state.selectedTaskID, validTaskIDs.contains(selected) {
            orderedTaskIDs.append(selected)
        }
        orderedTaskIDs.append(contentsOf: keptTasks.map(\.id).filter { !orderedTaskIDs.contains($0) })
        var remainingInteractionCharacters = 250_000
        var remainingInteractionCount = 500
        for taskID in orderedTaskIDs {
            var keptRequests: [RemoteUIRequest] = []
            for request in (state.pendingUIRequestsByTask[taskID] ?? []).prefix(50) {
                guard remainingInteractionCount > 0 else { break }
                let cost = request.id.count
                    + (request.title?.count ?? 0)
                    + (request.message?.count ?? 0)
                    + request.options.reduce(0) { $0 + $1.count }
                    + (request.defaultValue?.count ?? 0)
                    + (request.placeholder?.count ?? 0)
                guard cost <= remainingInteractionCharacters else { continue }
                remainingInteractionCharacters -= cost
                remainingInteractionCount -= 1
                keptRequests.append(request)
            }
            state.pendingUIRequestsByTask[taskID] = keptRequests
        }
        for taskID in orderedTaskIDs {
            let messages = state.messagesByTask[taskID] ?? []
            var kept: [RemoteMessage] = []
            for var message in messages.reversed() where remainingCharacters > 0 {
                if message.text.count > remainingCharacters {
                    message.text = String(message.text.prefix(remainingCharacters))
                }
                message.isStreaming = false
                remainingCharacters -= message.text.count
                kept.append(message)
            }
            kept.reverse()
            state.messagesByTask[taskID] = kept
            let keptIDs = Set(kept.map(\.id))
            state.pendingLocalMessageIDsByTask[taskID]?.removeAll { !keptIDs.contains($0) }
            if state.pendingLocalMessageIDsByTask[taskID]?.isEmpty == true {
                state.pendingLocalMessageIDsByTask.removeValue(forKey: taskID)
            }
        }
        return state
    }
}

actor CacheStore {
    private let fileURL: URL
    private var activeIdentity: RemoteHostIdentity?
    private var latestWriteSequence: UInt64 = 0

    init(fileURL: URL? = nil) {
        self.fileURL = fileURL ?? (try? StorePaths.applicationSupport().appendingPathComponent("cache.json"))
            ?? FileManager.default.temporaryDirectory.appendingPathComponent("pad-remote-cache.json")
    }

    func activate(for identity: RemoteHostIdentity) throws -> CachedRemoteState {
        if let envelope = loadEnvelope(), envelope.identity == identity {
            let trimmed = CachedStateBudget.trim(envelope.state)
            let previousIdentity = activeIdentity
            let previousSequence = latestWriteSequence
            activeIdentity = identity
            do {
                if trimmed != envelope.state { try write(trimmed, identity: identity) }
                latestWriteSequence = 0
                return trimmed
            } catch {
                activeIdentity = previousIdentity
                latestWriteSequence = previousSequence
                throw error
            }
        }
        let empty = CachedRemoteState()
        let previousIdentity = activeIdentity
        let previousSequence = latestWriteSequence
        activeIdentity = identity
        do {
            try write(empty, identity: identity)
            latestWriteSequence = 0
            return empty
        } catch {
            activeIdentity = previousIdentity
            latestWriteSequence = previousSequence
            throw error
        }
    }

    func reset(for identity: RemoteHostIdentity) throws -> CachedRemoteState {
        let empty = CachedRemoteState()
        let previousIdentity = activeIdentity
        let previousSequence = latestWriteSequence
        activeIdentity = identity
        do {
            try write(empty, identity: identity)
            latestWriteSequence = 0
            return empty
        } catch {
            activeIdentity = previousIdentity
            latestWriteSequence = previousSequence
            throw error
        }
    }

    @discardableResult
    func save(
        _ state: CachedRemoteState,
        for identity: RemoteHostIdentity,
        sequence: UInt64? = nil
    ) throws -> Bool {
        guard activeIdentity == identity else { throw BoundStoreError.staleIdentity }
        let resolvedSequence = sequence ?? (latestWriteSequence &+ 1)
        guard resolvedSequence > latestWriteSequence else { return false }
        try write(state, identity: identity)
        latestWriteSequence = resolvedSequence
        return true
    }

    func remove(matching identity: RemoteHostIdentity) throws {
        if let activeIdentity, activeIdentity != identity { return }
        if activeIdentity == nil, loadEnvelope()?.identity != identity { return }
        if FileManager.default.fileExists(atPath: fileURL.path) {
            try FileManager.default.removeItem(at: fileURL)
        }
        activeIdentity = nil
        latestWriteSequence = 0
    }

    private func loadEnvelope() -> CacheEnvelope? {
        guard let data = try? Data(contentsOf: fileURL) else { return nil }
        return try? JSONDecoder().decode(CacheEnvelope.self, from: data)
    }

    private func write(_ state: CachedRemoteState, identity: RemoteHostIdentity) throws {
        let envelope = CacheEnvelope(version: 1, identity: identity, state: CachedStateBudget.trim(state))
        let data = try JSONEncoder().encode(envelope)
        guard data.count <= CachedStateBudget.maximumEnvelopeBytes else {
            throw CacheCapacityError.storageBudgetExceeded
        }
        try data.write(
            to: fileURL,
            options: [.atomic, .completeFileProtectionUntilFirstUserAuthentication]
        )
    }
}

private struct OutboxEnvelope: Codable {
    let version: Int
    let identity: RemoteHostIdentity
    let commands: [PendingCommand]
}

actor OutboxStore {
    typealias AtomicWriter = @Sendable (Data, URL) throws -> Void

    private let fileURL: URL
    private let writer: AtomicWriter
    private var activeIdentity: RemoteHostIdentity?
    private var commands: [PendingCommand] = []
    private static let maximumCommandCount = 256
    private static let maximumEnvelopeBytes = 8 * 1_024 * 1_024

    init(fileURL: URL? = nil, writer: AtomicWriter? = nil) {
        let resolved = fileURL ?? (try? StorePaths.applicationSupport().appendingPathComponent("outbox.json"))
            ?? FileManager.default.temporaryDirectory.appendingPathComponent("pad-remote-outbox.json")
        self.fileURL = resolved
        self.writer = writer ?? { data, url in
            try data.write(to: url, options: [.atomic, .completeFileProtectionUntilFirstUserAuthentication])
        }
    }

    func activate(for identity: RemoteHostIdentity) throws {
        let previousIdentity = activeIdentity
        let previousCommands = commands
        activeIdentity = identity
        if let envelope = loadEnvelope(), envelope.identity == identity {
            let valid = envelope.commands.filter { (try? FrameCodec().encode($0.frame)) != nil }
            do {
                try validateBudget(valid)
                if valid.count != envelope.commands.count { try persist(valid) }
                commands = valid
                return
            } catch {
                activeIdentity = previousIdentity
                commands = previousCommands
                throw error
            }
        }
        do {
            try persist([])
            commands = []
        } catch {
            activeIdentity = previousIdentity
            commands = previousCommands
            throw error
        }
    }

    func reset(for identity: RemoteHostIdentity) throws {
        let previousIdentity = activeIdentity
        let previousCommands = commands
        activeIdentity = identity
        do {
            try persist([])
            commands = []
        } catch {
            activeIdentity = previousIdentity
            commands = previousCommands
            throw error
        }
    }

    @discardableResult
    func enqueue(_ command: PendingCommand, for identity: RemoteHostIdentity) throws -> PendingCommand {
        try enqueue([command], for: identity).first ?? command
    }

    /// Persists a command transaction in one atomic envelope write. Prompt
    /// transactions use this so start_task and prompt cannot be torn apart.
    @discardableResult
    func enqueue(_ incoming: [PendingCommand], for identity: RemoteHostIdentity) throws -> [PendingCommand] {
        guard activeIdentity != nil else { throw BoundStoreError.notActivated }
        guard activeIdentity == identity else { throw BoundStoreError.staleIdentity }
        for command in incoming { _ = try FrameCodec().encode(command.frame) }
        var next = commands
        for command in incoming where !next.contains(where: { $0.id == command.id }) {
            next.append(command)
        }
        guard next != commands else { return incoming }
        try validateBudget(next)
        try persist(next)
        commands = next
        return incoming
    }

    func all(for identity: RemoteHostIdentity) throws -> [PendingCommand] {
        guard activeIdentity != nil else { throw BoundStoreError.notActivated }
        guard activeIdentity == identity else { throw BoundStoreError.staleIdentity }
        return commands
    }

    func removeSucceeded(id: UUID, for identity: RemoteHostIdentity) throws {
        guard activeIdentity != nil else { throw BoundStoreError.notActivated }
        guard activeIdentity == identity else { throw BoundStoreError.staleIdentity }
        let next = commands.filter { $0.id != id }
        guard next.count != commands.count else { return }
        try persist(next)
        commands = next
    }

    func removeAll(matching identity: RemoteHostIdentity) throws {
        if let activeIdentity, activeIdentity != identity { return }
        if activeIdentity == nil, loadEnvelope()?.identity != identity { return }
        if FileManager.default.fileExists(atPath: fileURL.path) {
            try FileManager.default.removeItem(at: fileURL)
        }
        activeIdentity = nil
        commands = []
    }

    private func persist(_ commands: [PendingCommand]) throws {
        guard let activeIdentity else { throw BoundStoreError.notActivated }
        let envelope = OutboxEnvelope(version: 1, identity: activeIdentity, commands: commands)
        let data = try JSONEncoder().encode(envelope)
        guard data.count <= Self.maximumEnvelopeBytes else { throw OutboxCapacityError.storageBudgetExceeded }
        try writer(data, fileURL)
    }

    private func validateBudget(_ commands: [PendingCommand]) throws {
        guard commands.count <= Self.maximumCommandCount else { throw OutboxCapacityError.tooManyCommands }
    }

    private func loadEnvelope() -> OutboxEnvelope? {
        guard let data = try? Data(contentsOf: fileURL) else { return nil }
        return try? JSONDecoder().decode(OutboxEnvelope.self, from: data)
    }
}
