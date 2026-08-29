import { useEffect, useMemo, useRef, useState } from "react";
import type { KeyboardEvent as ReactKeyboardEvent, PointerEvent as ReactPointerEvent } from "react";
import type { AccountSummary, ProjectSummary, ProviderAuthentication, SidebarHierarchy, SidebarRow, SidebarView } from "../types";
import { Icon } from "./Icons";

interface SidebarProps {
  accounts: AccountSummary[];
  projects: ProjectSummary[];
  hierarchy: SidebarHierarchy;
  selectedTaskId: string | null;
  collapsedKeys: string[];
  width: number;
  searchOpen: boolean;
  attentionOnly: boolean;
  onSearchOpenChange(value: boolean): void;
  onAttentionOnlyChange(value: boolean): void;
  onAddProject(): void;
  onNewTask(projectId: string | null): void;
  onSelectTask(taskId: string): void;
  onOpenSettings(section?: "accounts"): void;
  onResizeStart(event: ReactPointerEvent<HTMLDivElement>): void;
  onResizeBy(delta: number): void;
  onSwitchAccount(accountId: string): void;
  onCollapsedChange(keys: string[]): void;
  onViewChange(view: SidebarView): void;
}

const expandableKinds = new Set<SidebarRow["kind"]>(["profile", "section", "project"]);

function StatusDot({ status, unread }: Pick<SidebarRow, "status" | "unread">) {
  if (status === "none") return <span className="status-dot status-none" aria-hidden="true" />;
  return <span className={`status-dot status-${status}${unread ? " is-unread" : ""}`} aria-hidden="true" />;
}

function taskStatusLabel(row: SidebarRow): string | null {
  const labels: string[] = [];
  if (row.status === "running") labels.push("运行中");
  else if (row.status === "attention") labels.push("需关注");
  if (row.unread) labels.push("未读");
  return labels.length ? labels.join(" · ") : null;
}

function authenticationLabel(authentication: ProviderAuthentication | undefined): string {
  if (authentication === "authenticated") return "已登录";
  if (authentication === "partial") return "部分可用";
  if (authentication === "missing") return "未登录";
  return "状态未知";
}

function filteredRows(rows: SidebarRow[], query: string, collapsed: Set<string>, attentionOnly: boolean): SidebarRow[] {
  const normalized = query.trim().toLocaleLowerCase("zh-CN");
  const visible = new Set<number>();
  const ancestors: Array<{ index: number; depth: number }> = [];

  rows.forEach((row, index) => {
    while (ancestors.length && (ancestors.at(-1)?.depth ?? -1) >= row.depth) ancestors.pop();
    const matchesQuery = !normalized || row.title.toLocaleLowerCase("zh-CN").includes(normalized);
    const matchesAttention = !attentionOnly || (row.kind === "task" && (row.status === "attention" || row.unread));
    if (matchesQuery && matchesAttention) {
      visible.add(index);
      ancestors.forEach((ancestor) => visible.add(ancestor.index));
    }
    if (expandableKinds.has(row.kind)) ancestors.push({ index, depth: row.depth });
  });

  const hiddenDepths: number[] = [];
  return rows.filter((row, index) => {
    while (hiddenDepths.length && row.depth <= (hiddenDepths.at(-1) ?? -1)) hiddenDepths.pop();
    if (hiddenDepths.length) return false;
    if (!normalized && !attentionOnly && collapsed.has(row.key)) hiddenDepths.push(row.depth);
    return visible.has(index);
  });
}

export function Sidebar({
  accounts,
  projects,
  hierarchy,
  selectedTaskId,
  collapsedKeys,
  width,
  searchOpen,
  attentionOnly,
  onSearchOpenChange,
  onAttentionOnlyChange,
  onAddProject,
  onNewTask,
  onSelectTask,
  onOpenSettings,
  onResizeStart,
  onResizeBy,
  onSwitchAccount,
  onCollapsedChange,
  onViewChange,
}: SidebarProps) {
  const [query, setQuery] = useState("");
  const [accountMenuOpen, setAccountMenuOpen] = useState(false);
  const [focusedKey, setFocusedKey] = useState<string | null>(null);
  const accountWrapRef = useRef<HTMLDivElement>(null);
  const accountButtonRef = useRef<HTMLButtonElement>(null);
  const searchButtonRef = useRef<HTMLButtonElement>(null);
  const treeItemRefs = useRef(new Map<string, HTMLButtonElement>());
  const collapsed = useMemo(() => new Set(collapsedKeys), [collapsedKeys]);
  const projectById = useMemo(() => new Map(projects.map((project) => [project.id, project])), [projects]);
  const newTaskRow = hierarchy.rows.find((row) => row.kind === "new_task");
  const rows = useMemo(
    () => filteredRows(hierarchy.rows.filter((row) => row.kind !== "new_task"), query, collapsed, attentionOnly),
    [attentionOnly, collapsed, hierarchy.rows, query],
  );
  const attentionCount = hierarchy.rows.filter((row) => row.kind === "task" && (row.status === "attention" || row.unread)).length;
  const activeAccount = accounts.find((account) => account.active) ?? accounts[0];
  const activeAuthenticationLabel = authenticationLabel(activeAccount?.authentication);
  const hierarchyTitle = attentionOnly ? "需要关注" : hierarchy.view === "pinned" ? "置顶" : hierarchy.view === "archive" ? "归档" : "任务";
  const selectedRowKey = rows.find((row) => row.kind === "task" && row.id === selectedTaskId)?.key;
  const rovingKey = rows.some((row) => row.key === focusedKey)
    ? focusedKey
    : selectedRowKey ?? (rows.some((row) => row.key === hierarchy.selectedKey) ? hierarchy.selectedKey : null) ?? rows[0]?.key ?? null;

  useEffect(() => {
    if (focusedKey !== rovingKey) setFocusedKey(rovingKey);
  }, [focusedKey, rovingKey]);

  useEffect(() => {
    if (!searchOpen && query) setQuery("");
  }, [query, searchOpen]);

  useEffect(() => {
    if (!accountMenuOpen) return;
    const closeOnOutsidePointer = (event: PointerEvent) => {
      if (!accountWrapRef.current?.contains(event.target as Node)) setAccountMenuOpen(false);
    };
    const closeOnEscape = (event: KeyboardEvent) => {
      if (event.key !== "Escape") return;
      setAccountMenuOpen(false);
      accountButtonRef.current?.focus();
    };
    window.addEventListener("pointerdown", closeOnOutsidePointer);
    window.addEventListener("keydown", closeOnEscape);
    return () => {
      window.removeEventListener("pointerdown", closeOnOutsidePointer);
      window.removeEventListener("keydown", closeOnEscape);
    };
  }, [accountMenuOpen]);

  function toggle(row: SidebarRow) {
    const next = new Set(collapsed);
    next.has(row.key) ? next.delete(row.key) : next.add(row.key);
    onCollapsedChange([...next]);
  }

  function activate(row: SidebarRow) {
    if (row.kind === "new_task") onNewTask(null);
    else if (row.kind === "task" && row.id) onSelectTask(row.id);
    else if (expandableKinds.has(row.kind)) toggle(row);
  }

  function focusRow(index: number) {
    const row = rows[index];
    if (!row) return;
    setFocusedKey(row.key);
    treeItemRefs.current.get(row.key)?.focus();
  }

  function handleTreeKeyDown(event: ReactKeyboardEvent<HTMLButtonElement>, row: SidebarRow, index: number) {
    if (event.key === "ArrowDown") {
      event.preventDefault();
      focusRow(Math.min(index + 1, rows.length - 1));
      return;
    }
    if (event.key === "ArrowUp") {
      event.preventDefault();
      focusRow(Math.max(index - 1, 0));
      return;
    }
    if (event.key === "Home") {
      event.preventDefault();
      focusRow(0);
      return;
    }
    if (event.key === "End") {
      event.preventDefault();
      focusRow(rows.length - 1);
      return;
    }
    if (event.key === "ArrowRight" && expandableKinds.has(row.kind)) {
      event.preventDefault();
      if (collapsed.has(row.key)) toggle(row);
      else if ((rows[index + 1]?.depth ?? row.depth) > row.depth) focusRow(index + 1);
      return;
    }
    if (event.key === "ArrowLeft") {
      if (expandableKinds.has(row.kind) && !collapsed.has(row.key)) {
        event.preventDefault();
        toggle(row);
        return;
      }
      for (let parentIndex = index - 1; parentIndex >= 0; parentIndex -= 1) {
        if ((rows[parentIndex]?.depth ?? row.depth) >= row.depth) continue;
        event.preventDefault();
        focusRow(parentIndex);
        return;
      }
    }
    if (event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      activate(row);
    }
  }

  return (
    <aside
      className="sidebar"
      style={{ width }}
      aria-label="任务侧边栏"
      data-sidebar-layout="tiled"
      data-sidebar-width={width}
      data-active-profile-id={hierarchy.activeProfileId ?? ""}
    >
      <div className="sidebar-titlebar-spacer" />
      <div className="sidebar-primary-nav">
        <button className="nav-row nav-row-primary" aria-label={newTaskRow?.title ?? "新任务"} onClick={() => onNewTask(null)}>
          <Icon name="plus" /><span>{newTaskRow?.title ?? "新任务"}</span><kbd>⌘N</kbd>
        </button>
        <button ref={searchButtonRef} className={`nav-row ${searchOpen ? "is-active" : ""}`} aria-label="搜索" aria-expanded={searchOpen} onClick={() => onSearchOpenChange(!searchOpen)}>
          <Icon name="search" /><span>搜索</span><kbd>⌘K</kbd>
        </button>
        <button
          className={`nav-row nav-row-attention ${attentionOnly ? "is-active" : ""}`}
          aria-label={`需要关注，${attentionCount} 个任务`}
          aria-pressed={attentionOnly}
          onClick={() => onAttentionOnlyChange(!attentionOnly)}
        >
          <Icon name="sparkles" /><span>需要关注</span><small>{attentionCount}</small>
        </button>
        {searchOpen && (
          <label className="sidebar-search">
            <Icon name="search" />
            <input
              autoFocus
              aria-label="搜索当前账号的任务"
              value={query}
              onChange={(event) => setQuery(event.target.value)}
              onKeyDown={(event) => {
                if (event.key !== "Escape") return;
                event.preventDefault();
                event.stopPropagation();
                setQuery("");
                onSearchOpenChange(false);
                queueMicrotask(() => searchButtonRef.current?.focus());
              }}
              placeholder="搜索当前账号的任务"
            />
            {query && <button onClick={() => setQuery("")} aria-label="清空搜索"><Icon name="x" /></button>}
          </label>
        )}
      </div>

      <div className="sidebar-tree-header">
        <span>{hierarchyTitle}</span>
        <button type="button" aria-label="添加项目" title="添加项目" onClick={onAddProject}><Icon name="plus" /></button>
      </div>
      <div className="sidebar-view-switch" role="group" aria-label="任务视图">
        {([
          ["all", "全部"],
          ["pinned", "置顶"],
          ["archive", "归档"],
        ] as const).map(([view, label]) => (
          <button
            type="button"
            key={view}
            aria-pressed={hierarchy.view === view}
            className={hierarchy.view === view ? "is-active" : undefined}
            onClick={() => onViewChange(view)}
          >
            {label}
          </button>
        ))}
      </div>
      <div className="sidebar-scroll" role="tree" aria-label="任务层级" data-sidebar-view={hierarchy.view} data-local-filter={attentionOnly ? "attention" : "none"}>
        {rows.map((row, index) => {
          const project = row.kind === "project" && row.id ? projectById.get(row.id) : undefined;
          const isExpandable = expandableKinds.has(row.kind);
          const isCollapsed = collapsed.has(row.key);
          const active = row.kind === "task" && row.id === selectedTaskId;
          const statusLabel = row.kind === "task" ? taskStatusLabel(row) : null;
          return (
            <div className={`hierarchy-row-wrap hierarchy-${row.kind}`} key={row.key} role="none">
              <div className="hierarchy-row-container">
                <button
                  ref={(node) => {
                    if (node) treeItemRefs.current.set(row.key, node);
                    else treeItemRefs.current.delete(row.key);
                  }}
                  className={`hierarchy-row${active ? " is-active" : ""}${row.missingReference ? " is-missing" : ""}`}
                  role="treeitem"
                  aria-label={row.kind === "project" ? `${row.title} 项目` : row.title}
                  aria-level={row.depth + 1}
                  aria-selected={active}
                  aria-expanded={isExpandable ? !isCollapsed : undefined}
                  aria-current={active ? "page" : undefined}
                  data-row-kind={row.kind}
                  data-depth={row.depth}
                  data-sidebar-key={row.key}
                  data-collapsed={isExpandable ? String(isCollapsed) : undefined}
                  tabIndex={row.key === rovingKey ? 0 : -1}
                  style={{ paddingInlineStart: `calc(8px + ${row.depth} * var(--hierarchy-indent))` }}
                  onClick={() => activate(row)}
                  onFocus={() => setFocusedKey(row.key)}
                  onKeyDown={(event) => handleTreeKeyDown(event, row, index)}
                  title={statusLabel ? `${row.title} · ${statusLabel}` : row.title}
                >
                  <span className="hierarchy-leading">
                    {isExpandable ? <Icon name={isCollapsed ? "chevron-right" : "chevron-down"} /> : <span className="hierarchy-chevron-space" />}
                    {row.kind === "profile" && <span className="avatar avatar-small">{activeAccount?.initials ?? "P"}</span>}
                    {row.kind === "section" && <Icon name="layout" />}
                    {row.kind === "project" && <span className="project-glyph" style={{ background: project?.accent }}><Icon name="folder" /></span>}
                    {row.kind === "task" && <StatusDot status={row.status} unread={row.unread} />}
                  </span>
                  <span className="hierarchy-copy">
                    <strong>{row.title}</strong>
                    {statusLabel && <small className="hierarchy-status-copy" aria-hidden="true">{statusLabel}</small>}
                  </span>
                </button>
                {row.kind === "project" && row.id && (
                  <button
                    className="hierarchy-add"
                    aria-label={`在 ${row.title} 中新建任务`}
                    onClick={() => onNewTask(row.id ?? null)}
                  ><Icon name="plus" /></button>
                )}
              </div>
            </div>
          );
        })}
        {rows.length === 0 && <div className="sidebar-empty">{attentionOnly ? "没有需要关注的任务" : "当前账号没有可显示的任务"}</div>}
      </div>

      <div className="sidebar-footer">
        <div className="account-wrap" ref={accountWrapRef}>
          <button
            ref={accountButtonRef}
            className="account-row"
            onClick={() => setAccountMenuOpen((value) => !value)}
            aria-haspopup="menu"
            aria-expanded={accountMenuOpen}
          >
            <span className="avatar">{activeAccount?.initials ?? "P"}</span>
            <span className="account-copy"><strong>{activeAccount?.name ?? "PAD"}</strong><small>{activeAccount?.provider ?? "Pi"} · {activeAuthenticationLabel}</small></span>
            <span className={`account-auth-dot auth-${activeAccount?.authentication ?? "unknown"}`} aria-hidden="true" title={activeAuthenticationLabel} />
            <Icon name="chevron-down" />
          </button>
          {accountMenuOpen && (
            <div className="account-menu" role="menu">
              <div className="menu-label">切换账号</div>
              {accounts.map((account) => (
                <button key={account.id} role="menuitem" onClick={() => { onSwitchAccount(account.id); setAccountMenuOpen(false); }}>
                  <span className="avatar avatar-small">{account.initials}</span>
                  <span><strong>{account.name}</strong><small>{account.provider} · {authenticationLabel(account.authentication)}</small></span>
                  {account.active && <Icon name="check" />}
                </button>
              ))}
              <button className="account-menu-settings" role="menuitem" onClick={() => { setAccountMenuOpen(false); onOpenSettings("accounts"); }}>
                <Icon name="settings" /><span><strong>管理账号</strong><small>登录、退出与权限</small></span>
              </button>
            </div>
          )}
        </div>
        <button className="footer-settings" onClick={() => onOpenSettings()}><Icon name="settings" /><span>设置</span><kbd>⌘,</kbd></button>
      </div>
      <div
        className="sidebar-resize"
        role="separator"
        aria-label="调整侧边栏宽度"
        aria-orientation="vertical"
        aria-valuemin={240}
        aria-valuemax={Math.round(Math.min(520, Math.max(240, window.innerWidth - 320)))}
        aria-valuenow={Math.round(width)}
        tabIndex={0}
        onPointerDown={onResizeStart}
        onKeyDown={(event) => {
          if (event.key !== "ArrowLeft" && event.key !== "ArrowRight") return;
          event.preventDefault();
          const step = event.shiftKey ? 32 : 8;
          onResizeBy(event.key === "ArrowLeft" ? -step : step);
        }}
      />
    </aside>
  );
}
