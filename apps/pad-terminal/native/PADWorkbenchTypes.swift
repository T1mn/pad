#if os(macOS)
import Foundation

struct PADWorkspace: Codable, Identifiable, Equatable {
    let id: String
    let name: String
    let path: String
}

struct PADProfile: Codable, Identifiable, Equatable {
    let id: String
    let name: String
}

struct PADTask: Codable, Identifiable, Equatable {
    let id: String
    let workspaceId: String
    let profileId: String
    var title: String
    var status: String
    var provider: String?
    var modelId: String?
    var sessionFile: String?
    var updatedAt: String
}

struct PADSnapshot: Codable {
    var workspaces: [PADWorkspace] = []
    var profiles: [PADProfile] = []
    var tasks: [PADTask] = []
}

struct PADProvider: Codable, Identifiable {
    let id: String
    let name: String
    let authTypes: [String]
    let authenticated: Bool
}

struct PADModelInfo: Codable, Hashable {
    let provider: String
    let id: String
    let name: String
    var source: String? = nil
    var selectable: Bool? = nil
    var isSelectable: Bool { selectable ?? true }
    var selectionKey: String { "\(provider)/\(id)" }
}

struct PADOpenAIDiscovery: Codable {
    let state: String
    var updatedAt: String? = nil
    var message: String? = nil
}

struct PADCatalog: Codable {
    var profileId: String = ""
    var providers: [PADProvider] = []
    var models: [PADModelInfo] = []
    var openaiDiscovery: PADOpenAIDiscovery? = nil
}

struct PADAuthOption: Codable, Identifiable {
    let id: String
    let label: String
}

struct PADAuthState: Codable {
    let profileId: String
    let provider: String
    let method: String
    let phase: String
    var message: String?
    var promptId: String?
    var promptKind: String?
    var placeholder: String?
    var options: [PADAuthOption]?
    var url: String?
    var userCode: String?
    var attemptId: String? = nil
}

struct PADChatItem: Identifiable {
    let id: String
    var role: String
    var text: String
    var toolName: String?
    var status: String?
    var isError: Bool = false
}
#endif
