import type {
  AuthResultDto,
  DesktopBootstrapResult,
  DesktopEvent as ProtocolEvent,
  DesktopRecords,
  DesktopRequestParams,
  DesktopUiStateDto,
  DesktopUiStateResultDto,
  PadDesktopApi,
  ProfileDto,
  ProjectDto,
  RemotePairBeginResultDto,
  RemoteStatusResultDto,
  TaskDto,
  TerminalOpenDto,
  TerminalSnapshotDto,
} from "../../../shared/protocol";
import type {
  AccountSummary,
  AuthPhase,
  AuthSession,
  AuthType,
  DesktopAdapter,
  DesktopEvent,
  DesktopSnapshot,
  DesktopUiState,
  InteractionResponse,
  PendingInteraction,
  ProjectSummary,
  ProviderAuthentication,
  RemoteHostState,
  RemoteHostStatus,
  RemotePairing,
  SendMessageInput,
  SidebarHierarchy,
  SidebarNodeKind,
  SidebarRow,
  TaskStatus,
  TaskSummary,
  TerminalPane,
  TerminalSize,
  TerminalSnapshot,
  TurnArtifact,
  TurnArtifactKind,
  TurnArtifactOperation,
  TurnEntry,
  TurnKind,
} from "../types";
import { localizePiAuthOption, localizePiAuthPrompt } from "./labels";
import {
  modelCatalogWithFallback,
  parseModelCatalog,
  type ModelCatalog,
} from "./model-catalog";

declare global {
  interface Window {
    padDesktop?: PadDesktopApi;
  }
}

interface ProviderStatus {
  providerAuthentication: ProviderAuthentication;
  authenticatedProviders: string[];
  selectedProvider: string | null;
  selectedModel: string | null;
  modelCatalog: ModelCatalog;
}

type AuthAction = "auth_begin" | "auth_status" | "auth_respond" | "auth_cancel" | "logout";

const remoteGatewayCapability = "remote_gateway_v1";
const remotePairingCapability = "remote_pairing";
const remoteDeviceCapability = "remote_device_management";

const listeners = new Set<(event: DesktopEvent) => void>();

function emit(event: DesktopEvent) {
  listeners.forEach((listener) => listener(event));
}

export function createBridgeAdapter(api: PadDesktopApi): DesktopAdapter {
  let bootstrap: DesktopBootstrapResult | null = null;
  let selectedProfileId: string | null = null;
  let rawSidebar: unknown = null;
  let uiState = defaultUiState();
  let uiStateWriteQueue: Promise<void> = Promise.resolve();
  let providerStatuses = new Map<string, ProviderStatus>();
  let snapshot = emptySnapshot();
  let remoteStatus: RemoteHostStatus | null = null;
  const pollingTasks = new Map<string, ReturnType<typeof setTimeout>>();
  let refreshQueued = false;
  let accountTransitionInProgress = false;

  function requireStableAccount() {
    if (accountTransitionInProgress) throw new Error("账号正在切换，请等待完成后重试。");
  }

  async function runAccountTransition<T>(operation: () => Promise<T>): Promise<T> {
    requireStableAccount();
    accountTransitionInProgress = true;
    try {
      return await operation();
    } finally {
      accountTransitionInProgress = false;
    }
  }

  const requestAuth = <A extends AuthAction>(action: A, params: DesktopRequestParams[A]): Promise<AuthResultDto> =>
    api.request<A, AuthResultDto>(action, params);

  async function bootstrapWithCapabilities(): Promise<DesktopBootstrapResult> {
    const result = await api.bootstrap();
    const hello = await api.request<"hello", { capabilities?: string[] }>("hello", {}).catch(() => ({ capabilities: [] }));
    return { ...result, capabilities: [...new Set([...result.capabilities, ...(hello.capabilities ?? [])])] };
  }

  function supportsRemote(capability = remoteGatewayCapability): boolean {
    return bootstrap?.capabilities.includes(capability) ?? false;
  }

  async function refreshRemoteStatus(shouldEmit = false): Promise<RemoteHostStatus | null> {
    if (!supportsRemote()) {
      remoteStatus = null;
    } else {
      const result = await api.request<"remote_status", RemoteStatusResultDto>("remote_status", {});
      remoteStatus = mapRemoteStatus(result);
    }
    snapshot = { ...snapshot, remote: remoteStatus };
    if (shouldEmit) emit({ type: "remote-updated", status: remoteStatus });
    return remoteStatus;
  }

  async function tryRefreshRemoteStatus(): Promise<RemoteHostStatus | null> {
    try {
      return await refreshRemoteStatus();
    } catch {
      remoteStatus = null;
      snapshot = { ...snapshot, remote: null };
      return null;
    }
  }

  async function refresh(loadHistory = false, refreshAuthentication = false): Promise<DesktopSnapshot> {
    if (!bootstrap) bootstrap = await bootstrapWithCapabilities();
    const result = await api.request<"list_sidebar", { sidebar?: unknown; ui_state?: DesktopUiStateDto; records: DesktopRecords }>("list_sidebar", {});
    const records = result.records ?? bootstrap.records;
    rawSidebar = result.sidebar ?? rawSidebar ?? bootstrap.sidebar;
    if (result.ui_state) uiState = mapUiState(result.ui_state);
    bootstrap = { ...bootstrap, records, sidebar: rawSidebar, ui_state: result.ui_state ?? bootstrap.ui_state };
    selectedProfileId = uiState.activeProfileId ?? selectedProfileId ?? bootstrap.profile.id;
    if (refreshAuthentication || providerStatuses.size === 0) {
      providerStatuses = await loadProviderStatuses(api, records, bootstrap, refreshAuthentication, providerStatuses);
    }

    const next = snapshotFromRecords(
      records,
      selectedProfileId,
      bootstrap,
      rawSidebar,
      providerStatuses,
      snapshot.turnsByTask,
      snapshot.interactionsByTask,
      uiState,
      remoteStatus,
    );
    if (loadHistory) await hydrateVisibleHistory(api, next);
    snapshot = next;
    return next;
  }

  function scheduleRefresh() {
    if (refreshQueued) return;
    refreshQueued = true;
    queueMicrotask(() => {
      refreshQueued = false;
      void refresh(true, true).then((next) => emit({ type: "snapshot", snapshot: next })).catch(() => undefined);
    });
  }

  function handleProtocolEvent(event: ProtocolEvent) {
    const rawEvent = event as unknown as Record<string, unknown>;
    if (rawEvent.type === "menu_action") {
      const action = textValue(rawEvent.action);
      if (["new_task", "search", "settings", "toggle_sidebar", "toggle_terminal"].includes(action)) {
        emit({ type: "menu-action", action: action as "new_task" | "search" | "settings" | "toggle_sidebar" | "toggle_terminal" });
      }
    } else if (event.type === "backend_event") {
      if (desktopServerEventKind(event.payload) === "remote_changed") {
        void refreshRemoteStatus(true).catch(() => {
          remoteStatus = null;
          snapshot = { ...snapshot, remote: null };
          emit({ type: "remote-updated", status: null });
        });
        return;
      }
      const auth = authSessionFromEvent(event.payload, selectedProfileId ?? "");
      if (auth) emit({ type: "auth-updated", session: auth });
      scheduleRefresh();
    } else if (event.status === "ready") {
      scheduleRefresh();
    }
  }

  function scheduleTaskPoll(taskId: string, delay = 120) {
    if (pollingTasks.has(taskId)) return;
    const timer = setTimeout(async () => {
      pollingTasks.delete(taskId);
      try {
        const pollResult = await api.request<"poll", unknown>("poll", { task_id: taskId });
        const next = await refresh(false);
        const discovered = pendingInteractionsFromPoll(pollResult);
        if (discovered.length > 0) {
          next.interactionsByTask[taskId] = mergePendingInteractions(next.interactionsByTask[taskId] ?? [], discovered);
        }
        if (next.tasks.some((task) => task.id === taskId)) next.turnsByTask[taskId] = await loadTaskHistory(api, taskId);
        snapshot = next;
        emit({ type: "snapshot", snapshot: next });
        const task = next.tasks.find((candidate) => candidate.id === taskId);
        if (task?.status === "running") scheduleTaskPoll(taskId, 220);
      } catch {
        // The next server event or explicit send will establish fresh state.
      }
    }, delay);
    pollingTasks.set(taskId, timer);
  }

  function persistUiState(patch: Partial<DesktopUiState>): Promise<DesktopUiState> {
    let operation!: Promise<DesktopUiState>;
    operation = uiStateWriteQueue.then(async () => {
      const nextState = { ...uiState, ...patch };
      if (patch.sidebarView !== undefined && bootstrap) {
        const profileId = nextState.activeProfileId ?? selectedProfileId;
        const visibleTasks = bootstrap.records.tasks
          .filter((task) => task.profile_id === profileId && taskVisibleInSidebarView(task, nextState.sidebarView))
          .sort((left, right) => right.updated_at - left.updated_at);
        if (!visibleTasks.some((task) => task.id === nextState.selectedTaskId)) {
          nextState.selectedTaskId = visibleTasks[0]?.id ?? null;
        }
      }
      const result = await api.request<"set_ui_state", DesktopUiStateResultDto>("set_ui_state", {
        state: uiStateToDto(nextState),
      });
      uiState = mapUiState(result.state);
      rawSidebar = result.sidebar ?? rawSidebar;
      selectedProfileId = uiState.activeProfileId ?? selectedProfileId;
      if (bootstrap) bootstrap = { ...bootstrap, ui_state: result.state, sidebar: rawSidebar };
      snapshot = snapshotFromRecords(
        bootstrap?.records ?? { profiles: [], projects: [], tasks: [] },
        selectedProfileId,
        bootstrap,
        rawSidebar,
        providerStatuses,
        snapshot.turnsByTask,
        snapshot.interactionsByTask,
        uiState,
        remoteStatus,
      );
      emit({ type: "snapshot", snapshot });
      return uiState;
    });
    uiStateWriteQueue = operation.then(() => undefined, () => undefined);
    return operation;
  }

  return {
    async loadSnapshot() {
      bootstrap = await bootstrapWithCapabilities();
      uiState = mapUiState(bootstrap.ui_state);
      selectedProfileId = uiState.activeProfileId ?? bootstrap.profile.id;
      rawSidebar = bootstrap.sidebar;
      providerStatuses = await loadProviderStatuses(api, bootstrap.records, bootstrap, false, new Map());
      snapshot = snapshotFromRecords(bootstrap.records, selectedProfileId, bootstrap, rawSidebar, providerStatuses, {}, {}, uiState, null);
      await tryRefreshRemoteStatus();
      await hydrateVisibleTaskData(api, snapshot);
      snapshot.tasks.filter((task) => task.status === "running").forEach((task) => scheduleTaskPoll(task.id));
      return snapshot;
    },
    async loadTaskData(taskId) {
      requireStableAccount();
      const task = snapshot.tasks.find((candidate) => candidate.id === taskId);
      if (!task || task.profileId !== selectedProfileId) throw new Error("当前账号无权访问该任务。");
      const next: DesktopSnapshot = {
        ...snapshot,
        turnsByTask: { ...snapshot.turnsByTask },
        interactionsByTask: { ...snapshot.interactionsByTask },
      };
      await hydrateTaskData(api, next, task, true);
      snapshot = next;
      emit({ type: "snapshot", snapshot: next });
      return next;
    },
    chooseProjectDirectory() {
      return api.chooseProjectDirectory();
    },
    async chooseAttachments() {
      if (!api.chooseAttachments) throw new Error("当前 PAD 版本未提供原生附件选择器，请升级后重试。");
      return normalizeAttachmentPaths(await api.chooseAttachments());
    },
    async createProject(name, directory) {
      requireStableAccount();
      if (!bootstrap) bootstrap = await bootstrapWithCapabilities();
      const profileId = selectedProfileId ?? bootstrap.profile.id;
      const result = await api.request<"create_project", { project: ProjectDto; sidebar?: unknown; records: DesktopRecords }>("create_project", {
        profile_id: profileId,
        name,
        cwd: directory,
      });
      bootstrap = { ...bootstrap, records: result.records };
      rawSidebar = result.sidebar ?? rawSidebar;
      snapshot = snapshotFromRecords(
        result.records,
        profileId,
        bootstrap,
        rawSidebar,
        providerStatuses,
        snapshot.turnsByTask,
        snapshot.interactionsByTask,
        uiState,
        remoteStatus,
      );
      emit({ type: "snapshot", snapshot });
      return snapshot;
    },
    async createTask(projectId) {
      requireStableAccount();
      if (!bootstrap) bootstrap = await bootstrapWithCapabilities();
      if (uiState.sidebarView !== "all") {
        await persistUiState({ sidebarView: "all", selectedTaskId: null });
      }
      const profileId = selectedProfileId ?? bootstrap.profile.id;
      const project = bootstrap.records.projects.find((candidate) => candidate.id === projectId);
      const result = await api.request<"create_task", { task: TaskDto; sidebar?: unknown; records?: DesktopRecords }>("create_task", {
        project_id: projectId ?? undefined,
        profile_id: profileId,
        title: "新任务",
        cwd: project?.primary_root,
        environment: "local",
      });
      if (result.records) bootstrap = { ...bootstrap, records: result.records };
      rawSidebar = result.sidebar ?? rawSidebar;
      const task = mapTask(result.task);
      snapshot = snapshotFromRecords(
        bootstrap.records,
        profileId,
        bootstrap,
        rawSidebar,
        providerStatuses,
        snapshot.turnsByTask,
        snapshot.interactionsByTask,
        uiState,
        remoteStatus,
      );
      snapshot.turnsByTask[task.id] = [];
      snapshot.interactionsByTask[task.id] = [];
      emit({ type: "snapshot", snapshot });
      return task;
    },
    async createAccount(name, provider) {
      return runAccountTransition(async () => {
        const result = await api.request<"create_profile", { profile: ProfileDto; sidebar?: unknown; records: DesktopRecords }>("create_profile", {
          name,
          default_provider: provider,
        });
        if (!bootstrap) bootstrap = await bootstrapWithCapabilities();
        bootstrap = { ...bootstrap, records: result.records };
        rawSidebar = result.sidebar ?? rawSidebar;
        selectedProfileId = result.profile.id;
        providerStatuses = await loadProviderStatuses(api, result.records, bootstrap, true, providerStatuses);
        remoteStatus = null;
        snapshot = { ...snapshot, remote: null };
        await persistUiState({ activeProfileId: selectedProfileId, selectedTaskId: null });
        snapshot = snapshotFromRecords(result.records, selectedProfileId, bootstrap, rawSidebar, providerStatuses, {}, {}, uiState, null);
        await tryRefreshRemoteStatus();
        emit({ type: "snapshot", snapshot });
        return snapshot;
      });
    },
    async sendMessage(input: SendMessageInput) {
      requireStableAccount();
      if (!bootstrap) bootstrap = await bootstrapWithCapabilities();
      if (input.accountId !== selectedProfileId) throw new Error("账号已切换，请在当前账号中重新选择任务。");
      const task = snapshot.tasks.find((candidate) => candidate.id === input.taskId);
      if (!task || task.profileId !== input.accountId) throw new Error("当前账号无权访问该任务。");

      const provider = input.provider.trim();
      const model = input.model.trim();
      if (!!provider !== !!model) throw new Error("请同时填写模型提供商和模型名称。");
      const attachmentPaths = normalizeAttachmentPaths(input.attachmentPaths);
      const prompt = promptWithAttachments(input.text, attachmentPaths);
      if (!prompt) throw new Error("请输入任务内容。");

      let pendingModelSelection: { status: ProviderStatus } | null = null;
      if (provider && model) {
        const result = await api.request<"set_profile", { sidebar?: unknown; records?: DesktopRecords }>("set_profile", {
          profile_id: input.accountId,
          default_provider: provider,
          default_model: model,
        });
        if (result.records) bootstrap = { ...bootstrap, records: result.records };
        rawSidebar = result.sidebar ?? rawSidebar;
        const currentStatus = providerStatuses.get(input.accountId) ?? fallbackProviderStatus(
          bootstrap.records.profiles.find((profile) => profile.id === input.accountId) ?? bootstrap.profile,
          bootstrap,
        );
        const selectedCatalog: ModelCatalog = {
          ...currentStatus.modelCatalog,
          selectedProvider: provider,
          selectedModel: model,
        };
        pendingModelSelection = {
          status: {
            ...currentStatus,
            selectedProvider: provider,
            selectedModel: model,
            modelCatalog: selectedCatalog,
          },
        };
      }
      await api.request("prompt", {
        task_id: input.taskId,
        prompt,
        ...(provider && model ? { provider, model } : {}),
        ...(input.thinkingLevel !== "default" ? { thinking_level: input.thinkingLevel } : {}),
        fast_mode: input.fastMode,
      });
      if (pendingModelSelection) {
        providerStatuses = new Map(providerStatuses).set(input.accountId, pendingModelSelection.status);
        snapshot = snapshotFromRecords(
          bootstrap.records,
          selectedProfileId,
          bootstrap,
          rawSidebar,
          providerStatuses,
          snapshot.turnsByTask,
          snapshot.interactionsByTask,
          uiState,
          remoteStatus,
        );
        emit({ type: "snapshot", snapshot });
      }
      const turn: TurnEntry = { id: `local-${Date.now()}`, kind: "user", body: prompt, meta: "刚刚" };
      emit({ type: "turn-added", taskId: input.taskId, turn });
      scheduleTaskPoll(input.taskId);
    },
    async switchAccount(accountId) {
      return runAccountTransition(async () => {
        if (!bootstrap) bootstrap = await bootstrapWithCapabilities();
        if (!bootstrap.records.profiles.some((profile) => profile.id === accountId)) throw new Error("账号不存在或已经移除。");
        if (accountId === selectedProfileId) return snapshot;

        const previousProfileId = selectedProfileId;
        const previousUiState: DesktopUiState = {
          ...uiState,
          collapsedSectionIds: [...uiState.collapsedSectionIds],
          collapsedProjectIds: [...uiState.collapsedProjectIds],
        };
        const previousBootstrap = bootstrap;
        const previousRawSidebar = rawSidebar;
        const previousProviderStatuses = providerStatuses;
        const previousSnapshot = snapshot;
        const previousRemoteStatus = remoteStatus;
        let persistedTarget = false;

        try {
          remoteStatus = null;
          snapshot = { ...snapshot, remote: null };
          await persistUiState({ activeProfileId: accountId, selectedTaskId: null });
          persistedTarget = true;
          const next = await refresh(true, true);
          await tryRefreshRemoteStatus();
          snapshot = next;
          snapshot = { ...snapshot, remote: remoteStatus };
          return snapshot;
        } catch (error) {
          let rollbackError: unknown = null;
          if (persistedTarget) {
            try {
              await api.request<"set_ui_state", DesktopUiStateResultDto>("set_ui_state", {
                state: uiStateToDto(previousUiState),
              });
            } catch (failure) {
              rollbackError = failure;
            }
          }

          selectedProfileId = previousProfileId;
          uiState = previousUiState;
          bootstrap = previousBootstrap;
          rawSidebar = previousRawSidebar;
          providerStatuses = previousProviderStatuses;
          remoteStatus = previousRemoteStatus;
          snapshot = previousSnapshot;
          emit({ type: "snapshot", snapshot: previousSnapshot });

          if (rollbackError) {
            const original = error instanceof Error ? error.message : String(error);
            const rollback = rollbackError instanceof Error ? rollbackError.message : String(rollbackError);
            throw new Error(`账号切换失败，原账号状态回滚也未能写回：${original}；${rollback}`);
          }
          throw error;
        }
      });
    },
    async setFullAccess(accountId, enabled) {
      requireStableAccount();
      const result = await api.request<"set_profile", { sidebar?: unknown; records?: DesktopRecords }>("set_profile", {
        profile_id: accountId,
        permission_mode: enabled ? "system_full" : "guarded",
        unattended: enabled,
      });
      if (bootstrap && result.records) bootstrap = { ...bootstrap, records: result.records };
      rawSidebar = result.sidebar ?? rawSidebar;
      return refresh(false, false);
    },
    async beginLogin(accountId, provider, authType = "oauth") {
      requireStableAccount();
      const selectedProvider = provider ?? bootstrap?.records.profiles.find((profile) => profile.id === accountId)?.default_provider ?? null;
      if (!selectedProvider) throw new Error("请先选择要登录的模型提供商。");
      const result = await requestAuth("auth_begin", { profile_id: accountId, provider: selectedProvider, auth_type: authType });
      const session = parseAuthSession(result, accountId, selectedProvider, authType);
      emit({ type: "auth-updated", session });
      return session;
    },
    async getLoginStatus(accountId) {
      requireStableAccount();
      const current = authSessionsByProfile.get(accountId);
      const result = await requestAuth("auth_status", current?.attemptId ? { attempt_id: current.attemptId } : { profile_id: accountId });
      const session = parseAuthSession(result, accountId, current?.provider ?? null, current?.authType);
      authSessionsByProfile.set(accountId, session);
      emit({ type: "auth-updated", session });
      if (session.phase === "authenticated") await refresh(false, true);
      return session;
    },
    async respondLogin(accountId, value) {
      requireStableAccount();
      const current = authSessionsByProfile.get(accountId);
      if (!current?.attemptId || !current.promptId) throw new Error("登录会话已失效，请重新开始。");
      const result = await requestAuth("auth_respond", {
        attempt_id: current.attemptId,
        prompt_id: current.promptId,
        value,
      });
      const session = parseAuthSession(result, accountId, current.provider, current.authType);
      authSessionsByProfile.set(accountId, session);
      emit({ type: "auth-updated", session });
      if (session.phase === "authenticated") emit({ type: "snapshot", snapshot: await refresh(false, true) });
      return session;
    },
    async cancelLogin(accountId) {
      requireStableAccount();
      const current = authSessionsByProfile.get(accountId);
      if (current?.attemptId) await requestAuth("auth_cancel", { attempt_id: current.attemptId });
      authSessionsByProfile.delete(accountId);
    },
    async logout(accountId, provider) {
      requireStableAccount();
      const selectedProvider = provider ?? providerStatuses.get(accountId)?.selectedProvider;
      if (!selectedProvider) throw new Error("当前账号没有可退出的模型账号。");
      await requestAuth("logout", { profile_id: accountId, provider: selectedProvider });
      providerStatuses.delete(accountId);
      return refresh(false, true);
    },
    async abortTask(taskId) {
      requireStableAccount();
      const task = snapshot.tasks.find((candidate) => candidate.id === taskId);
      if (!task) throw new Error("当前账号无权访问该任务。");
      await api.request("stop_task", { task_id: taskId });
      const next = await refresh(false);
      emit({ type: "snapshot", snapshot: next });
      return next;
    },
    async retryTask(taskId) {
      requireStableAccount();
      const task = snapshot.tasks.find((candidate) => candidate.id === taskId);
      if (!task) throw new Error("当前账号无权访问该任务。");
      await api.request("retry_task", { task_id: taskId });
      scheduleTaskPoll(taskId);
      const next = await refresh(false);
      emit({ type: "snapshot", snapshot: next });
      return next;
    },
    async respondInteraction(taskId, interactionId, value) {
      requireStableAccount();
      const task = snapshot.tasks.find((candidate) => candidate.id === taskId);
      if (!task) throw new Error("当前账号无权访问该任务。");
      const interaction = snapshot.interactionsByTask[taskId]?.find((candidate) => candidate.id === interactionId);
      if (!interaction || !interaction.requiresResponse) throw new Error("这项交互已经失效，请等待任务状态刷新。");
      validateInteractionResponse(interaction, value);
      await api.request("respond_ui", {
        task_id: taskId,
        request_id: interaction.id,
        response_kind: interaction.kind,
        value,
      });
      snapshot = {
        ...snapshot,
        interactionsByTask: {
          ...snapshot.interactionsByTask,
          [taskId]: (snapshot.interactionsByTask[taskId] ?? []).filter((candidate) => candidate.id !== interaction.id),
        },
      };
      emit({ type: "snapshot", snapshot });
      scheduleTaskPoll(taskId);
    },
    async updateTask(taskId, patch) {
      requireStableAccount();
      const task = snapshot.tasks.find((candidate) => candidate.id === taskId);
      if (!task) throw new Error("当前账号无权访问该任务。");
      const result = await api.request<"set_task", { sidebar?: unknown; records?: DesktopRecords }>("set_task", {
        task_id: taskId,
        title: patch.title,
        pinned: patch.pinned,
        archived: patch.archived,
        unread: patch.unread,
      });
      if (bootstrap && result.records) bootstrap = { ...bootstrap, records: result.records };
      rawSidebar = result.sidebar ?? rawSidebar;
      const next = await refresh(false);
      emit({ type: "snapshot", snapshot: next });
      return next;
    },
    async openTerminal(taskId, size) {
      requireStableAccount();
      const result = await api.request<"terminal_open", TerminalOpenDto>("terminal_open", {
        task_id: taskId,
        label: "任务终端",
        columns: clampTerminalColumns(size.columns),
        rows: clampTerminalRows(size.rows),
      });
      return mapTerminalPane(result);
    },
    async writeTerminal(paneId, data) {
      requireStableAccount();
      await api.request("terminal_input", { pane_id: paneId, data });
    },
    async resizeTerminal(paneId, size) {
      requireStableAccount();
      await api.request("terminal_resize", {
        pane_id: paneId,
        columns: clampTerminalColumns(size.columns),
        rows: clampTerminalRows(size.rows),
      });
    },
    async getTerminalSnapshot(paneId) {
      const result = await api.request<"terminal_snapshot", TerminalSnapshotDto>("terminal_snapshot", { pane_id: paneId });
      return mapTerminalSnapshot(result);
    },
    async closeTerminal(paneId) {
      await api.request("terminal_close", { pane_id: paneId });
    },
    updateUiState(patch) {
      requireStableAccount();
      return persistUiState(patch);
    },
    getRemoteStatus() {
      requireStableAccount();
      return refreshRemoteStatus(true);
    },
    async setRemoteEnabled(enabled) {
      requireStableAccount();
      if (!supportsRemote()) throw new Error("当前 PAD 控制面不支持远程连接。");
      const result = await api.request<"remote_set_enabled", RemoteStatusResultDto>("remote_set_enabled", { enabled });
      remoteStatus = mapRemoteStatus(result);
      snapshot = { ...snapshot, remote: remoteStatus };
      emit({ type: "remote-updated", status: remoteStatus });
      return remoteStatus;
    },
    async beginRemotePairing() {
      requireStableAccount();
      if (!supportsRemote(remotePairingCapability)) throw new Error("当前 PAD 控制面不支持设备配对。");
      const result = await api.request<"remote_pair_begin", RemotePairBeginResultDto>("remote_pair_begin", {});
      return mapRemotePairing(result);
    },
    async cancelRemotePairing(pairingId) {
      requireStableAccount();
      if (!supportsRemote(remotePairingCapability)) throw new Error("当前 PAD 控制面不支持设备配对。");
      const result = await api.request<"remote_pair_cancel", RemoteStatusResultDto>("remote_pair_cancel", { pairing_id: pairingId });
      remoteStatus = mapRemoteStatus(result);
      snapshot = { ...snapshot, remote: remoteStatus };
      emit({ type: "remote-updated", status: remoteStatus });
      return remoteStatus;
    },
    async revokeRemoteDevice(deviceId) {
      requireStableAccount();
      if (!supportsRemote(remoteDeviceCapability)) throw new Error("当前 PAD 控制面不支持设备撤销。");
      const result = await api.request<"remote_device_revoke", RemoteStatusResultDto>("remote_device_revoke", { device_id: deviceId });
      remoteStatus = mapRemoteStatus(result);
      snapshot = { ...snapshot, remote: remoteStatus };
      emit({ type: "remote-updated", status: remoteStatus });
      return remoteStatus;
    },
    subscribe(listener) {
      listeners.add(listener);
      const unsubscribeProtocol = api.subscribe(handleProtocolEvent);
      return () => {
        listeners.delete(listener);
        unsubscribeProtocol();
      };
    },
  };
}

function clampTerminalColumns(value: number): number {
  return Math.min(240, Math.max(20, Math.round(value) || 100));
}

function clampTerminalRows(value: number): number {
  return Math.min(80, Math.max(4, Math.round(value) || 20));
}

function mapTerminalPane(value: TerminalOpenDto): TerminalPane {
  return {
    paneId: value.pane_id,
    taskId: value.task_id,
    epoch: value.epoch,
    status: value.status,
    size: { columns: clampTerminalColumns(value.size.columns), rows: clampTerminalRows(value.size.rows) },
  };
}

function mapTerminalSnapshot(value: TerminalSnapshotDto): TerminalSnapshot {
  return {
    paneId: value.pane_id,
    taskId: value.task_id,
    epoch: value.epoch,
    revision: value.revision,
    status: value.status,
    isOpen: value.is_open,
    size: { columns: clampTerminalColumns(value.size.columns), rows: clampTerminalRows(value.size.rows) },
    lines: value.lines.slice(-80).map((line) => String(line)),
    cursor: value.cursor,
    mode: {
      alternateScreen: value.mode.alternate_screen,
      bracketedPaste: value.mode.bracketed_paste,
      mouseReporting: value.mode.mouse_reporting,
      applicationCursor: value.mode.application_cursor,
    },
    error: value.error,
    exit: value.exit,
  };
}

function desktopServerEventKind(value: unknown): string {
  if (!isRecord(value)) return "";
  const envelope = isRecord(value.event) ? value.event : value;
  return textValue(envelope.kind);
}

function publicText(value: unknown, maximum: number): string {
  if (typeof value !== "string") return "";
  return value.replace(/[\u0000-\u001f\u007f]/g, "").slice(0, maximum);
}

function publicTimestamp(value: unknown): number {
  return typeof value === "number" && Number.isFinite(value) && value >= 0 ? value : 0;
}

function mapRemoteState(value: unknown): RemoteHostState {
  return ["disabled", "starting", "ready", "degraded", "failed"].includes(String(value))
    ? value as RemoteHostState
    : "failed";
}

/**
 * Reduce an untrusted host result to the public remote DTO. Unknown members
 * such as tokens, filesystem paths, endpoints and raw error messages cannot
 * survive this projection into renderer state.
 */
export function mapRemoteStatus(value: unknown): RemoteHostStatus {
  const result = isRecord(value) ? value : {};
  const source = isRecord(result.remote) ? result.remote : result;
  const rawDevices = Array.isArray(source.devices) ? source.devices : [];
  const devices = rawDevices.flatMap((entry) => {
    if (!isRecord(entry)) return [];
    const id = publicText(entry.id, 256);
    const displayName = publicText(entry.display_name, 120);
    const platform = publicText(entry.platform, 64);
    if (!id || !displayName || !platform || typeof entry.online !== "boolean") return [];
    const lastSeenAt = publicTimestamp(entry.last_seen_at);
    return [{
      id,
      displayName,
      platform,
      online: entry.online,
      pairedAt: publicTimestamp(entry.paired_at),
      ...(lastSeenAt > 0 ? { lastSeenAt } : {}),
    }];
  });
  const errorCode = publicText(source.error_code, 80);
  return {
    enabled: source.enabled === true,
    state: mapRemoteState(source.state),
    displayName: publicText(source.display_name, 120) || "这台 Mac",
    activeConnections: Math.min(10_000, Math.max(0, Math.trunc(numberValue(source.active_connections)))),
    devices,
    updatedAt: publicTimestamp(source.updated_at),
    ...(errorCode ? { errorCode } : {}),
  };
}

export function mapRemotePairing(value: unknown): RemotePairing {
  const result = isRecord(value) ? value : {};
  const pairing = isRecord(result.pairing) ? result.pairing : {};
  const pairingId = publicText(pairing.pairing_id, 256);
  const qrPayload = typeof pairing.qr_payload === "string" ? pairing.qr_payload : "";
  const expiresAt = publicTimestamp(pairing.expires_at);
  if (!pairingId || !qrPayload || qrPayload.length > 16 * 1024 || expiresAt <= 0) {
    throw new Error("远程配对信息无效，请重新开始配对。");
  }
  return { pairingId, qrPayload, expiresAt };
}

const authSessionsByProfile = new Map<string, AuthSession>();

async function loadProviderStatuses(
  api: PadDesktopApi,
  records: DesktopRecords,
  bootstrap: DesktopBootstrapResult,
  refresh = false,
  previous = new Map<string, ProviderStatus>(),
): Promise<Map<string, ProviderStatus>> {
  const pairs = await Promise.all(records.profiles.map(async (profile) => {
    let status: ProviderStatus;
    try {
      const result = await api.request<"provider_status", unknown>("provider_status", { profile_id: profile.id });
      status = parseProviderStatus(result);
    } catch {
      status = fallbackProviderStatus(profile, bootstrap);
    }
    const previousCatalog = previous.get(profile.id)?.modelCatalog;
    const defaults = {
      selectedProvider: status.selectedProvider ?? profile.default_provider ?? null,
      selectedModel: status.selectedModel ?? profile.default_model ?? null,
    };
    let rawCatalog: unknown;
    try {
      rawCatalog = await api.request<"model_catalog", unknown>("model_catalog", {
        profile_id: profile.id,
        ...(refresh ? { refresh: true } : {}),
      });
    } catch {
      // The model catalog is additive. Older sidecars or a temporary Pi
      // failure must not make the account/sidebar unavailable.
      rawCatalog = { error: "模型列表读取失败" };
    }
    const modelCatalog = modelCatalogWithFallback(rawCatalog, previousCatalog, defaults);
    return [profile.id, {
      ...status,
      selectedProvider: status.selectedProvider ?? modelCatalog.selectedProvider,
      selectedModel: status.selectedModel ?? modelCatalog.selectedModel,
      modelCatalog,
    }] as const;
  }));
  return new Map(pairs);
}

function fallbackProviderStatus(profile: ProfileDto, bootstrap: DesktopBootstrapResult): ProviderStatus {
  const active = profile.id === bootstrap.profile.id;
  return {
    providerAuthentication: normalizeAuthentication(active ? bootstrap.backend.provider_authentication : "unknown"),
    authenticatedProviders: active ? bootstrap.backend.authenticated_providers : [],
    selectedProvider: profile.default_provider ?? null,
    selectedModel: profile.default_model ?? null,
    modelCatalog: emptyModelCatalog({
      selectedProvider: profile.default_provider ?? null,
      selectedModel: profile.default_model ?? null,
    }),
  };
}

function parseProviderStatus(value: unknown): ProviderStatus {
  const object = isRecord(value) ? value : {};
  return {
    providerAuthentication: normalizeAuthentication(textValue(object.provider_authentication ?? object.status)),
    authenticatedProviders: stringArray(object.authenticated_providers),
    selectedProvider: optionalText(object.selected_provider),
    selectedModel: optionalText(object.selected_model),
    modelCatalog: emptyModelCatalog({
      selectedProvider: optionalText(object.selected_provider),
      selectedModel: optionalText(object.selected_model),
    }),
  };
}

function emptyModelCatalog(defaults: Partial<Pick<ModelCatalog, "selectedProvider" | "selectedModel">> = {}): ModelCatalog {
  return { ...parseModelCatalog(null, defaults), source: "fallback" };
}

function snapshotFromRecords(
  records: DesktopRecords,
  selectedProfileId: string | null,
  bootstrap: DesktopBootstrapResult | null,
  rawSidebar: unknown,
  providerStatuses: Map<string, ProviderStatus>,
  previousTurns: Record<string, TurnEntry[]>,
  previousInteractions: Record<string, PendingInteraction[]>,
  uiState: DesktopUiState,
  remote: RemoteHostStatus | null = null,
): DesktopSnapshot {
  const projects = records.projects
    .filter((project) =>
      (project.profile_id === selectedProfileId || project.profile_id === null)
      && projectVisibleInSidebarView(project, records.tasks, uiState.sidebarView, selectedProfileId),
    )
    .map(mapProject);
  const tasks = records.tasks
    .filter((task) => task.profile_id === selectedProfileId && taskVisibleInSidebarView(task, uiState.sidebarView))
    .sort((left, right) => right.updated_at - left.updated_at)
    .map(mapTask);
  const visibleTaskIds = new Set(tasks.map((task) => task.id));
  const turnsByTask = Object.fromEntries(Object.entries(previousTurns).filter(([taskId]) => visibleTaskIds.has(taskId)));
  const interactionsByTask = Object.fromEntries(Object.entries(previousInteractions).filter(([taskId]) => visibleTaskIds.has(taskId)));
  const activeStatus = selectedProfileId ? providerStatuses.get(selectedProfileId) : undefined;
  const accounts = records.profiles.map((profile) => mapProfile(profile, profile.id === selectedProfileId, providerStatuses.get(profile.id)));
  return {
    accounts,
    modelCatalogByProfile: Object.fromEntries(accounts.map((account) => [account.id, account.modelCatalog])),
    projects,
    tasks,
    sidebar: buildSidebarHierarchy(rawSidebar, selectedProfileId, projects, tasks),
    backend: {
      status: bootstrap?.backend.status ?? "unavailable",
      capabilities: bootstrap?.capabilities ?? [],
      providerAuthentication: activeStatus?.providerAuthentication ?? bootstrap?.backend.provider_authentication ?? "unknown",
    },
    turnsByTask,
    interactionsByTask,
    uiState,
    remote,
  };
}

function taskVisibleInSidebarView(task: TaskDto, view: SidebarHierarchy["view"]): boolean {
  if (view === "archive") return task.archived;
  if (view === "pinned") return !task.archived && task.pinned;
  return !task.archived;
}

function projectVisibleInSidebarView(
  project: ProjectDto,
  tasks: TaskDto[],
  view: SidebarHierarchy["view"],
  selectedProfileId: string | null,
): boolean {
  const projectItselfVisible = view === "archive"
    ? project.archived
    : view === "pinned"
      ? !project.archived && project.pinned
      : !project.archived;
  return projectItselfVisible || tasks.some((task) =>
    task.profile_id === selectedProfileId
    && task.project_id === project.id
    && taskVisibleInSidebarView(task, view),
  );
}

function defaultUiState(): DesktopUiState {
  return {
    activeProfileId: null,
    selectedTaskId: null,
    sidebarView: "all",
    collapsedSectionIds: [],
    collapsedProjectIds: [],
    sidebarWidth: 275,
    theme: "system",
    rightPanelOpen: false,
    bottomPanelOpen: false,
    sidebarOpen: true,
  };
}

function mapUiState(value: DesktopUiStateDto | undefined): DesktopUiState {
  if (!value) return defaultUiState();
  return {
    activeProfileId: value.active_profile_id,
    selectedTaskId: value.selected_task_id,
    sidebarView: normalizeSidebarView(value.sidebar_view),
    collapsedSectionIds: value.collapsed_section_ids,
    collapsedProjectIds: value.collapsed_project_ids,
    sidebarWidth: Math.min(520, Math.max(240, value.sidebar_width || 275)),
    theme: value.theme,
    rightPanelOpen: value.right_panel_open,
    bottomPanelOpen: value.bottom_panel_open,
    sidebarOpen: value.sidebar_open,
  };
}

function uiStateToDto(value: DesktopUiState): DesktopUiStateDto {
  return {
    active_profile_id: value.activeProfileId,
    selected_task_id: value.selectedTaskId,
    sidebar_view: value.sidebarView,
    collapsed_section_ids: value.collapsedSectionIds,
    collapsed_project_ids: value.collapsedProjectIds,
    sidebar_width: Math.min(520, Math.max(240, value.sidebarWidth)),
    theme: value.theme,
    right_panel_open: value.rightPanelOpen,
    bottom_panel_open: value.bottomPanelOpen,
    sidebar_open: value.sidebarOpen,
  };
}

export function buildSidebarHierarchy(
  value: unknown,
  activeProfileId: string | null,
  projects: ProjectSummary[],
  tasks: TaskSummary[],
): SidebarHierarchy {
  const source = parseSidebar(value);
  const projectIds = new Set(projects.map((project) => project.id));
  const taskIds = new Set(tasks.map((task) => task.id));
  const eligible = source.rows.filter((row) => {
    if (row.kind === "profile") return row.id === activeProfileId;
    if (row.kind === "project") return !!row.id && projectIds.has(row.id);
    if (row.kind === "task") return !!row.id && taskIds.has(row.id);
    return true;
  });
  const canonicalRows = retainPopulatedSections(eligible).filter((row) => row.kind !== "profile");
  const rows = canonicalRows.some((row) => row.kind === "new_task")
    ? canonicalRows
    : [syntheticRow("new_task", "new-task", "新任务", 0), ...canonicalRows];
  // Profile switching has one stable home in the sidebar footer. Rendering
  // the active Profile again as a tree row creates a redundant hierarchy
  // level: projects and task groups are top-level within the selected account.

  return {
    view: source.view,
    query: source.query,
    activeProfileId,
    selectedKey: source.selectedKey,
    rows: deduplicateRows(rows),
  };
}

function parseSidebar(value: unknown): SidebarHierarchy {
  const object = isRecord(value) ? value : {};
  const rawRows = Array.isArray(object.rows) ? object.rows : [];
  return {
    view: normalizeSidebarView(textValue(object.view)),
    query: textValue(object.query),
    activeProfileId: optionalText(object.active_profile_id),
    selectedKey: optionalText(object.selected_key),
    rows: rawRows.map(parseSidebarRow).filter((row): row is SidebarRow => row !== null),
  };
}

function parseSidebarRow(value: unknown): SidebarRow | null {
  if (!isRecord(value)) return null;
  const node = isRecord(value.node) ? value.node : value;
  const kind = normalizeNodeKind(textValue(node.kind ?? value.kind));
  if (!kind) return null;
  return {
    key: textValue(value.key) || `${kind}:${textValue(node.id)}`,
    kind,
    id: optionalText(node.id) ?? undefined,
    depth: numberValue(value.depth),
    title: localizeSidebarTitle(kind, textValue(value.title)),
    status: mapSidebarStatus(textValue(value.status)),
    unread: booleanValue(value.unread),
    pinned: booleanValue(value.pinned),
    archived: booleanValue(value.archived),
    missingReference: booleanValue(value.missing_reference),
    agent: booleanValue(value.agent),
    hasChildren: booleanValue(value.has_children),
  };
}

function retainPopulatedSections(rows: SidebarRow[]): SidebarRow[] {
  const result: SidebarRow[] = [];
  for (let index = 0; index < rows.length; index += 1) {
    const row = rows[index];
    if (!row) continue;
    if (row.kind !== "section") {
      if (!hasSectionAncestor(rows, index)) result.push(row);
      continue;
    }
    const children: SidebarRow[] = [];
    for (let childIndex = index + 1; childIndex < rows.length; childIndex += 1) {
      const child = rows[childIndex];
      if (!child || child.depth <= row.depth) break;
      if (child.kind === "project" || child.kind === "task") children.push(child);
    }
    if (children.length) result.push(row, ...children);
  }
  return deduplicateRows(result);
}

function hasSectionAncestor(rows: SidebarRow[], index: number): boolean {
  const row = rows[index];
  if (!row) return false;
  for (let cursor = index - 1; cursor >= 0; cursor -= 1) {
    const candidate = rows[cursor];
    if (!candidate) continue;
    if (candidate.depth < row.depth) return candidate.kind === "section";
  }
  return false;
}

function deduplicateRows(rows: SidebarRow[]): SidebarRow[] {
  const seen = new Set<string>();
  return rows.filter((row) => {
    if (seen.has(row.key)) return false;
    seen.add(row.key);
    return true;
  });
}

function syntheticRow(
  kind: SidebarNodeKind,
  key: string,
  title: string,
  depth: number,
  id?: string,
  status: TaskStatus | "none" = "none",
  pinned = false,
  unread = false,
): SidebarRow {
  return { key, kind, id, depth, title, status, unread, pinned, archived: false, missingReference: false, agent: false, hasChildren: false };
}

function mapProfile(profile: ProfileDto, active: boolean, status?: ProviderStatus): AccountSummary {
  const modelCatalog = status?.modelCatalog ?? emptyModelCatalog({
    selectedProvider: profile.default_provider ?? null,
    selectedModel: profile.default_model ?? null,
  });
  const selectedProvider = status?.selectedProvider ?? modelCatalog.selectedProvider ?? profile.default_provider ?? null;
  const selectedModel = status?.selectedModel ?? modelCatalog.selectedModel ?? profile.default_model ?? null;
  const policy = profile.policy;
  const protectedNamespaces = Array.isArray(policy.protected_namespaces) ? policy.protected_namespaces : [];
  const name = profile.id === "default" && profile.name.trim() === "Default" ? "默认账号" : profile.name;
  return {
    id: profile.id,
    name,
    provider: selectedProvider ? `Pi · ${selectedProvider}` : "Pi · 未登录",
    selectedProvider,
    selectedModel,
    modelCatalog: {
      ...modelCatalog,
      selectedProvider,
      selectedModel,
    },
    authenticatedProviders: status?.authenticatedProviders ?? [],
    authentication: status?.providerAuthentication ?? "unknown",
    initials: Array.from(name.trim())[0] ?? "P",
    active,
    policy: {
      mode: policy.mode,
      unattended: policy.unattended === true,
      workspaceRootCount: Array.isArray(policy.workspace_roots) ? policy.workspace_roots.length : 0,
      protectedNamespaceNames: protectedNamespaces.map((item) => item.name),
    },
    fullAccess: policy.mode === "system_full" && policy.unattended === true,
  };
}

export function sanitizeProfileForRenderer(profile: ProfileDto, active = false): AccountSummary {
  return mapProfile(profile, active);
}

function mapProject(project: ProjectDto): ProjectSummary {
  const name = /^workspace$/i.test(project.name.trim()) ? "工作区" : project.name || "未命名项目";
  return {
    id: project.id,
    profileId: project.profile_id,
    name,
    path: project.primary_root,
    accent: accentFor(project.id),
    expanded: true,
    pinned: project.pinned,
  };
}

function mapTask(task: TaskDto): TaskSummary {
  const title = /^new task$/i.test(task.title.trim()) ? "新任务" : task.title || "未命名任务";
  return {
    id: task.id,
    projectId: task.project_id,
    profileId: task.profile_id,
    parentTaskId: task.parent_task_id ?? null,
    agentName: task.agent_name ?? null,
    title,
    updatedAt: relativeTime(task.updated_at),
    status: mapStatus(task.status),
    rawStatus: task.status,
    unread: task.unread,
    pinned: task.pinned,
    archived: task.archived,
  };
}

async function hydrateVisibleHistory(api: PadDesktopApi, snapshot: DesktopSnapshot) {
  await Promise.all(snapshot.tasks.slice(0, 8).map(async (task) => {
    const previous = snapshot.turnsByTask[task.id];
    try {
      const turns = await loadTaskHistory(api, task.id);
      snapshot.turnsByTask[task.id] = preserveNonEmptyHistory(previous, turns);
    } catch {
      // Absence is intentional: selecting the task will retry and surface a visible error.
      if (previous === undefined) delete snapshot.turnsByTask[task.id];
      else snapshot.turnsByTask[task.id] = previous;
    }
  }));
}

async function hydrateVisibleTaskData(api: PadDesktopApi, snapshot: DesktopSnapshot) {
  await Promise.all(snapshot.tasks.slice(0, 8).map(async (task) => {
    const previous = snapshot.turnsByTask[task.id];
    try {
      await hydrateTaskData(api, snapshot, task);
    } catch {
      // Preloading must not make the whole desktop unavailable. The selected task
      // retries through loadTaskData(), where the error is user-visible.
      if (previous === undefined) delete snapshot.turnsByTask[task.id];
      else snapshot.turnsByTask[task.id] = previous;
    }
  }));
}

async function hydrateTaskData(api: PadDesktopApi, target: DesktopSnapshot, task: TaskSummary, pollSelectedTask = false): Promise<void> {
  const turns = await loadTaskHistory(api, task.id);
  let interactions = target.interactionsByTask[task.id] ?? [];
  const terminal = ["failed", "error", "disconnected", "completed"].includes(task.rawStatus.toLowerCase());
  const active = task.status === "running" || ["needs_approval", "needs_input"].includes(task.rawStatus.toLowerCase());
  if (active || (pollSelectedTask && !terminal)) {
    try {
      const poll = await api.request<"poll", unknown>("poll", { task_id: task.id });
      interactions = mergePendingInteractions(interactions, pendingInteractionsFromPoll(poll));
    } catch (error) {
      // Persisted idle/completed tasks may not have a live Pi process. Selection
      // still performs a real poll, while history remains usable in that case.
      if (active) throw error;
    }
  }
  target.turnsByTask[task.id] = preserveNonEmptyHistory(target.turnsByTask[task.id], turns);
  target.interactionsByTask[task.id] = interactions;
}

export function pendingInteractionsFromPoll(value: unknown): PendingInteraction[] {
  if (!isRecord(value)) return [];
  const poll = isRecord(value.poll) ? value.poll : value;
  const requests = Array.isArray(poll.pending_ui_requests) ? poll.pending_ui_requests : [];
  const mapped = requests.map((request): PendingInteraction | null => {
    if (!isRecord(request) || textValue(request.response_action) !== "respond_ui") return null;
    const id = optionalText(request.id);
    const rawKind = textValue(request.kind).toLowerCase();
    const kind = (["confirm", "select", "input", "editor", "unknown"] as const).find((candidate) => candidate === rawKind);
    if (!id || !kind) return null;
    const options = Array.isArray(request.options)
      ? request.options.filter((option): option is string => typeof option === "string")
      : [];
    const rawDefaultIndex = request.default_index;
    const defaultIndex = typeof rawDefaultIndex === "number" && Number.isInteger(rawDefaultIndex) && rawDefaultIndex >= 0 && rawDefaultIndex < options.length
      ? rawDefaultIndex
      : undefined;
    return {
      id,
      kind,
      title: optionalText(request.title) ?? undefined,
      message: optionalText(request.message) ?? undefined,
      options,
      defaultIndex,
      defaultValue: optionalText(request.default) ?? undefined,
      requiresResponse: request.requires_response === true && kind !== "unknown",
    };
  }).filter((interaction): interaction is PendingInteraction => interaction !== null);
  return mergePendingInteractions([], mapped);
}

function mergePendingInteractions(current: PendingInteraction[], discovered: PendingInteraction[]): PendingInteraction[] {
  const byId = new Map(current.map((interaction) => [interaction.id, interaction]));
  discovered.forEach((interaction) => byId.set(interaction.id, interaction));
  return [...byId.values()];
}

function validateInteractionResponse(interaction: PendingInteraction, value: InteractionResponse) {
  if (interaction.kind === "confirm") {
    if (typeof value !== "boolean") throw new Error("确认响应格式无效，请重试。");
    return;
  }
  if (interaction.kind === "select") {
    if (!Number.isInteger(value) || typeof value !== "number" || value < 0 || value >= interaction.options.length) {
      throw new Error("请选择一个有效选项。");
    }
    return;
  }
  if (interaction.kind === "input" || interaction.kind === "editor") {
    if (typeof value !== "string") throw new Error("输入响应格式无效，请重试。");
    return;
  }
  throw new Error("当前版本无法处理这项 Pi 交互，请升级 PAD 后重试。");
}

async function loadTaskHistory(api: PadDesktopApi, taskId: string): Promise<TurnEntry[]> {
  const result = await api.request<"history", unknown>("history", { task_id: taskId });
  if (isRecord(result) && result.pending === true) {
    throw new Error("Pi 会话仍在载入，请稍后重试。");
  }
  return historyMessages(result).map(mapHistoryMessage);
}

function preserveNonEmptyHistory(current: TurnEntry[] | undefined, next: TurnEntry[]): TurnEntry[] {
  return next.length === 0 && current && current.length > 0 ? current : next;
}

function historyMessages(value: unknown): unknown[] {
  if (!isRecord(value)) return [];
  if (Array.isArray(value.messages)) return value.messages;
  const response = isRecord(value.response) ? value.response : value;
  const data = isRecord(response.data) ? response.data : null;
  return data && Array.isArray(data.messages) ? data.messages : [];
}

export function mapHistoryMessage(value: unknown, index: number): TurnEntry {
  if (!isRecord(value)) return { id: `history-${index}`, kind: "notice", body: String(value) };
  const id = textValue(value.id) || `history-${index}`;
  const kind = historyTurnKind(value);
  const artifacts = historyArtifacts(value, id);
  const title = historyTurnTitle(value, kind);
  const state = mapTurnState(textValue(value.status ?? value.state), kind);
  const provider = optionalText(value.provider);
  const model = optionalText(value.model ?? value.modelId ?? value.model_id);
  return {
    id,
    kind,
    ...(title ? { title } : {}),
    body: historyMessageBody(value, kind),
    meta: displayTimestamp(value.timestamp ?? value.created_at),
    ...(provider ? { provider } : {}),
    ...(model ? { model } : {}),
    ...(state ? { state } : {}),
    ...(artifacts.length > 0 ? { artifacts } : {}),
  };
}

function historyTurnKind(value: Record<string, unknown>): TurnKind {
  if (historyAssistantFailure(value)) return "error";
  const semanticSignals = [value.kind, value.message_type, value.event_type, value.type]
    .map((item) => turnKindFromToken(textValue(item)))
    .filter((item): item is TurnKind => item !== null);
  const specialized = semanticSignals.find((kind) =>
    ["reasoning", "error", "status", "final", "activity"].includes(kind),
  );
  if (specialized) return specialized;

  const role = turnKindFromToken(textValue(value.role));
  if (role) return role;
  return semanticSignals[0] ?? "notice";
}

function turnKindFromToken(value: string): TurnKind | null {
  const token = normalizedToken(value);
  if (["user", "human", "user_message", "input"].includes(token)) return "user";
  if (["assistant", "model", "assistant_message", "model_message"].includes(token)) return "assistant";
  if (["tool", "tool_call", "tool_result", "function", "function_call", "function_result"].includes(token)) return "tool";
  if (["reasoning", "thinking", "analysis", "reasoning_message", "thought"].includes(token)) return "reasoning";
  if (["error", "failure", "failed", "error_message"].includes(token)) return "error";
  if (["status", "progress", "state", "status_message", "progress_update"].includes(token)) return "status";
  if (["final", "final_answer", "answer", "completion"].includes(token)) return "final";
  if (["activity", "event", "lifecycle", "activity_message"].includes(token)) return "activity";
  if (["notice", "system", "system_message", "message"].includes(token)) return "notice";
  return null;
}

function historyTurnTitle(value: Record<string, unknown>, kind: TurnKind): string | undefined {
  const explicit = optionalText(value.title ?? value.label);
  if (explicit) return explicit;
  if (kind === "tool") return optionalText(value.name ?? value.tool_name) ?? "工具调用";
  if (kind === "reasoning") return "推理过程";
  if (kind === "error") return "执行错误";
  if (kind === "status") return "状态更新";
  if (kind === "final") return "最终答复";
  if (kind === "activity") return optionalText(value.name ?? value.event_name) ?? "活动";
  return undefined;
}

function historyMessageBody(value: Record<string, unknown>, kind: TurnKind): string {
  const specialized = kind === "reasoning"
    ? value.reasoning ?? value.summary
    : kind === "error"
      ? value.errorMessage ?? value.error_message ?? value.error ?? value.detail
      : kind === "status"
        ? value.status_text ?? value.description
        : kind === "final"
          ? value.final ?? value.answer
          : undefined;
  const body = messageBody(specialized ?? value.content ?? value.message ?? value.text);
  return kind === "error" ? localizePiRuntimeError(body) : body;
}

function historyAssistantFailure(value: Record<string, unknown>): boolean {
  const role = normalizedToken(textValue(value.role));
  if (!["assistant", "model", "assistant_message", "model_message"].includes(role)) return false;
  const stopReason = normalizedToken(textValue(value.stopReason ?? value.stop_reason));
  const errorMessage = optionalText(value.errorMessage ?? value.error_message);
  return stopReason === "error" || errorMessage !== null;
}

function localizePiRuntimeError(message: string): string {
  const normalized = message.trim().toLowerCase();
  if (!normalized) return "模型请求失败，请重试。";
  if (
    normalized.includes("unable to connect")
    || normalized.includes("fetch failed")
    || normalized.includes("network error")
    || normalized.includes("connection reset")
  ) return "无法连接模型服务。请检查网络或代理后重试。";
  if (normalized.includes("timed out") || normalized.includes("timeout")) {
    return "模型请求超时，请重试。";
  }
  return message;
}

function historyArtifacts(value: Record<string, unknown>, turnId: string): TurnArtifact[] {
  const artifacts: TurnArtifact[] = [];
  const seen = new Set<string>();
  let ordinal = 0;
  const add = (candidate: unknown, forcedKind?: TurnArtifactKind) => {
    const artifact = mapHistoryArtifact(candidate, turnId, ordinal, forcedKind);
    ordinal += 1;
    if (!artifact) return;
    const key = `${artifact.id}\u0000${artifact.kind}\u0000${artifact.path}\u0000${artifact.diff ?? ""}`;
    if (seen.has(key)) return;
    seen.add(key);
    artifacts.push(artifact);
  };

  for (const container of [value, isRecord(value.metadata) ? value.metadata : null]) {
    if (!container) continue;
    if (Array.isArray(container.artifacts)) container.artifacts.forEach((artifact) => add(artifact));
    else if (container.artifact !== undefined) add(container.artifact);
    if (Array.isArray(container.files)) container.files.forEach((artifact) => add(artifact, "file"));
    if (Array.isArray(container.changes)) container.changes.forEach((artifact) => add(artifact, "change"));
    if (hasExplicitArtifactFields(container)) add(container);
  }
  return artifacts;
}

function hasExplicitArtifactFields(value: Record<string, unknown>): boolean {
  return [
    "file",
    "file_path",
    "path",
    "target_path",
    "new_path",
    "diff",
    "patch",
    "unified_diff",
    "operation",
    "change_type",
    "artifact_kind",
  ].some((key) => Object.prototype.hasOwnProperty.call(value, key));
}

function mapHistoryArtifact(
  value: unknown,
  turnId: string,
  ordinal: number,
  forcedKind?: TurnArtifactKind,
): TurnArtifact | null {
  if (typeof value === "string") {
    const path = structuredArtifactPath(value);
    if (!path || forcedKind !== "file") return null;
    return { id: `${turnId}-artifact-${ordinal}`, kind: "file", path, operation: "read" };
  }
  if (!isRecord(value)) return null;

  const path = structuredArtifactPath(value.path ?? value.file_path ?? value.file ?? value.target_path ?? value.new_path);
  if (!path) return null;
  const previousPath = structuredArtifactPath(value.previous_path ?? value.old_path ?? value.from) ?? undefined;
  const diff = optionalText(value.diff ?? value.patch ?? value.unified_diff) ?? undefined;
  const operation = artifactOperation(value.operation ?? value.change_type ?? value.action);
  const declaredKind = normalizedToken(textValue(value.kind ?? value.artifact_kind ?? value.type));
  const kind: TurnArtifactKind = forcedKind
    ?? (["change", "diff", "patch"].includes(declaredKind)
      || diff !== undefined
      || ["created", "modified", "deleted", "renamed"].includes(operation)
      ? "change"
      : "file");
  return {
    id: optionalText(value.id ?? value.artifact_id) ?? `${turnId}-artifact-${ordinal}`,
    kind,
    path,
    operation: operation === "unknown" && kind === "file" ? "read" : operation,
    ...(previousPath ? { previousPath } : {}),
    ...(diff ? { diff } : {}),
    ...(optionalText(value.title ?? value.label) ? { title: optionalText(value.title ?? value.label)! } : {}),
  };
}

function structuredArtifactPath(value: unknown): string | null {
  if (typeof value !== "string") return null;
  const path = value.trim();
  if (!path || path === "/dev/null" || path.includes("\n") || path.includes("\r") || path.includes("\0") || path.includes("://")) return null;
  return path;
}

function artifactOperation(value: unknown): TurnArtifactOperation {
  const operation = normalizedToken(textValue(value));
  if (["read", "opened", "inspected", "viewed"].includes(operation)) return "read";
  if (["create", "created", "add", "added", "write", "wrote"].includes(operation)) return "created";
  if (["update", "updated", "modify", "modified", "edit", "edited", "patch", "patched"].includes(operation)) return "modified";
  if (["delete", "deleted", "remove", "removed"].includes(operation)) return "deleted";
  if (["rename", "renamed", "move", "moved"].includes(operation)) return "renamed";
  return "unknown";
}

function normalizedToken(value: string): string {
  return value.trim().toLowerCase().replace(/[^a-z0-9]+/g, "_").replace(/^_+|_+$/g, "");
}

function parseAuthSession(value: unknown, profileId: string, provider: string | null, authType?: AuthType): AuthSession {
  const root = isRecord(value) ? value : {};
  const object = isRecord(root.auth) ? root.auth : isRecord(root.session) ? root.session : root;
  const prompt = isRecord(object.prompt) ? object.prompt : {};
  const options = authPromptOptions(prompt.options);
  const notices = Array.isArray(object.notices) ? object.notices.filter(isRecord) : [];
  const notice = notices[0] ?? {};
  const resolvedAuthType = normalizeAuthType(object.auth_type) ?? authType;
  const phase = normalizeAuthPhase(
    textValue(object.phase ?? object.status),
    Object.keys(prompt).length > 0,
    notices.some((item) => !!optionalText(item.url)),
  );
  const session: AuthSession = {
    attemptId: optionalText(object.attempt_id) ?? undefined,
    promptId: optionalText(prompt.id ?? object.prompt_id) ?? undefined,
    profileId: optionalText(object.profile_id) ?? profileId,
    provider: optionalText(object.provider) ?? provider,
    authType: resolvedAuthType,
    phase,
    title: authTitle(phase),
    message: authMessage(phase, optionalText(notice.user_code)),
    verificationUrl: optionalText(object.verification_url ?? object.auth_url ?? object.url ?? notice.url) ?? undefined,
    promptKind: optionalText(prompt.kind) ?? undefined,
    promptMessage: localizePiAuthPrompt(optionalText(prompt.message), options.length > 0),
    options: options.length > 0 ? options : undefined,
    inputLabel: phase === "waiting_input" && options.length === 0 ? authInputLabel(prompt, object) : undefined,
    inputSecret: resolvedAuthType === "api_key"
      || booleanValue(prompt.secret ?? object.input_secret)
      || /password|secret|api[\s_-]*key/i.test(textValue(prompt.kind ?? prompt.message ?? prompt.placeholder)),
    error: optionalText(object.error) ?? undefined,
  };
  authSessionsByProfile.set(profileId, session);
  return session;
}

function authSessionFromEvent(value: unknown, fallbackProfileId: string): AuthSession | null {
  if (!isRecord(value)) return null;
  const envelope = isRecord(value.event) ? value.event : value;
  if (textValue(envelope.kind) !== "auth_changed" && !isRecord(envelope.auth)) return null;
  const payload = isRecord(envelope.payload) ? envelope.payload : envelope;
  const profileId = optionalText(payload.profile_id ?? envelope.profile_id) ?? fallbackProfileId;
  const current = authSessionsByProfile.get(profileId);
  return parseAuthSession(
    payload,
    profileId,
    optionalText(payload.provider ?? envelope.provider) ?? current?.provider ?? null,
    current?.authType,
  );
}

function idleAuth(profileId: string): AuthSession {
  return { profileId, provider: null, phase: "idle", title: "登录模型账号", message: "通过 Pi 的安全登录流程连接模型提供商。" };
}

function emptySnapshot(): DesktopSnapshot {
  return {
    accounts: [], modelCatalogByProfile: {}, projects: [], tasks: [], turnsByTask: {}, interactionsByTask: {},
    sidebar: { view: "all", query: "", activeProfileId: null, selectedKey: null, rows: [] },
    backend: { status: "unavailable", capabilities: [], providerAuthentication: "unknown" },
    uiState: defaultUiState(),
    remote: null,
  };
}

function mapStatus(status: string): TaskStatus {
  const normalized = status.toLowerCase();
  if (["running", "starting", "streaming", "tool_running", "queued", "active", "compacting", "retrying"].includes(normalized)) return "running";
  if (["attention", "needs_approval", "needs_input", "disconnected", "blocked", "failed", "error"].includes(normalized)) return "attention";
  if (["complete", "completed", "done", "stopped"].includes(normalized)) return "complete";
  return "idle";
}

function mapSidebarStatus(status: string): TaskStatus | "none" {
  return status === "none" || status === "" ? "none" : mapStatus(status);
}

function normalizeAuthentication(value: string): ProviderAuthentication {
  const normalized = value.toLowerCase();
  if (["authenticated", "ready", "ok"].includes(normalized)) return "authenticated";
  if (["missing", "unauthenticated", "logged_out"].includes(normalized)) return "missing";
  if (normalized === "partial") return "partial";
  return "unknown";
}

function normalizeAuthType(value: unknown): AuthType | undefined {
  return value === "oauth" || value === "api_key" ? value : undefined;
}

function normalizeAuthPhase(value: string, hasPrompt = false, hasUrl = false): AuthPhase {
  const normalized = value.toLowerCase();
  if (["running", "starting", "pending"].includes(normalized)) return hasPrompt ? "waiting_input" : hasUrl ? "waiting_browser" : "starting";
  if (["waiting_browser", "browser", "oauth"].includes(normalized)) return "waiting_browser";
  if (["waiting_input", "prompt", "challenge"].includes(normalized)) return "waiting_input";
  if (["authenticated", "succeeded", "complete", "completed", "success"].includes(normalized)) return "authenticated";
  if (["failed", "error"].includes(normalized)) return "failed";
  if (["cancelled", "canceled"].includes(normalized)) return "cancelled";
  return "idle";
}

function normalizeSidebarView(value: string): SidebarHierarchy["view"] {
  return value === "pinned" || value === "archive" ? value : "all";
}

function normalizeNodeKind(value: string): SidebarNodeKind | null {
  return ["new_task", "profile", "section", "project", "task"].includes(value) ? value as SidebarNodeKind : null;
}

function localizeSidebarTitle(kind: SidebarNodeKind, title: string): string {
  if (kind === "new_task") return "新任务";
  if (kind === "task" && /^new task$/i.test(title.trim())) return "新任务";
  if (kind === "project" && /^workspace$/i.test(title.trim())) return "工作区";
  return title || (kind === "section" ? "分组" : "未命名");
}

function authTitle(phase: AuthPhase): string {
  if (phase === "authenticated") return "登录成功";
  if (phase === "failed") return "登录失败";
  if (phase === "cancelled") return "已取消登录";
  return "登录模型账号";
}

function authMessage(phase: AuthPhase, userCode: string | null = null): string {
  if (phase === "waiting_browser") {
    return userCode
      ? `请在浏览器中完成授权，然后返回 PAD。授权码：${userCode}`
      : "请在浏览器中完成授权，然后返回 PAD。";
  }
  if (phase === "waiting_input") return "请完成 Pi 提供的验证步骤。";
  if (phase === "authenticated") return "账号已经可以用于新的 Pi 任务。";
  if (phase === "failed") return "登录未完成，请检查错误后重试。";
  return "正在准备安全登录流程。";
}

function authInputLabel(prompt: Record<string, unknown>, session: Record<string, unknown>): string {
  const value = textValue(prompt.label ?? prompt.placeholder ?? prompt.message ?? session.input_label).trim();
  if (/api[\s_-]*key/i.test(value)) return "API 密钥";
  if (/password|passphrase/i.test(value)) return "密码";
  if (/verification|device|authori[sz]ation|auth(?:entication)?[\s_-]*code|验证码|授权码/i.test(value)) return "验证码";
  if (/token/i.test(value)) return "访问令牌";
  if (/[\u3400-\u9fff]/u.test(value)) return value;
  return /secret|password|api_key/i.test(textValue(prompt.kind)) ? "验证密钥" : "验证信息";
}

function authPromptOptions(value: unknown): NonNullable<AuthSession["options"]> {
  if (!Array.isArray(value)) return [];
  return value.flatMap((item) => {
    if (typeof item === "string") {
      const label = item.trim();
      return label ? [{ id: label, label: localizePiAuthOption(label) ?? label }] : [];
    }
    if (!isRecord(item)) return [];
    const id = optionalText(item.id);
    const label = optionalText(item.label);
    if (!id || !label) return [];
    const localizedLabel = localizePiAuthOption(label) ?? label;
    const description = localizePiAuthOption(optionalText(item.description)) ?? undefined;
    return [{ id, label: localizedLabel, ...(description ? { description } : {}) }];
  });
}

function messageBody(value: unknown): string {
  if (typeof value === "string") return value;
  if (Array.isArray(value)) return value.map((part) => isRecord(part) ? textValue(part.text ?? part.content) : textValue(part)).filter(Boolean).join("\n");
  if (isRecord(value)) return textValue(value.text ?? value.content) || JSON.stringify(value);
  return textValue(value);
}

function mapTurnState(status: string, kind: TurnKind): TurnEntry["state"] {
  const normalized = status.toLowerCase();
  if (["failed", "error"].includes(normalized)) return "failed";
  if (["complete", "completed", "done", "success"].includes(normalized)) return "complete";
  if (["running", "starting", "pending", "active", "streaming"].includes(normalized)) return "running";
  if (kind === "error") return "failed";
  if (kind === "final") return "complete";
  return undefined;
}

function relativeTime(timestamp: number): string {
  const millis = timestamp > 10_000_000_000 ? timestamp : timestamp * 1_000;
  const delta = Math.max(0, Date.now() - millis);
  if (delta < 60_000) return "刚刚";
  if (delta < 3_600_000) return `${Math.floor(delta / 60_000)} 分钟前`;
  if (delta < 86_400_000) return `${Math.floor(delta / 3_600_000)} 小时前`;
  if (delta < 604_800_000) return `${Math.floor(delta / 86_400_000)} 天前`;
  return new Intl.DateTimeFormat("zh-CN", { month: "numeric", day: "numeric" }).format(new Date(millis));
}

function displayTimestamp(value: unknown): string | undefined {
  if (typeof value === "number") return relativeTime(value);
  if (typeof value !== "string") return undefined;
  const numeric = Number(value);
  if (Number.isFinite(numeric) && value.trim()) return relativeTime(numeric);
  const parsed = Date.parse(value);
  return Number.isFinite(parsed) ? relativeTime(parsed) : value || undefined;
}

function accentFor(id: string): string {
  const palette = ["#6d5dfc", "#0f9f6e", "#dc7b29", "#3584e4", "#c94f7c"];
  let hash = 0;
  for (const character of id) hash = (hash * 31 + character.charCodeAt(0)) >>> 0;
  return palette[hash % palette.length] ?? palette[0];
}

function textValue(value: unknown): string {
  return typeof value === "string" ? value : typeof value === "number" || typeof value === "boolean" ? String(value) : "";
}

function optionalText(value: unknown): string | null {
  const text = textValue(value).trim();
  return text ? text : null;
}

function stringArray(value: unknown): string[] {
  return Array.isArray(value) ? value.map(textValue).filter(Boolean) : [];
}

export function normalizeAttachmentPaths(values: readonly string[]): string[] {
  const paths: string[] = [];
  for (const value of values) {
    const path = typeof value === "string" ? value.trim() : "";
    if (!path.startsWith("/") || path.includes("\n") || path.includes("\r") || path.includes("\0") || paths.includes(path)) continue;
    paths.push(path);
    if (paths.length === 20) break;
  }
  return paths;
}

export function promptWithAttachments(text: string, attachmentPaths: readonly string[]): string {
  const prompt = text.trim();
  if (!prompt) return "";
  const paths = normalizeAttachmentPaths(attachmentPaths);
  if (paths.length === 0) return prompt;
  return `${prompt}\n\n附件路径（用户明确选择）：\n${paths.map((path) => `- ${path}`).join("\n")}`;
}

function booleanValue(value: unknown): boolean {
  return value === true;
}

function numberValue(value: unknown): number {
  return typeof value === "number" && Number.isFinite(value) ? value : 0;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function createUnavailableAdapter(): DesktopAdapter {
  const unavailable = async (): Promise<never> => { throw new Error("PAD Desktop 安全桥未加载，请重新启动应用。"); };
  return {
    loadSnapshot: unavailable,
    loadTaskData: unavailable,
    chooseProjectDirectory: unavailable,
    chooseAttachments: unavailable,
    createProject: unavailable,
    createTask: unavailable,
    createAccount: unavailable,
    sendMessage: unavailable,
    switchAccount: unavailable,
    setFullAccess: unavailable,
    beginLogin: unavailable,
    getLoginStatus: unavailable,
    respondLogin: unavailable,
    cancelLogin: unavailable,
    logout: unavailable,
    abortTask: unavailable,
    retryTask: unavailable,
    respondInteraction: unavailable,
    updateTask: unavailable,
    openTerminal: unavailable,
    writeTerminal: unavailable,
    resizeTerminal: unavailable,
    getTerminalSnapshot: unavailable,
    closeTerminal: unavailable,
    updateUiState: unavailable,
    getRemoteStatus: unavailable,
    setRemoteEnabled: unavailable,
    beginRemotePairing: unavailable,
    cancelRemotePairing: unavailable,
    revokeRemoteDevice: unavailable,
    subscribe: () => () => undefined,
  };
}

export const desktop: DesktopAdapter = window.padDesktop
  ? createBridgeAdapter(window.padDesktop)
  : createUnavailableAdapter();
