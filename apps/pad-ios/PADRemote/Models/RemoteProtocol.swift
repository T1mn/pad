import Foundation

enum JSONValue: Codable, Equatable, Sendable {
    case string(String)
    case number(Double)
    case bool(Bool)
    case object([String: JSONValue])
    case array([JSONValue])
    case null

    init(from decoder: Decoder) throws {
        let container = try decoder.singleValueContainer()
        if container.decodeNil() { self = .null }
        else if let value = try? container.decode(Bool.self) { self = .bool(value) }
        else if let value = try? container.decode(Double.self) { self = .number(value) }
        else if let value = try? container.decode(String.self) { self = .string(value) }
        else if let value = try? container.decode([String: JSONValue].self) { self = .object(value) }
        else { self = .array(try container.decode([JSONValue].self)) }
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

    var objectValue: [String: JSONValue]? {
        guard case let .object(value) = self else { return nil }
        return value
    }

    var stringValue: String? {
        guard case let .string(value) = self else { return nil }
        return value
    }
}

enum FrameType: String, Codable, Sendable {
    case pair
    case paired
    case resume
    case welcome
    case command
    case commandResult = "command_result"
    case event
    case ack
    case ping
    case pong
    case resyncRequired = "resync_required"
    case error
}

enum RemoteAction: String, Codable, CaseIterable, Sendable {
    case bootstrap
    case listSidebar = "list_sidebar"
    case history
    case createTask = "create_task"
    case startTask = "start_task"
    case prompt
    case abort
    case stop
    case stopTask = "stop_task"
    case retryTask = "retry_task"
    case respondUI = "respond_ui"
    case setTask = "set_task"
    case runtimeSnapshot = "runtime_snapshot"
}

struct RemoteFrame: Codable, Equatable, Sendable {
    var type: FrameType
    var device: DeviceDescriptor?
    var deviceID: String?
    var pairingID: String?
    var secret: String?
    var deviceToken: String?
    var serverEpoch: String?
    var revision: Int64?
    var latestRevision: Int64?
    var profileAvailable: Bool?
    var afterRevision: Int64?
    var throughRevision: Int64?
    var kind: String?
    var commandID: UUID?
    var action: RemoteAction?
    var params: JSONValue?
    var payload: JSONValue?
    var result: JSONValue?
    var ok: Bool?
    var error: RemoteWireError?

    enum CodingKeys: String, CodingKey {
        case type
        case device
        case deviceID = "device_id"
        case pairingID = "pairing_id"
        case secret
        case deviceToken = "device_token"
        case serverEpoch = "server_epoch"
        case revision
        case latestRevision = "latest_revision"
        case profileAvailable = "profile_available"
        case afterRevision = "after_revision"
        case throughRevision = "through_revision"
        case kind
        case commandID = "command_id"
        case action
        case params
        case payload
        case result
        case ok
        case error
    }
}

struct RemoteWireError: Codable, Equatable, Sendable {
    let code: String?
    let message: String

    enum CodingKeys: String, CodingKey { case code, message }

    init(code: String? = nil, message: String) {
        self.code = code
        self.message = message
    }

    init(from decoder: Decoder) throws {
        let single = try decoder.singleValueContainer()
        if let value = try? single.decode(String.self) {
            code = nil
            message = value
            return
        }
        let object = try decoder.container(keyedBy: CodingKeys.self)
        code = try object.decodeIfPresent(String.self, forKey: .code)
        message = try object.decodeIfPresent(String.self, forKey: .message) ?? code ?? "Mac 无法完成这个操作。"
    }

    func encode(to encoder: Encoder) throws {
        var object = encoder.container(keyedBy: CodingKeys.self)
        try object.encodeIfPresent(code, forKey: .code)
        try object.encode(message, forKey: .message)
    }
}

struct DeviceDescriptor: Codable, Equatable, Sendable {
    let displayName: String
    let platform: String

    enum CodingKeys: String, CodingKey {
        case displayName = "display_name"
        case platform
    }
}

enum FrameCodecError: LocalizedError, Equatable {
    case tooLarge

    var errorDescription: String? { "远程消息超过 1 MiB 安全上限。" }
}

struct FrameCodec {
    static let maximumFrameBytes = RemoteProtocolConstants.maximumFrameBytes

    private let encoder: JSONEncoder
    private let decoder: JSONDecoder

    init() {
        encoder = JSONEncoder()
        decoder = JSONDecoder()
    }

    func encode(_ frame: RemoteFrame) throws -> Data {
        let data = try encoder.encode(frame)
        guard data.count <= Self.maximumFrameBytes else { throw FrameCodecError.tooLarge }
        return data
    }

    func decode(_ data: Data) throws -> RemoteFrame {
        guard data.count <= Self.maximumFrameBytes else { throw FrameCodecError.tooLarge }
        return try decoder.decode(RemoteFrame.self, from: data)
    }
}

struct PendingCommand: Codable, Identifiable, Equatable, Sendable {
    let id: UUID
    let action: RemoteAction
    let payload: JSONValue
    let createdAt: Date
    /// Links a durable prompt command to its cached optimistic bubble. This is
    /// local-only metadata and is never included in the wire frame.
    let localMessageID: String?

    init(
        id: UUID = UUID(),
        action: RemoteAction,
        payload: JSONValue,
        createdAt: Date = Date(),
        localMessageID: String? = nil
    ) {
        self.id = id
        self.action = action
        self.payload = payload
        self.createdAt = createdAt
        self.localMessageID = localMessageID
    }

    var frame: RemoteFrame {
        RemoteFrame(type: .command, commandID: id, action: action, params: payload)
    }
}

extension PendingCommand {
    static func createTask(taskID: String = UUID().uuidString.lowercased()) -> PendingCommand {
        PendingCommand(action: .createTask, payload: .object(["task_id": .string(taskID)]))
    }

    static func promptTransaction(
        taskID: String,
        prompt: String,
        localMessageID: String? = nil
    ) -> [PendingCommand] {
        [
            PendingCommand(action: .startTask, payload: .object(["task_id": .string(taskID)])),
            PendingCommand(
                action: .prompt,
                payload: .object(["task_id": .string(taskID), "prompt": .string(prompt)]),
                localMessageID: localMessageID
            ),
        ]
    }

    static func respondUI(taskID: String, request: RemoteUIRequest, value: JSONValue) -> PendingCommand {
        PendingCommand(action: .respondUI, payload: .object([
            "task_id": .string(taskID),
            "request_id": .string(request.id),
            "response_kind": .string(request.kind.rawValue),
            "cancelled": .bool(false),
            "value": value,
        ]))
    }

    static func cancelUI(taskID: String, request: RemoteUIRequest) -> PendingCommand {
        PendingCommand(action: .respondUI, payload: .object([
            "task_id": .string(taskID),
            "request_id": .string(request.id),
            "response_kind": .string(request.kind.rawValue),
            "cancelled": .bool(true),
        ]))
    }
}
