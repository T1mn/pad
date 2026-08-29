import type { AccountSummary, DesktopSnapshot } from "../types";

export function account(id: string, name: string, active: boolean): AccountSummary {
  return {
    id,
    name,
    provider: id === "personal" ? "Pi · openai" : "Pi · anthropic",
    selectedProvider: id === "personal" ? "openai" : "anthropic",
    selectedModel: id === "personal" ? "gpt-5.4" : "claude-sonnet",
    authenticatedProviders: [id === "personal" ? "openai" : "anthropic"],
    authentication: "authenticated",
    initials: name[0] ?? "P",
    active,
    policy: { mode: "guarded", unattended: false, workspaceRootCount: 0, protectedNamespaceNames: [] },
    fullAccess: false,
  };
}

export function snapshot(profileId = "personal"): DesktopSnapshot {
  const personal = profileId === "personal";
  const taskId = personal ? "personal-task" : "team-task";
  const projectId = personal ? "personal-project" : "team-project";
  return {
    accounts: [account("personal", "个人账号", personal), account("team", "团队账号", !personal)],
    projects: [{ id: projectId, profileId, name: personal ? "PAD" : "团队项目", path: `/work/${projectId}`, accent: "#6d5dfc", expanded: true, pinned: false }],
    tasks: [{ id: taskId, projectId, profileId, title: personal ? "个人任务" : "团队机密任务", updatedAt: "刚刚", status: "idle", rawStatus: "idle" }],
    sidebar: {
      view: "all",
      query: "",
      activeProfileId: profileId,
      selectedKey: `task:${taskId}`,
      rows: [
        { key: "new-task", kind: "new_task", depth: 0, title: "新任务", status: "none", unread: false, pinned: false, archived: false, missingReference: false },
        { key: `project:${projectId}`, kind: "project", id: projectId, depth: 0, title: personal ? "PAD" : "团队项目", status: "none", unread: false, pinned: false, archived: false, missingReference: false },
        { key: `task:${taskId}`, kind: "task", id: taskId, depth: 1, title: personal ? "个人任务" : "团队机密任务", status: "idle", unread: false, pinned: false, archived: false, missingReference: false },
      ],
    },
    backend: { status: "ready", capabilities: ["history", "full_access_policy", "auth_begin", "auth_status", "auth_respond", "auth_cancel", "logout"], providerAuthentication: "authenticated" },
    turnsByTask: { [taskId]: [] },
    interactionsByTask: { [taskId]: [] },
    uiState: {
      activeProfileId: profileId,
      selectedTaskId: taskId,
      sidebarView: "all",
      collapsedSectionIds: [],
      collapsedProjectIds: [],
      sidebarWidth: 275,
      theme: "light",
      rightPanelOpen: false,
      bottomPanelOpen: false,
      sidebarOpen: true,
    },
  };
}
