import AppKit
import Foundation
import SwiftUI

// MARK: - Domain model

enum SidebarFilter: String, CaseIterable, Identifiable {
    case all = "全部任务"
    case pinned = "已置顶"
    case archive = "归档"

    var id: String { rawValue }

    var symbol: String {
        switch self {
        case .all: return "square.stack.3d.up"
        case .pinned: return "pin"
        case .archive: return "archivebox"
        }
    }
}

enum TaskState: String {
    case idle
    case running
    case waiting
    case failed

    var symbol: String {
        switch self {
        case .idle: return "circle"
        case .running: return "circle.fill"
        case .waiting: return "exclamationmark.circle.fill"
        case .failed: return "xmark.circle.fill"
        }
    }

    var color: Color {
        switch self {
        case .idle: return .secondary
        case .running: return .green
        case .waiting: return .orange
        case .failed: return .red
        }
    }
}

struct Profile: Identifiable, Hashable {
    let id: String
    var name: String
    var subtitle: String
    var accent: Color
    var agentDirectory: String? = nil
    var sessionDirectory: String? = nil
}

struct Project: Identifiable, Hashable {
    let id: String
    var name: String
    var path: String
    var profileID: String
}

struct DesktopTask: Identifiable, Hashable {
    let id: String
    var title: String
    var projectID: String?
    var profileID: String
    var state: TaskState
    var isPinned: Bool
    var isArchived: Bool
    var hasUnread: Bool
    var messages: [Message]
}

struct Message: Identifiable, Hashable {
    enum Role: String {
        case user
        case assistant
        case system
    }

    let id: UUID
    let role: Role
    let text: String
    let timestamp: Date

    init(id: UUID = UUID(), role: Role, text: String, timestamp: Date = Date()) {
        self.id = id
        self.role = role
        self.text = text
        self.timestamp = timestamp
    }
}

// MARK: - PAD desktop-server bridge

/// Native JSONL client for PAD's control-plane host.  The macOS renderer never
/// opens SQLite or spawns Pi directly: the bundled `pad __internal
/// desktop-server` process owns persistence, profile isolation and Pi RPC.
@MainActor
final class PiRPCClient: NSObject, ObservableObject {
    enum ConnectionState: Equatable {
        case ready
        case connecting
        case connected
        case unavailable(String)
        case failed(String)

        var label: String {
            switch self {
            case .ready: return "Pi 已就绪"
            case .connecting: return "正在启动 Pi"
            case .connected: return "Pi 已连接"
            case .unavailable(let reason): return reason
            case .failed(let reason): return reason
            }
        }

        var color: Color {
            switch self {
            case .ready, .connected: return .green
            case .connecting: return .orange
            case .unavailable, .failed: return .secondary
            }
        }
    }

    struct ServerError: LocalizedError {
        let message: String
        var errorDescription: String? { message }
    }

    @Published private(set) var state: ConnectionState = .ready
    var onPoll: (([String: Any]) -> Void)?
    var onBackendError: ((String) -> Void)?

    private var process: Process?
    private var input: FileHandle?
    private var output: FileHandle?
    private var outputBuffer = Data()
    private var pending: [String: ([String: Any]?, Error?) -> Void] = [:]

    func bootstrap(completion: @escaping ([String: Any]?, Error?) -> Void) {
        request(action: "bootstrap", fields: [:], completion: completion)
    }

    func createTask(profileID: String, projectID: String?, title: String, cwd: String, completion: @escaping ([String: Any]?, Error?) -> Void) {
        var fields: [String: Any] = ["profile_id": profileID, "title": title, "cwd": cwd]
        if let projectID { fields["project_id"] = projectID }
        request(action: "create_task", fields: fields, completion: completion)
    }

    func createProfile(name: String, completion: @escaping ([String: Any]?, Error?) -> Void) {
        request(action: "create_profile", fields: ["name": name], completion: completion)
    }

    func setProfile(profileID: String, fullAccess: Bool, completion: @escaping ([String: Any]?, Error?) -> Void = { _, _ in }) {
        request(action: "set_profile", fields: [
            "profile_id": profileID,
            "permission_mode": fullAccess ? "system_full" : "guarded",
            "unattended": fullAccess
        ], completion: completion)
    }

    func startTask(taskID: String, completion: @escaping ([String: Any]?, Error?) -> Void) {
        request(action: "start_task", fields: ["task_id": taskID], completion: completion)
    }

    func send(prompt: String, taskID: String, completion: @escaping ([String: Any]?, Error?) -> Void = { _, _ in }) {
        request(action: "prompt", fields: ["task_id": taskID, "prompt": prompt], completion: completion)
    }

    func poll(taskID: String, completion: @escaping ([String: Any]?, Error?) -> Void) {
        request(action: "poll", fields: ["task_id": taskID], completion: completion)
    }

    func stopTask(taskID: String) {
        request(action: "stop_task", fields: ["task_id": taskID]) { _, _ in }
    }

    func setTaskFlags(taskID: String, pinned: Bool?, archived: Bool?, unread: Bool?) {
        var fields: [String: Any] = ["task_id": taskID]
        if let pinned { fields["pinned"] = pinned }
        if let archived { fields["archived"] = archived }
        if let unread { fields["unread"] = unread }
        request(action: "set_task", fields: fields) { _, _ in }
    }

    func stop() {
        if process?.isRunning == true, let input {
            let request: [String: Any] = ["id": UUID().uuidString, "action": "shutdown"]
            if let data = try? JSONSerialization.data(withJSONObject: request) {
                try? input.write(contentsOf: data + Data([0x0A]))
            }
        }
        output?.readabilityHandler = nil
        process?.terminate()
        process = nil
        input = nil
        output = nil
        state = .ready
    }

    private func request(action: String, fields: [String: Any], completion: @escaping ([String: Any]?, Error?) -> Void) {
        ensureStarted()
        guard let input else {
            let error = ServerError(message: "PAD 后端未找到。请先运行打包脚本，或设置 PAD_BIN。")
            state = .unavailable(error.message)
            onBackendError?(error.message)
            completion(nil, error)
            return
        }
        let id = UUID().uuidString
        var request = fields
        request["id"] = id
        request["action"] = action
        guard let data = try? JSONSerialization.data(withJSONObject: request) else {
            completion(nil, ServerError(message: "无法编码 PAD 请求"))
            return
        }
        pending[id] = completion
        do {
            try input.write(contentsOf: data + Data([0x0A]))
        } catch {
            pending.removeValue(forKey: id)
            completion(nil, error)
            state = .failed(error.localizedDescription)
        }
    }

    private func ensureStarted() {
        guard process?.isRunning != true else { return }
        guard let executable = backendExecutable() else {
            state = .unavailable("PAD 后端未打包")
            return
        }
        let process = Process()
        let stdin = Pipe()
        let stdout = Pipe()
        let stderr = Pipe()
        process.executableURL = executable
        process.arguments = ["__internal", "desktop-server"]
        process.standardInput = stdin
        process.standardOutput = stdout
        process.standardError = stderr
        var environment = ProcessInfo.processInfo.environment
        let bundledBin = Bundle.main.resourceURL?.appendingPathComponent("bin", isDirectory: true).path
        let inheritedPath = environment["PATH"] ?? "/usr/bin:/bin:/usr/sbin:/sbin"
        environment["PATH"] = ([bundledBin, "/opt/homebrew/bin", "/usr/local/bin", inheritedPath]
            .compactMap { $0 }.joined(separator: ":"))
        process.environment = environment
        process.terminationHandler = { [weak self] terminated in
            Swift.Task { @MainActor in
                guard let self, self.process === process else { return }
                self.state = terminated.terminationStatus == 0 ? .ready : .failed("PAD 后端已退出")
                let error = ServerError(message: self.state.label)
                let callbacks = self.pending.values
                self.pending.removeAll()
                for callback in callbacks { callback(nil, error) }
            }
        }
        state = .connecting
        do {
            try process.run()
            self.process = process
            input = stdin.fileHandleForWriting
            output = stdout.fileHandleForReading
            output?.readabilityHandler = { [weak self] handle in
                let data = handle.availableData
                guard !data.isEmpty else { return }
                Swift.Task { @MainActor in self?.consume(data: data) }
            }
            state = .connected
        } catch {
            state = .unavailable("PAD 后端启动失败：\(error.localizedDescription)")
        }
    }

    private func consume(data: Data) {
        outputBuffer.append(data)
        while let newline = outputBuffer.firstIndex(of: 0x0A) {
            let line = outputBuffer.prefix(upTo: newline)
            outputBuffer.removeSubrange(...newline)
            guard let object = try? JSONSerialization.jsonObject(with: line),
                  let json = object as? [String: Any] else { continue }
            guard let id = json["id"] as? String else { continue }
            let callback = pending.removeValue(forKey: id)
            if json["ok"] as? Bool == true {
                callback?(json["result"] as? [String: Any] ?? [:], nil)
            } else {
                let errorObject = json["error"] as? [String: Any]
                let message = errorObject?["message"] as? String ?? "PAD 请求失败"
                callback?(nil, ServerError(message: message))
                onBackendError?(message)
            }
        }
    }

    private func backendExecutable() -> URL? {
        let environment = ProcessInfo.processInfo.environment
        let candidates = [
            environment["PAD_BIN"],
            Bundle.main.url(forResource: "pad", withExtension: nil)?.path,
            environment["PAD_RUST_BINARY"],
            FileManager.default.currentDirectoryPath + "/rust-tui/target/debug/pad",
            FileManager.default.currentDirectoryPath + "/../../rust-tui/target/debug/pad",
            FileManager.default.currentDirectoryPath + "/../../rust-tui/target/release/pad",
            FileManager.default.currentDirectoryPath + "/target/debug/pad",
            "/Users/tim/.local/bin/pad",
            "/opt/homebrew/bin/pad",
            "/usr/local/bin/pad"
        ].compactMap { $0 }.map { URL(fileURLWithPath: $0) }
        return candidates.first(where: { FileManager.default.isExecutableFile(atPath: $0.path) })
    }

    deinit {
        output?.readabilityHandler = nil
        process?.terminate()
    }
}

// MARK: - Application state

@MainActor
final class DesktopModel: ObservableObject {
    // The Rust bridge is the source of truth. Keep the renderer empty until
    // bootstrap completes instead of showing non-persistent demo records.
    @Published var profiles: [Profile] = []
    @Published var projects: [Project] = []
    @Published var tasks: [DesktopTask] = []

    @Published var selectedFilter: SidebarFilter = .all
    @Published var selectedTaskID: String?
    @Published var selectedProfileID = ""
    @Published var searchText = ""
    @Published var composerText = ""
    // The authoritative value is replaced by the bootstrap Profile policy.
    // Keep a conservative value while the bridge is still connecting.
    @Published var fullAccess = false
    @Published var isShowingProfilePicker = false
    @Published var isShowingOutputPanel = false
    @Published var isShowingPiLogin = false
    @Published var notice: String?

    let pi = PiRPCClient()
    let piLogin = PiLoginCoordinator()
    private var pollingTaskIDs = Set<String>()
    private var startedTaskIDs = Set<String>()
    private var profileFullAccess: [String: Bool] = [:]

    init() {
        pi.onBackendError = { [weak self] message in
            self?.notice = message
        }
        pi.bootstrap { [weak self] result, error in
            guard let self else { return }
            if let error {
                self.notice = error.localizedDescription
                return
            }
            guard let result else { return }
            self.applyRecords(result["records"] as? [String: Any])
            if self.tasks.isEmpty { self.createTask() }
        }
    }

    var selectedTask: DesktopTask? {
        guard let selectedTaskID else { return nil }
        return tasks.first(where: { $0.id == selectedTaskID })
    }

    var selectedProfile: Profile {
        profiles.first(where: { $0.id == selectedProfileID }) ??
            Profile(id: "", name: "PAD 桌面", subtitle: "连接中", accent: .secondary)
    }

    var visibleTasks: [DesktopTask] {
        tasks.filter { task in
            guard task.profileID == selectedProfileID else { return false }
            switch selectedFilter {
            case .all: return !task.isArchived
            case .pinned: return task.isPinned && !task.isArchived
            case .archive: return task.isArchived
            }
        }.filter { task in
            searchText.isEmpty || task.title.localizedCaseInsensitiveContains(searchText)
        }
    }

    var visibleProjects: [Project] {
        projects.filter { project in
            project.profileID == selectedProfileID && visibleTasks.contains(where: { $0.projectID == project.id })
        }
    }

    func select(task: DesktopTask) {
        selectedTaskID = task.id
        if let index = tasks.firstIndex(where: { $0.id == task.id }) {
            tasks[index].hasUnread = false
        }
    }

    func createTask() {
        let profileID = selectedProfileID
        guard !profileID.isEmpty else {
            notice = "PAD 后端尚未完成初始化，请稍后重试。"
            return
        }
        let projectID = visibleProjects.first?.id
        pi.createTask(profileID: profileID, projectID: projectID, title: "新任务", cwd: FileManager.default.currentDirectoryPath) { [weak self] result, error in
            guard let self else { return }
            if let error { self.notice = error.localizedDescription; return }
            if let records = result?["records"] as? [String: Any] { self.applyRecords(records) }
            if let task = result?["task"] as? [String: Any], let id = task["id"] as? String {
                self.selectedFilter = .all
                self.selectedTaskID = id
            }
            self.composerText = ""
        }
    }

    func createProfile() {
        let name = "配置 \(profiles.count + 1)"
        pi.createProfile(name: name) { [weak self] result, error in
            guard let self else { return }
            if let error { self.notice = error.localizedDescription; return }
            if let records = result?["records"] as? [String: Any] { self.applyRecords(records) }
            if let profile = result?["profile"] as? [String: Any], let id = profile["id"] as? String {
                self.recordProfilePolicy(profile)
                self.selectProfile(id)
                self.selectedTaskID = nil
            }
        }
    }

    func openPiLogin() {
        guard profiles.contains(where: { $0.id == selectedProfileID }) else {
            notice = "请先选择一个 Pi 账号配置。"
            return
        }
        isShowingPiLogin = true
    }

    func selectProfile(_ profileID: String) {
        selectedProfileID = profileID
        selectedTaskID = tasks.first(where: { $0.profileID == profileID && !$0.isArchived })?.id
        fullAccess = profileFullAccess[profileID] ?? false
    }

    func send() {
        let prompt = composerText.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !prompt.isEmpty, let taskID = selectedTaskID,
              let index = tasks.firstIndex(where: { $0.id == taskID }) else { return }
        tasks[index].messages.append(Message(role: .user, text: prompt))
        tasks[index].state = .running
        if tasks[index].title == "New task" || tasks[index].title == "新任务" {
            tasks[index].title = String(prompt.prefix(48))
        }
        composerText = ""
        if startedTaskIDs.contains(taskID) {
            sendPrompt(prompt, taskID: taskID)
            return
        }
        pi.startTask(taskID: taskID) { [weak self] _, error in
            guard let self else { return }
            if let error {
                self.notice = error.localizedDescription
                self.tasks.firstIndex(where: { $0.id == taskID }).map { self.tasks[$0].state = .failed }
                return
            }
            self.startedTaskIDs.insert(taskID)
            self.sendPrompt(prompt, taskID: taskID)
        }
    }

    private func sendPrompt(_ prompt: String, taskID: String) {
        pi.send(prompt: prompt, taskID: taskID) { [weak self] _, error in
            guard let self, let error else { return }
            self.notice = error.localizedDescription
            self.tasks.firstIndex(where: { $0.id == taskID }).map { self.tasks[$0].state = .failed }
        }
        schedulePoll(taskID: taskID)
    }

    func togglePin() {
        guard let taskID = selectedTaskID,
              let index = tasks.firstIndex(where: { $0.id == taskID }) else { return }
        tasks[index].isPinned.toggle()
        pi.setTaskFlags(taskID: taskID, pinned: tasks[index].isPinned, archived: nil, unread: nil)
    }

    func archiveSelected() {
        guard let taskID = selectedTaskID,
              let index = tasks.firstIndex(where: { $0.id == taskID }) else { return }
        tasks[index].isArchived.toggle()
        selectedFilter = tasks[index].isArchived ? .archive : .all
        pi.setTaskFlags(taskID: taskID, pinned: nil, archived: tasks[index].isArchived, unread: nil)
    }

    func setFullAccess(_ enabled: Bool) {
        guard !selectedProfileID.isEmpty else {
            fullAccess = enabled
            return
        }
        let previous = fullAccess
        fullAccess = enabled
        profileFullAccess[selectedProfileID] = enabled
        let profileID = selectedProfileID
        pi.setProfile(profileID: profileID, fullAccess: enabled) { [weak self] result, error in
            guard let self else { return }
            if let error {
                self.fullAccess = previous
                self.profileFullAccess[profileID] = previous
                self.notice = error.localizedDescription
                return
            }
            guard let profile = result?["profile"] as? [String: Any],
                  let policy = profile["policy"] as? [String: Any],
                  let mode = policy["mode"] as? String else { return }
            let persisted = mode == "workspace_full" || mode == "system_full"
            self.profileFullAccess[profileID] = persisted
            if self.selectedProfileID == profileID { self.fullAccess = persisted }
        }
    }

    private func schedulePoll(taskID: String) {
        guard pollingTaskIDs.insert(taskID).inserted else { return }
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.15) { [weak self] in
            self?.pollOnce(taskID: taskID)
        }
    }

    private func pollOnce(taskID: String) {
        pi.poll(taskID: taskID) { [weak self] result, error in
            guard let self else { return }
            if let error {
                self.pollingTaskIDs.remove(taskID)
                self.notice = error.localizedDescription
                return
            }
            if let result { self.applyPoll(result, taskID: taskID) }
            let state = self.tasks.first(where: { $0.id == taskID })?.state ?? .idle
            if state == .running || state == .waiting {
                DispatchQueue.main.asyncAfter(deadline: .now() + 0.2) { [weak self] in self?.pollOnce(taskID: taskID) }
            } else {
                self.pollingTaskIDs.remove(taskID)
            }
        }
    }

    private func applyPoll(_ result: [String: Any], taskID: String) {
        if let poll = result["poll"] as? [String: Any], let messages = poll["messages"] as? [[String: Any]] {
            for message in messages {
                if let value = message["value"] { appendAssistantText(extractText(value), taskID: taskID) }
            }
            if let diagnostics = poll["diagnostics"] as? [String], !diagnostics.isEmpty {
                notice = diagnostics.joined(separator: "\n")
            }
        }
        if let runtime = result["runtime"] as? [String: Any], let status = runtime["status"] as? String,
           let index = tasks.firstIndex(where: { $0.id == taskID }) {
            tasks[index].state = taskState(status)
        }
        if let poll = result["poll"] as? [String: Any],
           let exit = poll["exit_status"] as? [String: Any] {
            pollingTaskIDs.remove(taskID)
            let success = exit["success"] as? Bool ?? false
            tasks.firstIndex(where: { $0.id == taskID }).map {
                tasks[$0].state = success ? .idle : .failed
            }
            startedTaskIDs.remove(taskID)
        }
    }

    private func appendAssistantText(_ text: String?, taskID: String) {
        guard let text, !text.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty,
              let index = tasks.firstIndex(where: { $0.id == taskID }) else { return }
        tasks[index].messages.append(Message(role: .assistant, text: text))
        tasks[index].state = .idle
    }

    private func applyRecords(_ records: [String: Any]?) {
        guard let records else { return }
        if let rawProfiles = records["profiles"] as? [[String: Any]] {
            let colors: [Color] = [.blue, .purple, .orange, .green, .pink]
            let parsed = rawProfiles.enumerated().compactMap { offset, raw -> Profile? in
                guard let id = raw["id"] as? String, let name = raw["name"] as? String else { return nil }
                let provider = raw["default_provider"] as? String
                let subtitle = provider.map { "默认服务商：\($0)" } ?? "PAD 配置"
                return Profile(id: id, name: displayProfileName(name, id: id), subtitle: subtitle, accent: colors[offset % colors.count],
                               agentDirectory: raw["agent_dir"] as? String, sessionDirectory: raw["session_dir"] as? String)
            }
            if !parsed.isEmpty {
                profiles = parsed
                if !profiles.contains(where: { $0.id == selectedProfileID }) { selectedProfileID = profiles[0].id }
            }
            rawProfiles.forEach(recordProfilePolicy)
            fullAccess = profileFullAccess[selectedProfileID] ?? false
        }
        if let rawProjects = records["projects"] as? [[String: Any]] {
            projects = rawProjects.compactMap { raw in
                guard let id = raw["id"] as? String, let name = raw["name"] as? String else { return nil }
                return Project(id: id, name: displayProjectName(name, id: id), path: raw["primary_root"] as? String ?? "", profileID: raw["profile_id"] as? String ?? selectedProfileID)
            }
        }
        if let rawTasks = records["tasks"] as? [[String: Any]] {
            tasks = rawTasks.compactMap { raw in
                guard let id = raw["id"] as? String, let profileID = raw["profile_id"] as? String else { return nil }
                let title = raw["title"] as? String ?? "新任务"
                return DesktopTask(id: id, title: title == "New task" ? "新任务" : title, projectID: raw["project_id"] as? String,
                                   profileID: profileID, state: taskState(raw["status"] as? String ?? "idle"),
                                   isPinned: raw["pinned"] as? Bool ?? false, isArchived: raw["archived"] as? Bool ?? false,
                                   hasUnread: raw["unread"] as? Bool ?? false, messages: [])
            }
            if selectedTaskID == nil || !tasks.contains(where: { $0.id == selectedTaskID }) {
                selectedTaskID = tasks.first(where: { $0.profileID == selectedProfileID && !$0.isArchived })?.id
            }
        }
    }

    private func recordProfilePolicy(_ raw: [String: Any]) {
        guard let id = raw["id"] as? String,
              let policy = raw["policy"] as? [String: Any],
              let mode = policy["mode"] as? String else { return }
        profileFullAccess[id] = mode == "workspace_full" || mode == "system_full"
    }

    /// Localize records created by older PAD Desktop builds without changing
    /// the persisted metadata or user-authored names.
    private func displayProfileName(_ name: String, id: String) -> String {
        id == "default" && name == "Default" ? "默认配置" : name
    }

    private func displayProjectName(_ name: String, id: String) -> String {
        id == "project-default" && name == "Workspace" ? "默认工作区" : name
    }

    private func taskState(_ status: String) -> TaskState {
        switch status {
        case "starting", "running", "streaming", "tool_running": return .running
        case "needs_approval", "needs_input": return .waiting
        case "failed", "disconnected": return .failed
        default: return .idle
        }
    }

    private func extractText(_ value: Any) -> String? {
        if let string = value as? String { return string }
        if let object = value as? [String: Any] {
            for key in ["text", "message", "delta", "content"] {
                if let nested = object[key], let text = extractText(nested) { return text }
            }
        }
        if let array = value as? [Any] {
            let parts = array.compactMap(extractText)
            return parts.isEmpty ? nil : parts.joined()
        }
        return nil
    }
}

// MARK: - UI

/// Shared geometry for the clean-room Codex-style shell. Keeping these values
/// together makes it possible to tune the desktop as one surface instead of
/// accumulating slightly different constants in each view.
private enum CodexMetrics {
    static let windowMinWidth: CGFloat = 1080
    static let windowMinHeight: CGFloat = 680
    static let windowDefaultWidth: CGFloat = 1460
    static let windowDefaultHeight: CGFloat = 900
    static let sidebarMinWidth: CGFloat = 260
    static let sidebarIdealWidth: CGFloat = 300
    static let sidebarMaxWidth: CGFloat = 380
    static let transcriptMaxWidth: CGFloat = 960
    static let composerMaxWidth: CGFloat = 960
}

@main
struct PADDesktopApp: App {
    @StateObject private var model = DesktopModel()

    var body: some Scene {
        WindowGroup("PAD 桌面") {
            DesktopShell(model: model)
                .frame(minWidth: CodexMetrics.windowMinWidth, minHeight: CodexMetrics.windowMinHeight)
                // Keep system-provided controls and accessibility strings in
                // Simplified Chinese even when macOS itself uses another
                // display language.
                .environment(\.locale, Locale(identifier: "zh_CN"))
        }
        .defaultSize(width: CodexMetrics.windowDefaultWidth, height: CodexMetrics.windowDefaultHeight)
        .commands {
            CommandGroup(after: .newItem) {
                Button("新建任务") { model.createTask() }
                    .keyboardShortcut("n", modifiers: [.command, .shift])
            }
            CommandMenu("PAD") {
                Toggle(
                    "完全访问",
                    isOn: Binding(
                        get: { model.fullAccess },
                        set: { model.setFullAccess($0) }
                    )
                )
                    .keyboardShortcut("f", modifiers: [.command, .shift])
                Button("登录 Pi 账号") { model.openPiLogin() }
                Button("停止 Pi") { model.pi.stop() }
            }
        }
    }
}

struct DesktopShell: View {
    @ObservedObject var model: DesktopModel

    var body: some View {
        NavigationSplitView {
            SidebarView(model: model)
                .navigationSplitViewColumnWidth(min: CodexMetrics.sidebarMinWidth,
                                                 ideal: CodexMetrics.sidebarIdealWidth,
                                                 max: CodexMetrics.sidebarMaxWidth)
        } detail: {
            ConversationView(model: model)
        }
        .toolbar {
            ToolbarItem(placement: .navigation) {
                Button(action: model.createTask) {
                    Label("新建任务", systemImage: "square.and.pencil")
                }
                .help("新建任务（⇧⌘N）")
            }
            ToolbarItem(placement: .automatic) {
                FullAccessBadge(enabled: model.fullAccess)
            }
        }
        .alert("PAD 桌面", isPresented: Binding(
            get: { model.notice != nil },
            set: { if !$0 { model.notice = nil } }
        )) {
            Button("确定") { model.notice = nil }
        } message: {
            Text(model.notice ?? "")
        }
        .sheet(isPresented: $model.isShowingPiLogin, onDismiss: {
            model.piLogin.cancel()
        }) {
            PiLoginSheet(profile: model.selectedProfile, coordinator: model.piLogin) {
                if model.piLogin.phase == .succeeded {
                    model.notice = "Pi 账号登录成功。"
                }
                model.isShowingPiLogin = false
            }
        }
    }
}

struct SidebarView: View {
    @ObservedObject var model: DesktopModel

    var body: some View {
        VStack(spacing: 0) {
            HStack(spacing: 9) {
                ZStack {
                    RoundedRectangle(cornerRadius: 7, style: .continuous)
                        .fill(Color.accentColor.gradient)
                    Image(systemName: "sparkles")
                        .font(.caption.weight(.bold))
                        .foregroundStyle(.white)
                }
                .frame(width: 25, height: 25)
                Text("PAD")
                    .font(.headline.weight(.semibold))
                Image(systemName: "chevron.down")
                    .font(.caption2.weight(.semibold))
                    .foregroundStyle(.secondary)
                Spacer()
                Button { model.createTask() } label: {
                    Image(systemName: "square.and.pencil")
                }
                .buttonStyle(.plain)
                .help("新建任务")
            }
            .padding(.horizontal, 14)
            .padding(.top, 13)
            .padding(.bottom, 9)

            Button(action: model.createTask) {
                HStack(spacing: 9) {
                    Image(systemName: "square.and.pencil")
                        .font(.callout.weight(.medium))
                    Text("新建任务")
                        .font(.callout.weight(.medium))
                    Spacer()
                    Image(systemName: "plus.circle")
                        .foregroundStyle(.secondary)
                }
                .padding(.horizontal, 10)
                .padding(.vertical, 8)
                .contentShape(Rectangle())
            }
            .buttonStyle(.plain)
            // Codex keeps the primary action visually quiet; selection is
            // reserved for the task/filter rows below it.
            .background(Color.clear, in: RoundedRectangle(cornerRadius: 8, style: .continuous))
            .padding(.horizontal, 10)
            .padding(.bottom, 9)

            HStack(spacing: 8) {
                Image(systemName: "magnifyingglass")
                    .foregroundStyle(.secondary)
                TextField("搜索任务", text: $model.searchText)
                    .textFieldStyle(.plain)
                if !model.searchText.isEmpty {
                    Button { model.searchText = "" } label: {
                        Image(systemName: "xmark.circle.fill")
                    }
                    .buttonStyle(.plain)
                    .foregroundStyle(.secondary)
                }
            }
            .padding(.horizontal, 9)
            .padding(.vertical, 7)
            .background(.quaternary.opacity(0.6), in: RoundedRectangle(cornerRadius: 7))
            .padding(.horizontal, 12)
            .padding(.bottom, 10)

            List(selection: $model.selectedTaskID) {
                Section {
                    ForEach(SidebarFilter.allCases) { filter in
                        SidebarFilterRow(filter: filter, selected: model.selectedFilter == filter) {
                            model.selectedFilter = filter
                        }
                    }
                }

                Section("项目") {
                    if model.visibleProjects.isEmpty && model.visibleTasks.isEmpty {
                        Text("暂无任务")
                            .foregroundStyle(.secondary)
                            .font(.caption)
                    }
                    ForEach(model.visibleProjects) { project in
                        ProjectGroup(project: project, model: model)
                    }
                }

                Section("最近") {
                    ForEach(model.visibleTasks.prefix(20)) { task in
                        TaskRow(task: task)
                            .tag(task.id)
                            .onTapGesture { model.select(task: task) }
                    }
                }
            }
            .listStyle(.sidebar)

            Divider()
            ProfilePicker(model: model)
                .padding(.horizontal, 10)
                .padding(.top, 8)
            HStack(spacing: 7) {
                Circle()
                    .fill(model.pi.state.color)
                    .frame(width: 7, height: 7)
                Text(model.pi.state.label)
                    .font(.caption2)
                    .foregroundStyle(.secondary)
                    .lineLimit(1)
                Spacer()
                Image(systemName: "gearshape")
                    .font(.caption)
                    .foregroundStyle(.tertiary)
                    .help("设置")
            }
            .padding(.horizontal, 14)
            .padding(.top, 5)
            .padding(.bottom, 11)
        }
        .background(Color(nsColor: .windowBackgroundColor))
    }
}

struct ProfilePicker: View {
    @ObservedObject var model: DesktopModel

    var body: some View {
        ZStack {
            // Render the account row independently. macOS's borderless Menu
            // otherwise flattens a compound SwiftUI label and leaves only the
            // first character visible in a narrow sidebar.
            HStack(spacing: 9) {
                Circle()
                    .fill(model.selectedProfile.accent.gradient)
                    .frame(width: 25, height: 25)
                    .overlay {
                        Text(String(model.selectedProfile.name.prefix(1)))
                            .font(.caption.bold())
                            .foregroundStyle(.white)
                    }
                VStack(alignment: .leading, spacing: 1) {
                    Text(model.selectedProfile.name)
                        .font(.headline)
                        .lineLimit(1)
                        .truncationMode(.tail)
                    Text(model.selectedProfile.subtitle)
                        .font(.caption)
                        .foregroundStyle(.secondary)
                        .lineLimit(1)
                        .truncationMode(.tail)
                }
                .layoutPriority(1)
                Spacer()
                Image(systemName: "chevron.up.chevron.down")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }
            .frame(maxWidth: .infinity, alignment: .leading)
            .contentShape(Rectangle())
            .allowsHitTesting(false)

            Menu {
                Section("Pi 账号") {
                    ForEach(model.profiles) { profile in
                        Button {
                            model.selectProfile(profile.id)
                        } label: {
                            Label(profile.name, systemImage: profile.id == model.selectedProfileID ? "checkmark.circle.fill" : "person.circle")
                        }
                    }
                }
                Divider()
                Button {
                    model.openPiLogin()
                } label: {
                    Label("登录 Pi 账号", systemImage: "person.crop.circle.badge.checkmark")
                }
                Button {
                    model.createProfile()
                } label: {
                    Label("添加 Pi 账号", systemImage: "person.badge.plus")
                }
            } label: {
                Color.clear
                    .frame(maxWidth: .infinity, minHeight: 36)
                    .contentShape(Rectangle())
            }
            .menuStyle(.borderlessButton)
            .menuIndicator(.hidden)
            .accessibilityLabel("切换 Pi 账号：\(model.selectedProfile.name)")
        }
        .frame(maxWidth: .infinity, minHeight: 36, alignment: .leading)
    }
}

struct SidebarFilterRow: View {
    let filter: SidebarFilter
    let selected: Bool
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            Label(filter.rawValue, systemImage: filter.symbol)
                .frame(maxWidth: .infinity, alignment: .leading)
        }
        .buttonStyle(.plain)
        .padding(.vertical, 3)
        .foregroundStyle(selected ? .primary : .secondary)
        .background(selected ? Color.accentColor.opacity(0.14) : .clear, in: RoundedRectangle(cornerRadius: 5))
    }
}

struct ProjectGroup: View {
    let project: Project
    @ObservedObject var model: DesktopModel

    var body: some View {
        DisclosureGroup {
            ForEach(model.visibleTasks.filter { $0.projectID == project.id }) { task in
                TaskRow(task: task)
                    .tag(task.id)
                    .onTapGesture { model.select(task: task) }
            }
        } label: {
            HStack(spacing: 7) {
                Image(systemName: "folder")
                    .foregroundStyle(.secondary)
                VStack(alignment: .leading, spacing: 1) {
                    Text(project.name)
                        .lineLimit(1)
                    Text(project.path)
                        .font(.caption2)
                        .foregroundStyle(.secondary)
                        .lineLimit(1)
                }
            }
        }
    }
}

struct TaskRow: View {
    let task: DesktopTask

    var body: some View {
        HStack(spacing: 7) {
            Image(systemName: task.state.symbol)
                .font(.caption2)
                .foregroundStyle(task.state.color)
            Text(task.title)
                .lineLimit(2)
            Spacer(minLength: 0)
            if task.isPinned {
                Image(systemName: "pin.fill")
                    .font(.caption2)
                    .foregroundStyle(.secondary)
            }
            if task.hasUnread {
                Circle()
                    .fill(Color.accentColor)
                    .frame(width: 6, height: 6)
            }
        }
        .padding(.vertical, 2)
    }
}

struct ConversationView: View {
    @ObservedObject var model: DesktopModel

    var body: some View {
        HStack(spacing: 0) {
            VStack(spacing: 0) {
                ConversationHeader(model: model)
                Divider()
                if let task = model.selectedTask {
                    ScrollViewReader { proxy in
                        ScrollView {
                            LazyVStack(alignment: .leading, spacing: 20) {
                                if task.messages.isEmpty {
                                    EmptyTaskView()
                                } else {
                                    ForEach(task.messages) { message in
                                        MessageBubble(message: message)
                                            .id(message.id)
                                    }
                                }
                                Color.clear.frame(height: 2).id("bottom")
                            }
                            .frame(maxWidth: CodexMetrics.transcriptMaxWidth)
                            .frame(maxWidth: .infinity)
                            .padding(.horizontal, 42)
                            .padding(.vertical, 30)
                        }
                        .onChange(of: task.messages.count) { _ in
                            withAnimation(.easeOut(duration: 0.2)) {
                                proxy.scrollTo("bottom", anchor: .bottom)
                            }
                        }
                    }
                    Composer(model: model)
                } else {
                    EmptyWorkspaceView(create: model.createTask)
                }
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity)

            if model.isShowingOutputPanel {
                Divider()
                OutputPanel(model: model)
                    .frame(width: 280)
            }
        }
        .background(Color(nsColor: .textBackgroundColor))
    }
}

struct ConversationHeader: View {
    @ObservedObject var model: DesktopModel

    var body: some View {
        HStack(spacing: 12) {
            VStack(alignment: .leading, spacing: 2) {
                Text(model.selectedTask?.title ?? "PAD 桌面")
                    .font(.headline)
                    .lineLimit(1)
                Text(model.selectedProfile.name + " · Pi 通道")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }
            Spacer()
            if let task = model.selectedTask {
                Button {
                    model.togglePin()
                } label: {
                    Image(systemName: task.isPinned ? "pin.fill" : "pin")
                }
                .buttonStyle(.borderless)
                .help(task.isPinned ? "取消置顶" : "置顶任务")
                Button {
                    model.archiveSelected()
                } label: {
                    Image(systemName: task.isArchived ? "archivebox.fill" : "archivebox")
                }
                .buttonStyle(.borderless)
                .help(task.isArchived ? "恢复任务" : "归档任务")
            }
            Button {
                model.isShowingOutputPanel.toggle()
            } label: {
                Image(systemName: model.isShowingOutputPanel ? "sidebar.trailing" : "sidebar.leading")
            }
            .buttonStyle(.borderless)
            .help(model.isShowingOutputPanel ? "隐藏工作区" : "显示工作区")
            FullAccessBadge(enabled: model.fullAccess)
        }
        .padding(.horizontal, 28)
        .padding(.vertical, 15)
    }
}

struct OutputPanel: View {
    @ObservedObject var model: DesktopModel

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            HStack {
                Text("工作区")
                    .font(.headline)
                Spacer()
                Image(systemName: "plus")
                    .foregroundStyle(.secondary)
                    .help("添加输出")
            }
            .padding(.horizontal, 16)
            .padding(.vertical, 14)
            Divider()

            ScrollView {
                VStack(alignment: .leading, spacing: 18) {
                    OutputSection(title: "输出内容", icon: "doc.text") {
                        Text("任务执行后生成的文件、差异和工具结果会显示在这里。")
                            .font(.caption)
                            .foregroundStyle(.secondary)
                            .fixedSize(horizontal: false, vertical: true)
                    }
                    OutputSection(title: "当前任务", icon: "checklist") {
                        if let task = model.selectedTask {
                            VStack(alignment: .leading, spacing: 7) {
                                Text(task.title)
                                    .font(.subheadline.weight(.medium))
                                    .lineLimit(3)
                                Label(taskStateLabel(task.state), systemImage: task.state.symbol)
                                    .font(.caption)
                                    .foregroundStyle(task.state.color)
                                Text("消息 \(task.messages.count) 条")
                                    .font(.caption)
                                    .foregroundStyle(.secondary)
                            }
                        } else {
                            Text("尚未选择任务")
                                .font(.caption)
                                .foregroundStyle(.secondary)
                        }
                    }
                    OutputSection(title: "运行配置", icon: "person.crop.circle") {
                        VStack(alignment: .leading, spacing: 7) {
                            Text(model.selectedProfile.name)
                                .font(.subheadline.weight(.medium))
                            Text(model.selectedProfile.subtitle)
                                .font(.caption)
                                .foregroundStyle(.secondary)
                            FullAccessBadge(enabled: model.fullAccess)
                        }
                    }
                }
                .padding(16)
            }
            Spacer(minLength: 0)
        }
        .background(Color(nsColor: .windowBackgroundColor))
    }

    private func taskStateLabel(_ state: TaskState) -> String {
        switch state {
        case .idle: return "已就绪"
        case .running: return "运行中"
        case .waiting: return "等待输入"
        case .failed: return "失败"
        }
    }
}

struct OutputSection<Content: View>: View {
    let title: String
    let icon: String
    @ViewBuilder let content: () -> Content

    var body: some View {
        VStack(alignment: .leading, spacing: 9) {
            Label(title, systemImage: icon)
                .font(.subheadline.weight(.semibold))
            content()
        }
    }
}

struct FullAccessBadge: View {
    let enabled: Bool

    var body: some View {
        Label(enabled ? "完全访问" : "受保护模式", systemImage: enabled ? "checkmark.shield.fill" : "shield")
            .font(.caption)
            .foregroundStyle(enabled ? .green : .secondary)
            .padding(.horizontal, 8)
            .padding(.vertical, 5)
            .background((enabled ? Color.green : Color.secondary).opacity(0.12), in: Capsule())
            .help(enabled ? "工具调用将按完全访问策略执行" : "工具调用会在执行前请求确认")
    }
}

struct MessageBubble: View {
    let message: Message

    var body: some View {
        HStack(alignment: .top, spacing: 12) {
            Image(systemName: message.role == .user ? "person.circle.fill" : "sparkles")
                .font(.title3)
                .foregroundStyle(message.role == .user ? .blue : .purple)
                .frame(width: 25)
            VStack(alignment: .leading, spacing: 6) {
                Text(message.role == .user ? "你" : "Pi")
                    .font(.subheadline.weight(.semibold))
                Text(message.text)
                    .font(.body)
                    .textSelection(.enabled)
                    .fixedSize(horizontal: false, vertical: true)
            }
            Spacer(minLength: 0)
        }
        .padding(.horizontal, 18)
        .padding(.vertical, 15)
        .background(message.role == .user ? Color.accentColor.opacity(0.08) : Color.clear, in: RoundedRectangle(cornerRadius: 12))
    }
}

struct Composer: View {
    @ObservedObject var model: DesktopModel

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack(alignment: .bottom, spacing: 10) {
                Button { } label: {
                    Image(systemName: "plus")
                        .font(.callout.weight(.medium))
                        .frame(width: 28, height: 28)
                }
                .buttonStyle(.plain)
                .foregroundStyle(.secondary)
                .help("添加附件或上下文（即将支持）")
                TextField("向 Pi 发送消息…", text: $model.composerText, axis: .vertical)
                    .textFieldStyle(.plain)
                    .lineLimit(1...6)
                    .onSubmit {
                        if !NSEvent.modifierFlags.contains(.shift) { model.send() }
                    }
                Button(action: model.send) {
                    Image(systemName: "arrow.up.circle.fill")
                        .font(.title2)
                        .foregroundStyle(model.composerText.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty ? Color.secondary : Color.accentColor)
                }
                .buttonStyle(.plain)
                .disabled(model.composerText.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
            }
            HStack(spacing: 12) {
                Button {
                    model.setFullAccess(!model.fullAccess)
                } label: {
                    Label(model.fullAccess ? "完全访问" : "受保护模式", systemImage: model.fullAccess ? "checkmark.shield" : "shield")
                        .font(.caption)
                }
                .buttonStyle(.plain)
                .foregroundStyle(model.fullAccess ? .orange : .secondary)
                .help(model.fullAccess ? "关闭完全访问" : "开启完全访问")
                Spacer()
                Text(model.selectedProfile.subtitle)
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .lineLimit(1)
                Text("按回车发送")
                    .font(.caption2)
                    .foregroundStyle(.tertiary)
            }
        }
        .padding(.horizontal, 14)
        .padding(.top, 11)
        .padding(.bottom, 11)
        .background(.bar, in: RoundedRectangle(cornerRadius: 14, style: .continuous))
        .overlay {
            RoundedRectangle(cornerRadius: 14, style: .continuous)
                .stroke(Color.primary.opacity(0.10), lineWidth: 1)
        }
        .frame(maxWidth: CodexMetrics.composerMaxWidth)
        .padding(.horizontal, 24)
        .padding(.top, 10)
        .padding(.bottom, 16)
        .background(Color(nsColor: .textBackgroundColor))
    }
}

struct EmptyTaskView: View {
    var body: some View {
        VStack(spacing: 10) {
            Image(systemName: "sparkles")
                .font(.system(size: 32))
                .foregroundStyle(.purple)
            Text("开始对话")
                .font(.title3.weight(.semibold))
            Text("向 Pi 描述你要完成的工作。")
                .foregroundStyle(.secondary)
        }
        .frame(maxWidth: .infinity, minHeight: 280)
    }
}

struct EmptyWorkspaceView: View {
    let create: () -> Void

    var body: some View {
        VStack(spacing: 14) {
            Image(systemName: "rectangle.3.group.bubble.left")
                .font(.system(size: 42))
                .foregroundStyle(.secondary)
            Text("选择一个任务，或新建任务")
                .font(.title2.weight(.semibold))
            Button("新建任务", action: create)
                .buttonStyle(.borderedProminent)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }
}
