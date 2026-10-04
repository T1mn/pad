#if os(macOS)
import Foundation
import Combine

struct PADLocalSession: Codable, Identifiable, Equatable {
    let id: String
    let tool: String
    let sessionId: String
    let title: String
    let cwd: String
    let file: String
    let updatedAt: String
    var toolName: String { tool == "claude" ? "Claude Code" : tool.capitalized }
}
struct PADLocalRoot: Codable {
    let tool: String
    let path: String
    let status: String
}
struct PADLocalSessionList: Codable {
    let sessions: [PADLocalSession]
    let roots: [PADLocalRoot]
    let truncated: Bool
    let scannedAt: String
}
struct PADLocalMessage: Codable, Identifiable {
    let id: String
    let role: String
    let text: String
}
struct PADLocalHistory: Codable {
    let messages: [PADLocalMessage]
    let truncated: Bool
}
struct PADLocalImport: Codable {
    let task: PADTask
    let snapshot: PADSnapshot
}
struct PADTerminalLaunch: Codable {
    let cwd: String
    let executable: String
    let args: [String]

    /// Fixed host-generated argv, quoted as individual shell words. Never inject
    /// a source title/message or send text to an already running shell.
    var shellCommand: String {
        ([executable] + args).map { "'" + $0.replacingOccurrences(of: "'", with: "'\"'\"'") + "'" }.joined(separator: " ")
    }
}

@MainActor
final class PADLocalSessionsModel: ObservableObject {
    @Published private(set) var sessions: [PADLocalSession] = []
    @Published private(set) var roots: [PADLocalRoot] = []
    @Published private(set) var selectedId: String?
    @Published private(set) var messages: [PADLocalMessage] = []
    @Published private(set) var refreshing = false
    @Published private(set) var loadingHistory = false
    @Published private(set) var acting = false
    @Published private(set) var listTruncated = false
    @Published private(set) var historyTruncated = false
    @Published private(set) var lastSync: Date?
    @Published var error: String?
    private var generation = 0
    private var alive = true
    private let workbench: PADWorkbenchModel
    var selected: PADLocalSession? { sessions.first { $0.id == selectedId } }

    init(workbench: PADWorkbenchModel) { self.workbench = workbench }

    func refresh() {
        guard alive, !refreshing else { return }
        refreshing = true
        let previous = selected
        workbench.localSessionRequest("local_sessions") { [weak self] (result: Result<PADLocalSessionList, Error>) in
            guard let self, self.alive else { return }
            self.refreshing = false
            switch result {
            case .success(let list):
                self.sessions = list.sessions; self.roots = list.roots
                self.listTruncated = list.truncated; self.lastSync = Date(); self.error = nil
                if let id = self.selectedId {
                    if let current = self.selected {
                        if current != previous { self.select(id) }
                    } else { self.clearSelection() }
                }
            case .failure(let error): self.error = error.localizedDescription
            }
        }
    }

    func select(_ id: String) {
        generation += 1
        let token = generation
        selectedId = id; messages = []; loadingHistory = true; error = nil
        workbench.localSessionRequest("local_session_history", fields: ["sessionId": .string(id)]) { [weak self] (result: Result<PADLocalHistory, Error>) in
            guard let self, self.alive, self.generation == token else { return }
            self.loadingHistory = false
            switch result {
            case .success(let history): self.messages = history.messages; self.historyTruncated = history.truncated
            case .failure(let error): self.error = error.localizedDescription
            }
        }
    }

    func clearSelection() {
        generation += 1; selectedId = nil; messages = []; loadingHistory = false; historyTruncated = false
    }

    func resume(_ session: PADLocalSession, open: @escaping (PADTerminalLaunch) -> Void) {
        guard !acting else { return }; acting = true
        workbench.localSessionRequest("local_session_resume", fields: ["sessionId": .string(session.id)]) { [weak self] (result: Result<PADTerminalLaunch, Error>) in
            guard let self, self.alive else { return }; self.acting = false
            switch result {
            case .success(let launch): open(launch)
            case .failure(let error): self.error = error.localizedDescription
            }
        }
    }

    func importPi(_ session: PADLocalSession, done: @escaping () -> Void) {
        guard !acting, let profile = workbench.defaultProfileId else { return }; acting = true
        workbench.localSessionRequest("local_session_import_pi", fields: ["sessionId": .string(session.id), "profileId": .string(profile)]) { [weak self] (result: Result<PADLocalImport, Error>) in
            guard let self, self.alive else { return }; self.acting = false
            switch result {
            case .success(let imported): self.workbench.acceptLocalImport(imported); done()
            case .failure(let error): self.error = error.localizedDescription
            }
        }
    }

    func close() { alive = false; generation += 1 }
}
#endif
