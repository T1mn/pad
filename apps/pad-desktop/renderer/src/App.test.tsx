import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { snapshot } from "./test/fixtures";
import type { DesktopEvent, DesktopSnapshot } from "./types";

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason?: unknown) => void;
  const promise = new Promise<T>((nextResolve, nextReject) => {
    resolve = nextResolve;
    reject = nextReject;
  });
  return { promise, resolve, reject };
}

const desktopMock = vi.hoisted(() => ({
  loadSnapshot: vi.fn(),
  loadTaskData: vi.fn(),
  chooseProjectDirectory: vi.fn(),
  chooseAttachments: vi.fn(),
  createProject: vi.fn(),
  createTask: vi.fn(),
  createAccount: vi.fn(),
  sendMessage: vi.fn(),
  switchAccount: vi.fn(),
  setFullAccess: vi.fn(),
  beginLogin: vi.fn(),
  getLoginStatus: vi.fn(),
  respondLogin: vi.fn(),
  cancelLogin: vi.fn(),
  logout: vi.fn(),
  abortTask: vi.fn(),
  retryTask: vi.fn(),
  updateTask: vi.fn(),
  respondInteraction: vi.fn(),
  openTerminal: vi.fn(),
  writeTerminal: vi.fn(),
  resizeTerminal: vi.fn(),
  getTerminalSnapshot: vi.fn(),
  closeTerminal: vi.fn(),
  updateUiState: vi.fn(),
  getRemoteStatus: vi.fn(),
  setRemoteEnabled: vi.fn(),
  beginRemotePairing: vi.fn(),
  cancelRemotePairing: vi.fn(),
  revokeRemoteDevice: vi.fn(),
  subscribe: vi.fn((_listener: (event: DesktopEvent) => void) => () => undefined),
}));

vi.mock("./lib/desktop", () => ({ desktop: desktopMock }));

import App, { clampSidebarWidth, isCompactWindow } from "./App";

describe("App account isolation and compact layout", () => {
  let emitDesktopEvent: ((event: DesktopEvent) => void) | null;

  beforeEach(() => {
    emitDesktopEvent = null;
    desktopMock.loadSnapshot.mockReset().mockResolvedValue(snapshot("personal"));
    desktopMock.loadTaskData.mockReset().mockImplementation(async () => snapshot("personal"));
    desktopMock.chooseAttachments.mockReset().mockResolvedValue([]);
    desktopMock.createTask.mockReset();
    desktopMock.sendMessage.mockReset().mockResolvedValue(undefined);
    desktopMock.updateTask.mockReset();
    desktopMock.switchAccount.mockReset().mockResolvedValue(snapshot("team"));
    desktopMock.updateUiState.mockReset().mockImplementation(async (patch) => ({ ...snapshot("personal").uiState, ...patch }));
    desktopMock.getRemoteStatus.mockReset().mockResolvedValue(null);
    desktopMock.setRemoteEnabled.mockReset();
    desktopMock.beginRemotePairing.mockReset();
    desktopMock.cancelRemotePairing.mockReset();
    desktopMock.revokeRemoteDevice.mockReset();
    desktopMock.subscribe.mockReset().mockImplementation((listener: (event: DesktopEvent) => void) => {
      emitDesktopEvent = listener;
      return () => undefined;
    });
    window.innerWidth = 1280;
    Object.defineProperty(window, "matchMedia", {
      configurable: true,
      writable: true,
      value: (query: string) => ({
        matches: false,
        media: query,
        onchange: null,
        addListener: () => undefined,
        removeListener: () => undefined,
        addEventListener: () => undefined,
        removeEventListener: () => undefined,
        dispatchEvent: () => false,
      }),
    });
  });

  it("切换账号后只展示新账号任务", async () => {
    render(<App />);
    const user = userEvent.setup();
    expect((await screen.findAllByText("个人任务")).length).toBeGreaterThan(0);

    await user.click(screen.getByRole("button", { name: /个人账号/ }));
    await user.click(screen.getByRole("menuitem", { name: /团队账号/ }));

    await waitFor(() => expect(screen.getAllByText("团队机密任务").length).toBeGreaterThan(0));
    expect(screen.queryByText("个人任务")).not.toBeInTheDocument();
  });

  it("账号切换失败时原子恢复原账号层级", async () => {
    desktopMock.switchAccount.mockRejectedValueOnce({ code: "profile_not_found", message: "missing profile" });
    render(<App />);
    const user = userEvent.setup();
    expect((await screen.findAllByText("个人任务")).length).toBeGreaterThan(0);

    await user.click(screen.getByRole("button", { name: /个人账号/ }));
    await user.click(screen.getByRole("menuitem", { name: /团队账号/ }));

    expect(await screen.findByRole("alert")).toHaveTextContent("账号切换失败");
    expect(screen.getAllByText("个人任务").length).toBeGreaterThan(0);
    expect(screen.queryByText("团队机密任务")).not.toBeInTheDocument();
  });

  it("账号切换事务锁住快捷键与侧边栏竞态，并在成功前保持旧层级", async () => {
    const pending = deferred<DesktopSnapshot>();
    desktopMock.switchAccount.mockReturnValueOnce(pending.promise);
    render(<App />);
    await screen.findAllByText("个人任务");
    const user = userEvent.setup();
    await user.click(screen.getByRole("button", { name: /个人账号/ }));
    const team = screen.getByRole("menuitem", { name: /团队账号/ });
    act(() => {
      fireEvent.click(team);
      fireEvent.click(team);
    });

    expect(document.querySelector(".app-shell")).toHaveAttribute("aria-busy", "true");
    expect(document.querySelector(".app-body")).toHaveAttribute("inert");
    expect(screen.getAllByText("个人任务").length).toBeGreaterThan(0);
    expect(screen.queryByText("团队机密任务")).not.toBeInTheDocument();
    for (const key of ["n", "k", "b", "j"]) fireEvent.keyDown(window, { key, metaKey: true });
    fireEvent.click(screen.getByRole("treeitem", { name: "个人任务" }));
    expect(desktopMock.switchAccount).toHaveBeenCalledTimes(1);
    expect(desktopMock.createTask).not.toHaveBeenCalled();
    expect(desktopMock.updateUiState).not.toHaveBeenCalled();

    await act(async () => pending.resolve(snapshot("team")));
    await waitFor(() => expect(document.querySelector(".app-shell")).toHaveAttribute("aria-busy", "false"));
    expect(screen.getAllByText("团队机密任务").length).toBeGreaterThan(0);
    expect(screen.queryByText("个人任务")).not.toBeInTheDocument();
  });

  it("compact 侧边栏覆盖时主区 inert，Escape 关闭后焦点回到标题栏入口", async () => {
    window.innerWidth = 700;
    render(<App />);
    await screen.findAllByText("个人任务");
    expect(document.querySelector(".workspace")).toHaveAttribute("inert");

    fireEvent.keyDown(window, { key: "Escape" });
    await waitFor(() => expect(screen.queryByLabelText("任务侧边栏")).not.toBeInTheDocument());
    expect(document.querySelector(".workspace")).not.toHaveAttribute("inert");
    expect(screen.getByRole("button", { name: "显示侧边栏" })).toHaveFocus();
  });

  it("snapshot 事件统一同步 persisted selection、sidebar、theme 与 panels", async () => {
    render(<App />);
    await screen.findAllByText("个人任务");
    const next = snapshot("team");
    next.uiState = {
      ...next.uiState,
      sidebarWidth: 333,
      sidebarOpen: true,
      theme: "dark",
      rightPanelOpen: true,
      bottomPanelOpen: false,
    };
    act(() => emitDesktopEvent?.({ type: "snapshot", snapshot: next }));

    await waitFor(() => expect(document.querySelector(".app-shell")).toHaveAttribute("data-selected-task-id", "team-task"));
    expect(screen.getByLabelText("任务侧边栏")).toHaveAttribute("data-sidebar-width", "333");
    expect(document.querySelector(".app-shell")).toHaveAttribute("data-theme", "dark");
    expect(screen.getByRole("button", { name: "切换右侧面板" })).toHaveAttribute("aria-pressed", "true");
    expect(screen.getByRole("button", { name: "切换终端" })).toHaveAttribute("aria-pressed", "false");
  });

  it("Command+W 菜单动作依次关闭终端和右侧面板，不关闭窗口", async () => {
    const initial = snapshot("personal");
    let persisted = { ...initial.uiState, rightPanelOpen: true, bottomPanelOpen: true };
    initial.uiState = persisted;
    desktopMock.loadSnapshot.mockResolvedValueOnce(initial);
    desktopMock.updateUiState.mockImplementation(async (patch) => {
      persisted = { ...persisted, ...patch };
      return persisted;
    });
    desktopMock.openTerminal.mockResolvedValueOnce({
      paneId: "terminal-1",
      taskId: "personal-task",
      label: "任务终端",
      status: "running",
    });
    desktopMock.getTerminalSnapshot.mockResolvedValue({
      paneId: "terminal-1",
      taskId: "personal-task",
      label: "任务终端",
      status: "running",
      lines: [],
    });
    desktopMock.closeTerminal.mockResolvedValue(undefined);
    render(<App />);

    expect(await screen.findByLabelText("终端面板")).toBeInTheDocument();
    expect(screen.getByLabelText("任务详情面板")).toBeInTheDocument();
    act(() => emitDesktopEvent?.({ type: "menu-action", action: "close_active" }));
    await waitFor(() => expect(screen.queryByLabelText("终端面板")).not.toBeInTheDocument());
    expect(screen.getByLabelText("任务详情面板")).toBeInTheDocument();

    act(() => emitDesktopEvent?.({ type: "menu-action", action: "close_active" }));
    await waitFor(() => expect(screen.queryByLabelText("任务详情面板")).not.toBeInTheDocument());
  });

  it("任务选择与右侧面板持久化失败时回滚并给出可见错误", async () => {
    const initial = snapshot("personal");
    initial.tasks.push({ ...initial.tasks[0]!, id: "second-task", title: "第二任务" });
    initial.sidebar.rows.push({ ...initial.sidebar.rows[2]!, key: "task:second-task", id: "second-task", title: "第二任务" });
    desktopMock.loadSnapshot.mockResolvedValueOnce(initial);
    desktopMock.updateUiState.mockRejectedValueOnce(new Error("selection write failed"));
    render(<App />);
    await screen.findAllByText("个人任务");
    const user = userEvent.setup();

    await user.click(screen.getByRole("treeitem", { name: "第二任务" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("任务选择状态保存失败，请重试。");
    expect(document.querySelector(".app-shell")).toHaveAttribute("data-selected-task-id", "personal-task");

    await user.click(screen.getByRole("button", { name: "关闭错误提示" }));
    desktopMock.updateUiState.mockRejectedValueOnce(new Error("panel write failed"));
    await user.click(screen.getByRole("button", { name: "切换右侧面板" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("任务详情面板状态保存失败，请重试。");
    expect(screen.getByRole("button", { name: "切换右侧面板" })).toHaveAttribute("aria-pressed", "false");
    expect(screen.queryByLabelText("任务详情面板")).not.toBeInTheDocument();
  });

  it("compact 右侧面板使用 backdrop 和 inert，Escape 关闭", async () => {
    window.innerWidth = 700;
    const initial = snapshot("personal");
    initial.uiState = { ...initial.uiState, sidebarOpen: false };
    desktopMock.loadSnapshot.mockResolvedValueOnce(initial);
    desktopMock.updateUiState
      .mockResolvedValueOnce({ ...initial.uiState, rightPanelOpen: true })
      .mockResolvedValueOnce({ ...initial.uiState, rightPanelOpen: false });
    render(<App />);
    const user = userEvent.setup();
    await user.click(await screen.findByRole("button", { name: "切换右侧面板" }));

    expect(await screen.findByLabelText("任务详情面板")).toBeInTheDocument();
    expect(document.querySelector(".primary-workspace-content")).toHaveAttribute("inert");
    expect(screen.getByRole("button", { name: "关闭任务详情面板" })).toBeInTheDocument();
    fireEvent.keyDown(window, { key: "Escape" });
    await waitFor(() => expect(screen.queryByLabelText("任务详情面板")).not.toBeInTheDocument());
    expect(document.querySelector(".primary-workspace-content")).not.toHaveAttribute("inert");
  });

  it("通过原生目录 Sheet 添加当前账号项目", async () => {
    desktopMock.chooseProjectDirectory.mockResolvedValueOnce("/tmp/PAD 新项目");
    const next = snapshot("personal");
    next.projects = [
      ...next.projects,
      { id: "new-project", profileId: "personal", name: "PAD 新项目", path: "/tmp/PAD 新项目", accent: "#3584e4", expanded: true, pinned: false },
    ];
    next.sidebar.rows.push({
      key: "project:new-project",
      kind: "project",
      id: "new-project",
      depth: 0,
      title: "PAD 新项目",
      status: "none",
      unread: false,
      pinned: false,
      archived: false,
      missingReference: false,
    });
    desktopMock.createProject.mockResolvedValueOnce(next);
    render(<App />);
    const user = userEvent.setup();
    await screen.findAllByText("个人任务");

    await user.click(screen.getByRole("button", { name: "添加项目" }));
    await user.click(screen.getByRole("button", { name: "选择…" }));
    await user.click(screen.getByRole("button", { name: "添加项目" }));

    await waitFor(() => expect(desktopMock.createProject).toHaveBeenCalledWith("PAD 新项目", "/tmp/PAD 新项目"));
    expect(screen.queryByRole("dialog", { name: "添加项目" })).not.toBeInTheDocument();
    expect(screen.getByRole("treeitem", { name: "PAD 新项目 项目" })).toBeInTheDocument();
  });

  it("窄窗口选择任务后收起覆盖式侧边栏", async () => {
    window.innerWidth = 700;
    render(<App />);
    const user = userEvent.setup();
    await user.click(await screen.findByRole("treeitem", { name: /个人任务/ }));

    expect(screen.queryByLabelText("任务侧边栏")).not.toBeInTheDocument();
    expect(isCompactWindow(720)).toBe(true);
    expect(isCompactWindow(721)).toBe(false);
  });

  it("平铺侧边栏始终为主内容保留 320px", () => {
    expect(clampSidebarWidth(520, 1280)).toBe(520);
    expect(clampSidebarWidth(520, 800)).toBe(480);
    expect(clampSidebarWidth(100, 800)).toBe(240);
  });

  it("backend unavailable 显示中文提示，英文只进入诊断折叠项", async () => {
    desktopMock.loadSnapshot.mockRejectedValue({ code: "backend_unavailable", message: "raw host process unavailable" });
    render(<App />);

    const alert = await screen.findByRole("alert");
    expect(alert).toHaveTextContent("PAD 本地服务暂时不可用，请重新启动应用。");
    expect(screen.getByText("诊断信息")).toBeInTheDocument();
  });

  it("auth failed 显示中文登录提示", async () => {
    const data = snapshot("personal");
    data.accounts[0] = { ...data.accounts[0]!, authentication: "missing", authenticatedProviders: [] };
    desktopMock.loadSnapshot.mockResolvedValue(data);
    desktopMock.beginLogin.mockRejectedValue({ code: "auth_failed", message: "provider authentication failed" });
    render(<App />);
    const user = userEvent.setup();
    await screen.findAllByText("个人任务");

    await user.click(screen.getByRole("button", { name: /设置/ }));
    await user.click(screen.getByRole("button", { name: "账号" }));
    await user.click(screen.getByRole("button", { name: "登录" }));
    await user.click(screen.getByRole("button", { name: "开始登录" }));

    const alerts = await screen.findAllByRole("alert");
    expect(alerts.some((alert) => alert.textContent?.includes("模型账号登录失败，请重新登录后再试。"))).toBe(true);
  });

  it("request timeout 显示中文任务提示", async () => {
    desktopMock.sendMessage.mockRejectedValue({ code: "request_timeout", message: "deadline exceeded while waiting" });
    render(<App />);
    const user = userEvent.setup();
    await user.type(await screen.findByLabelText("任务输入"), "执行测试");
    await user.click(screen.getByRole("button", { name: "发送" }));

    expect(await screen.findByRole("alert")).toHaveTextContent("请求超时，Pi 可能仍在后台运行，请稍后重试。");
    expect(screen.getByLabelText("任务输入")).toHaveValue("执行测试");
  });

  it("把原生附件、当前账号模型和推理强度完整传给当前任务", async () => {
    desktopMock.chooseAttachments.mockResolvedValueOnce(["/tmp/spec.md"]);
    render(<App />);
    const user = userEvent.setup();
    const input = await screen.findByLabelText("任务输入");

    await user.click(screen.getByRole("button", { name: "添加附件" }));
    await screen.findByText("spec.md");
    await user.selectOptions(screen.getByLabelText("推理强度"), "high");
    await user.type(input, "执行验收");
    await user.click(screen.getByRole("button", { name: "发送" }));

    await waitFor(() => expect(desktopMock.sendMessage).toHaveBeenCalledWith({
      taskId: "personal-task",
      accountId: "personal",
      fullAccess: false,
      text: "执行验收",
      attachmentPaths: ["/tmp/spec.md"],
      provider: "openai",
      model: "gpt-5.4",
      thinkingLevel: "high",
      fastMode: true,
    }));
    expect(input).toHaveValue("");
    expect(screen.queryByText("spec.md")).not.toBeInTheDocument();
  });

  it("切换侧边栏视图会持久化，失败时回滚并显示中文错误", async () => {
    desktopMock.updateUiState.mockResolvedValueOnce({
      ...snapshot("personal").uiState,
      sidebarView: "archive",
      selectedTaskId: null,
    });
    render(<App />);
    const user = userEvent.setup();
    await screen.findAllByText("个人任务");

    await user.click(screen.getByRole("button", { name: "归档" }));
    await waitFor(() => expect(desktopMock.updateUiState).toHaveBeenCalledWith({ sidebarView: "archive" }));
    expect(screen.getByRole("button", { name: "归档" })).toHaveAttribute("aria-pressed", "true");
    expect(document.querySelector(".app-shell")).toHaveAttribute("data-selected-task-id", "");

    desktopMock.updateUiState.mockRejectedValueOnce(new Error("persist failed"));
    await user.click(screen.getByRole("button", { name: "置顶" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("任务视图保存失败，请重试。");
    expect(screen.getByRole("button", { name: "归档" })).toHaveAttribute("aria-pressed", "true");
  });

  it("置顶视图取消当前任务固定后选择首个仍可见任务或清空", async () => {
    const pinned = snapshot("personal");
    pinned.uiState = { ...pinned.uiState, sidebarView: "pinned" };
    pinned.sidebar = {
      ...pinned.sidebar,
      view: "pinned",
      rows: pinned.sidebar.rows.map((row) => row.kind === "task" ? { ...row, pinned: true } : row),
    };
    pinned.tasks = pinned.tasks.map((task) => ({ ...task, pinned: true }));
    const afterUnpin = structuredClone(pinned);
    afterUnpin.tasks = [];
    afterUnpin.sidebar.rows = afterUnpin.sidebar.rows.filter((row) => row.kind === "new_task");
    desktopMock.loadSnapshot.mockResolvedValueOnce(pinned);
    desktopMock.updateTask.mockResolvedValueOnce(afterUnpin);
    desktopMock.updateUiState.mockResolvedValueOnce({ ...afterUnpin.uiState, selectedTaskId: null });
    render(<App />);
    const user = userEvent.setup();

    await user.click(await screen.findByRole("button", { name: "更多任务操作" }));
    await user.click(screen.getByRole("menuitem", { name: "取消固定" }));

    await waitFor(() => expect(desktopMock.updateTask).toHaveBeenCalledWith("personal-task", { pinned: false }));
    expect(desktopMock.updateUiState).toHaveBeenCalledWith({ selectedTaskId: null });
    expect(document.querySelector(".app-shell")).toHaveAttribute("data-selected-task-id", "");
  });

  it("当前视图没有任务时发送会创建任务后再提交完整 Composer 参数", async () => {
    const emptyPinned = snapshot("personal");
    emptyPinned.tasks = [];
    emptyPinned.projects = [];
    emptyPinned.turnsByTask = {};
    emptyPinned.interactionsByTask = {};
    emptyPinned.sidebar = { ...emptyPinned.sidebar, view: "pinned", rows: emptyPinned.sidebar.rows.filter((row) => row.kind === "new_task") };
    emptyPinned.uiState = { ...emptyPinned.uiState, sidebarView: "pinned", selectedTaskId: null };
    const created = { ...snapshot("personal").tasks[0]!, id: "created-task", title: "新任务" };
    desktopMock.loadSnapshot.mockResolvedValueOnce(emptyPinned);
    desktopMock.createTask.mockResolvedValueOnce(created);
    render(<App />);
    const user = userEvent.setup();

    await user.type(await screen.findByLabelText("任务输入"), "从空视图开始");
    await user.click(screen.getByRole("button", { name: "发送" }));

    await waitFor(() => expect(desktopMock.createTask).toHaveBeenCalledWith(null));
    expect(desktopMock.sendMessage).toHaveBeenCalledWith(expect.objectContaining({
      taskId: "created-task",
      accountId: "personal",
      text: "从空视图开始",
    }));
    expect(desktopMock.createTask.mock.invocationCallOrder[0]).toBeLessThan(desktopMock.sendMessage.mock.invocationCallOrder[0]!);
  });

  it("选择第九个任务时真实按需加载，失败可见且重试后显示历史", async () => {
    const initial = snapshot("personal");
    const baseTask = initial.tasks[0]!;
    for (let index = 2; index <= 9; index += 1) {
      const taskId = `personal-task-${index}`;
      initial.tasks.push({ ...baseTask, id: taskId, title: `第${index}任务` });
      initial.sidebar.rows.push({
        key: `task:${taskId}`,
        kind: "task",
        id: taskId,
        depth: 1,
        title: `第${index}任务`,
        status: "idle",
        unread: false,
        pinned: false,
        archived: false,
        missingReference: false,
      });
      if (index < 9) initial.turnsByTask[taskId] = [];
    }
    desktopMock.loadSnapshot.mockResolvedValue(initial);
    const hydrated = structuredClone(initial);
    hydrated.turnsByTask["personal-task-9"] = [{
      id: "history-9",
      kind: "assistant",
      body: "第九任务历史已加载",
    }];
    desktopMock.loadTaskData
      .mockRejectedValueOnce({ code: "history_unavailable", message: "history storage temporarily unavailable" })
      .mockResolvedValueOnce(hydrated);

    render(<App />);
    const user = userEvent.setup();
    await user.click(await screen.findByRole("treeitem", { name: /第9任务/ }));

    expect(await screen.findByRole("alert")).toHaveTextContent("无法加载任务记录");
    expect(screen.queryByText("从一个任务开始")).not.toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "重试加载任务记录" }));

    expect(await screen.findByText("第九任务历史已加载")).toBeInTheDocument();
    expect(desktopMock.loadTaskData).toHaveBeenNthCalledWith(1, "personal-task-9");
    expect(desktopMock.loadTaskData).toHaveBeenNthCalledWith(2, "personal-task-9");
  });

  it("创建第二个 Profile 后立即切换并隔离旧任务", async () => {
    const created = snapshot("personal");
    created.accounts = [
      ...created.accounts.map((account) => ({ ...account, active: false })),
      {
        ...created.accounts[0]!,
        id: "second",
        name: "第二账号",
        active: true,
        authentication: "missing",
        provider: "Pi · anthropic",
        selectedProvider: "anthropic",
      },
    ];
    created.projects = [];
    created.tasks = [];
    created.turnsByTask = {};
    created.sidebar = { ...created.sidebar, activeProfileId: "second", selectedKey: null, rows: [created.sidebar.rows[0]!] };
    desktopMock.createAccount.mockResolvedValue(created);
    render(<App />);
    const user = userEvent.setup();
    await screen.findAllByText("个人任务");

    await user.click(screen.getByRole("button", { name: /设置/ }));
    await user.click(screen.getByRole("button", { name: "账号" }));
    await user.click(screen.getByRole("button", { name: "新增账号" }));
    await user.type(screen.getByLabelText("账号名称"), "第二账号");
    const provider = screen.getByLabelText("模型提供商");
    await user.selectOptions(provider, "anthropic");
    await user.click(screen.getByRole("button", { name: "创建账号" }));

    await waitFor(() => expect(document.querySelector('[data-active-profile-id="second"]')).toBeInTheDocument());
    expect(document.querySelector('[data-account-id="second"]')).toHaveTextContent("第二账号");
    expect(screen.queryByText("个人任务")).not.toBeInTheDocument();
    expect(desktopMock.createAccount).toHaveBeenCalledWith("第二账号", "anthropic");
  });

  it("启动从本地 UI state 恢复活动账号与最后选择的任务", async () => {
    const persisted = snapshot("team");
    persisted.uiState = { ...persisted.uiState, activeProfileId: "team", selectedTaskId: "team-task", sidebarWidth: 318, theme: "dark" };
    desktopMock.loadSnapshot.mockResolvedValue(persisted);
    render(<App />);
    await waitFor(() => expect(document.querySelector('[data-active-profile-id="team"]')).toHaveAttribute("data-selected-task-id", "team-task"));
    expect(screen.getAllByText("团队机密任务").length).toBeGreaterThan(0);
    expect(screen.queryByText("个人任务")).not.toBeInTheDocument();
    expect(screen.getByLabelText("任务侧边栏")).toHaveAttribute("data-sidebar-width", "318");
    expect(document.querySelector(".app-shell")).toHaveAttribute("data-theme", "dark");
  });

  it("保留跟随系统偏好并响应 macOS 外观变化", async () => {
    const listeners = new Set<() => void>();
    let dark = false;
    Object.defineProperty(window, "matchMedia", {
      configurable: true,
      writable: true,
      value: (query: string) => ({
        get matches() { return dark; },
        media: query,
        onchange: null,
        addListener: () => undefined,
        removeListener: () => undefined,
        addEventListener: (_type: string, listener: () => void) => listeners.add(listener),
        removeEventListener: (_type: string, listener: () => void) => listeners.delete(listener),
        dispatchEvent: () => false,
      }),
    });
    const persisted = snapshot("personal");
    persisted.uiState = { ...persisted.uiState, theme: "system" };
    desktopMock.loadSnapshot.mockResolvedValue(persisted);
    render(<App />);

    await waitFor(() => expect(document.querySelector(".app-shell")).toHaveAttribute("data-theme-preference", "system"));
    expect(document.querySelector(".app-shell")).toHaveAttribute("data-theme", "light");
    dark = true;
    listeners.forEach((listener) => listener());
    await waitFor(() => expect(document.querySelector(".app-shell")).toHaveAttribute("data-theme", "dark"));
  });
});
