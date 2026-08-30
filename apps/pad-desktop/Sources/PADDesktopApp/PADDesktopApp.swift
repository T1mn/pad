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
    case starting
    case running
    case streaming
    case toolRunning
    case retrying
    case needsApproval
    case needsInput
    case failed

    var symbol: String {
        switch self {
        case .idle: return "circle"
        case .starting: return "ellipsis.circle"
        case .running, .streaming: return "circle.fill"
        case .toolRunning: return "hammer.circle.fill"
        case .retrying: return "arrow.clockwise.circle.fill"
        case .needsApproval, .needsInput: return "exclamationmark.circle.fill"
        case .failed: return "xmark.circle.fill"
        }
    }

    var color: Color {
        switch self {
        case .idle: return .secondary
        case .starting, .running, .streaming, .toolRunning: return .green
        case .retrying: return .orange
        case .needsApproval, .needsInput: return .orange
        case .failed: return .red
        }
    }

    var label: String {
        switch self {
        case .idle: return "已就绪"
        case .starting: return "正在启动"
        case .running: return "运行中"
        case .streaming: return "正在生成"
        case .toolRunning: return "执行工具"
        case .retrying: return "重试中"
        case .needsApproval: return "等待审批"
        case .needsInput: return "等待输入"
        case .failed: return "失败"
        }
    }

    var isActive: Bool {
        switch self {
        case .starting, .running, .streaming, .toolRunning, .retrying: return true
        case .idle, .needsApproval, .needsInput, .failed: return false
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
    var credentialRef: String? = nil
    var defaultModel: String? = nil
}

/// A model returned by Pi's model catalog. Keep the stable provider/model ID
/// separate from the human-readable name because `set_model` must send the
/// former while the menu should display the latter when Pi provides it.
struct PADModelOption: Identifiable, Hashable {
    let id: String
    let name: String
    let provider: String
    let reasoning: String?

    var displayName: String {
        name.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty ? id : name
    }
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
    var cwd: String
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
    var text: String
    let timestamp: Date

    init(id: UUID = UUID(), role: Role, text: String, timestamp: Date = Date()) {
        self.id = id
        self.role = role
        self.text = text
        self.timestamp = timestamp
    }
}

struct PendingInteraction: Identifiable, Hashable {
    let id: String
    let taskID: String
    let kind: String
    let message: String
    let options: [String]
    let responseKind: String
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
            case .ready: return "Pi 引擎已就绪"
            case .connecting: return "正在启动 Pi 引擎"
            case .connected: return "Pi 引擎已连接"
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

    /// Read the Pi-backed model catalog for one isolated PAD Profile. The
    /// bridge may return either the normalized `models` list or provider
    /// groups; DesktopModel intentionally accepts both shapes for rolling
    /// upgrades where the Rust sidecar and Swift shell are not upgraded in
    /// the same process.
    func modelCatalog(profileID: String, completion: @escaping ([String: Any]?, Error?) -> Void) {
        request(action: "model_catalog", fields: ["profile_id": profileID], completion: completion)
    }

    func createTask(profileID: String, projectID: String?, title: String, cwd: String, completion: @escaping ([String: Any]?, Error?) -> Void) {
        var fields: [String: Any] = ["profile_id": profileID, "title": title, "cwd": cwd]
        if let projectID { fields["project_id"] = projectID }
        request(action: "create_task", fields: fields, completion: completion)
    }

    func createProject(profileID: String, name: String, cwd: String, completion: @escaping ([String: Any]?, Error?) -> Void) {
        request(action: "create_project", fields: [
            "profile_id": profileID,
            "name": name,
            "cwd": cwd
        ], completion: completion)
    }

    func createProfile(name: String, provider: String = "openai-codex", completion: @escaping ([String: Any]?, Error?) -> Void) {
        request(action: "create_profile", fields: ["name": name, "default_provider": provider], completion: completion)
    }

    func setProfile(profileID: String, fullAccess: Bool, completion: @escaping ([String: Any]?, Error?) -> Void = { _, _ in }) {
        request(action: "set_profile", fields: [
            "profile_id": profileID,
            "permission_mode": fullAccess ? "system_full" : "guarded",
            "unattended": fullAccess
        ], completion: completion)
    }

    func setProfileMetadata(profileID: String, provider: String, model: String? = nil, credentialRef: String? = nil,
                            completion: @escaping ([String: Any]?, Error?) -> Void = { _, _ in }) {
        var fields: [String: Any] = [
            "profile_id": profileID,
            "default_provider": provider
        ]
        if let model { fields["default_model"] = model }
        if let credentialRef { fields["credential_ref"] = credentialRef }
        request(action: "set_profile", fields: fields, completion: completion)
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

    func history(taskID: String, completion: @escaping ([String: Any]?, Error?) -> Void) {
        request(action: "history", fields: ["task_id": taskID], completion: completion)
    }

    func respondUI(taskID: String, interactionID: String, responseKind: String, value: Any, completion: @escaping ([String: Any]?, Error?) -> Void = { _, _ in }) {
        request(action: "extension_ui_response", fields: [
            "task_id": taskID,
            "interaction_id": interactionID,
            "response_kind": responseKind,
            "value": value
        ], completion: completion)
    }

    func setModel(taskID: String, provider: String, model: String, completion: @escaping ([String: Any]?, Error?) -> Void = { _, _ in }) {
        request(action: "set_model", fields: [
            "task_id": taskID,
            "provider": provider,
            "model": model
        ], completion: completion)
    }

    func setThinkingLevel(taskID: String, level: String, completion: @escaping ([String: Any]?, Error?) -> Void = { _, _ in }) {
        request(action: "set_thinking_level", fields: [
            "task_id": taskID,
            "thinking_level": level
        ], completion: completion)
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
    @Published var draftProjectID: String?
    @Published var draftWorkingDirectory = FileManager.default.currentDirectoryPath
    @Published var pendingInteraction: PendingInteraction?
    @Published var selectedProvider = "openai-codex"
    @Published var selectedModel = "auto"
    @Published var availableModels = ["auto"]
    @Published private(set) var modelOptions: [PADModelOption] = []
    @Published private(set) var isLoadingModels = false
    @Published private(set) var modelCatalogError: String?
    @Published var selectedThinkingLevel = "medium"
    @Published var attachedPaths: [String] = []
    @Published private(set) var backendCapabilities: Set<String> = []
    // The authoritative value is replaced by the bootstrap Profile policy.
    // Keep a conservative value while the bridge is still connecting.
    @Published var fullAccess = false
    @Published var isShowingProfilePicker = false
    @Published var isShowingOutputPanel = false
    @Published var isShowingPiLogin = false
    @Published var isShowingProfileWizard = false
    @Published var profileWizardName = ""
    @Published var profileWizardProvider = "openai-codex"
    @Published var notice: String?

    let pi = PiRPCClient()
    let piLogin = PiLoginCoordinator()
    private var pollingTaskIDs = Set<String>()
    private var startedTaskIDs = Set<String>()
    private var profileFullAccess: [String: Bool] = [:]
    private var loadedHistoryTaskIDs = Set<String>()
    private var streamingMessageIDs: [String: UUID] = [:]

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
            if let capabilities = result["capabilities"] as? [String] {
                self.backendCapabilities = Set(capabilities)
            }
            self.applyRecords(result["records"] as? [String: Any])
            self.prepareDraftIfNeeded()
            self.refreshModelCatalog()
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

    var hasDraft: Bool { !selectedProfileID.isEmpty && selectedTaskID == nil }

    var selectedTaskState: TaskState? { selectedTask?.state }

    var selectedProfileIsLoggedIn: Bool {
        guard let reference = selectedProfile.credentialRef else { return false }
        return !reference.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
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
            guard project.profileID == selectedProfileID else { return false }
            return searchText.isEmpty
                || project.name.localizedCaseInsensitiveContains(searchText)
                || visibleTasks.contains(where: { $0.projectID == project.id })
        }
    }

    func select(task: DesktopTask) {
        selectedTaskID = task.id
        if let index = tasks.firstIndex(where: { $0.id == task.id }) {
            tasks[index].hasUnread = false
        }
        pi.setTaskFlags(taskID: task.id, pinned: nil, archived: nil, unread: false)
        if backendCapabilities.contains("history") { loadHistoryIfNeeded(taskID: task.id) }
    }

    func createTask() {
        guard !selectedProfileID.isEmpty else {
            notice = "PAD 后端尚未完成初始化，请稍后重试。"
            return
        }
        selectedFilter = .all
        selectedTaskID = nil
        draftProjectID = visibleProjects.first?.id
        draftWorkingDirectory = visibleProjects.first?.path ?? FileManager.default.currentDirectoryPath
        composerText = ""
        attachedPaths = []
    }

    func chooseWorkingDirectory() {
        let panel = NSOpenPanel()
        panel.canChooseFiles = false
        panel.canChooseDirectories = true
        panel.allowsMultipleSelection = false
        panel.prompt = "选择工作目录"
        if panel.runModal() == .OK, let url = panel.url {
            createOrSelectProject(for: url)
        }
    }

    func addProject() {
        if !hasDraft { createTask() }
        chooseWorkingDirectory()
    }

    private func createOrSelectProject(for url: URL) {
        draftWorkingDirectory = url.path
        if let project = projects.first(where: { $0.path == url.path && $0.profileID == selectedProfileID }) {
            draftProjectID = project.id
            return
        }
        let name = url.lastPathComponent.isEmpty ? "本地项目" : url.lastPathComponent
        pi.createProject(profileID: selectedProfileID, name: name, cwd: url.path) { [weak self] result, error in
            guard let self else { return }
            if let error { self.notice = error.localizedDescription; return }
            if let records = result?["records"] as? [String: Any] { self.applyRecords(records) }
            self.selectedTaskID = nil
            if let project = result?["project"] as? [String: Any], let id = project["id"] as? String {
                self.draftProjectID = id
                self.draftWorkingDirectory = url.path
            }
        }
    }

    func chooseAttachments() {
        let panel = NSOpenPanel()
        panel.canChooseFiles = true
        panel.canChooseDirectories = false
        panel.allowsMultipleSelection = true
        panel.prompt = "添加上下文"
        if panel.runModal() == .OK {
            attachedPaths = panel.urls.map(\.path)
        }
    }

    func selectDraftProject(_ project: Project?) {
        draftProjectID = project?.id
        if let path = project?.path, !path.isEmpty { draftWorkingDirectory = path }
    }

    func prepareDraftIfNeeded() {
        guard selectedTaskID == nil, !selectedProfileID.isEmpty else { return }
        let project = draftProjectID.flatMap { selectedID in
            visibleProjects.first(where: { $0.id == selectedID })
        } ?? visibleProjects.first
        if draftProjectID == nil { draftProjectID = project?.id }
        // Finder launches apps with `/` as their process cwd. A new task must
        // inherit the selected PAD project instead of presenting or sending
        // that unsafe implementation detail as the task working directory.
        if draftWorkingDirectory.isEmpty || draftWorkingDirectory == "/" {
            if let projectPath = project?.path, !projectPath.isEmpty {
                draftWorkingDirectory = projectPath
            } else {
                draftWorkingDirectory = FileManager.default.homeDirectoryForCurrentUser.path
            }
        }
    }

    func startProfileWizard() {
        profileWizardName = ""
        profileWizardProvider = "openai-codex"
        isShowingProfileWizard = true
    }

    func createProfile(name: String, provider: String) {
        let cleanName = name.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !cleanName.isEmpty else { return }
        isShowingProfileWizard = false
        pi.createProfile(name: cleanName, provider: provider) { [weak self] result, error in
            guard let self else { return }
            if let error { self.notice = error.localizedDescription; return }
            if let records = result?["records"] as? [String: Any] { self.applyRecords(records) }
            if let profile = result?["profile"] as? [String: Any], let id = profile["id"] as? String {
                self.recordProfilePolicy(profile)
                self.selectProfile(id)
                self.selectedTaskID = nil
                self.isShowingPiLogin = true
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

    func finishPiLogin() {
        guard piLogin.phase == .succeeded,
              let profileIndex = profiles.firstIndex(where: { $0.id == selectedProfileID }) else { return }
        let profileID = profiles[profileIndex].id
        let provider = piLogin.provider.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !provider.isEmpty else { return }
        // This is only a stable reference for the UI/store. The actual secret
        // remains in the Profile-private Pi auth.json managed by PiLogin.
        let credentialRef = "pi-auth://\(profileID)"
        profiles[profileIndex].credentialRef = credentialRef
        pi.setProfileMetadata(profileID: profileID, provider: provider, credentialRef: credentialRef) { [weak self] result, error in
            guard let self else { return }
            if let error {
                self.notice = error.localizedDescription
                return
            }
            if let profile = result?["profile"] as? [String: Any] {
                self.recordProfilePolicy(profile)
            }
            self.notice = "Pi 账号登录成功。"
            self.refreshFromBackend()
        }
    }

    func refreshFromBackend() {
        pi.bootstrap { [weak self] result, error in
            guard let self else { return }
            if let error { self.notice = error.localizedDescription; return }
            guard let result else { return }
            self.applyRecords(result["records"] as? [String: Any])
            self.prepareDraftIfNeeded()
            self.refreshModelCatalog()
        }
    }

    func selectProfile(_ profileID: String) {
        selectedProfileID = profileID
        selectedTaskID = tasks.first(where: { $0.profileID == profileID && !$0.isArchived })?.id
        fullAccess = profileFullAccess[profileID] ?? false
        selectedProvider = providerForProfile(profileID)
        selectedModel = profiles.first(where: { $0.id == profileID })?.defaultModel ?? "auto"
        resetModelCatalogForProfile()
        draftProjectID = visibleProjects.first?.id
        draftWorkingDirectory = visibleProjects.first?.path ?? FileManager.default.currentDirectoryPath
        if backendCapabilities.contains("history"), let taskID = selectedTaskID {
            loadHistoryIfNeeded(taskID: taskID)
        }
        refreshModelCatalog()
    }

    /// Refresh the catalog from Pi for the active Profile. A failed refresh
    /// never makes sending impossible: the safe `auto` choice remains
    /// available and the localized status is shown inside the model menu.
    func refreshModelCatalog(showError: Bool = false) {
        let profileID = selectedProfileID
        guard !profileID.isEmpty else {
            resetModelCatalogForProfile()
            return
        }
        isLoadingModels = true
        modelCatalogError = nil
        pi.modelCatalog(profileID: profileID) { [weak self] result, error in
            guard let self else { return }
            // A profile switch can race an earlier catalog request. Never let
            // the old account's models leak into the newly selected account.
            guard self.selectedProfileID == profileID else { return }
            self.isLoadingModels = false
            if let error {
                self.resetModelCatalogForProfile()
                let message = error.localizedDescription.trimmingCharacters(in: .whitespacesAndNewlines)
                self.modelCatalogError = message.isEmpty
                    ? "模型列表读取失败，将使用自动选择。"
                    : "模型列表读取失败：\(message)"
                if showError { self.notice = self.modelCatalogError }
                return
            }
            guard let result else {
                self.resetModelCatalogForProfile()
                self.modelCatalogError = "模型列表为空，将使用自动选择。"
                if showError { self.notice = self.modelCatalogError }
                return
            }
            self.applyModelCatalog(result, profileID: profileID)
        }
    }

    private func resetModelCatalogForProfile() {
        modelOptions = []
        availableModels = ["auto"]
        let profileModel = profiles.first(where: { $0.id == selectedProfileID })?.defaultModel
        selectedModel = profileModel.flatMap { model in
            let clean = model.trimmingCharacters(in: .whitespacesAndNewlines)
            return clean.isEmpty ? nil : clean
        } ?? "auto"
        isLoadingModels = false
    }

    private func applyModelCatalog(_ result: [String: Any], profileID: String) {
        guard selectedProfileID == profileID else { return }
        let payload = (result["catalog"] as? [String: Any]) ?? result
        let provider = nonEmptyString(payload["selected_provider"])
            ?? nonEmptyString(payload["provider"])
            ?? providerForProfile(profileID)
        if !provider.isEmpty { selectedProvider = provider }

        var parsed = modelOptionsFromCatalog(payload, provider: provider)
        let selectedFromCatalog = nonEmptyString(payload["selected_model"])
        let profileDefault = profiles.first(where: { $0.id == profileID })?.defaultModel
        let preferred = selectedFromCatalog ?? profileDefault ?? "auto"

        // Keep a backend-selected model visible even when a partially stale
        // catalog omitted it; this avoids silently changing the user's choice.
        if preferred != "auto", !parsed.contains(where: { $0.id == preferred }) {
            parsed.insert(PADModelOption(id: preferred, name: preferred, provider: provider, reasoning: nil), at: 0)
        }

        modelOptions = deduplicatedModelOptions(parsed)
        availableModels = ["auto"] + modelOptions.map(\.id)
        if preferred == "auto" || availableModels.contains(preferred) {
            selectedModel = preferred
        } else {
            selectedModel = "auto"
        }
        modelCatalogError = modelOptions.isEmpty ? "未读取到可用模型，将使用自动选择。" : nil
        if modelOptions.isEmpty {
            availableModels = ["auto"]
        }
    }

    func modelDisplayName(for modelID: String) -> String {
        modelOptions.first(where: { $0.id == modelID })?.displayName ?? modelID
    }

    private func modelOptionsFromCatalog(_ payload: [String: Any], provider: String) -> [PADModelOption] {
        var options: [PADModelOption] = []

        // Preferred normalized shape: models/available_models at the top
        // level. Entries can be strings or model objects.
        if let models = payload["models"] {
            options.append(contentsOf: parseModelValues(models, fallbackProvider: provider))
        }
        if options.isEmpty, let models = payload["available_models"] {
            options.append(contentsOf: parseModelValues(models, fallbackProvider: provider))
        }

        // Compatibility shape: providers[].models, as emitted by the Pi
        // runtime adapter. Prefer the selected provider, then authenticated
        // providers if the response has no matching group.
        if let providers = payload["providers"] as? [Any] {
            var matching: [PADModelOption] = []
            var authenticated: [PADModelOption] = []
            for rawProvider in providers {
                guard let item = rawProvider as? [String: Any] else { continue }
                let providerID = nonEmptyString(item["id"])
                    ?? nonEmptyString(item["provider"])
                    ?? nonEmptyString(item["name"])
                    ?? provider
                let isAuthenticated = (item["authenticated"] as? Bool) ?? true
                guard isAuthenticated, let rawModels = item["models"] else { continue }
                let values = parseModelValues(rawModels, fallbackProvider: providerID)
                if providerID == provider {
                    matching.append(contentsOf: values)
                }
                authenticated.append(contentsOf: values)
            }
            if options.isEmpty {
                options = matching.isEmpty ? authenticated : matching
            }
        }
        return options
    }

    private func parseModelValues(_ value: Any, fallbackProvider: String) -> [PADModelOption] {
        if let values = value as? [Any] {
            return values.compactMap { parseModelValue($0, fallbackProvider: fallbackProvider) }
        }
        if let values = value as? [String: Any] {
            // A map keyed by model ID is also accepted for old Pi stores.
            return values.flatMap { key, item -> [PADModelOption] in
                if let option = parseModelValue(item, fallbackProvider: fallbackProvider) {
                    return [option]
                }
                return [PADModelOption(id: key, name: key, provider: fallbackProvider, reasoning: nil)]
            }
        }
        return parseModelValue(value, fallbackProvider: fallbackProvider).map { [$0] } ?? []
    }

    private func parseModelValue(_ value: Any, fallbackProvider: String) -> PADModelOption? {
        if let id = value as? String {
            let clean = id.trimmingCharacters(in: .whitespacesAndNewlines)
            guard !clean.isEmpty else { return nil }
            return PADModelOption(id: clean, name: clean, provider: fallbackProvider, reasoning: nil)
        }
        guard let item = value as? [String: Any] else { return nil }
        guard let id = nonEmptyString(item["id"])
            ?? nonEmptyString(item["model"])
            ?? nonEmptyString(item["model_id"])
            ?? nonEmptyString(item["value"]) else { return nil }
        let name = nonEmptyString(item["name"])
            ?? nonEmptyString(item["label"])
            ?? nonEmptyString(item["display_name"])
            ?? id
        let itemProvider = nonEmptyString(item["provider"]) ?? fallbackProvider
        let reasoning = nonEmptyString(item["reasoning"])
            ?? nonEmptyString(item["reasoning_level"])
            ?? nonEmptyString(item["thinking_level"])
        return PADModelOption(id: id, name: name, provider: itemProvider, reasoning: reasoning)
    }

    private func deduplicatedModelOptions(_ options: [PADModelOption]) -> [PADModelOption] {
        var seen = Set<String>()
        return options.filter { option in
            guard !option.id.isEmpty else { return false }
            return seen.insert(option.id).inserted
        }
    }

    private func nonEmptyString(_ value: Any?) -> String? {
        guard let string = value as? String else { return nil }
        let clean = string.trimmingCharacters(in: .whitespacesAndNewlines)
        return clean.isEmpty ? nil : clean
    }

    func send() {
        let prompt = composerText.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !prompt.isEmpty else { return }
        let runtimePrompt = promptWithAttachments(prompt)
        guard let taskID = selectedTaskID else {
            createTaskAndSend(prompt, runtimePrompt: runtimePrompt)
            return
        }
        appendUserPrompt(prompt, taskID: taskID)
        dispatchPrompt(runtimePrompt, taskID: taskID)
    }

    func stopSelectedTask() {
        guard let taskID = selectedTaskID else { return }
        pi.stopTask(taskID: taskID)
        pollingTaskIDs.remove(taskID)
        startedTaskIDs.remove(taskID)
        if let index = tasks.firstIndex(where: { $0.id == taskID }) {
            tasks[index].state = .idle
        }
    }

    func retrySelectedTask() {
        guard let taskID = selectedTaskID,
              let task = selectedTask,
              let prompt = task.messages.last(where: { $0.role == .user })?.text else { return }
        if let index = tasks.firstIndex(where: { $0.id == taskID }) { tasks[index].state = .retrying }
        dispatchPrompt(prompt, taskID: taskID)
    }

    private func createTaskAndSend(_ prompt: String, runtimePrompt: String) {
        guard !selectedProfileID.isEmpty else { return }
        let title = String(prompt.prefix(48))
        pi.createTask(profileID: selectedProfileID, projectID: draftProjectID, title: title, cwd: draftWorkingDirectory) { [weak self] result, error in
            guard let self else { return }
            if let error { self.notice = error.localizedDescription; return }
            if let records = result?["records"] as? [String: Any] { self.applyRecords(records) }
            guard let task = result?["task"] as? [String: Any], let id = task["id"] as? String else {
                self.notice = "后端未返回新任务 ID。"
                return
            }
            self.selectedFilter = .all
            self.selectedTaskID = id
            self.appendUserPrompt(prompt, taskID: id)
            self.dispatchPrompt(runtimePrompt, taskID: id)
        }
    }

    private func promptWithAttachments(_ prompt: String) -> String {
        guard !attachedPaths.isEmpty else { return prompt }
        let files = attachedPaths.map { "- \($0)" }.joined(separator: "\n")
        return """
        请把以下本地文件作为本次任务的上下文：
        \(files)

        用户请求：
        \(prompt)
        """
    }

    private func appendUserPrompt(_ prompt: String, taskID: String) {
        guard let index = tasks.firstIndex(where: { $0.id == taskID }) else { return }
        tasks[index].messages.append(Message(role: .user, text: prompt))
        tasks[index].state = .starting
        if tasks[index].title == "New task" || tasks[index].title == "新任务" {
            tasks[index].title = String(prompt.prefix(48))
        }
        composerText = ""
        if !attachedPaths.isEmpty {
            tasks[index].messages.append(Message(role: .system, text: "已附加 \(attachedPaths.count) 个上下文文件"))
            attachedPaths = []
        }
    }

    private func dispatchPrompt(_ prompt: String, taskID: String) {
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
            self.configureRuntimeAndSend(prompt, taskID: taskID)
        }
    }

    private func configureRuntimeAndSend(_ prompt: String, taskID: String) {
        pi.setThinkingLevel(taskID: taskID, level: selectedThinkingLevel) { [weak self] _, thinkingError in
            guard let self else { return }
            if let thinkingError { self.notice = thinkingError.localizedDescription }
            guard self.selectedModel != "auto" else {
                self.sendPrompt(prompt, taskID: taskID)
                return
            }
            self.pi.setModel(taskID: taskID, provider: self.selectedProvider, model: self.selectedModel) { [weak self] _, modelError in
                guard let self else { return }
                if let modelError { self.notice = modelError.localizedDescription }
                self.sendPrompt(prompt, taskID: taskID)
            }
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

    private func loadHistoryIfNeeded(taskID: String) {
        guard loadedHistoryTaskIDs.insert(taskID).inserted else { return }
        pi.history(taskID: taskID) { [weak self] result, error in
            guard let self else { return }
            // Older servers can legitimately omit history. Do not disrupt the
            // selected task; a newer bridge can populate the transcript here.
            guard error == nil, let result,
                  let messages = result["messages"] as? [[String: Any]],
                  let index = self.tasks.firstIndex(where: { $0.id == taskID }) else { return }
            let history = messages.compactMap(self.parseMessage)
            if !history.isEmpty { self.tasks[index].messages = history }
        }
    }

    private func parseMessage(_ raw: [String: Any]) -> Message? {
        let nested = raw["message"] as? [String: Any]
        guard let rawText = raw["text"] ?? raw["content"] ?? nested?["content"] ?? nested?["text"] ?? raw["message"],
              let text = extractText(rawText) else { return nil }
        let roleName = (raw["role"] as? String) ?? (nested?["role"] as? String)
        let role: Message.Role = roleName == "user" ? .user : (roleName == "system" ? .system : .assistant)
        return Message(role: role, text: text)
    }

    func respond(to interaction: PendingInteraction, value: String) {
        let response: Any
        switch interaction.responseKind {
        case "confirm":
            response = value == "y" || value == "true" || value == "允许一次"
        case "select":
            response = Int(value) ?? value
        default:
            response = value
        }
        pi.respondUI(taskID: interaction.taskID, interactionID: interaction.id, responseKind: interaction.responseKind, value: response) { [weak self] _, error in
            if let error { self?.notice = error.localizedDescription }
        }
        pendingInteraction = nil
    }

    func chooseModel(_ model: String) {
        selectedModel = model
        persistModelSelection()
        guard model != "auto", let taskID = selectedTaskID, startedTaskIDs.contains(taskID) else { return }
        pi.setModel(taskID: taskID, provider: selectedProvider, model: model) { [weak self] _, error in
            if let error { self?.notice = error.localizedDescription }
        }
    }

    func chooseProvider(_ provider: String) {
        selectedProvider = provider
        persistModelSelection()
        guard selectedModel != "auto", let taskID = selectedTaskID, startedTaskIDs.contains(taskID) else { return }
        pi.setModel(taskID: taskID, provider: provider, model: selectedModel) { [weak self] _, error in
            if let error { self?.notice = error.localizedDescription }
        }
    }

    func chooseThinkingLevel(_ level: String) {
        selectedThinkingLevel = level
        guard let taskID = selectedTaskID, startedTaskIDs.contains(taskID) else { return }
        pi.setThinkingLevel(taskID: taskID, level: level) { [weak self] _, error in
            if let error { self?.notice = error.localizedDescription }
        }
    }

    private func persistModelSelection() {
        guard !selectedProfileID.isEmpty else { return }
        pi.setProfileMetadata(profileID: selectedProfileID, provider: selectedProvider, model: selectedModel) { [weak self] _, error in
            if let error { self?.notice = error.localizedDescription }
        }
    }

    private func providerForProfile(_ profileID: String) -> String {
        guard let profile = profiles.first(where: { $0.id == profileID }) else { return "openai-codex" }
        guard profile.subtitle.hasPrefix("默认服务商：") else { return "openai-codex" }
        let provider = String(profile.subtitle.dropFirst("默认服务商：".count))
        return provider.isEmpty ? "openai-codex" : provider
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
            if state.isActive || state == .needsApproval || state == .needsInput {
                DispatchQueue.main.asyncAfter(deadline: .now() + 0.2) { [weak self] in self?.pollOnce(taskID: taskID) }
            } else {
                self.pollingTaskIDs.remove(taskID)
            }
        }
    }

    private func applyPoll(_ result: [String: Any], taskID: String) {
        if let poll = result["poll"] as? [String: Any] {
            if let events = poll["events"] as? [[String: Any]] {
                events.forEach { applyRuntimeEvent($0, taskID: taskID) }
            }
            if let messages = poll["messages"] as? [[String: Any]] {
                for message in messages where message["type"] as? String == "response" {
                    guard let value = message["value"] as? [String: Any], value["success"] as? Bool == false else { continue }
                    notice = extractText(value["error"] ?? value) ?? "Pi 请求失败"
                }
            }
            if let diagnostics = poll["diagnostics"] as? [String], !diagnostics.isEmpty {
                notice = diagnostics.joined(separator: "\n")
            }
        }
        if let runtime = result["runtime"] as? [String: Any], let status = runtime["status"] as? String,
           let index = tasks.firstIndex(where: { $0.id == taskID }) {
            tasks[index].state = taskState(status)
            let pending = (result["poll"] as? [String: Any])?["pending_ui_requests"] as? [[String: Any]]
            if let request = pending?.first(where: { ($0["requires_response"] as? Bool) ?? false }),
               let id = request["id"] as? String {
                let responseKind = request["kind"] as? String ?? "unknown"
                let kind = responseKind == "confirm" ? "审批请求" : (responseKind == "select" ? "选择" : "需要输入")
                let message = request["message"] as? String ?? request["title"] as? String ?? "Pi 需要你的输入才能继续。"
                let requestOptions = request["options"] as? [String] ?? []
                let options = responseKind == "confirm" ? ["允许一次", "拒绝"] : requestOptions
                pendingInteraction = PendingInteraction(id: id, taskID: taskID, kind: kind, message: message, options: options, responseKind: responseKind)
            } else if status == "needs_approval" || status == "needs_input" {
                // The Pi request is emitted once and subsequent polls only
                // carry the reducer status. Preserve its real request id until
                // the user answers; replacing it with a synthetic UUID would
                // make the visible approval card impossible to submit.
            } else {
                if pendingInteraction?.taskID == taskID { pendingInteraction = nil }
            }
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

    private func applyRuntimeEvent(_ event: [String: Any], taskID: String) {
        switch event["type"] as? String {
        case "message_update":
            guard let deltaEvent = event["assistantMessageEvent"] as? [String: Any],
                  deltaEvent["type"] as? String == "text_delta",
                  let delta = deltaEvent["delta"] as? String else { return }
            appendStreamingDelta(delta, taskID: taskID)
        case "message_end":
            guard let message = event["message"] as? [String: Any],
                  message["role"] as? String == "assistant",
                  let text = extractText(message["content"] ?? message["text"] ?? message),
                  !text.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else { return }
            finishStreamingMessage(text, taskID: taskID)
        case "tool_execution_start":
            let name = (event["toolName"] as? String)
                ?? (event["tool"] as? String)
                ?? "工具"
            appendSystemMessage("正在运行：\(name)", taskID: taskID)
        case "extension_error":
            notice = extractText(event["error"] ?? event) ?? "Pi 扩展执行失败"
        case "agent_settled":
            streamingMessageIDs.removeValue(forKey: taskID)
        default:
            break
        }
    }

    private func appendStreamingDelta(_ delta: String, taskID: String) {
        guard !delta.isEmpty, let taskIndex = tasks.firstIndex(where: { $0.id == taskID }) else { return }
        if let messageID = streamingMessageIDs[taskID],
           let messageIndex = tasks[taskIndex].messages.firstIndex(where: { $0.id == messageID }) {
            tasks[taskIndex].messages[messageIndex].text += delta
            return
        }
        let message = Message(role: .assistant, text: delta)
        streamingMessageIDs[taskID] = message.id
        tasks[taskIndex].messages.append(message)
    }

    private func finishStreamingMessage(_ text: String, taskID: String) {
        guard let taskIndex = tasks.firstIndex(where: { $0.id == taskID }) else { return }
        if let messageID = streamingMessageIDs.removeValue(forKey: taskID),
           let messageIndex = tasks[taskIndex].messages.firstIndex(where: { $0.id == messageID }) {
            tasks[taskIndex].messages[messageIndex].text = text
        } else if tasks[taskIndex].messages.last?.role != .assistant || tasks[taskIndex].messages.last?.text != text {
            tasks[taskIndex].messages.append(Message(role: .assistant, text: text))
        }
    }

    private func appendSystemMessage(_ text: String, taskID: String) {
        guard let taskIndex = tasks.firstIndex(where: { $0.id == taskID }) else { return }
        tasks[taskIndex].messages.append(Message(role: .system, text: text))
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
                               agentDirectory: raw["agent_dir"] as? String, sessionDirectory: raw["session_dir"] as? String,
                               credentialRef: raw["credential_ref"] as? String,
                               defaultModel: raw["default_model"] as? String)
            }
            if !parsed.isEmpty {
                profiles = parsed
                if !profiles.contains(where: { $0.id == selectedProfileID }) { selectedProfileID = profiles[0].id }
            }
            rawProfiles.forEach(recordProfilePolicy)
            fullAccess = profileFullAccess[selectedProfileID] ?? false
            selectedProvider = providerForProfile(selectedProfileID)
            selectedModel = profiles.first(where: { $0.id == selectedProfileID })?.defaultModel ?? "auto"
        }
        if let rawProjects = records["projects"] as? [[String: Any]] {
            projects = rawProjects.compactMap { raw in
                guard let id = raw["id"] as? String, let name = raw["name"] as? String else { return nil }
                return Project(id: id, name: displayProjectName(name, id: id), path: raw["primary_root"] as? String ?? "", profileID: raw["profile_id"] as? String ?? selectedProfileID)
            }
        }
        if let rawTasks = records["tasks"] as? [[String: Any]] {
            let previousMessages = Dictionary(uniqueKeysWithValues: tasks.map { ($0.id, $0.messages) })
            tasks = rawTasks.compactMap { raw in
                guard let id = raw["id"] as? String, let profileID = raw["profile_id"] as? String else { return nil }
                let title = raw["title"] as? String ?? "新任务"
                let cwd = raw["cwd"] as? String ?? ""
                let isLegacyAutoDraft = (title == "New task" || title == "新任务")
                    && (cwd.isEmpty || cwd == "/")
                    && (raw["session_file"] is NSNull || raw["session_file"] == nil)
                    && ((raw["summary"] as? String)?.isEmpty ?? true)
                if isLegacyAutoDraft { return nil }
                return DesktopTask(id: id, title: title == "New task" ? "新任务" : title, projectID: raw["project_id"] as? String,
                                   profileID: profileID, cwd: cwd, state: taskState(raw["status"] as? String ?? "idle"),
                                   isPinned: raw["pinned"] as? Bool ?? false, isArchived: raw["archived"] as? Bool ?? false,
                                   hasUnread: raw["unread"] as? Bool ?? false, messages: previousMessages[id] ?? [])
            }
            if selectedTaskID == nil || !tasks.contains(where: { $0.id == selectedTaskID }) {
                selectedTaskID = tasks.first(where: { $0.profileID == selectedProfileID && !$0.isArchived })?.id
            }
            if backendCapabilities.contains("history"), let taskID = selectedTaskID {
                loadHistoryIfNeeded(taskID: taskID)
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
        case "starting": return .starting
        case "running": return .running
        case "streaming": return .streaming
        case "tool_running": return .toolRunning
        case "retrying": return .retrying
        case "compacting": return .running
        case "needs_approval": return .needsApproval
        case "needs_input": return .needsInput
        case "failed", "disconnected": return .failed
        default: return .idle
        }
    }

    private func extractText(_ value: Any) -> String? {
        if let string = value as? String { return string }
        if let object = value as? [String: Any] {
            for key in ["text", "message", "delta", "content", "assistantMessageEvent", "event", "data", "value"] {
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
    static let sidebarMinWidth: CGFloat = 240
    static let sidebarIdealWidth: CGFloat = 275
    static let sidebarMaxWidth: CGFloat = 360
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
    @Environment(\.scenePhase) private var scenePhase
    @State private var isSidebarVisible = true
    @State private var sidebarWidth = CodexMetrics.sidebarIdealWidth
    @State private var sidebarDragStartWidth: CGFloat?

    var body: some View {
        Group {
            if isSidebarVisible {
                HStack(spacing: 0) {
                    SidebarView(model: model)
                        .frame(width: sidebarWidth)
                    TiledSidebarDivider(
                        width: $sidebarWidth,
                        dragStartWidth: $sidebarDragStartWidth
                    )
                    ConversationView(model: model)
                        .frame(minWidth: 640, maxWidth: .infinity)
                }
            } else {
                ConversationView(model: model)
            }
        }
        .toolbar {
            ToolbarItemGroup(placement: .navigation) {
                Button {
                    isSidebarVisible.toggle()
                } label: {
                    Label(isSidebarVisible ? "隐藏边栏" : "显示边栏",
                          systemImage: "sidebar.left")
                }
                .help(isSidebarVisible ? "隐藏边栏" : "显示边栏")
                Button(action: model.createTask) {
                    Label("新建任务", systemImage: "square.and.pencil")
                }
                .help("新建任务（⇧⌘N）")
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
                    model.finishPiLogin()
                }
                model.isShowingPiLogin = false
            }
        }
        .sheet(isPresented: $model.isShowingProfileWizard) {
            ProfileWizardSheet(
                name: $model.profileWizardName,
                provider: $model.profileWizardProvider,
                onCancel: { model.isShowingProfileWizard = false },
                onCreate: { model.createProfile(name: model.profileWizardName, provider: model.profileWizardProvider) }
            )
        }
        .onChange(of: scenePhase) { phase in
            if phase == .active {
                model.refreshFromBackend()
            }
        }
    }
}

struct TiledSidebarDivider: View {
    @Binding var width: CGFloat
    @Binding var dragStartWidth: CGFloat?

    var body: some View {
        Rectangle()
            .fill(Color.clear)
            .frame(width: 5)
            .overlay {
                Rectangle()
                    .fill(Color(nsColor: .separatorColor))
                    .frame(width: 1)
            }
            .contentShape(Rectangle())
            .gesture(
                DragGesture(minimumDistance: 0)
                    .onChanged { value in
                        let initialWidth = dragStartWidth ?? width
                        if dragStartWidth == nil { dragStartWidth = initialWidth }
                        width = min(
                            CodexMetrics.sidebarMaxWidth,
                            max(CodexMetrics.sidebarMinWidth, initialWidth + value.translation.width)
                        )
                    }
                    .onEnded { _ in dragStartWidth = nil }
            )
            .onHover { isHovered in
                if isHovered {
                    NSCursor.resizeLeftRight.push()
                } else {
                    NSCursor.pop()
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
                Spacer()
            }
            .padding(.horizontal, 12)
            .padding(.top, 11)
            .padding(.bottom, 8)

            VStack(spacing: 1) {
                SidebarActionRow(
                    title: "新建任务",
                    systemImage: "square.and.pencil",
                    trailingSystemImage: "plus",
                    action: model.createTask
                )
                SidebarActionRow(
                    title: "打开本地项目",
                    systemImage: "folder.badge.plus",
                    action: model.addProject
                )
            }
            .padding(.horizontal, 8)
            .padding(.bottom, 8)

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
                Menu {
                    ForEach(SidebarFilter.allCases) { filter in
                        Button {
                            model.selectedFilter = filter
                        } label: {
                            Label(filter.rawValue, systemImage: model.selectedFilter == filter ? "checkmark" : filter.symbol)
                        }
                    }
                } label: {
                    Image(systemName: "line.3.horizontal.decrease")
                        .foregroundStyle(.secondary)
                }
                .menuStyle(.borderlessButton)
                .menuIndicator(.hidden)
                .help("筛选任务")
            }
            .frame(height: 30)
            .padding(.horizontal, 9)
            .background(Color.primary.opacity(0.055), in: RoundedRectangle(cornerRadius: 8, style: .continuous))
            .padding(.horizontal, 8)
            .padding(.bottom, 8)

            ScrollView {
                LazyVStack(alignment: .leading, spacing: 14) {
                    SidebarSection(title: "项目") {
                        if model.visibleProjects.isEmpty && model.visibleTasks.isEmpty {
                            Text("暂无任务")
                                .foregroundStyle(.secondary)
                                .font(.caption)
                                .frame(maxWidth: .infinity, alignment: .leading)
                                .padding(.horizontal, 8)
                                .frame(height: 30)
                        }
                        ForEach(model.visibleProjects) { project in
                            ProjectGroup(project: project, model: model)
                        }
                    }

                    SidebarSection(title: "最近") {
                        ForEach(model.visibleTasks.filter { $0.projectID == nil }.prefix(20)) { task in
                            SidebarTaskButton(
                                task: task,
                                selected: model.selectedTaskID == task.id,
                                action: { model.select(task: task) }
                            )
                        }
                    }
                }
                .padding(.horizontal, 8)
                .padding(.bottom, 12)
            }

            ProfilePicker(model: model)
            HStack(spacing: 7) {
                Circle()
                    .fill(model.pi.state.color)
                    .frame(width: 7, height: 7)
                Text(model.pi.state.label)
                    .font(.caption2)
                    .foregroundStyle(.secondary)
                    .lineLimit(1)
                Spacer()
                if !model.selectedProfileIsLoggedIn {
                    Button("登录") { model.openPiLogin() }
                        .font(.caption2.weight(.medium))
                        .buttonStyle(.plain)
                        .foregroundStyle(Color.accentColor)
                        .controlSize(.small)
                }
            }
            .padding(.horizontal, 12)
            .padding(.top, 2)
            .padding(.bottom, 9)
        }
        .background(Color(nsColor: .underPageBackgroundColor))
    }
}

struct SidebarActionRow: View {
    let title: String
    let systemImage: String
    var trailingSystemImage: String? = nil
    let action: () -> Void
    @State private var isHovered = false

    var body: some View {
        Button(action: action) {
            HStack(spacing: 8) {
                Image(systemName: systemImage)
                    .frame(width: 16)
                Text(title)
                    .lineLimit(1)
                Spacer(minLength: 8)
                if let trailingSystemImage {
                    Image(systemName: trailingSystemImage)
                        .font(.caption2)
                        .foregroundStyle(.secondary)
                }
            }
            .font(.callout)
            .padding(.horizontal, 8)
            .frame(height: 30)
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .background(isHovered ? Color.primary.opacity(0.055) : .clear,
                    in: RoundedRectangle(cornerRadius: 8, style: .continuous))
        .onHover { isHovered = $0 }
    }
}

struct SidebarSection<Content: View>: View {
    let title: String
    @ViewBuilder let content: Content

    init(title: String, @ViewBuilder content: () -> Content) {
        self.title = title
        self.content = content()
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 3) {
            Text(title)
                .font(.caption2.weight(.medium))
                .foregroundStyle(.tertiary)
                .padding(.horizontal, 8)
                .frame(height: 20)
            VStack(spacing: 1) {
                content
            }
        }
    }
}

struct ProfilePicker: View {
    @ObservedObject var model: DesktopModel
    @State private var isHovered = false

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
                        .font(.callout.weight(.medium))
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
            .padding(.horizontal, 8)
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
                    model.startProfileWizard()
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
        .frame(maxWidth: .infinity, minHeight: 42, alignment: .leading)
        .background(isHovered ? Color.primary.opacity(0.055) : .clear,
                    in: RoundedRectangle(cornerRadius: 8, style: .continuous))
        .padding(.horizontal, 8)
        .onHover { isHovered = $0 }
    }
}

struct ProfileWizardSheet: View {
    @Binding var name: String
    @Binding var provider: String
    let onCancel: () -> Void
    let onCreate: () -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            HStack(spacing: 12) {
                Image(systemName: "person.badge.plus")
                    .font(.system(size: 27))
                    .foregroundStyle(Color.accentColor)
                VStack(alignment: .leading, spacing: 3) {
                    Text("添加 Pi 账号")
                        .font(.title3.weight(.semibold))
                    Text("先命名配置，再进入原生登录流程")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
                Spacer()
            }
            .padding(22)
            Divider()
            VStack(alignment: .leading, spacing: 16) {
                Text("账号名称")
                    .font(.subheadline.weight(.medium))
                TextField("例如：个人 OpenAI", text: $name)
                    .textFieldStyle(.roundedBorder)
                Text("服务商")
                    .font(.subheadline.weight(.medium))
                Picker("服务商", selection: $provider) {
                    Text("OpenAI / Codex").tag("openai-codex")
                    Text("Anthropic").tag("anthropic")
                    Text("Google").tag("google")
                    Text("OpenAI API").tag("openai")
                    Text("GitHub Copilot").tag("github-copilot")
                    Text("xAI").tag("xai")
                    Text("OpenRouter").tag("openrouter")
                    Text("DeepSeek").tag("deepseek")
                }
                .labelsHidden()
                .pickerStyle(.menu)
                Text("创建后会自动打开登录向导。凭据仅保存在该 Profile 的私有 Pi 目录中。")
                    .font(.callout)
                    .foregroundStyle(.secondary)
                    .fixedSize(horizontal: false, vertical: true)
                Spacer()
                HStack {
                    Spacer()
                    Button("取消", action: onCancel)
                        .keyboardShortcut(.cancelAction)
                    Button("创建并登录", action: onCreate)
                        .buttonStyle(.borderedProminent)
                        .disabled(name.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
                        .keyboardShortcut(.defaultAction)
                }
            }
            .padding(22)
        }
        .frame(width: 460, height: 360)
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
    @State private var isExpanded = true
    @State private var isHovered = false

    var body: some View {
        VStack(spacing: 1) {
            Button {
                isExpanded.toggle()
            } label: {
                HStack(spacing: 7) {
                    Image(systemName: isExpanded ? "chevron.down" : "chevron.right")
                        .font(.system(size: 9, weight: .semibold))
                        .foregroundStyle(.tertiary)
                        .frame(width: 10)
                    Image(systemName: "folder")
                        .foregroundStyle(.secondary)
                        .frame(width: 16)
                    Text(project.name)
                        .lineLimit(1)
                    Spacer(minLength: 0)
                }
                .font(.callout)
                .padding(.horizontal, 8)
                .frame(height: 30)
                .contentShape(Rectangle())
            }
            .buttonStyle(.plain)
            .background(isHovered ? Color.primary.opacity(0.055) : .clear,
                        in: RoundedRectangle(cornerRadius: 8, style: .continuous))
            .help(project.path)
            .onHover { isHovered = $0 }

            if isExpanded {
                ForEach(model.visibleTasks.filter { $0.projectID == project.id }) { task in
                    SidebarTaskButton(
                        task: task,
                        selected: model.selectedTaskID == task.id,
                        action: { model.select(task: task) }
                    )
                    .padding(.leading, 17)
                }
            }
        }
    }
}

struct SidebarTaskButton: View {
    let task: DesktopTask
    let selected: Bool
    let action: () -> Void
    @State private var isHovered = false

    var body: some View {
        Button(action: action) {
            TaskRow(task: task)
                .padding(.horizontal, 8)
                .frame(height: 30)
                .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .background(selected ? Color.primary.opacity(0.09) : (isHovered ? Color.primary.opacity(0.055) : .clear),
                    in: RoundedRectangle(cornerRadius: 8, style: .continuous))
        .onHover { isHovered = $0 }
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
                .lineLimit(1)
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
                    if let interaction = model.pendingInteraction {
                        InteractionCard(interaction: interaction, model: model)
                    }
                    Composer(model: model)
                } else if model.hasDraft {
                    DraftTaskView(model: model)
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
                HStack(spacing: 8) {
                    Label("Pi 引擎", systemImage: "bolt.horizontal.circle")
                        .foregroundStyle(model.pi.state.color)
                    Text("·")
                        .foregroundStyle(.tertiary)
                    if model.selectedProfileIsLoggedIn {
                        Label(model.selectedProfile.name, systemImage: "person.crop.circle.fill")
                            .foregroundStyle(.secondary)
                    } else {
                        Button("登录账号") { model.openPiLogin() }
                            .buttonStyle(.link)
                            .font(.caption)
                    }
                    if let state = model.selectedTaskState {
                        Text("·")
                            .foregroundStyle(.tertiary)
                        Label(state.label, systemImage: state.symbol)
                            .foregroundStyle(state.color)
                    }
                }
                .font(.caption)
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
        state.label
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
    @FocusState private var isFocused: Bool

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack(alignment: .bottom, spacing: 10) {
                Button(action: model.chooseAttachments) {
                    Image(systemName: "plus")
                        .font(.callout.weight(.medium))
                        .frame(width: 28, height: 28)
                }
                .buttonStyle(.plain)
                .foregroundStyle(.secondary)
                .help("添加上下文文件")
                TextField("向 Pi 发送消息…", text: $model.composerText, axis: .vertical)
                    .textFieldStyle(.plain)
                    .lineLimit(1...6)
                    .focused($isFocused)
                    .onSubmit {
                        if !NSEvent.modifierFlags.contains(.shift) { model.send() }
                    }
                Button {
                    if model.selectedTaskState?.isActive == true {
                        model.stopSelectedTask()
                    } else if model.selectedTaskState == .failed {
                        model.retrySelectedTask()
                    } else {
                        model.send()
                    }
                } label: {
                    Image(systemName: model.selectedTaskState?.isActive == true ? "stop.circle.fill" : (model.selectedTaskState == .failed ? "arrow.clockwise.circle.fill" : "arrow.up.circle.fill"))
                        .font(.title2)
                        .foregroundStyle(model.selectedTaskState?.isActive == true ? Color.red : (model.composerText.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty && model.selectedTaskState != .failed ? Color.secondary : Color.accentColor))
                }
                .buttonStyle(.plain)
                .disabled(model.selectedTaskState?.isActive != true && model.selectedTaskState != .failed && model.composerText.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
                .help(model.selectedTaskState?.isActive == true ? "停止任务" : (model.selectedTaskState == .failed ? "重试" : "发送"))
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
                if model.hasDraft {
                    ProjectContextPicker(model: model)
                    WorkingDirectoryButton(model: model)
                } else if let task = model.selectedTask, !task.cwd.isEmpty {
                    Label(URL(fileURLWithPath: task.cwd).lastPathComponent, systemImage: "folder")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                        .help(task.cwd)
                }
                Spacer()
                ModelPicker(model: model)
                Text(model.selectedTaskState?.label ?? "草稿")
                    .font(.caption)
                    .foregroundStyle(model.selectedTaskState?.color ?? .secondary)
                    .lineLimit(1)
                Text("回车发送")
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
        .onAppear { isFocused = true }
        .onChange(of: model.selectedTaskID) { _ in isFocused = true }
    }
}

struct DraftTaskView: View {
    @ObservedObject var model: DesktopModel

    var body: some View {
        VStack(spacing: 12) {
            Image(systemName: "sparkles")
                .font(.system(size: 34))
                .foregroundStyle(.purple)
            Text("新任务")
                .font(.title2.weight(.semibold))
            Text("输入第一条消息后，PAD 才会创建任务并写入本地 Store。")
                .font(.callout)
                .foregroundStyle(.secondary)
            HStack(spacing: 7) {
                Image(systemName: "folder")
                Text(model.draftWorkingDirectory)
                    .lineLimit(1)
                if let project = model.projects.first(where: { $0.id == model.draftProjectID }) {
                    Text("· \(project.name)")
                        .foregroundStyle(.secondary)
                }
            }
            .font(.caption)
            .foregroundStyle(.secondary)
            .padding(.horizontal, 12)
            .padding(.vertical, 8)
            .background(.quaternary.opacity(0.5), in: Capsule())
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .padding(36)
    }
}

struct InteractionCard: View {
    let interaction: PendingInteraction
    @ObservedObject var model: DesktopModel
    @State private var input = ""

    var body: some View {
        VStack(alignment: .leading, spacing: 9) {
            Label(interaction.kind, systemImage: interaction.kind == "审批请求" ? "checkmark.shield" : "questionmark.circle")
                .font(.subheadline.weight(.semibold))
                .foregroundStyle(.orange)
            Text(interaction.message)
                .font(.callout)
                .textSelection(.enabled)
            if interaction.responseKind == "confirm" {
                HStack {
                    Button("允许一次") { model.respond(to: interaction, value: "y") }
                        .buttonStyle(.borderedProminent)
                    Button("拒绝") { model.respond(to: interaction, value: "n") }
                        .buttonStyle(.bordered)
                }
            } else if interaction.responseKind == "select" {
                HStack {
                    ForEach(Array(interaction.options.enumerated()), id: \.offset) { index, option in
                        // Pi's select response is an option index, not the
                        // display string. Keep the label human-readable while
                        // sending the protocol's stable numeric value.
                        Button(option) { model.respond(to: interaction, value: String(index)) }
                            .buttonStyle(.bordered)
                    }
                }
            } else {
                HStack {
                    TextField("输入后继续", text: $input)
                        .textFieldStyle(.roundedBorder)
                    Button("提交") { model.respond(to: interaction, value: input) }
                        .buttonStyle(.borderedProminent)
                        .disabled(input.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
                }
            }
        }
        .padding(14)
        .frame(maxWidth: CodexMetrics.composerMaxWidth, alignment: .leading)
        .background(Color.orange.opacity(0.10), in: RoundedRectangle(cornerRadius: 12))
        .padding(.horizontal, 24)
        .padding(.top, 10)
    }
}

struct ProjectContextPicker: View {
    @ObservedObject var model: DesktopModel

    var body: some View {
        Menu {
            Button("不绑定项目") { model.selectDraftProject(nil) }
            ForEach(model.projects.filter { $0.profileID == model.selectedProfileID }) { project in
                Button {
                    model.selectDraftProject(project)
                } label: {
                    Label(project.name, systemImage: project.id == model.draftProjectID ? "checkmark" : "folder")
                }
            }
        } label: {
            Label("项目", systemImage: "folder")
                .font(.caption)
        }
        .menuStyle(.borderlessButton)
        .help("选择项目")
    }
}

struct WorkingDirectoryButton: View {
    @ObservedObject var model: DesktopModel

    var body: some View {
        Button(action: model.chooseWorkingDirectory) {
            Label("目录", systemImage: "folder.badge.gearshape")
                .font(.caption)
        }
        .buttonStyle(.plain)
        .foregroundStyle(.secondary)
        .help("选择工作目录")
    }
}

struct ModelPicker: View {
    @ObservedObject var model: DesktopModel

    var body: some View {
        Menu {
            Section("服务商") {
                Label(model.selectedProvider, systemImage: "server.rack")
            }
            Section("模型") {
                if model.isLoadingModels {
                    Label("正在读取模型…", systemImage: "arrow.triangle.2.circlepath")
                }
                Button {
                    model.chooseModel("auto")
                } label: {
                    Label("自动选择", systemImage: model.selectedModel == "auto" ? "checkmark" : "cpu")
                }
                ForEach(model.modelOptions) { option in
                    Button {
                        model.chooseModel(option.id)
                    } label: {
                        Label(option.displayName, systemImage: option.id == model.selectedModel ? "checkmark" : "cpu")
                    }
                    .help(option.reasoning.map { "思考等级：\($0)" } ?? option.id)
                }
                if let error = model.modelCatalogError {
                    Text(error)
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
                Button {
                    model.refreshModelCatalog(showError: true)
                } label: {
                    Label("刷新模型列表", systemImage: "arrow.clockwise")
                }
            }
            Section("思考强度") {
                ForEach(["off", "minimal", "low", "medium", "high", "xhigh", "max"], id: \.self) { level in
                    Button {
                        model.chooseThinkingLevel(level)
                    } label: {
                        Label(thinkingLevelLabel(level), systemImage: level == model.selectedThinkingLevel ? "checkmark" : "brain")
                    }
                }
            }
        } label: {
            Label(model.selectedModel == "auto" ? "自动 · \(thinkingLevelLabel(model.selectedThinkingLevel))" : "\(model.modelDisplayName(for: model.selectedModel)) · \(thinkingLevelLabel(model.selectedThinkingLevel))", systemImage: "cpu")
                .font(.caption)
        }
        .menuStyle(.borderlessButton)
        .help("选择服务商和模型")
    }

    private func thinkingLevelLabel(_ level: String) -> String {
        switch level {
        case "off": return "关闭思考"
        case "minimal": return "极简"
        case "low": return "低"
        case "high": return "高"
        case "xhigh": return "很高"
        case "max": return "最高"
        default: return "中"
        }
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
