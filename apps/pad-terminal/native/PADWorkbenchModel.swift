#if os(macOS)
import Combine
import Foundation

/// App-level state for the native workbench. Owns the single host transport and
/// exposes the frozen API from `host/PROTOCOL.md` for `PADWorkbenchView`.
///
/// This type never starts model work on its own: `start()` only loads the
/// snapshot. Login and prompts happen only when the user invokes the matching
/// method. Secrets passed to `respondAuth` stay in memory and go straight to the
/// transport; they are never written to `UserDefaults` or published state.
@MainActor
final class PADWorkbenchModel: ObservableObject {
    static let shared = PADWorkbenchModel()

    @Published private(set) var snapshot = PADSnapshot()
    @Published private(set) var selectedWorkspaceId: String?
    @Published private(set) var selectedTaskId: String?
    @Published private(set) var activeProfileId: String?
    @Published private(set) var catalog = PADCatalog()
    @Published private(set) var messages: [PADChatItem] = []
    @Published var draft: String = ""
    @Published private(set) var errorMessage: String?
    @Published private(set) var proxyWarning: String?
    @Published private(set) var connectionStatus = "offline"
    @Published private(set) var authState: PADAuthState?
    @Published private(set) var historyLoading = false
    @Published private(set) var catalogLoading = false
    @Published private(set) var openaiRefreshing = false
    @Published private var pendingModelTaskIds: Set<String> = []
    @Published private var pendingAccountProfileIds: Set<String> = []

    var selectedWorkspace: PADWorkspace? {
        snapshot.workspaces.first { $0.id == selectedWorkspaceId }
    }

    var selectedTask: PADTask? {
        snapshot.tasks.first { $0.id == selectedTaskId }
    }

    /// The first existing profile is the default; stored IDs and credentials stay untouched.
    var defaultProfileId: String? { snapshot.profiles.first?.id }

    var isHistoricalAccount: Bool {
        guard let activeProfileId else { return false }
        return activeProfileId != defaultProfileId
    }

    var accountContextLabel: String {
        guard activeProfileId != nil else { return "暂无账号" }
        let name = snapshot.profiles.first { $0.id == activeProfileId }?.name ?? "未命名"
        return isHistoricalAccount ? "历史任务账号 · \(name)" : "默认账号"
    }

    var isBusy: Bool {
        guard let task = selectedTask else { return false }
        if task.status == "running" || task.status == "starting" { return true }
        return promptingTaskIds.contains(task.id)
    }

    var hasOpenAIDiscovery: Bool {
        catalog.profileId == activeProfileId && catalog.openaiDiscovery != nil &&
            catalog.providers.contains { $0.id == "openai" && $0.authenticated }
    }

    var canConfigureModels: Bool {
        guard let profileId = activeProfileId else { return false }
        return connectionStatus == "ready" && catalog.profileId == profileId &&
            !catalogLoading && !authInProgress(for: profileId) &&
            !pendingAccountProfileIds.contains(profileId) && !hasActiveTask(profileId: profileId) &&
            !snapshot.tasks.contains { $0.profileId == profileId && pendingModelTaskIds.contains($0.id) }
    }

    var canSyncOpenAI: Bool { hasOpenAIDiscovery && canConfigureModels }

    func syncOpenAIModels() {
        guard canSyncOpenAI, let profileId = activeProfileId else { return }
        loadCatalog(profileId: profileId, refreshOpenAI: true)
    }

    var canSend: Bool {
        guard let task = selectedTask, !isBusy else { return false }
        guard !authInProgress(for: task.profileId),
              !pendingAccountProfileIds.contains(task.profileId),
              !pendingModelTaskIds.contains(task.id) else { return false }
        guard !draft.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else { return false }
        guard connectionStatus == "ready", !catalogLoading, catalog.profileId == task.profileId else { return false }
        return catalog.models.contains { $0.provider == task.provider && $0.id == task.modelId && $0.isSelectable }
    }

    /// A running login for the task's profile blocks new model work on that
    /// profile; the host enforces the same rule, this keeps the UI honest.
    private func authInProgress(for profileId: String) -> Bool {
        guard let state = authState, state.profileId == profileId else { return false }
        return state.phase == "running"
    }

    // MARK: - Private state

    private enum SelectionKey: String {
        case workspace = "selectedWorkspaceId"
        case task = "selectedTaskId"
        case profile = "selectedProfileId"
    }

    private static let defaultsPrefix = "pad.terminal.workbench."
    private static let coalesceInterval: TimeInterval = 0.2

    private var transport: PADHostTransport?
    private var taskMessages: [String: [PADChatItem]] = [:]
    private var streams: [String: PADStreamState] = [:]
    private var promptingTaskIds: Set<String> = []
    private var selectionGeneration = 0
    private var catalogGeneration = 0
    private var accountContextGeneration = 0
    private var connectionGeneration = 0
    private var authGeneration = 0
    private var activeAuthAttemptId: String?
    private var authSuppressedProfile: String?
    private var historyInFlight: Set<String> = []
    private var historyReloadPending: Set<String> = []
    private var coalesceTimer: Timer?
    private var pendingPublish = false
    private var appStarted = false

    private init() {}

    // MARK: - Lifecycle

    func start() {
        guard !appStarted else { return }
        appStarted = true
        connectionStatus = "connecting"
        proxyWarning = nil
        let transport = PADHostTransport()
        transport.onEvent = { [weak self] event in self?.handle(event) }
        transport.onExit = { [weak self] status in self?.handleExit(status) }
        self.transport = transport
        do {
            try transport.start()
        } catch {
            connectionStatus = "error"
            setError(error.localizedDescription)
            return
        }
        reload()
    }

    func reload() {
        if transport?.isRunning != true {
            appStarted = false
            start()
            return
        }
        loadSnapshot()
        if let profileId = activeProfileId { loadCatalog(profileId: profileId) }
    }

    // MARK: - Metadata commands

    func addWorkspace(path: String) {
        let trimmed = path.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else { return }
        request("add_workspace", fields: ["path": .string(trimmed)]) { [weak self] result in
            guard let self else { return }
            switch result {
            case let .success(data):
                guard let workspace = data.decoded(PADWorkspace.self) else {
                    self.setError("添加项目返回格式无效。")
                    return
                }
                if let index = self.snapshot.workspaces.firstIndex(where: { $0.id == workspace.id }) {
                    self.snapshot.workspaces[index] = workspace
                } else {
                    self.snapshot.workspaces.append(workspace)
                }
                self.selectedWorkspaceId = workspace.id
                self.persistSelections()
            case let .failure(error):
                self.fail(error)
            }
        }
    }

    func addProfile(name: String) {
        let trimmed = name.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else { return }
        request("add_profile", fields: ["name": .string(trimmed)]) { [weak self] result in
            guard let self else { return }
            switch result {
            case let .success(data):
                guard let profile = data.decoded(PADProfile.self) else {
                    self.setError("添加配置返回格式无效。")
                    return
                }
                if let index = self.snapshot.profiles.firstIndex(where: { $0.id == profile.id }) {
                    self.snapshot.profiles[index] = profile
                } else {
                    self.snapshot.profiles.append(profile)
                }
            case let .failure(error):
                self.fail(error)
            }
        }
    }

    func createTask(title: String) {
        guard let workspaceId = selectedWorkspaceId else {
            setError("请先选择项目。")
            return
        }
        guard let profileId = defaultProfileId else {
            setError("默认账号尚未就绪。")
            return
        }
        var fields: [String: PADJSONValue] = [
            "workspaceId": .string(workspaceId),
            "profileId": .string(profileId),
        ]
        let trimmed = title.trimmingCharacters(in: .whitespacesAndNewlines)
        if !trimmed.isEmpty { fields["title"] = .string(trimmed) }
        request("create_task", fields: fields) { [weak self] result in
            guard let self else { return }
            switch result {
            case let .success(data):
                guard let task = data.decoded(PADTask.self) else {
                    self.setError("创建任务返回格式无效。")
                    return
                }
                if let index = self.snapshot.tasks.firstIndex(where: { $0.id == task.id }) {
                    self.snapshot.tasks[index] = task
                } else {
                    self.snapshot.tasks.append(task)
                }
                self.selectTask(task.id)
            case let .failure(error):
                self.fail(error)
            }
        }
    }

    func setModel(provider: String, modelId: String) {
        guard let task = selectedTask else {
            setError("请先选择任务。")
            return
        }
        guard canConfigureModels,
              catalog.models.contains(where: { $0.provider == provider && $0.id == modelId && $0.isSelectable }) else {
            setError("当前模型不可选择，或账号仍有进行中的操作。")
            return
        }
        pendingModelTaskIds.insert(task.id)
        let contextGeneration = accountContextGeneration
        let connectionGeneration = self.connectionGeneration
        let fields: [String: PADJSONValue] = [
            "taskId": .string(task.id),
            "provider": .string(provider),
            "modelId": .string(modelId),
        ]
        request("set_model", fields: fields) { [weak self] result in
            guard let self else { return }
            guard self.connectionGeneration == connectionGeneration else { return }
            self.pendingModelTaskIds.remove(task.id)
            guard self.accountContextGeneration == contextGeneration,
                  self.selectedTaskId == task.id, self.activeProfileId == task.profileId,
                  self.connectionStatus == "ready" else { return }
            switch result {
            case let .success(data):
                guard let updated = data.decoded(PADTask.self),
                      updated.id == task.id, updated.profileId == task.profileId else {
                    self.setError("模型切换返回格式无效。")
                    return
                }
                self.replace(task: updated)
            case let .failure(error):
                self.fail(error)
            }
        }
    }

    // MARK: - Explicit local history integration (no credentials / model work)

    func localSessionRequest<T: Decodable>(
        _ command: String, fields: [String: PADJSONValue] = [:],
        completion: @escaping (Result<T, Error>) -> Void
    ) {
        guard ["local_sessions", "local_session_history", "local_session_resume", "local_session_import_pi"].contains(command) else {
            completion(.failure(PADHostFailure(message: "无效的本地会话操作。")))
            return
        }
        request(command, fields: fields, timeout: 60) { result in
            completion(result.flatMap { data in
                guard let decoded = data.decoded(T.self) else {
                    return .failure(PADHostFailure(message: "本地会话响应格式无效。"))
                }
                return .success(decoded)
            })
        }
    }

    func acceptLocalImport(_ imported: PADLocalImport) {
        apply(snapshot: imported.snapshot)
        selectTask(imported.task.id)
    }

    // MARK: - Selection

    func selectWorkspace(_ id: String) {
        guard snapshot.workspaces.contains(where: { $0.id == id }) else { return }
        if let task = selectedTask, task.workspaceId != id {
            selectDefaultAccount()
        }
        selectedWorkspaceId = id
        persistSelections()
    }

    func selectTask(_ id: String) {
        guard let task = snapshot.tasks.first(where: { $0.id == id }) else { return }
        selectionGeneration += 1
        selectedTaskId = id
        selectedWorkspaceId = task.workspaceId
        setEffectiveProfile(task.profileId)
        persistSelections()
        cancelPendingPublish()
        errorMessage = nil
        historyLoading = false
        messages = taskMessages[id] ?? []
        loadHistory(taskId: id, force: true)
    }

    /// Leave a task's historical account context without exposing a profile picker.
    func selectDefaultAccount() {
        let id = defaultProfileId
        selectionGeneration += 1
        selectedTaskId = nil
        setEffectiveProfile(id)
        cancelPendingPublish()
        errorMessage = nil
        historyLoading = false
        messages = []
        persistSelections()
    }

    // MARK: - Prompt / abort

    func send() {
        guard let task = selectedTask else {
            setError("请先选择任务。")
            return
        }
        guard !isBusy else {
            setError("任务正在运行，请等待或先停止。")
            return
        }
        if authInProgress(for: task.profileId) {
            setError("该配置正在登录，暂时无法发送。")
            return
        }
        guard canSend else {
            setError("请先为任务选择模型，并确保配置可用。")
            return
        }
        let text = draft.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !text.isEmpty else { return }
        let sentDraft = draft

        let localId = "local-\(UUID().uuidString)"
        var transcript = taskMessages[task.id] ?? []
        transcript.append(PADChatItem(id: localId, role: "user", text: text, toolName: nil, status: nil))
        taskMessages[task.id] = PADMessageReducer.bound(transcript)
        publish(taskId: task.id)

        promptingTaskIds.insert(task.id)
        let fields: [String: PADJSONValue] = [
            "taskId": .string(task.id),
            "message": .string(text),
        ]
        request("prompt", fields: fields, timeout: 60) { [weak self] result in
            guard let self else { return }
            switch result {
            case .success:
                // Clear only the exact draft that was accepted; never discard
                // edits the user made while waiting, and never touch another
                // task's draft after a selection change.
                if self.selectedTaskId == task.id, self.draft == sentDraft {
                    self.draft = ""
                    self.errorMessage = nil
                }
                // Stay pending until an authoritative running status or
                // `agent_settled` arrives, so canSend cannot briefly reopen.
            case let .failure(error):
                self.promptingTaskIds.remove(task.id)
                var current = self.taskMessages[task.id] ?? []
                current.removeAll { $0.id == localId }
                self.taskMessages[task.id] = current
                self.publish(taskId: task.id)
                self.fail(error)
            }
        }
    }

    func abort() {
        guard let taskId = selectedTaskId else { return }
        request("abort", fields: ["taskId": .string(taskId)], timeout: 20) { [weak self] result in
            if case let .failure(error) = result { self?.fail(error) }
        }
    }

    // MARK: - Auth

    func beginAuth(provider: String, method: String) {
        guard let profileId = activeProfileId else {
            setError("请先选择配置。")
            return
        }
        guard !provider.isEmpty, !method.isEmpty else { return }
        if hasActiveTask(profileId: profileId) || catalogLoading || pendingAccountProfileIds.contains(profileId) ||
            snapshot.tasks.contains(where: { $0.profileId == profileId && pendingModelTaskIds.contains($0.id) }) {
            setError("该配置仍有进行中的操作，请等待或停止后再登录。")
            return
        }
        invalidateCatalog()
        // Keep provider/login controls intact, but never reuse an account
        // discovery list across a new OpenAI login attempt.
        if provider == "openai", catalog.openaiDiscovery != nil {
            catalog.models.removeAll { $0.provider == "openai" }
            catalog.openaiDiscovery = PADOpenAIDiscovery(state: "not_loaded")
        }
        authGeneration += 1
        let generation = authGeneration
        let attemptId = UUID().uuidString
        activeAuthAttemptId = attemptId
        authSuppressedProfile = nil
        authState = PADAuthState(
            profileId: profileId, provider: provider, method: method,
            phase: "running", attemptId: attemptId
        )
        let fields: [String: PADJSONValue] = [
            "profileId": .string(profileId),
            "provider": .string(provider),
            "method": .string(method),
            "attemptId": .string(attemptId),
        ]
        request("auth_begin", fields: fields, timeout: 30) { [weak self] result in
            guard let self else { return }
            guard self.authGeneration == generation, self.activeAuthAttemptId == attemptId else { return }
            switch result {
            case let .success(data):
                if let state = data.decoded(PADAuthState.self) {
                    // A synchronous SDK prompt may arrive before the begin response.
                    if self.authState?.promptId != nil, state.promptId == nil, state.phase == "running" { return }
                    self.applyAuth(state)
                }
            case let .failure(error):
                self.cancelAuth()
                self.fail(error)
            }
        }
    }

    func respondAuth(promptId: String, value: String) {
        guard let state = authState, state.promptId == promptId else { return }
        guard !value.isEmpty else { return }
        let attemptId = state.attemptId
        let fields: [String: PADJSONValue] = [
            "profileId": .string(state.profileId),
            "promptId": .string(promptId),
            "value": .string(value),
        ]
        request("auth_respond", fields: fields, timeout: 30) { [weak self] result in
            guard let self else { return }
            switch result {
            case .success:
                // Only clear the prompt on the still-active attempt.
                guard let current = self.authState, current.promptId == promptId else { return }
                if let attemptId, let active = self.activeAuthAttemptId, attemptId != active { return }
                self.authState?.promptId = nil
                self.authState?.placeholder = nil
                self.authState?.options = nil
            case let .failure(error):
                self.fail(error)
            }
        }
    }

    func cancelAuth() {
        guard let profileId = activeProfileId else { return }
        let attemptId = activeAuthAttemptId
        // Supersede the attempt so late events/responses cannot touch the UI,
        // while the cancel itself still names the attempt it is aborting.
        authGeneration += 1
        authSuppressedProfile = profileId
        activeAuthAttemptId = nil
        if let state = authState, state.profileId == profileId {
            authState = PADAuthState(
                profileId: state.profileId, provider: state.provider, method: state.method,
                phase: "cancelled", message: state.message, attemptId: attemptId
            )
        }
        var fields: [String: PADJSONValue] = ["profileId": .string(profileId)]
        if let attemptId { fields["attemptId"] = .string(attemptId) }
        request("auth_cancel", fields: fields, timeout: 30) { [weak self] result in
            if case let .failure(error) = result { self?.fail(error) }
        }
    }

    func logout(provider: String) {
        guard let profileId = activeProfileId else { return }
        if hasActiveTask(profileId: profileId) || authInProgress(for: profileId) ||
            pendingAccountProfileIds.contains(profileId) ||
            snapshot.tasks.contains(where: { $0.profileId == profileId && pendingModelTaskIds.contains($0.id) }) {
            setError("该配置仍有进行中的操作，暂时无法退出登录。")
            return
        }
        invalidateCatalog()
        catalog = PADCatalog(profileId: profileId)
        pendingAccountProfileIds.insert(profileId)
        let contextGeneration = accountContextGeneration
        let connectionGeneration = self.connectionGeneration
        let fields: [String: PADJSONValue] = [
            "profileId": .string(profileId),
            "provider": .string(provider),
        ]
        request("logout", fields: fields, timeout: 30) { [weak self] result in
            guard let self else { return }
            guard self.connectionGeneration == connectionGeneration else { return }
            self.pendingAccountProfileIds.remove(profileId)
            guard self.accountContextGeneration == contextGeneration,
                  self.activeProfileId == profileId, self.connectionStatus == "ready" else { return }
            self.loadCatalog(profileId: profileId)
            switch result {
            case .success:
                break
            case let .failure(error):
                self.fail(error)
            }
        }
    }

    // MARK: - Commands

    private func loadSnapshot() {
        request("snapshot", timeout: 20) { [weak self] result in
            guard let self else { return }
            switch result {
            case let .success(data):
                guard let snapshot = data.decoded(PADSnapshot.self) else {
                    self.setError("工作台快照格式无效。")
                    return
                }
                self.apply(snapshot: snapshot)
                self.connectionStatus = "ready"
            case let .failure(error):
                self.fail(error)
            }
        }
    }

    /// Every effective-account transition supersedes the old request, even
    /// when the last catalog happens to match the account being revisited.
    private func setEffectiveProfile(_ profileId: String?) {
        guard activeProfileId != profileId else {
            if let profileId, catalog.profileId != profileId, !catalogLoading {
                loadCatalog(profileId: profileId)
            }
            return
        }
        if authState?.phase == "running" { cancelAuth() }
        accountContextGeneration += 1
        invalidateCatalog()
        activeProfileId = profileId
        if authState?.profileId != profileId { authState = nil }
        catalog = PADCatalog(profileId: profileId ?? "")
        if let profileId { loadCatalog(profileId: profileId) }
    }

    private func invalidateCatalog() {
        catalogGeneration += 1
        catalogLoading = false
        openaiRefreshing = false
    }

    private func loadCatalog(profileId: String, refreshOpenAI: Bool = false) {
        guard activeProfileId == profileId, !authInProgress(for: profileId),
              !pendingAccountProfileIds.contains(profileId) else { return }
        // An offline reload must not supersede an explicit metadata sync.
        guard !openaiRefreshing || refreshOpenAI else { return }
        catalogGeneration += 1
        let generation = catalogGeneration
        let contextGeneration = accountContextGeneration
        catalogLoading = true
        openaiRefreshing = refreshOpenAI
        var fields: [String: PADJSONValue] = ["profileId": .string(profileId)]
        if refreshOpenAI { fields["refreshOpenAI"] = .bool(true) }
        request("catalog", fields: fields, timeout: 30) { [weak self] result in
            guard let self, self.catalogGeneration == generation,
                  self.accountContextGeneration == contextGeneration,
                  self.activeProfileId == profileId else { return }
            self.catalogLoading = false
            self.openaiRefreshing = false
            switch result {
            case let .success(data):
                if let catalog = data.decoded(PADCatalog.self), catalog.profileId == profileId {
                    self.catalog = catalog
                } else {
                    self.setError("模型目录返回格式无效。")
                }
            case let .failure(error):
                self.fail(error)
            }
        }
    }

    private func loadHistory(taskId: String, force: Bool = false) {
        // Coalesce duplicate requests: `apply(snapshot:)` fires on every status
        // change, and a second in-flight history load would keep bumping the
        // selection generation without ever applying a result.
        if historyInFlight.contains(taskId) {
            if force { historyReloadPending.insert(taskId) }
            return
        }
        historyInFlight.insert(taskId)
        selectionGeneration += 1
        let generation = selectionGeneration
        if selectedTaskId == taskId { historyLoading = true }
        request("history", fields: ["taskId": .string(taskId)], timeout: 60) { [weak self] result in
            guard let self else { return }
            let needsReload = self.historyReloadPending.remove(taskId) != nil
            self.historyInFlight.remove(taskId)
            if self.selectionGeneration == generation {
                if self.selectedTaskId == taskId { self.historyLoading = false }
                switch result {
                case let .success(data):
                    let parsed = PADMessageReducer.parseHistory(data)
                    var authoritative = parsed.items
                    if parsed.truncated {
                        authoritative.insert(PADChatItem(
                            id: "history-truncated", role: "system",
                            text: "历史较长，仅显示最近的消息。", toolName: nil, status: nil
                        ), at: 0)
                    }
                    let existing = self.taskMessages[taskId] ?? []
                    let streamId = self.streams[taskId]?.messageId
                    let streamText = streamId.flatMap { id in existing.last { $0.id == id }?.text }
                    let merged = PADMessageReducer.mergeHistory(
                        authoritative, existing: existing,
                        activeStreamId: streamId, activeStreamText: streamText
                    )
                    self.taskMessages[taskId] = merged
                    if self.selectedTaskId == taskId { self.publish(taskId: taskId) }
                case let .failure(error):
                    // A late failure must not overwrite another selection's error.
                    if self.selectedTaskId == taskId { self.fail(error) }
                }
            }
            if needsReload && self.selectedTaskId == taskId {
                self.loadHistory(taskId: taskId, force: true)
            }
        }
    }

    private func request(
        _ command: String,
        fields: [String: PADJSONValue] = [:],
        timeout: TimeInterval = 30,
        _ completion: @escaping (Result<PADJSONValue, Error>) -> Void
    ) {
        guard let transport else {
            completion(.failure(PADHostFailure(message: "工作台宿主未启动。")))
            return
        }
        transport.send(command: command, fields: fields, timeout: timeout, completion: completion)
    }

    // MARK: - Snapshot handling

    private func apply(snapshot newSnapshot: PADSnapshot) {
        let hadTask = selectedTaskId != nil
        snapshot = newSnapshot

        let workspaceIds = Set(newSnapshot.workspaces.map(\.id))
        let taskIds = Set(newSnapshot.tasks.map(\.id))

        // An accepted prompt stays pending until an authoritative running
        // status (or settled/error) clears it, so canSend cannot reopen in the
        // gap between acceptance and the task actually starting.
        promptingTaskIds = promptingTaskIds.filter { id in
            guard let task = newSnapshot.tasks.first(where: { $0.id == id }) else { return false }
            return task.status != "running" && task.status != "starting"
        }

        if let id = selectedWorkspaceId, !workspaceIds.contains(id) { selectedWorkspaceId = nil }
        if let id = selectedTaskId, !taskIds.contains(id) {
            selectionGeneration += 1
            selectedTaskId = nil
            historyLoading = false
            if hadTask { cancelPendingPublish(); messages = [] }
        }

        if selectedTaskId == nil,
           let stored = Self.storedSelection(.task), taskIds.contains(stored) {
            selectedTaskId = stored
            messages = taskMessages[stored] ?? []
        }

        let effectiveProfileId = selectedTask?.profileId ?? defaultProfileId
        setEffectiveProfile(effectiveProfileId)
        if let task = selectedTask {
            selectedWorkspaceId = task.workspaceId
        } else {
            if selectedWorkspaceId == nil {
                selectedWorkspaceId = Self.storedSelection(.workspace)
                    .flatMap { workspaceIds.contains($0) ? $0 : nil } ?? newSnapshot.workspaces.first?.id
            }
            // A standalone persisted profile selection never overrides the default.
        }
        persistSelections()

        if let taskId = selectedTaskId, taskMessages[taskId] == nil {
            loadHistory(taskId: taskId)
        }
    }

    private func replace(task updated: PADTask) {
        guard let index = snapshot.tasks.firstIndex(where: { $0.id == updated.id }) else {
            snapshot.tasks.append(updated)
            return
        }
        snapshot.tasks[index] = updated
    }

    private func hasActiveTask(profileId: String) -> Bool {
        snapshot.tasks.contains {
            $0.profileId == profileId && ($0.status == "running" || $0.status == "starting" || promptingTaskIds.contains($0.id))
        }
    }

    // MARK: - Event handling

    private func handle(_ event: PADHostTransport.Event) {
        switch event {
        case let .snapshot(snapshot): apply(snapshot: snapshot)
        case let .pi(taskId, data): handlePi(taskId: taskId, data: data)
        case let .auth(state): applyAuth(state)
        case .systemProxyNotAdopted:
            proxyWarning = "未采用 macOS 系统代理：PAD 不支持其绕过规则或自动代理（PAC）。请显式配置 HTTP_PROXY/HTTPS_PROXY 和 NO_PROXY；未配置显式代理的路由保持直连。"
        }
    }

    private func handlePi(taskId: String, data: PADJSONValue) {
        var transcript = taskMessages[taskId] ?? []
        var stream = streams[taskId]
        let result = PADMessageReducer.applyPiEvent(
            data, taskId: taskId, transcript: &transcript, stream: &stream
        )
        streams[taskId] = stream
        taskMessages[taskId] = transcript

        if let status = result.statusHint,
           let index = snapshot.tasks.firstIndex(where: { $0.id == taskId }) {
            switch status {
            case "idle":
                if snapshot.tasks[index].status != "error" { snapshot.tasks[index].status = "idle" }
            case "error":
                snapshot.tasks[index].status = "error"
            case "running":
                if snapshot.tasks[index].status == "idle" { snapshot.tasks[index].status = "running" }
            default:
                break
            }
        }
        if let errorText = result.errorText { setError(errorText) }

        if result.settled || result.errorText != nil || result.statusHint == "running" {
            promptingTaskIds.remove(taskId)
        }

        if result.immediateFlush {
            publish(taskId: taskId)
        } else {
            schedulePublish(taskId: taskId)
        }
        if result.settled, selectedTaskId == taskId {
            loadHistory(taskId: taskId, force: true)
        }
    }

    private func applyAuth(_ state: PADAuthState) {
        // A superseded attempt must never mutate the current UI, even if a late
        // success/error event arrives after the user cancelled or retried.
        if let attemptId = state.attemptId, attemptId != activeAuthAttemptId { return }
        if authSuppressedProfile == state.profileId, state.phase == "running" { return }
        if state.phase != "running" { authSuppressedProfile = nil }
        guard state.profileId == activeProfileId else { return }
        authState = state
        if state.phase == "succeeded" {
            activeAuthAttemptId = nil
            loadCatalog(profileId: state.profileId)
        } else if state.phase != "running" {
            activeAuthAttemptId = nil
        }
    }

    private func handleExit(_ status: Int32?) {
        connectionStatus = (status ?? 0) == 0 ? "offline" : "error"
        if let status, status != 0 {
            setError("工作台宿主已退出（code \(status)）。")
        } else {
            setError("工作台宿主已退出。")
        }
        // Drop in-flight streams, coalescing, and cached transcripts so a
        // restart reloads authoritative history instead of resurrecting stale
        // running items or silently keeping optimistic replies.
        cancelPendingPublish()
        streams.removeAll()
        taskMessages.removeAll()
        historyInFlight.removeAll()
        historyReloadPending.removeAll()
        messages = []
        for index in snapshot.tasks.indices
        where snapshot.tasks[index].status == "running" || snapshot.tasks[index].status == "starting" {
            snapshot.tasks[index].status = "error"
        }
        authGeneration += 1
        accountContextGeneration += 1
        connectionGeneration += 1
        invalidateCatalog()
        catalog = PADCatalog(profileId: activeProfileId ?? "")
        authState = nil
        activeAuthAttemptId = nil
        authSuppressedProfile = nil
        promptingTaskIds.removeAll()
        pendingModelTaskIds.removeAll()
        pendingAccountProfileIds.removeAll()
        historyLoading = false
    }

    // MARK: - Publishing / coalescing

    private func schedulePublish(taskId: String) {
        guard selectedTaskId == taskId, !pendingPublish else { return }
        pendingPublish = true
        coalesceTimer?.invalidate()
        let timer = Timer(timeInterval: Self.coalesceInterval, repeats: false) { [weak self] _ in
            Task { @MainActor in self?.flushPendingPublish() }
        }
        coalesceTimer = timer
        // Common modes keep streaming alive while the user drags a splitter.
        RunLoop.main.add(timer, forMode: .common)
    }

    private func flushPendingPublish() {
        pendingPublish = false
        coalesceTimer = nil
        guard let taskId = selectedTaskId else { return }
        messages = taskMessages[taskId] ?? []
    }

    /// Immediate authoritative publish (final/settled/history/selection).
    private func publish(taskId: String) {
        guard selectedTaskId == taskId else { return }
        cancelPendingPublish()
        messages = taskMessages[taskId] ?? []
    }

    private func cancelPendingPublish() {
        coalesceTimer?.invalidate()
        coalesceTimer = nil
        pendingPublish = false
    }

    // MARK: - Errors / persistence

    private func fail(_ error: Error) {
        setError(error.localizedDescription)
    }

    private func setError(_ message: String) {
        errorMessage = message
    }

    private static func storedSelection(_ key: SelectionKey) -> String? {
        UserDefaults.standard.string(forKey: defaultsPrefix + key.rawValue)
    }

    /// Persists only preview selection ids; never secrets, prompts, or drafts.
    private func persistSelections() {
        let defaults = UserDefaults.standard
        defaults.set(selectedWorkspaceId, forKey: Self.defaultsPrefix + SelectionKey.workspace.rawValue)
        defaults.set(selectedTaskId, forKey: Self.defaultsPrefix + SelectionKey.task.rawValue)
        defaults.set(activeProfileId, forKey: Self.defaultsPrefix + SelectionKey.profile.rawValue)
    }
}
#endif
