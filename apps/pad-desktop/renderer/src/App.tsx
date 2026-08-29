import { useEffect, useMemo, useRef, useState } from "react";
import type { PointerEvent as ReactPointerEvent, RefObject } from "react";
import { Sidebar } from "./components/Sidebar";
import { SettingsView, type SettingsSection } from "./components/SettingsView";
import { BottomPanel, RightPanel, TaskView } from "./components/TaskView";
import { ProjectSheet } from "./components/ProjectSheet";
import { Icon } from "./components/Icons";
import { desktop } from "./lib/desktop";
import { toUserFacingError, type UserFacingError } from "./lib/errors";
import type { AuthSession, AuthType, ComposerMessageInput, DesktopEvent, DesktopSnapshot, DesktopUiState, SidebarView, TaskSummary } from "./types";

type Route = "task" | "settings";

const emptySnapshot: DesktopSnapshot = {
  accounts: [],
  projects: [],
  tasks: [],
  turnsByTask: {},
  interactionsByTask: {},
  sidebar: { view: "all", query: "", activeProfileId: null, selectedKey: null, rows: [] },
  backend: { status: "starting", capabilities: [], providerAuthentication: "unknown" },
  uiState: {
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
  },
};

export function isCompactWindow(width: number): boolean {
  return width <= 720;
}

export function clampSidebarWidth(width: number, viewportWidth: number): number {
  const viewportMaximum = viewportWidth > 720 ? viewportWidth - 320 : 520;
  return Math.round(Math.min(520, Math.max(240, viewportMaximum), Math.max(240, width)));
}

function resolvedTheme(theme: "light" | "dark" | "system"): "light" | "dark" {
  if (theme === "light" || theme === "dark") return theme;
  return matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
}

function collapsedKey(kind: "section" | "project", id: string): string {
  return id.startsWith(`${kind}:`) ? id : `${kind}:${id}`;
}

function mergeTask(tasks: TaskSummary[], task: TaskSummary): TaskSummary[] {
  const index = tasks.findIndex((candidate) => candidate.id === task.id);
  if (index === -1) return [task, ...tasks];
  const next = [...tasks];
  next[index] = task;
  return next;
}

function applyDesktopEvent(snapshot: DesktopSnapshot, event: DesktopEvent): DesktopSnapshot {
  if (event.type === "snapshot") return event.snapshot;
  if (event.type === "task-updated") return { ...snapshot, tasks: mergeTask(snapshot.tasks, event.task) };
  if (event.type === "auth-updated") return snapshot;
  if (event.type === "menu-action") return snapshot;
  const turns = snapshot.turnsByTask[event.taskId] ?? [];
  if (turns.some((turn) => turn.id === event.turn.id)) return snapshot;
  return { ...snapshot, turnsByTask: { ...snapshot.turnsByTask, [event.taskId]: [...turns, event.turn] } };
}

function Titlebar({
  sidebarOpen,
  disabled,
  sidebarButtonRef,
  onSidebarToggle,
}: {
  sidebarOpen: boolean;
  disabled: boolean;
  sidebarButtonRef: RefObject<HTMLButtonElement | null>;
  onSidebarToggle(): void;
}) {
  return (
    <header className="global-titlebar">
      <div className="traffic-light-safe-area" />
      <button ref={sidebarButtonRef} className="titlebar-button" disabled={disabled} onClick={onSidebarToggle} aria-label={sidebarOpen ? "隐藏侧边栏" : "显示侧边栏"}><Icon name="layout" /></button>
      <div className="titlebar-drag-region" aria-hidden="true" />
      <div className="titlebar-trailing-space" aria-hidden="true" />
    </header>
  );
}

export default function App() {
  const [snapshot, setSnapshot] = useState<DesktopSnapshot>(emptySnapshot);
  const [loading, setLoading] = useState(true);
  const [route, setRoute] = useState<Route>("task");
  const [settingsSection, setSettingsSection] = useState<SettingsSection>("general");
  const [authSession, setAuthSession] = useState<AuthSession | null>(null);
  const [projectSheetOpen, setProjectSheetOpen] = useState(false);
  const [projectBusy, setProjectBusy] = useState(false);
  const [switchingAccount, setSwitchingAccount] = useState(false);
  const [selectedTaskId, setSelectedTaskId] = useState<string | null>(null);
  const [sidebarOpen, setSidebarOpen] = useState(true);
  const [searchOpen, setSearchOpen] = useState(false);
  const [attentionOnly, setAttentionOnly] = useState(false);
  const [compactWindow, setCompactWindow] = useState(() => isCompactWindow(window.innerWidth));
  const [sidebarWidth, setSidebarWidth] = useState(275);
  const [rightPanelOpen, setRightPanelOpen] = useState(false);
  const [bottomPanelOpen, setBottomPanelOpen] = useState(false);
  const [notice, setNotice] = useState<UserFacingError | null>(null);
  const [taskDataLoading, setTaskDataLoading] = useState<Record<string, boolean>>({});
  const [taskDataErrors, setTaskDataErrors] = useState<Record<string, UserFacingError>>({});
  const [themePreference, setThemePreference] = useState<DesktopUiState["theme"]>("system");
  const [systemTheme, setSystemTheme] = useState<"light" | "dark">(() => resolvedTheme("system"));
  const accountSwitchLockRef = useRef(false);
  const accountEpochRef = useRef(0);
  const sidebarToggleRef = useRef<HTMLButtonElement>(null);
  const sidebarOpenRef = useRef(true);
  const rightPanelOpenRef = useRef(false);
  const bottomPanelOpenRef = useRef(false);
  const tasksRef = useRef<TaskSummary[]>([]);
  tasksRef.current = snapshot.tasks;
  const theme = themePreference === "system" ? systemTheme : themePreference;

  function applyPersistedUiState(state: DesktopUiState, tasks: TaskSummary[], fallbackToFirst = false): string | null {
    const nextTaskId = state.selectedTaskId && tasks.some((task) => task.id === state.selectedTaskId)
      ? state.selectedTaskId
      : fallbackToFirst ? tasks[0]?.id ?? null : null;
    setSelectedTaskId(nextTaskId);
    sidebarOpenRef.current = state.sidebarOpen;
    setSidebarOpen(state.sidebarOpen);
    setSidebarWidth(clampSidebarWidth(state.sidebarWidth, window.innerWidth));
    rightPanelOpenRef.current = state.rightPanelOpen;
    setRightPanelOpen(state.rightPanelOpen);
    bottomPanelOpenRef.current = state.bottomPanelOpen;
    setBottomPanelOpen(state.bottomPanelOpen);
    setThemePreference(state.theme);
    return nextTaskId;
  }

  async function setSidebarVisibility(nextOpen: boolean, restoreToggleFocus = false): Promise<boolean> {
    if (accountSwitchLockRef.current) return false;
    const operationEpoch = accountEpochRef.current;
    const previous = sidebarOpenRef.current;
    sidebarOpenRef.current = nextOpen;
    setSidebarOpen(nextOpen);
    try {
      const nextState = await desktop.updateUiState({ sidebarOpen: nextOpen });
      if (operationEpoch !== accountEpochRef.current) return false;
      setSnapshot((current) => ({ ...current, uiState: nextState }));
      applyPersistedUiState(nextState, tasksRef.current);
      if (restoreToggleFocus && !nextState.sidebarOpen) queueMicrotask(() => sidebarToggleRef.current?.focus());
      return true;
    } catch (error) {
      sidebarOpenRef.current = previous;
      setSidebarOpen(previous);
      setNotice(toUserFacingError(error, "侧边栏状态保存失败，请重试。"));
      return false;
    }
  }

  async function setPanelVisibility(panel: "right" | "bottom", nextOpen: boolean): Promise<boolean> {
    if (accountSwitchLockRef.current) return false;
    const operationEpoch = accountEpochRef.current;
    const previous = panel === "right" ? rightPanelOpenRef.current : bottomPanelOpenRef.current;
    if (panel === "right") {
      rightPanelOpenRef.current = nextOpen;
      setRightPanelOpen(nextOpen);
    } else {
      bottomPanelOpenRef.current = nextOpen;
      setBottomPanelOpen(nextOpen);
    }
    try {
      const patch = panel === "right" ? { rightPanelOpen: nextOpen } : { bottomPanelOpen: nextOpen };
      const nextState = await desktop.updateUiState(patch);
      if (operationEpoch !== accountEpochRef.current) return false;
      setSnapshot((current) => ({ ...current, uiState: nextState }));
      applyPersistedUiState(nextState, tasksRef.current);
      return true;
    } catch (error) {
      if (panel === "right") {
        rightPanelOpenRef.current = previous;
        setRightPanelOpen(previous);
      } else {
        bottomPanelOpenRef.current = previous;
        setBottomPanelOpen(previous);
      }
      setNotice(toUserFacingError(error, panel === "right" ? "任务详情面板状态保存失败，请重试。" : "终端面板状态保存失败，请重试。"));
      return false;
    }
  }

  useEffect(() => {
    let alive = true;
    void desktop.loadSnapshot().then((initial) => {
      if (!alive) return;
      tasksRef.current = initial.tasks;
      setSnapshot(initial);
      const state = initial.uiState;
      const nextTaskId = applyPersistedUiState(initial.uiState, initial.tasks, true);
      const nextSidebarWidth = clampSidebarWidth(state.sidebarWidth, window.innerWidth);
      setLoading(false);
      if (nextTaskId && !Object.prototype.hasOwnProperty.call(initial.turnsByTask, nextTaskId)) {
        void handleLoadTaskData(nextTaskId);
      }
      if (nextTaskId !== state.selectedTaskId || nextSidebarWidth !== state.sidebarWidth) {
        void desktop.updateUiState({ selectedTaskId: nextTaskId, sidebarWidth: nextSidebarWidth }).catch(() => undefined);
      }
    }).catch((error: unknown) => {
      if (alive) {
        setNotice(toUserFacingError(error, "无法连接 PAD 本地服务，请重新启动应用。"));
        setLoading(false);
      }
    });
    const unsubscribe = desktop.subscribe((event) => {
      if (accountSwitchLockRef.current) return;
      if (event.type === "auth-updated") setAuthSession(event.session);
      else if (event.type === "menu-action") {
        if (event.action === "new_task") void handleNewTask(null);
        else if (event.action === "search") {
          void setSidebarVisibility(true);
          setSearchOpen(true);
        }
        else if (event.action === "settings") { setSettingsSection("general"); setRoute("settings"); }
        else if (event.action === "toggle_sidebar") void setSidebarVisibility(!sidebarOpenRef.current);
        else if (event.action === "toggle_terminal") void setPanelVisibility("bottom", !bottomPanelOpenRef.current);
      }
      else if (event.type === "snapshot") {
        tasksRef.current = event.snapshot.tasks;
        setSnapshot(event.snapshot);
        applyPersistedUiState(event.snapshot.uiState, event.snapshot.tasks);
      }
      else setSnapshot((current) => applyDesktopEvent(current, event));
    });
    return () => { alive = false; unsubscribe(); };
  }, []);

  useEffect(() => {
    document.documentElement.dataset.theme = theme;
  }, [theme]);

  useEffect(() => {
    if (themePreference !== "system") return undefined;
    const media = window.matchMedia("(prefers-color-scheme: dark)");
    const updateSystemTheme = () => setSystemTheme(media.matches ? "dark" : "light");
    updateSystemTheme();
    media.addEventListener("change", updateSystemTheme);
    return () => media.removeEventListener("change", updateSystemTheme);
  }, [themePreference]);

  useEffect(() => {
    const keepSidebarWithinViewport = () => {
      const nextCompact = isCompactWindow(window.innerWidth);
      setCompactWindow(nextCompact);
      if (nextCompact) return;
      setSidebarWidth((value) => {
        const next = clampSidebarWidth(value, window.innerWidth);
        if (next !== value) void desktop.updateUiState({ sidebarWidth: next }).catch(() => undefined);
        return next;
      });
    };
    window.addEventListener("resize", keepSidebarWithinViewport);
    return () => window.removeEventListener("resize", keepSidebarWithinViewport);
  }, []);

  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape" && !event.metaKey) {
        if (accountSwitchLockRef.current) return;
        if (compactWindow && sidebarOpen) {
          event.preventDefault();
          void setSidebarVisibility(false, true);
        } else if (compactWindow && rightPanelOpen) {
          event.preventDefault();
          void setPanelVisibility("right", false);
        }
        return;
      }
      if (!event.metaKey) return;
      const key = event.key.toLowerCase();
      if (accountSwitchLockRef.current && [",", "b", "j", "k", "n"].includes(key)) {
        event.preventDefault();
        return;
      }
      if (key === "b") {
        event.preventDefault();
        void setSidebarVisibility(!sidebarOpen);
      } else if (event.key === ",") {
        event.preventDefault();
        setSettingsSection("general");
        setRoute("settings");
      } else if (key === "k") {
        event.preventDefault();
        void setSidebarVisibility(true);
        setSearchOpen(true);
      } else if (key === "n") {
        event.preventDefault();
        void handleNewTask(null);
      } else if (key === "j") {
        event.preventDefault();
        void setPanelVisibility("bottom", !bottomPanelOpen);
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [bottomPanelOpen, compactWindow, rightPanelOpen, sidebarOpen]);

  const selectedTask = snapshot.tasks.find((task) => task.id === selectedTaskId) ?? null;
  const selectedProject = snapshot.projects.find((project) => project.id === selectedTask?.projectId) ?? null;
  const selectedTurns = selectedTaskId ? snapshot.turnsByTask[selectedTaskId] ?? [] : [];
  const selectedInteractions = selectedTaskId ? snapshot.interactionsByTask[selectedTaskId] ?? [] : [];
  const selectedTaskDataLoading = selectedTaskId ? taskDataLoading[selectedTaskId] === true : false;
  const selectedTaskDataError = selectedTaskId ? taskDataErrors[selectedTaskId] ?? null : null;

  async function handleLoadTaskData(taskId: string) {
    const requestEpoch = accountEpochRef.current;
    setTaskDataLoading((current) => ({ ...current, [taskId]: true }));
    setTaskDataErrors((current) => {
      const next = { ...current };
      delete next[taskId];
      return next;
    });
    try {
      const next = await desktop.loadTaskData(taskId);
      if (requestEpoch !== accountEpochRef.current || accountSwitchLockRef.current) return;
      tasksRef.current = next.tasks;
      setSnapshot(next);
    } catch (error) {
      if (requestEpoch !== accountEpochRef.current || accountSwitchLockRef.current) return;
      setTaskDataErrors((current) => ({
        ...current,
        [taskId]: toUserFacingError(error, "任务历史加载失败，请重试。"),
      }));
    } finally {
      setTaskDataLoading((current) => {
        const next = { ...current };
        delete next[taskId];
        return next;
      });
    }
  }

  async function handleNewTask(projectId: string | null) {
    if (accountSwitchLockRef.current) return;
    const operationEpoch = accountEpochRef.current;
    try {
      const task = await desktop.createTask(projectId);
      if (accountSwitchLockRef.current || operationEpoch !== accountEpochRef.current) return;
      setSnapshot((current) => ({ ...current, tasks: mergeTask(current.tasks, task), turnsByTask: { ...current.turnsByTask, [task.id]: current.turnsByTask[task.id] ?? [] } }));
      setSelectedTaskId(task.id);
      void desktop.updateUiState({ activeProfileId: task.profileId, selectedTaskId: task.id }).catch(() => undefined);
      setRoute("task");
      if (isCompactWindow(window.innerWidth)) {
        setSidebarOpen(false);
        void desktop.updateUiState({ sidebarOpen: false }).catch(() => undefined);
      }
    } catch (error) {
      setNotice(toUserFacingError(error, "新建任务失败，请稍后重试。"));
    }
  }

  async function handleSend(input: ComposerMessageInput) {
    if (accountSwitchLockRef.current) return;
    const operationEpoch = accountEpochRef.current;
    try {
      if (!selectedTaskId) {
        const task = await desktop.createTask(null);
        if (accountSwitchLockRef.current || operationEpoch !== accountEpochRef.current) return;
        setSnapshot((current) => ({ ...current, tasks: mergeTask(current.tasks, task), turnsByTask: { ...current.turnsByTask, [task.id]: [] } }));
        setSelectedTaskId(task.id);
        void desktop.updateUiState({ activeProfileId: task.profileId, selectedTaskId: task.id }).catch(() => undefined);
        await desktop.sendMessage({ ...input, taskId: task.id, accountId: activeAccountId, fullAccess });
        return;
      }
      await desktop.sendMessage({ ...input, taskId: selectedTaskId, accountId: activeAccountId, fullAccess });
    } catch (error) {
      setNotice(toUserFacingError(error, "发送任务失败，请稍后重试。"));
      throw error;
    }
  }

  async function handleStopTask() {
    if (accountSwitchLockRef.current || !selectedTaskId) return;
    const operationEpoch = accountEpochRef.current;
    try {
      const next = await desktop.abortTask(selectedTaskId);
      if (operationEpoch !== accountEpochRef.current) return;
      tasksRef.current = next.tasks;
      setSnapshot(next);
      applyPersistedUiState(next.uiState, next.tasks);
    } catch (error) {
      setNotice(toUserFacingError(error, "停止任务失败，请稍后重试。"));
    }
  }

  async function handleRetryTask() {
    if (accountSwitchLockRef.current || !selectedTaskId) return;
    const operationEpoch = accountEpochRef.current;
    try {
      const next = await desktop.retryTask(selectedTaskId);
      if (operationEpoch !== accountEpochRef.current) return;
      tasksRef.current = next.tasks;
      setSnapshot(next);
      applyPersistedUiState(next.uiState, next.tasks);
    } catch (error) {
      setNotice(toUserFacingError(error, "重试任务失败，请稍后重试。"));
    }
  }

  async function handleUpdateTask(patch: { pinned?: boolean; archived?: boolean; unread?: boolean }) {
    if (accountSwitchLockRef.current || !selectedTaskId) return;
    const operationEpoch = accountEpochRef.current;
    try {
      const next = await desktop.updateTask(selectedTaskId, patch);
      if (operationEpoch !== accountEpochRef.current) return;
      tasksRef.current = next.tasks;
      setSnapshot(next);
      applyPersistedUiState(next.uiState, next.tasks);
      if (!next.tasks.some((task) => task.id === selectedTaskId)) {
        const persistedSelection = next.uiState.selectedTaskId && next.tasks.some((task) => task.id === next.uiState.selectedTaskId)
          ? next.uiState.selectedTaskId
          : null;
        const nextTaskId = persistedSelection ?? next.tasks[0]?.id ?? null;
        setSelectedTaskId(nextTaskId);
        if (nextTaskId !== next.uiState.selectedTaskId) {
          const nextUiState = await desktop.updateUiState({ selectedTaskId: nextTaskId });
          setSnapshot((current) => ({ ...current, uiState: nextUiState }));
        }
      }
    } catch (error) {
      setNotice(toUserFacingError(error, "任务操作失败，请稍后重试。"));
    }
  }

  async function handleSwitchAccount(accountId: string) {
    if (accountSwitchLockRef.current || accountId === activeAccountId) return;
    accountSwitchLockRef.current = true;
    accountEpochRef.current += 1;
    setSwitchingAccount(true);
    try {
      const next = await desktop.switchAccount(accountId);
      tasksRef.current = next.tasks;
      setSnapshot(next);
      const nextTaskId = applyPersistedUiState(next.uiState, next.tasks, true);
      setTaskDataLoading({});
      setTaskDataErrors({});
      setAttentionOnly(false);
      if (nextTaskId !== next.uiState.selectedTaskId) void desktop.updateUiState({ selectedTaskId: nextTaskId }).catch(() => undefined);
      if (nextTaskId && !Object.prototype.hasOwnProperty.call(next.turnsByTask, nextTaskId)) {
        queueMicrotask(() => void handleLoadTaskData(nextTaskId));
      }
    } catch (error) {
      setNotice(toUserFacingError(error, "账号切换失败，请重试。"));
    } finally {
      accountSwitchLockRef.current = false;
      setSwitchingAccount(false);
    }
  }

  async function handleCreateProject(name: string, directory: string) {
    if (accountSwitchLockRef.current) return;
    const operationEpoch = accountEpochRef.current;
    setProjectBusy(true);
    try {
      const next = await desktop.createProject(name, directory);
      if (operationEpoch !== accountEpochRef.current) return;
      tasksRef.current = next.tasks;
      setSnapshot(next);
      applyPersistedUiState(next.uiState, next.tasks);
      setProjectSheetOpen(false);
    } catch (error) {
      setNotice(toUserFacingError(error, "添加项目失败，请检查目录后重试。"));
    } finally {
      setProjectBusy(false);
    }
  }

  async function handleCreateAccount(name: string, provider?: string) {
    if (accountSwitchLockRef.current) return;
    const operationEpoch = accountEpochRef.current;
    const next = await desktop.createAccount(name, provider);
    if (operationEpoch !== accountEpochRef.current) return;
    tasksRef.current = next.tasks;
    setSnapshot(next);
    applyPersistedUiState(next.uiState, next.tasks);
  }

  function handleSidebarResizeStart(event: ReactPointerEvent<HTMLDivElement>) {
    if (accountSwitchLockRef.current) return;
    event.currentTarget.setPointerCapture(event.pointerId);
    const startX = event.clientX;
    const startWidth = sidebarWidth;
    const onMove = (moveEvent: PointerEvent) => {
      const next = clampSidebarWidth(startWidth + moveEvent.clientX - startX, window.innerWidth);
      setSidebarWidth(next);
    };
    const onEnd = () => {
      window.removeEventListener("pointermove", onMove);
      window.removeEventListener("pointerup", onEnd);
      setSidebarWidth((value) => {
        void desktop.updateUiState({ sidebarWidth: value }).catch(() => undefined);
        return value;
      });
    };
    window.addEventListener("pointermove", onMove);
    window.addEventListener("pointerup", onEnd);
  }

  function handleSidebarResizeBy(delta: number) {
    if (accountSwitchLockRef.current) return;
    setSidebarWidth((value) => {
      const next = clampSidebarWidth(value + delta, window.innerWidth);
      void desktop.updateUiState({ sidebarWidth: next }).catch(() => undefined);
      return next;
    });
  }

  const activeAccount = useMemo(
    () => snapshot.accounts.find((account) => account.active) ?? snapshot.accounts[0] ?? null,
    [snapshot.accounts],
  );
  const activeAccountId = activeAccount?.id ?? "default";
  const fullAccess = activeAccount?.fullAccess ?? false;

  async function handleFullAccessChange(enabled: boolean) {
    if (accountSwitchLockRef.current) return;
    const operationEpoch = accountEpochRef.current;
    const previous = snapshot;
    setSnapshot((current) => ({
      ...current,
      accounts: current.accounts.map((account) => account.id === activeAccountId
        ? { ...account, fullAccess: enabled, policy: { ...account.policy, mode: enabled ? "system_full" : "guarded", unattended: enabled } }
        : account),
    }));
    try {
      const next = await desktop.setFullAccess(activeAccountId, enabled);
      if (operationEpoch !== accountEpochRef.current) return;
      tasksRef.current = next.tasks;
      setSnapshot(next);
      applyPersistedUiState(next.uiState, next.tasks);
    } catch (error) {
      setSnapshot(previous);
      setNotice(toUserFacingError(error, "权限设置保存失败，请重试。"));
    }
  }

  async function handleBeginLogin(accountId: string, provider: string, authType: AuthType) {
    setAuthSession(await desktop.beginLogin(accountId, provider, authType));
  }

  async function handleRefreshLogin(accountId: string) {
    setAuthSession(await desktop.getLoginStatus(accountId));
  }

  async function handleRespondLogin(accountId: string, value: string) {
    setAuthSession(await desktop.respondLogin(accountId, value));
  }

  async function handleCancelLogin(accountId: string) {
    if (authSession && ["authenticated", "failed", "cancelled"].includes(authSession.phase)) {
      setAuthSession(null);
      return;
    }
    await desktop.cancelLogin(accountId);
    setAuthSession(null);
  }

  async function handleLogout(accountId: string, provider?: string) {
    try {
      const next = await desktop.logout(accountId, provider);
      setSnapshot(next);
    } catch (error) {
      setNotice(toUserFacingError(error, "退出登录失败，请稍后重试。"));
    }
  }

  async function handleSidebarViewChange(view: SidebarView) {
    if (accountSwitchLockRef.current) return false;
    const operationEpoch = accountEpochRef.current;
    const previousView = snapshot.uiState.sidebarView;
    const previousTaskId = selectedTaskId;
    setSnapshot((current) => ({
      ...current,
      sidebar: { ...current.sidebar, view },
      uiState: { ...current.uiState, sidebarView: view },
    }));
    try {
      const nextUiState = await desktop.updateUiState({ sidebarView: view });
      if (operationEpoch !== accountEpochRef.current) return false;
      const nextTaskId = applyPersistedUiState(nextUiState, tasksRef.current);
      setSnapshot((current) => ({
        ...current,
        sidebar: { ...current.sidebar, view: nextUiState.sidebarView },
        uiState: nextUiState,
      }));
      if (nextTaskId && nextTaskId !== previousTaskId && !Object.prototype.hasOwnProperty.call(snapshot.turnsByTask, nextTaskId)) {
        void handleLoadTaskData(nextTaskId);
      }
      return true;
    } catch (error) {
      setSelectedTaskId(previousTaskId);
      setSnapshot((current) => ({
        ...current,
        sidebar: { ...current.sidebar, view: previousView },
        uiState: { ...current.uiState, sidebarView: previousView },
      }));
      setNotice(toUserFacingError(error, "任务视图保存失败，请重试。"));
      return false;
    }
  }

  async function handleAttentionOnlyChange(nextAttentionOnly: boolean) {
    if (accountSwitchLockRef.current) return;
    if (nextAttentionOnly && snapshot.uiState.sidebarView !== "all") {
      const switched = await handleSidebarViewChange("all");
      if (!switched) return;
    }
    setAttentionOnly(nextAttentionOnly);
  }

  async function handleSelectTask(taskId: string) {
    if (accountSwitchLockRef.current) return;
    const operationEpoch = accountEpochRef.current;
    const task = snapshot.tasks.find((candidate) => candidate.id === taskId);
    if (!task) return;
    const previousTaskId = selectedTaskId;
    const previousUiState = snapshot.uiState;
    setSelectedTaskId(taskId);
    setSnapshot((current) => ({ ...current, uiState: { ...current.uiState, selectedTaskId: taskId } }));
    try {
      const nextUiState = await desktop.updateUiState({ selectedTaskId: taskId });
      if (operationEpoch !== accountEpochRef.current) return;
      setSnapshot((current) => ({ ...current, uiState: nextUiState }));
      applyPersistedUiState(nextUiState, tasksRef.current);
      if (!Object.prototype.hasOwnProperty.call(snapshot.turnsByTask, taskId)) {
        void handleLoadTaskData(taskId);
      }
      setRoute("task");
      if (compactWindow) void setSidebarVisibility(false);
    } catch (error) {
      setSelectedTaskId(previousTaskId);
      setSnapshot((current) => ({ ...current, uiState: previousUiState }));
      setNotice(toUserFacingError(error, "任务选择状态保存失败，请重试。"));
    }
  }

  return (
    <div
      className={`app-shell ${sidebarOpen ? "sidebar-visible" : "sidebar-hidden"}${loading ? " is-loading" : ""}`}
      data-active-profile-id={activeAccountId}
      data-selected-task-id={selectedTaskId ?? ""}
      data-theme={theme}
      data-theme-preference={themePreference}
      data-compact-window={String(compactWindow)}
      aria-busy={switchingAccount}
    >
      <Titlebar
        sidebarOpen={sidebarOpen}
        disabled={switchingAccount}
        sidebarButtonRef={sidebarToggleRef}
        onSidebarToggle={() => void setSidebarVisibility(!sidebarOpenRef.current)}
      />
      <div id="overlay-root" className="overlay-root" />
      {notice && <div className="error-banner" role="alert">
        <span>{notice.message}</span>
        {notice.diagnostic && <details><summary>诊断信息</summary><code>{notice.diagnostic}</code></details>}
        <button onClick={() => setNotice(null)} aria-label="关闭错误提示"><Icon name="x" /></button>
      </div>}
      <div className="app-body" inert={switchingAccount || undefined}>
        {sidebarOpen && <>
          <div className="sidebar-backdrop" onClick={() => {
            void setSidebarVisibility(false, true);
          }} />
          <Sidebar
            accounts={snapshot.accounts}
            projects={snapshot.projects}
            hierarchy={snapshot.sidebar}
            selectedTaskId={selectedTaskId}
            collapsedKeys={[
              ...snapshot.uiState.collapsedSectionIds.map((id) => collapsedKey("section", id)),
              ...snapshot.uiState.collapsedProjectIds.map((id) => collapsedKey("project", id)),
            ]}
            width={sidebarWidth}
            searchOpen={searchOpen}
            attentionOnly={attentionOnly}
            onSearchOpenChange={(value) => { if (!accountSwitchLockRef.current) setSearchOpen(value); }}
            onAttentionOnlyChange={(value) => void handleAttentionOnlyChange(value)}
            onAddProject={() => { if (!accountSwitchLockRef.current) setProjectSheetOpen(true); }}
            onNewTask={(projectId) => void handleNewTask(projectId)}
            onSelectTask={(taskId) => void handleSelectTask(taskId)}
            onOpenSettings={(section) => { setSettingsSection(section ?? "general"); setRoute("settings"); }}
            onResizeStart={handleSidebarResizeStart}
            onResizeBy={handleSidebarResizeBy}
            onSwitchAccount={(accountId) => void handleSwitchAccount(accountId)}
            onViewChange={(view) => {
              if (accountSwitchLockRef.current) return;
              setAttentionOnly(false);
              void handleSidebarViewChange(view);
            }}
            onCollapsedChange={(keys) => {
              if (accountSwitchLockRef.current) return;
              const collapsedSectionIds = keys.filter((key) => key.startsWith("section:"));
              const collapsedProjectIds = keys.filter((key) => key.startsWith("project:"));
              setSnapshot((current) => ({ ...current, uiState: { ...current.uiState, collapsedSectionIds, collapsedProjectIds } }));
              void desktop.updateUiState({ collapsedSectionIds, collapsedProjectIds }).catch((error: unknown) => {
                setNotice(toUserFacingError(error, "侧边栏折叠状态保存失败。"));
              });
            }}
          />
        </>}
        <div className="workspace" inert={(compactWindow && sidebarOpen) || undefined}>
          {route === "settings" ? (
            <SettingsView
              accounts={snapshot.accounts}
              backend={snapshot.backend}
              theme={themePreference}
              fullAccess={fullAccess}
              initialSection={settingsSection}
              authSession={authSession}
              onThemeChange={(next) => {
                const previous = themePreference;
                setThemePreference(next);
                void desktop.updateUiState({ theme: next }).catch((error: unknown) => {
                  setThemePreference(previous);
                  setNotice(toUserFacingError(error, "外观设置保存失败。"));
                });
              }}
              onFullAccessChange={(value) => void handleFullAccessChange(value)}
              onSwitchAccount={async (accountId) => { await handleSwitchAccount(accountId); }}
              onBeginLogin={handleBeginLogin}
              onCreateAccount={handleCreateAccount}
              onRefreshLogin={handleRefreshLogin}
              onRespondLogin={handleRespondLogin}
              onCancelLogin={handleCancelLogin}
              onDismissLogin={() => setAuthSession(null)}
              onLogout={handleLogout}
              onBack={() => setRoute("task")}
            />
          ) : (
            <div className="workspace-stack">
              <div className="workspace-row">
                <div className="primary-workspace-content" inert={(compactWindow && rightPanelOpen) || undefined}>
                {selectedTask && selectedTaskDataError ? (
                  <div className="empty-task task-data-error" role="alert">
                    <div className="empty-logo"><Icon name="archive" /></div>
                    <h1>无法加载任务记录</h1>
                    <p>{selectedTaskDataError.message}</p>
                    {selectedTaskDataError.diagnostic && <details><summary>诊断信息</summary><code>{selectedTaskDataError.diagnostic}</code></details>}
                    <button className="settings-primary-button" onClick={() => void handleLoadTaskData(selectedTask.id)}>重试加载任务记录</button>
                  </div>
                ) : selectedTask && selectedTaskDataLoading ? (
                  <div className="empty-task task-data-loading" role="status" aria-live="polite">
                    <span className="loading-spinner" />
                    <h1>正在加载任务记录…</h1>
                  </div>
                ) : <TaskView
                  task={selectedTask}
                  project={selectedProject}
                  activeAccount={activeAccount}
                  turns={selectedTurns}
                  interactions={selectedInteractions}
                  fullAccess={fullAccess}
                  rightPanelOpen={rightPanelOpen}
                  bottomPanelOpen={bottomPanelOpen}
                  onFullAccessChange={(value) => void handleFullAccessChange(value)}
                  onRightPanelToggle={() => void setPanelVisibility("right", !rightPanelOpenRef.current)}
                  onBottomPanelToggle={() => void setPanelVisibility("bottom", !bottomPanelOpenRef.current)}
                  onChooseAttachments={desktop.chooseAttachments}
                  onSend={handleSend}
                  onStop={handleStopTask}
                  onRetry={handleRetryTask}
                  onRespondInteraction={async (taskId, requestId, response) => {
                    if (accountSwitchLockRef.current) return;
                    await desktop.respondInteraction(taskId, requestId, response);
                  }}
                  onUpdateTask={handleUpdateTask}
                  onOpenSettings={() => setRoute("settings")}
                />}
                </div>
                {compactWindow && rightPanelOpen && <button className="right-panel-backdrop" aria-label="关闭任务详情面板" onClick={() => void setPanelVisibility("right", false)} />}
                {rightPanelOpen && <RightPanel task={selectedTask} project={selectedProject} turns={selectedTurns} onClose={() => {
                  void setPanelVisibility("right", false);
                }} />}
              </div>
              {bottomPanelOpen && (
                <BottomPanel
                  task={selectedTask}
                  onClose={() => {
                    void setPanelVisibility("bottom", false);
                  }}
                  onOpenTerminal={desktop.openTerminal}
                  onTerminalInput={desktop.writeTerminal}
                  onTerminalResize={desktop.resizeTerminal}
                  onTerminalSnapshot={desktop.getTerminalSnapshot}
                  onTerminalClose={desktop.closeTerminal}
                />
              )}
            </div>
          )}
        </div>
      </div>
      {switchingAccount && <div className="account-switching-overlay" role="status" aria-live="polite"><span className="loading-spinner" />正在切换账号…</div>}
      {projectSheetOpen && (
        <ProjectSheet
          busy={projectBusy}
          onChooseDirectory={desktop.chooseProjectDirectory}
          onCreate={handleCreateProject}
          onCancel={() => { if (!projectBusy) setProjectSheetOpen(false); }}
        />
      )}
    </div>
  );
}
