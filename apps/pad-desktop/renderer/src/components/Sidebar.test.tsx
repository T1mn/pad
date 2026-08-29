import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { account, snapshot } from "../test/fixtures";
import { Sidebar } from "./Sidebar";

describe("Sidebar", () => {
  it("把新任务放在搜索之前，并从树的单一数据流中移除重复入口", async () => {
    const data = snapshot();
    const onNewTask = vi.fn();
    renderSidebarWith(data, { onNewTask });
    const user = userEvent.setup();
    const newTask = screen.getByRole("button", { name: "新任务" });
    const search = screen.getByRole("button", { name: "搜索" });

    expect(newTask.compareDocumentPosition(search) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    expect(screen.queryByRole("treeitem", { name: "新任务" })).not.toBeInTheDocument();
    await user.click(newTask);
    expect(onNewTask).toHaveBeenCalledWith(null);
  });

  it("按单一层级渲染 section/project/task，不重复任务", () => {
    const data = snapshot();
    data.sidebar.rows.splice(1, 0, {
      key: "section:custom",
      kind: "section",
      id: "custom",
      depth: 0,
      title: "我的分组",
      status: "none",
      unread: false,
      pinned: false,
      archived: false,
      missingReference: false,
    });
    data.sidebar.rows[2] = { ...data.sidebar.rows[2]!, depth: 1 };
    data.sidebar.rows[3] = { ...data.sidebar.rows[3]!, depth: 2 };

    renderSidebar(data);

    expect(screen.getByRole("tree", { name: "任务层级" })).toBeInTheDocument();
    expect(screen.getByText("任务")).toBeInTheDocument();
    expect(screen.queryByText("项目", { selector: ".sidebar-tree-header span" })).not.toBeInTheDocument();
    expect(screen.getByRole("treeitem", { name: /我的分组/ })).toHaveAttribute("aria-level", "1");
    expect(screen.getByRole("treeitem", { name: /PAD/ })).toHaveAttribute("aria-level", "2");
    expect(screen.getByRole("treeitem", { name: "个人任务" })).toHaveAttribute("aria-level", "3");
    expect(screen.getByRole("treeitem", { name: "个人任务" })).toHaveStyle({
      paddingInlineStart: "calc(8px + 2 * var(--hierarchy-indent))",
    });
    expect(screen.getAllByText("个人任务")).toHaveLength(1);
    expect(screen.getByLabelText("任务侧边栏")).toHaveAttribute("data-sidebar-layout", "tiled");
  });

  it("按 Rust canonical view 显示中性层级标题", () => {
    const data = snapshot();
    data.sidebar.view = "archive";
    renderSidebar(data);

    expect(screen.getByText("归档", { selector: ".sidebar-tree-header span" })).toBeInTheDocument();
    expect(screen.getByRole("tree", { name: "任务层级" })).toHaveAttribute("data-sidebar-view", "archive");
  });

  it("通过可见视图控件切换全部、置顶与归档，并暴露当前持久化状态", async () => {
    const data = snapshot();
    const onViewChange = vi.fn();
    renderSidebarWith(data, { onViewChange });
    const user = userEvent.setup();

    expect(screen.getByRole("button", { name: "全部" })).toHaveAttribute("aria-pressed", "true");
    expect(screen.getByRole("button", { name: "置顶" })).toHaveAttribute("aria-pressed", "false");
    await user.click(screen.getByRole("button", { name: "置顶" }));
    await user.click(screen.getByRole("button", { name: "归档" }));

    expect(onViewChange).toHaveBeenNthCalledWith(1, "pinned");
    expect(onViewChange).toHaveBeenNthCalledWith(2, "archive");
  });

  it("用紧凑可见文本标记运行中、需关注和未读，同时保留任务原有可访问名称", () => {
    const data = snapshot();
    data.sidebar.rows[2] = { ...data.sidebar.rows[2]!, status: "running", unread: true };
    renderSidebar(data);

    const task = screen.getByRole("treeitem", { name: "个人任务" });
    expect(task).toHaveTextContent("运行中 · 未读");
    expect(task).toHaveAttribute("title", "个人任务 · 运行中 · 未读");
  });

  it("需要关注是 all 上的本地过滤而非伪造第四个持久视图，并提供空态", async () => {
    const data = snapshot();
    data.sidebar.rows[2] = { ...data.sidebar.rows[2]!, status: "attention", unread: false };
    const onAttentionOnlyChange = vi.fn();
    const onViewChange = vi.fn();
    const rendered = renderSidebarWith(data, { attentionOnly: true, onAttentionOnlyChange, onViewChange });

    expect(screen.getByRole("tree", { name: "任务层级" })).toHaveAttribute("data-sidebar-view", "all");
    expect(screen.getByRole("tree", { name: "任务层级" })).toHaveAttribute("data-local-filter", "attention");
    expect(screen.getByRole("treeitem", { name: "个人任务" })).toHaveTextContent("需关注");
    await userEvent.setup().click(screen.getByRole("button", { name: "需要关注，1 个任务" }));
    expect(onAttentionOnlyChange).toHaveBeenCalledWith(false);
    expect(onViewChange).not.toHaveBeenCalled();

    rendered.unmount();
    const empty = snapshot();
    renderSidebarWith(empty, { attentionOnly: true });
    expect(screen.getByText("没有需要关注的任务")).toBeInTheDocument();
  });

  it("搜索框 Escape 会同时清空关闭并把焦点还给搜索入口", async () => {
    const data = snapshot();
    const onSearchOpenChange = vi.fn();
    renderSidebarWith(data, { searchOpen: true, onSearchOpenChange });
    const user = userEvent.setup();
    const input = screen.getByLabelText("搜索当前账号的任务");
    await user.type(input, "个人");
    await user.keyboard("{Escape}");

    expect(input).toHaveValue("");
    expect(onSearchOpenChange).toHaveBeenCalledWith(false);
    expect(screen.getByRole("button", { name: "搜索" })).toHaveFocus();
  });

  it("从账号菜单切换 Profile", async () => {
    const onSwitchAccount = vi.fn();
    const data = snapshot();
    renderSidebar(data, onSwitchAccount);
    const user = userEvent.setup();

    await user.click(screen.getByRole("button", { name: /个人账号/ }));
    await user.click(screen.getByRole("menuitem", { name: /团队账号/ }));

    expect(onSwitchAccount).toHaveBeenCalledWith("team");
  });

  it("项目新建按钮可独立通过 Tab 与 Enter 激活", async () => {
    const onNewTask = vi.fn();
    const data = snapshot();
    render(
      <Sidebar
        accounts={data.accounts}
        projects={data.projects}
        hierarchy={data.sidebar}
        selectedTaskId={null}
        collapsedKeys={[]}
        width={275}
        searchOpen={false}
        attentionOnly={false}
        onSearchOpenChange={vi.fn()}
        onAttentionOnlyChange={vi.fn()}
        onAddProject={vi.fn()}
        onNewTask={onNewTask}
        onSelectTask={vi.fn()}
        onOpenSettings={vi.fn()}
        onResizeStart={vi.fn()}
        onResizeBy={vi.fn()}
        onSwitchAccount={vi.fn()}
        onCollapsedChange={vi.fn()}
        onViewChange={vi.fn()}
      />,
    );
    const user = userEvent.setup();

    const projectAdd = screen.getByRole("button", { name: "在 PAD 中新建任务" });
    for (let index = 0; index < 8 && document.activeElement !== projectAdd; index += 1) await user.tab();
    expect(projectAdd).toHaveFocus();
    await user.keyboard("{Enter}");

    expect(onNewTask).toHaveBeenCalledWith("personal-project");
  });

  it("树使用 roving tabindex，并通过方向键、Home 与 End 移动焦点", async () => {
    const data = snapshot();
    renderSidebar(data);
    const user = userEvent.setup();
    const project = screen.getByRole("treeitem", { name: "PAD 项目" });
    const task = screen.getByRole("treeitem", { name: "个人任务" });

    expect(screen.getAllByRole("treeitem").filter((item) => item.tabIndex === 0)).toEqual([task]);
    project.focus();
    await user.keyboard("{ArrowRight}");
    expect(task).toHaveFocus();
    await user.keyboard("{ArrowLeft}");
    expect(project).toHaveFocus();
    await user.keyboard("{End}");
    expect(task).toHaveFocus();
    await user.keyboard("{Home}");
    expect(project).toHaveFocus();
    await user.keyboard("{ArrowDown}");
    expect(task).toHaveFocus();
    await user.keyboard("{ArrowUp}");
    expect(project).toHaveFocus();
    expect(screen.getAllByRole("treeitem").filter((item) => item.tabIndex === 0)).toEqual([project]);
  });

  it("ArrowLeft/Right 折叠展开 Section 与 Project，Enter/Space 激活当前项", async () => {
    const data = snapshot();
    data.sidebar.rows.splice(1, 0, {
      key: "section:recent",
      kind: "section",
      id: "recent",
      depth: 0,
      title: "最近任务",
      status: "none",
      unread: false,
      pinned: false,
      archived: false,
      missingReference: false,
    });
    data.sidebar.rows[2] = { ...data.sidebar.rows[2]!, depth: 1 };
    data.sidebar.rows[3] = { ...data.sidebar.rows[3]!, depth: 2 };
    const onCollapsedChange = vi.fn();
    const onSelectTask = vi.fn();
    const first = renderSidebarWith(data, { onCollapsedChange, onSelectTask });
    const user = userEvent.setup();

    screen.getByRole("treeitem", { name: "最近任务" }).focus();
    await user.keyboard("{ArrowLeft}");
    expect(onCollapsedChange).toHaveBeenLastCalledWith(["section:recent"]);
    screen.getByRole("treeitem", { name: "PAD 项目" }).focus();
    await user.keyboard("{ArrowLeft}");
    expect(onCollapsedChange).toHaveBeenLastCalledWith(["project:personal-project"]);
    await user.keyboard(" ");
    expect(onCollapsedChange).toHaveBeenLastCalledWith(["project:personal-project"]);
    screen.getByRole("treeitem", { name: "个人任务" }).focus();
    await user.keyboard("{Enter}");
    expect(onSelectTask).toHaveBeenCalledWith("personal-task");

    first.unmount();
    const expandProject = vi.fn();
    const second = renderSidebarWith(data, { collapsedKeys: ["project:personal-project"], onCollapsedChange: expandProject });
    screen.getByRole("treeitem", { name: "PAD 项目" }).focus();
    await user.keyboard("{ArrowRight}");
    expect(expandProject).toHaveBeenCalledWith([]);

    second.unmount();
    const expandSection = vi.fn();
    renderSidebarWith(data, { collapsedKeys: ["section:recent"], onCollapsedChange: expandSection });
    screen.getByRole("treeitem", { name: "最近任务" }).focus();
    await user.keyboard("{ArrowRight}");
    expect(expandSection).toHaveBeenCalledWith([]);
  });

  it("在 footer 与账号菜单中使用统一的中文 partial/unknown 状态", async () => {
    const data = snapshot();
    const accounts = [
      { ...account("personal", "个人账号", true), authentication: "partial" as const },
      { ...account("team", "团队账号", false), authentication: "unknown" as const },
    ];
    renderSidebarWith(data, { accounts });
    const user = userEvent.setup();

    expect(screen.getByText(/部分可用/)).toBeInTheDocument();
    expect(document.querySelector(".account-auth-dot.auth-partial")).toHaveAttribute("title", "部分可用");
    await user.click(screen.getByRole("button", { name: /个人账号/ }));
    expect(screen.getByText(/状态未知/)).toBeInTheDocument();
  });

  it("通过 Rust UI state 回调持久化项目折叠状态并提供稳定选择器", async () => {
    const data = snapshot();
    const onCollapsedChange = vi.fn();
    const first = renderSidebar(data, vi.fn(), [], onCollapsedChange);
    const user = userEvent.setup();
    const project = screen.getByRole("treeitem", { name: "PAD 项目" });

    expect(project).toHaveAttribute("data-sidebar-key", "project:personal-project");
    await user.click(project);
    expect(onCollapsedChange).toHaveBeenCalledWith(["project:personal-project"]);

    first.unmount();
    renderSidebar(data, vi.fn(), ["project:personal-project"]);
    expect(screen.getByRole("treeitem", { name: "PAD 项目" })).toHaveAttribute("data-collapsed", "true");
    expect(screen.queryByRole("treeitem", { name: "个人任务" })).not.toBeInTheDocument();
  });

  it("使用 30px 单行层级并允许键盘调整侧边栏", async () => {
    const data = snapshot();
    const onResizeBy = vi.fn();
    render(
      <Sidebar
        accounts={data.accounts}
        projects={data.projects}
        hierarchy={data.sidebar}
        selectedTaskId={data.tasks[0]?.id ?? null}
        collapsedKeys={[]}
        width={275}
        searchOpen={false}
        attentionOnly={false}
        onSearchOpenChange={vi.fn()}
        onAttentionOnlyChange={vi.fn()}
        onAddProject={vi.fn()}
        onNewTask={vi.fn()}
        onSelectTask={vi.fn()}
        onOpenSettings={vi.fn()}
        onResizeStart={vi.fn()}
        onResizeBy={onResizeBy}
        onSwitchAccount={vi.fn()}
        onCollapsedChange={vi.fn()}
        onViewChange={vi.fn()}
      />,
    );

    const resize = screen.getByRole("separator", { name: "调整侧边栏宽度" });
    expect(resize).toHaveAttribute("aria-valuenow", "275");
    await userEvent.setup().type(resize, "{ArrowRight}{Shift>}{ArrowLeft}{/Shift}");
    expect(onResizeBy).toHaveBeenNthCalledWith(1, 8);
    expect(onResizeBy).toHaveBeenNthCalledWith(2, -32);
    expect(screen.getByRole("treeitem", { name: "个人任务" }).querySelector("small")).toBeNull();
  });
});

function renderSidebar(
  data: ReturnType<typeof snapshot>,
  onSwitchAccount = vi.fn(),
  collapsedKeys: string[] = [],
  onCollapsedChange = vi.fn(),
) {
  return renderSidebarWith(data, { onSwitchAccount, collapsedKeys, onCollapsedChange });
}

function renderSidebarWith(
  data: ReturnType<typeof snapshot>,
  overrides: Partial<Parameters<typeof Sidebar>[0]> = {},
) {
  return render(
    <Sidebar
      accounts={[account("personal", "个人账号", true), account("team", "团队账号", false)]}
      projects={data.projects}
      hierarchy={data.sidebar}
      selectedTaskId={data.tasks[0]?.id ?? null}
      collapsedKeys={[]}
      width={275}
      searchOpen={false}
      attentionOnly={false}
      onSearchOpenChange={vi.fn()}
      onAttentionOnlyChange={vi.fn()}
      onAddProject={vi.fn()}
      onNewTask={vi.fn()}
      onSelectTask={vi.fn()}
      onOpenSettings={vi.fn()}
      onResizeStart={vi.fn()}
      onResizeBy={vi.fn()}
      onSwitchAccount={vi.fn()}
      onCollapsedChange={vi.fn()}
      onViewChange={vi.fn()}
      {...overrides}
    />,
  );
}
