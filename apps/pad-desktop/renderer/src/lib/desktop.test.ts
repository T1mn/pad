import { describe, expect, it, vi } from "vitest";
import type { DesktopBootstrapResult, DesktopUiStateDto, PadDesktopApi, ProfileDto } from "../../../shared/protocol";
import type { ProjectSummary, TaskSummary } from "../types";
import {
  buildSidebarHierarchy,
  createBridgeAdapter,
  mapHistoryMessage,
  mapRemotePairing,
  mapRemoteStatus,
  normalizeAttachmentPaths,
  pendingInteractionsFromPoll,
  promptWithAttachments,
  sanitizeProfileForRenderer,
} from "./desktop";

describe("远程连接 renderer DTO", () => {
  it("仅保留公开状态字段并丢弃 token、路径与原始错误", () => {
    const mapped = mapRemoteStatus({
      remote: {
        enabled: true,
        state: "ready",
        display_name: "Tim 的 Mac",
        active_connections: 2,
        updated_at: 1_800_000_000,
        error_code: "",
        token: "must-not-leak",
        path: "/private/remote.sqlite",
        raw_error: "secret stack trace",
        endpoint: "192.0.2.10:443",
        devices: [{
          id: "phone-1",
          display_name: "iPhone",
          platform: "iOS",
          online: true,
          paired_at: 1_700_000_000,
          last_seen_at: 1_800_000_000,
          token: "device-secret",
          profile_id: "private-profile",
        }],
      },
    });

    expect(mapped).toEqual({
      enabled: true,
      state: "ready",
      displayName: "Tim 的 Mac",
      activeConnections: 2,
      updatedAt: 1_800_000_000,
      devices: [{
        id: "phone-1",
        displayName: "iPhone",
        platform: "iOS",
        online: true,
        pairedAt: 1_700_000_000,
        lastSeenAt: 1_800_000_000,
      }],
    });
    expect(JSON.stringify(mapped)).not.toMatch(/token|path|raw_error|endpoint|profile/i);
  });

  it("配对结果只保留短期 id、原始 QR payload 与过期时间", () => {
    const payload = "pad-remote://pair?ticket=opaque";
    expect(mapRemotePairing({
      pairing: { pairing_id: "pair-1", qr_payload: payload, expires_at: 1_800_000_030, raw_secret: "drop" },
      path: "/private",
    })).toEqual({ pairingId: "pair-1", qrPayload: payload, expiresAt: 1_800_000_030 });
  });

  it("remote_changed 事件会重新读取状态而不信任事件 payload", async () => {
    const fixture = bridgeFixture(0);
    fixture.bootstrap.capabilities = ["remote_gateway_v1"];
    let protocolListener: Parameters<PadDesktopApi["subscribe"]>[0] = () => undefined;
    let remoteReads = 0;
    const api: PadDesktopApi = {
      bootstrap: vi.fn(async () => fixture.bootstrap),
      request: vi.fn(async (action: string) => {
        if (action === "hello") return { capabilities: ["remote_gateway_v1"] };
        if (action === "provider_status") return {};
        if (action === "remote_status") {
          remoteReads += 1;
          return { remote: { enabled: true, state: "ready", display_name: "Mac", active_connections: remoteReads - 1, devices: [], updated_at: remoteReads } };
        }
        throw new Error(`unexpected action ${action}`);
      }) as PadDesktopApi["request"],
      chooseProjectDirectory: vi.fn(),
      subscribe: vi.fn((listener) => {
        protocolListener = listener;
        return () => undefined;
      }),
    };
    const adapter = createBridgeAdapter(api);
    const initial = await adapter.loadSnapshot();
    expect(initial.remote?.activeConnections).toBe(0);
    let latest = initial.remote;
    const unsubscribe = adapter.subscribe((event) => {
      if (event.type === "remote-updated") latest = event.status;
    });
    protocolListener({
      type: "backend_event",
      payload: {
        type: "desktop_event",
        protocol_version: 2,
        sequence: 1,
        event: { kind: "remote_changed", payload: { remote: { token: "event-secret", active_connections: 999 } } },
      },
    });
    await vi.waitFor(() => expect(latest?.activeConnections).toBe(1));
    expect(remoteReads).toBe(2);
    unsubscribe();
  });
});

describe("结构化 history 时间线", () => {
  it("只从 metadata/artifacts 显式字段建立 typed artifacts，不解析看起来像 diff 的正文", () => {
    const fakeDiff = [
      "file_path: src/forged.ts",
      "diff --git a/src/forged.ts b/src/forged.ts",
      "--- a/src/forged.ts",
      "+++ b/src/forged.ts",
      "@@ -1 +1 @@",
      "-false",
      "+true",
    ].join("\n");
    const bodyOnly = mapHistoryMessage({ id: "body-only", role: "tool", content: fakeDiff }, 0);
    expect(bodyOnly.body).toBe(fakeDiff);
    expect(bodyOnly.artifacts).toBeUndefined();

    const structured = mapHistoryMessage({
      id: "structured",
      role: "tool",
      name: "apply_patch",
      content: fakeDiff,
      metadata: {
        artifacts: [{
          id: "change-main",
          kind: "change",
          path: "src/main.ts",
          operation: "modified",
          diff: "@@ -1 +1 @@\n-false\n+true",
        }],
        files: ["README.md"],
        changes: [{ path: "src/new.ts", change_type: "created" }],
      },
    }, 1);

    expect(structured.artifacts).toEqual([
      expect.objectContaining({ id: "change-main", kind: "change", path: "src/main.ts", operation: "modified" }),
      expect.objectContaining({ kind: "file", path: "README.md", operation: "read" }),
      expect.objectContaining({ kind: "change", path: "src/new.ts", operation: "created" }),
    ]);
  });

  it.each([
    [{ role: "assistant", type: "reasoning", content: "分析中" }, "reasoning", "推理过程", undefined],
    [{ role: "error", content: "失败原因" }, "error", "执行错误", "failed"],
    [{ type: "status", content: "正在检查" }, "status", "状态更新", undefined],
    [{ role: "assistant", kind: "final", content: "完成" }, "final", "最终答复", "complete"],
    [{ type: "activity", name: "索引项目", content: "完成扫描" }, "activity", "索引项目", undefined],
    [{ role: "assistant", content: "普通答复" }, "assistant", undefined, undefined],
  ])("保留并细分 %s 角色", (message, expectedKind, expectedTitle, expectedState) => {
    const turn = mapHistoryMessage(message, 0);
    expect(turn.kind).toBe(expectedKind);
    expect(turn.title).toBe(expectedTitle);
    expect(turn.state).toBe(expectedState);
  });

  it("把 Pi 的空正文 assistant 网络错误显示成中文失败卡片", () => {
    const turn = mapHistoryMessage({
      role: "assistant",
      content: [],
      stopReason: "error",
      errorMessage: "Unable to connect. Is the computer able to access the url?",
    }, 0);
    expect(turn).toMatchObject({
      kind: "error",
      title: "执行错误",
      body: "无法连接模型服务。请检查网络或代理后重试。",
      state: "failed",
    });
  });

  it("保留 Pi 实际执行回复时记录的模型", () => {
    const turn = mapHistoryMessage({
      role: "assistant",
      provider: "openai-codex",
      model: "gpt-5.6-terra",
      content: "我是 GPT-5.5。",
    }, 0);
    expect(turn).toMatchObject({
      kind: "assistant",
      provider: "openai-codex",
      model: "gpt-5.6-terra",
    });
  });
});

describe("buildSidebarHierarchy", () => {
  it("严格保留 canonical snapshot 的 Section、Project、Task 顺序与层级", () => {
    const projects: ProjectSummary[] = [
      { id: "pinned-project", profileId: "p", name: "固定项目", path: "/work/pinned", accent: "#000", expanded: true, pinned: true },
      { id: "normal-project", profileId: "p", name: "普通项目", path: "/work/normal", accent: "#000", expanded: true, pinned: false },
    ];
    const tasks: TaskSummary[] = [
      task("custom-task", null, "自定义任务"),
      task("pinned-child", "pinned-project", "固定项目任务"),
      task("recent-task", null, "最近任务 A"),
      task("normal-task", "normal-project", "普通项目任务"),
    ];
    const raw = {
      view: "all",
      active_profile_id: "p",
      rows: [
        row("new-task", "new_task", undefined, 0, "New task"),
        row("profile:p", "profile", "p", 0, "当前账号"),
        row("section:custom", "section", "custom", 0, "自定义分组"),
        row("task:custom-task", "task", "custom-task", 1, "自定义任务"),
        row("project:pinned-project", "project", "pinned-project", 0, "固定项目"),
        row("task:pinned-child", "task", "pinned-child", 1, "固定项目任务"),
        row("task:recent-task", "task", "recent-task", 0, "最近任务 A"),
        row("project:normal-project", "project", "normal-project", 0, "普通项目"),
        row("task:normal-task", "task", "normal-task", 1, "普通项目任务"),
      ],
    };

    const result = buildSidebarHierarchy(raw, "p", projects, tasks);
    expect(result.rows.map((item) => [item.kind, item.id, item.depth])).toEqual([
      ["new_task", undefined, 0],
      ["section", "custom", 0],
      ["task", "custom-task", 1],
      ["project", "pinned-project", 0],
      ["task", "pinned-child", 1],
      ["task", "recent-task", 0],
      ["project", "normal-project", 0],
      ["task", "normal-task", 1],
    ]);
  });

  it("不从 flat records 补造 canonical snapshot 中不存在的第二套树", () => {
    const projects: ProjectSummary[] = [
      { id: "p1", profileId: "p", name: "不应补造", path: "/work/p1", accent: "#000", expanded: true, pinned: false },
    ];
    const tasks: TaskSummary[] = [task("t1", "p1", "不应补造的任务")];

    const result = buildSidebarHierarchy(
      { rows: [row("new-task", "new_task", undefined, 0, "New task")] },
      "p",
      projects,
      tasks,
    );

    expect(result.rows.map((item) => item.key)).toEqual(["new-task"]);
  });

  it("将后端首启默认 Workspace 项目名显示为中文", () => {
    const projects: ProjectSummary[] = [
      { id: "workspace", profileId: "p", name: "Workspace", path: "/work", accent: "#000", expanded: true, pinned: false },
    ];
    const result = buildSidebarHierarchy(
      { rows: [row("project:workspace", "project", "workspace", 0, "Workspace")] },
      "p",
      projects,
      [],
    );

    expect(result.rows.find((item) => item.kind === "project")?.title).toBe("工作区");
  });
});

describe("renderer profile boundary", () => {
  it("立即丢弃 v1 私有目录、credential 与受保护绝对路径", () => {
    const account = sanitizeProfileForRenderer({
      id: "private",
      name: "私有账号",
      agent_dir: "/Users/tim/.pad/agent",
      session_dir: "/Users/tim/.pad/session",
      credential_ref: "token-secret",
      policy: {
        mode: "system_full",
        unattended: true,
        workspace_roots: ["/Users/tim/work"],
        protected_namespaces: [{ name: "codex", root: "/Users/tim/.codex" }],
      },
      created_at: 0,
      updated_at: 0,
    } as unknown as ProfileDto);
    const serialized = JSON.stringify(account);

    expect(serialized).not.toContain("/Users/tim");
    expect(serialized).not.toContain("token-secret");
    expect(serialized).not.toContain("agent_dir");
    expect(serialized).toContain('"workspaceRootCount":1');
    expect(account.policy.protectedNamespaceNames).toEqual(["codex"]);
  });

  it("将内置默认账号名本地化，但不改写用户账号名", () => {
    const builtIn = sanitizeProfileForRenderer({
      id: "default",
      name: "Default",
      policy: { mode: "guarded", unattended: false },
      created_at: 0,
      updated_at: 0,
    } as ProfileDto);
    const userNamed = sanitizeProfileForRenderer({
      id: "custom",
      name: "Default",
      policy: { mode: "guarded", unattended: false },
      created_at: 0,
      updated_at: 0,
    } as ProfileDto);

    expect(builtIn.name).toBe("默认账号");
    expect(userNamed.name).toBe("Default");
  });
});

describe("登录文本本地化", () => {
  it("将 Pi 英文验证提示转换为中文并保留授权码", async () => {
    const request = vi.fn(async (action: string) => {
      if (action === "auth_begin") return {
        auth: {
          attempt_id: "attempt-1",
          profile_id: "default",
          provider: "openai",
          phase: "waiting_browser",
          notices: [{ message: "Open the browser and enter this code", url: "https://example.com/login", user_code: "ABCD-1234" }],
        },
      };
      throw new Error(`unexpected action ${action}`);
    });
    const adapter = createBridgeAdapter({
      bootstrap: vi.fn(),
      request: request as unknown as PadDesktopApi["request"],
      chooseProjectDirectory: vi.fn(),
      subscribe: vi.fn(() => () => undefined),
    });

    const session = await adapter.beginLogin("default", "openai");

    expect(request).toHaveBeenCalledWith("auth_begin", { profile_id: "default", provider: "openai", auth_type: "oauth" });
    expect(session.authType).toBe("oauth");
    expect(session.title).toBe("登录模型账号");
    expect(session.message).toBe("请在浏览器中完成授权，然后返回 PAD。授权码：ABCD-1234");
    expect(session.message).not.toContain("Open the browser");
  });

  it("将英文密钥输入提示转换为中文标签", async () => {
    const request = vi.fn(async (action: string) => {
      if (action === "auth_begin") return {
        auth: {
          attempt_id: "attempt-2",
          profile_id: "default",
          provider: "openai",
          auth_type: "api_key",
          phase: "waiting_input",
          prompt: { id: "prompt-1", kind: "secret", message: "Enter your API key", placeholder: "API key" },
          notices: [],
        },
      };
      throw new Error(`unexpected action ${action}`);
    });
    const adapter = createBridgeAdapter({
      bootstrap: vi.fn(),
      request: request as unknown as PadDesktopApi["request"],
      chooseProjectDirectory: vi.fn(),
      subscribe: vi.fn(() => () => undefined),
    });

    const session = await adapter.beginLogin("default", "openai", "api_key");

    expect(request).toHaveBeenCalledWith("auth_begin", { profile_id: "default", provider: "openai", auth_type: "api_key" });
    expect(session.authType).toBe("api_key");
    expect(session.message).toBe("请完成 Pi 提供的验证步骤。");
    expect(session.inputLabel).toBe("API 密钥");
    expect(session.inputSecret).toBe(true);
  });

  it("保留 Pi prompt options 并把 option id 原样提交给 auth_respond", async () => {
    const request = vi.fn(async (action: string) => {
      if (action === "auth_begin") return {
        auth: {
          attempt_id: "attempt-options",
          profile_id: "default",
          provider: "openai",
          auth_type: "oauth",
          phase: "running",
          prompt: {
            id: "prompt-options",
            kind: "select",
            message: "Select a login method",
            options: [
              { id: "browser", label: "Browser login (default)", description: "Recommended for most users" },
              { id: "device", label: "Device code login (headless)", description: "For headless or remote environments" },
            ],
          },
          notices: [],
        },
      };
      if (action === "auth_respond") return {
        auth: {
          attempt_id: "attempt-options",
          profile_id: "default",
          provider: "openai",
          auth_type: "oauth",
          phase: "running",
          notices: [],
        },
      };
      throw new Error(`unexpected action ${action}`);
    });
    const adapter = createBridgeAdapter({
      bootstrap: vi.fn(),
      request: request as unknown as PadDesktopApi["request"],
      chooseProjectDirectory: vi.fn(),
      subscribe: vi.fn(() => () => undefined),
    });

    const waiting = await adapter.beginLogin("default", "openai", "oauth");
    expect(waiting.promptKind).toBe("select");
    expect(waiting.promptMessage).toBe("请选择登录方式。");
    expect(waiting.options).toEqual([
      { id: "browser", label: "浏览器登录（默认）", description: "推荐大多数用户使用" },
      { id: "device", label: "设备码登录（无界面环境）", description: "适用于远程或无界面环境" },
    ]);

    await adapter.respondLogin("default", "device");
    expect(request).toHaveBeenLastCalledWith("auth_respond", {
      attempt_id: "attempt-options",
      prompt_id: "prompt-options",
      value: "device",
    });
  });
});

describe("Pi v2 pending interactions", () => {
  it("只映射本地后端 poll 暴露的真实 respond_ui 请求", () => {
    const interactions = pendingInteractionsFromPoll({
      poll: {
        pending_ui_requests: [
          { id: "confirm-1", kind: "confirm", response_action: "respond_ui", requires_response: true, title: "Allow?", message: "Run tests" },
          { id: "select-1", kind: "select", response_action: "respond_ui", requires_response: true, title: "Target", options: ["staging", "production"], default_index: 1 },
          { id: "input-1", kind: "input", response_action: "respond_ui", requires_response: true, title: "Version", default: "1.0.0" },
          { id: "editor-1", kind: "editor", response_action: "respond_ui", requires_response: true, title: "Notes", default: "draft" },
          { id: "unknown-1", kind: "unknown", response_action: "respond_ui", requires_response: false },
          { id: "fake", kind: "confirm", response_action: "other", requires_response: true },
        ],
      },
    });

    expect(interactions).toEqual([
      expect.objectContaining({ id: "confirm-1", kind: "confirm", title: "Allow?", message: "Run tests", requiresResponse: true }),
      expect.objectContaining({ id: "select-1", kind: "select", options: ["staging", "production"], defaultIndex: 1, requiresResponse: true }),
      expect.objectContaining({ id: "input-1", kind: "input", defaultValue: "1.0.0", requiresResponse: true }),
      expect.objectContaining({ id: "editor-1", kind: "editor", defaultValue: "draft", requiresResponse: true }),
      expect.objectContaining({ id: "unknown-1", kind: "unknown", requiresResponse: false }),
    ]);
    expect(interactions.some((interaction) => interaction.id === "fake")).toBe(false);
  });

  it("按原始 request id 与 kind 发送 respond_ui，成功后移除 pending 卡片", async () => {
    vi.useFakeTimers();
    try {
      const profile: ProfileDto = {
        id: "personal", name: "个人账号", default_provider: "openai", policy: { mode: "guarded", unattended: false }, created_at: 1, updated_at: 1,
      };
      const taskRecord: DesktopBootstrapResult["records"]["tasks"][number] = {
        id: "task-1", project_id: null, profile_id: "personal", title: "审批任务", summary: "", cwd: "/work", environment: "local",
        status: "needs_approval", unread: false, pinned: false, archived: false, created_at: 1, updated_at: 1,
      };
      const uiState: DesktopUiStateDto = {
        active_profile_id: "personal", selected_task_id: "task-1", collapsed_section_ids: [], collapsed_project_ids: [], sidebar_width: 275,
        sidebar_view: "all", theme: "system", right_panel_open: false, bottom_panel_open: false, sidebar_open: true,
      };
      const bootstrap: DesktopBootstrapResult = {
        protocol_version: 2,
        backend: { status: "ready", provider_authentication: "authenticated", authenticated_providers: ["openai"], selected_provider: "openai", selected_model: null },
        profile,
        capabilities: ["respond_ui"],
        sidebar: { rows: [] },
        ui_state: uiState,
        records: { profiles: [profile], projects: [], tasks: [taskRecord] },
      };
      const request = vi.fn(async (action: string) => {
        if (action === "hello") return { capabilities: ["respond_ui"] };
        if (action === "provider_status") return { provider_authentication: "authenticated", authenticated_providers: ["openai"], selected_provider: "openai" };
        if (action === "poll") return { poll: { pending_ui_requests: [{ id: "confirm-1", kind: "confirm", response_action: "respond_ui", requires_response: true, title: "Allow?" }] } };
        if (action === "history") return { messages: [] };
        if (action === "respond_ui") return { accepted: true };
        throw new Error(`unexpected action ${action}`);
      });
      const adapter = createBridgeAdapter({
        bootstrap: vi.fn().mockResolvedValue(bootstrap),
        request: request as unknown as PadDesktopApi["request"],
        chooseProjectDirectory: vi.fn(),
        subscribe: vi.fn(() => () => undefined),
      });

      const loaded = await adapter.loadSnapshot();
      expect(loaded.interactionsByTask["task-1"]).toEqual([expect.objectContaining({ id: "confirm-1", kind: "confirm" })]);
      let latest = loaded;
      const unsubscribe = adapter.subscribe((event) => { if (event.type === "snapshot") latest = event.snapshot; });
      await adapter.respondInteraction("task-1", "confirm-1", true);

      expect(request).toHaveBeenCalledWith("respond_ui", {
        task_id: "task-1",
        request_id: "confirm-1",
        response_kind: "confirm",
        value: true,
      });
      expect(latest.interactionsByTask["task-1"]).toEqual([]);
      unsubscribe();
    } finally {
      vi.useRealTimers();
    }
  });
});

describe("任务按需加载与账号切换事务", () => {
  it("停止按钮真正关闭 Pi runtime，而不是只发送无效 abort", async () => {
    const fixture = bridgeFixture(1);
    fixture.records.tasks[0] = { ...fixture.records.tasks[0]!, status: "starting" };
    fixture.bootstrap.records = fixture.records;
    const request = vi.fn(async (action: string) => {
      if (action === "hello") return { capabilities: ["history"] };
      if (action === "provider_status") return { provider_authentication: "authenticated" };
      if (action === "history") return { messages: [] };
      if (action === "stop_task") {
        fixture.records.tasks[0] = { ...fixture.records.tasks[0]!, status: "disconnected" };
        return { stopped: true };
      }
      if (action === "list_sidebar") return { records: fixture.records, sidebar: fixture.bootstrap.sidebar };
      throw new Error(`unexpected action ${action}`);
    });
    const adapter = createBridgeAdapter({
      bootstrap: vi.fn().mockResolvedValue(fixture.bootstrap),
      request: request as unknown as PadDesktopApi["request"],
      chooseProjectDirectory: vi.fn(),
      subscribe: vi.fn(() => () => undefined),
    });

    await adapter.loadSnapshot();
    const stopped = await adapter.abortTask("personal-task-1");
    expect(request).toHaveBeenCalledWith("stop_task", { task_id: "personal-task-1" });
    expect(request.mock.calls.some(([action]) => action === "abort")).toBe(false);
    expect(stopped.tasks[0]?.rawStatus).toBe("disconnected");
  });

  it("失败任务只读 Pi 历史，不对已经结束的 runtime 继续 poll", async () => {
    const fixture = bridgeFixture(1);
    fixture.records.tasks[0] = { ...fixture.records.tasks[0]!, status: "failed" };
    fixture.bootstrap.records = fixture.records;
    const request = vi.fn(async (action: string) => {
      if (action === "hello") return { capabilities: ["history", "poll"] };
      if (action === "provider_status") return { provider_authentication: "authenticated", authenticated_providers: ["openai"], selected_provider: "openai" };
      if (action === "history") return {
        messages: [{
          role: "assistant",
          content: [],
          stopReason: "error",
          errorMessage: "Unable to connect. Is the computer able to access the url?",
        }],
      };
      if (action === "poll") throw new Error("failed runtime no longer exists");
      throw new Error(`unexpected action ${action}`);
    });
    const adapter = createBridgeAdapter({
      bootstrap: vi.fn().mockResolvedValue(fixture.bootstrap),
      request: request as unknown as PadDesktopApi["request"],
      chooseProjectDirectory: vi.fn(),
      subscribe: vi.fn(() => () => undefined),
    });

    await adapter.loadSnapshot();
    const loaded = await adapter.loadTaskData("personal-task-1");

    expect(request.mock.calls.some(([action]) => action === "poll")).toBe(false);
    expect(loaded.turnsByTask["personal-task-1"]?.[0]).toMatchObject({
      kind: "error",
      body: "无法连接模型服务。请检查网络或代理后重试。",
    });
  });

  it("第九个任务只在选中时加载真实 history/poll，失败会抛出并允许原任务重试", async () => {
    const fixture = bridgeFixture(9);
    fixture.records.tasks[8] = { ...fixture.records.tasks[8]!, status: "needs_input" };
    let ninthHistoryAttempts = 0;
    const request = vi.fn(async (action: string, params: Record<string, unknown>) => {
      if (action === "hello") return { capabilities: ["history", "poll"] };
      if (action === "provider_status") return { provider_authentication: "authenticated", authenticated_providers: ["openai"], selected_provider: "openai" };
      if (action === "history") {
        if (params.task_id === "personal-task-9") {
          ninthHistoryAttempts += 1;
          if (ninthHistoryAttempts === 1) throw new Error("history temporarily unavailable");
          return { messages: [{ id: "history-9", role: "assistant", content: "第九任务历史" }] };
        }
        return { messages: [] };
      }
      if (action === "poll") return {
        poll: {
          pending_ui_requests: [{
            id: "input-9",
            kind: "input",
            response_action: "respond_ui",
            requires_response: true,
            title: "补充信息",
          }],
        },
      };
      throw new Error(`unexpected action ${action}`);
    });
    const adapter = createBridgeAdapter({
      bootstrap: vi.fn().mockResolvedValue(fixture.bootstrap),
      request: request as unknown as PadDesktopApi["request"],
      chooseProjectDirectory: vi.fn(),
      subscribe: vi.fn(() => () => undefined),
    });

    const initial = await adapter.loadSnapshot();
    expect(request.mock.calls.some(([action, params]) => action === "history" && params.task_id === "personal-task-9")).toBe(false);
    expect(initial.turnsByTask).not.toHaveProperty("personal-task-9");

    await expect(adapter.loadTaskData("personal-task-9")).rejects.toThrow("history temporarily unavailable");
    const recovered = await adapter.loadTaskData("personal-task-9");

    expect(request.mock.calls.filter(([action, params]) => action === "history" && params.task_id === "personal-task-9")).toHaveLength(2);
    expect(request).toHaveBeenCalledWith("poll", { task_id: "personal-task-9" });
    expect(recovered.turnsByTask["personal-task-9"]?.[0]?.body).toBe("第九任务历史");
    expect(recovered.interactionsByTask["personal-task-9"]).toEqual([expect.objectContaining({ id: "input-9" })]);
  });

  it("账号 UI state 持久化失败时保持 adapter 选中账号与旧快照", async () => {
    const fixture = bridgeFixture(1, true);
    const request = vi.fn(async (action: string, params: Record<string, unknown>) => {
      if (action === "hello") return { capabilities: ["history"] };
      if (action === "provider_status") return { provider_authentication: "authenticated", authenticated_providers: ["openai"], selected_provider: "openai" };
      if (action === "history") return { messages: [] };
      if (action === "set_ui_state") throw new Error("persist failed");
      throw new Error(`unexpected action ${action} ${JSON.stringify(params)}`);
    });
    const adapter = createBridgeAdapter({
      bootstrap: vi.fn().mockResolvedValue(fixture.bootstrap),
      request: request as unknown as PadDesktopApi["request"],
      chooseProjectDirectory: vi.fn(),
      subscribe: vi.fn(() => () => undefined),
    });
    await adapter.loadSnapshot();

    await expect(adapter.switchAccount("team")).rejects.toThrow("persist failed");
    const restored = await adapter.loadTaskData("personal-task-1");

    expect(restored.accounts.find((account) => account.active)?.id).toBe("personal");
    expect(restored.tasks.map((item) => item.id)).toEqual(["personal-task-1"]);
    expect(request.mock.calls.filter(([action]) => action === "set_ui_state")).toHaveLength(1);
  });

  it("目标账号 refresh 失败时把 UI state 写回旧账号并恢复 adapter", async () => {
    const fixture = bridgeFixture(1, true);
    let persisted = structuredClone(fixture.bootstrap.ui_state);
    const persistedDocuments: DesktopUiStateDto[] = [];
    const request = vi.fn(async (action: string, params: Record<string, unknown>) => {
      if (action === "hello") return { capabilities: ["history"] };
      if (action === "provider_status") return { provider_authentication: "authenticated", authenticated_providers: ["openai"], selected_provider: "openai" };
      if (action === "history") return { messages: [] };
      if (action === "set_ui_state") {
        persisted = structuredClone(params.state as DesktopUiStateDto);
        persistedDocuments.push(persisted);
        return { state: persisted, sidebar: { rows: [] } };
      }
      if (action === "list_sidebar") throw new Error("refresh failed");
      throw new Error(`unexpected action ${action}`);
    });
    const adapter = createBridgeAdapter({
      bootstrap: vi.fn().mockResolvedValue(fixture.bootstrap),
      request: request as unknown as PadDesktopApi["request"],
      chooseProjectDirectory: vi.fn(),
      subscribe: vi.fn(() => () => undefined),
    });
    await adapter.loadSnapshot();

    await expect(adapter.switchAccount("team")).rejects.toThrow("refresh failed");
    const restored = await adapter.loadTaskData("personal-task-1");

    expect(persistedDocuments.map((state) => state.active_profile_id)).toEqual(["team", "personal"]);
    expect(persisted.active_profile_id).toBe("personal");
    expect(persisted.selected_task_id).toBe("personal-task-1");
    expect(restored.accounts.find((account) => account.active)?.id).toBe("personal");
    expect(restored.uiState.activeProfileId).toBe("personal");
  });

  it("账号切换事务未完成时拒绝创建任务和 UI state 写入，避免命令落入错误账号", async () => {
    const fixture = bridgeFixture(1, true);
    let releasePersist: (() => void) | undefined;
    const persistGate = new Promise<void>((resolve) => { releasePersist = resolve; });
    let persisted = structuredClone(fixture.bootstrap.ui_state);
    const request = vi.fn(async (action: string, params: Record<string, unknown>) => {
      if (action === "hello") return { capabilities: ["history"] };
      if (action === "provider_status") return { provider_authentication: "authenticated", authenticated_providers: ["openai"], selected_provider: "openai" };
      if (action === "history") return { messages: [] };
      if (action === "set_ui_state") {
        const next = structuredClone(params.state as DesktopUiStateDto);
        await persistGate;
        persisted = next;
        return { state: persisted, sidebar: { rows: [] } };
      }
      if (action === "list_sidebar") return { records: fixture.records, sidebar: { rows: [] }, ui_state: persisted };
      throw new Error(`unexpected action ${action}`);
    });
    const adapter = createBridgeAdapter({
      bootstrap: vi.fn().mockResolvedValue(fixture.bootstrap),
      request: request as unknown as PadDesktopApi["request"],
      chooseProjectDirectory: vi.fn(),
      subscribe: vi.fn(() => () => undefined),
    });
    await adapter.loadSnapshot();

    const switching = adapter.switchAccount("team");
    await vi.waitFor(() => expect(request.mock.calls.some(([action]) => action === "set_ui_state")).toBe(true));

    await expect(adapter.createTask(null)).rejects.toThrow("账号正在切换");
    expect(() => adapter.updateUiState({ theme: "dark" })).toThrow("账号正在切换");
    expect(request.mock.calls.some(([action]) => action === "create_task")).toBe(false);

    releasePersist?.();
    const switched = await switching;
    expect(switched.accounts.find((account) => account.active)?.id).toBe("team");
  });

  it("账号切换先清空旧远程状态，再为目标账号重新读取", async () => {
    const fixture = bridgeFixture(0, true);
    fixture.bootstrap.capabilities = ["history", "remote_gateway_v1"];
    let persisted = structuredClone(fixture.bootstrap.ui_state);
    const remoteProfiles: string[] = [];
    const request = vi.fn(async (action: string, params: Record<string, unknown>) => {
      if (action === "hello") return { capabilities: ["history", "remote_gateway_v1"] };
      if (action === "provider_status") return { provider_authentication: "authenticated" };
      if (action === "remote_status") {
        const profileId = persisted.active_profile_id ?? "personal";
        remoteProfiles.push(profileId);
        return { remote: { enabled: true, state: "ready", display_name: `${profileId} Mac`, active_connections: 0, devices: [], updated_at: remoteProfiles.length } };
      }
      if (action === "set_ui_state") {
        persisted = structuredClone(params.state as DesktopUiStateDto);
        return { state: persisted, sidebar: { rows: [] } };
      }
      if (action === "list_sidebar") return { records: fixture.records, sidebar: { rows: [] }, ui_state: persisted };
      throw new Error(`unexpected action ${action}`);
    });
    const adapter = createBridgeAdapter({
      bootstrap: vi.fn().mockResolvedValue(fixture.bootstrap),
      request: request as unknown as PadDesktopApi["request"],
      chooseProjectDirectory: vi.fn(),
      subscribe: vi.fn(() => () => undefined),
    });
    await adapter.loadSnapshot();
    const transitionSnapshots: Array<{ profile: string | null; remote: string | null }> = [];
    const unsubscribe = adapter.subscribe((event) => {
      if (event.type === "snapshot") transitionSnapshots.push({
        profile: event.snapshot.uiState.activeProfileId,
        remote: event.snapshot.remote?.displayName ?? null,
      });
    });

    const switched = await adapter.switchAccount("team");

    expect(remoteProfiles).toEqual(["personal", "team"]);
    expect(transitionSnapshots).toContainEqual({ profile: "team", remote: null });
    expect(transitionSnapshots).not.toContainEqual({ profile: "team", remote: "personal Mac" });
    expect(switched.remote?.displayName).toBe("team Mac");
    unsubscribe();
  });
});

describe("Composer 到 Pi 的结构化发送事务", () => {
  it("只保留用户明确选择的绝对路径，去重并限制为 20 个", () => {
    const paths = Array.from({ length: 22 }, (_, index) => `/tmp/file-${index}.txt`);
    expect(normalizeAttachmentPaths([" relative.txt ", paths[0]!, paths[0]!, ...paths, "/tmp/bad\nname"])).toEqual(paths.slice(0, 20));
    expect(promptWithAttachments("  请检查这些文件  ", ["/tmp/spec.md", "/tmp/design.png"])).toBe(
      "请检查这些文件\n\n附件路径（用户明确选择）：\n- /tmp/spec.md\n- /tmp/design.png",
    );
    expect(promptWithAttachments("   ", ["/tmp/spec.md"])).toBe("");
  });

  it("一次原子 prompt 携带模型与推理配置，成功后才追加本地用户 turn", async () => {
    vi.useFakeTimers();
    try {
      const fixture = bridgeFixture(1);
      const request = vi.fn(async (action: string) => {
        if (action === "hello") return { capabilities: ["history"] };
        if (action === "provider_status") return { provider_authentication: "authenticated", authenticated_providers: ["openai"], selected_provider: "openai" };
        if (action === "history") return { messages: [] };
        if (action === "set_profile") return { records: fixture.records, sidebar: fixture.bootstrap.sidebar };
        if (action === "prompt") return { accepted: true };
        throw new Error(`unexpected action ${action}`);
      });
      const adapter = createBridgeAdapter({
        bootstrap: vi.fn().mockResolvedValue(fixture.bootstrap),
        request: request as unknown as PadDesktopApi["request"],
        chooseProjectDirectory: vi.fn(),
        subscribe: vi.fn(() => () => undefined),
      });
      await adapter.loadSnapshot();
      request.mockClear();
      const userTurns: string[] = [];
      const unsubscribe = adapter.subscribe((event) => {
        if (event.type === "turn-added" && event.turn.kind === "user") userTurns.push(event.turn.body);
      });

      await adapter.sendMessage({
        taskId: "personal-task-1",
        accountId: "personal",
        fullAccess: false,
        text: "分析附件",
        attachmentPaths: ["/tmp/spec.md", "relative.txt", "/tmp/spec.md", "/tmp/design.png"],
        provider: "custom-provider",
        model: "custom-model",
        thinkingLevel: "xhigh",
        fastMode: true,
      });

      expect(request.mock.calls.map(([action]) => action)).toEqual([
        "set_profile",
        "prompt",
      ]);
      expect(request).toHaveBeenCalledWith("set_profile", {
        profile_id: "personal",
        default_provider: "custom-provider",
        default_model: "custom-model",
      });
      expect(request).toHaveBeenCalledWith("prompt", {
        task_id: "personal-task-1",
        prompt: "分析附件\n\n附件路径（用户明确选择）：\n- /tmp/spec.md\n- /tmp/design.png",
        provider: "custom-provider",
        model: "custom-model",
        thinking_level: "xhigh",
        fast_mode: true,
      });
      expect(userTurns).toEqual(["分析附件\n\n附件路径（用户明确选择）：\n- /tmp/spec.md\n- /tmp/design.png"]);
      unsubscribe();
    } finally {
      vi.useRealTimers();
    }
  });

  it("默认推理只通过原子 prompt 发送且不携带 thinking_level", async () => {
    vi.useFakeTimers();
    try {
      const fixture = bridgeFixture(1);
      const request = vi.fn(async (action: string) => {
        if (action === "hello") return { capabilities: ["history"] };
        if (action === "provider_status") return { provider_authentication: "authenticated", authenticated_providers: ["openai"], selected_provider: "openai" };
        if (action === "history") return { messages: [] };
        if (action === "set_profile") return { records: fixture.records, sidebar: fixture.bootstrap.sidebar };
        if (action === "prompt") return { accepted: true };
        throw new Error(`unexpected action ${action}`);
      });
      const adapter = createBridgeAdapter({
        bootstrap: vi.fn().mockResolvedValue(fixture.bootstrap),
        request: request as unknown as PadDesktopApi["request"],
        chooseProjectDirectory: vi.fn(),
        subscribe: vi.fn(() => () => undefined),
      });
      await adapter.loadSnapshot();
      request.mockClear();

      await adapter.sendMessage({
        taskId: "personal-task-1",
        accountId: "personal",
        fullAccess: false,
        text: "继续",
        attachmentPaths: [],
        provider: "openai",
        model: "gpt-5.4",
        thinkingLevel: "default",
        fastMode: true,
      });

      expect(request.mock.calls.map(([action]) => action)).toEqual(["set_profile", "prompt"]);
      expect(request).toHaveBeenCalledWith("prompt", {
        task_id: "personal-task-1",
        prompt: "继续",
        provider: "openai",
        model: "gpt-5.4",
        fast_mode: true,
      });
    } finally {
      vi.useRealTimers();
    }
  });

  it("原子 prompt 配置失败时不伪造本地用户 turn", async () => {
    const fixture = bridgeFixture(1);
    const request = vi.fn(async (action: string) => {
      if (action === "hello") return { capabilities: ["history"] };
      if (action === "provider_status") return { provider_authentication: "authenticated", authenticated_providers: ["openai"], selected_provider: "openai" };
      if (action === "history") return { messages: [] };
      if (action === "set_profile") return { records: fixture.records, sidebar: fixture.bootstrap.sidebar };
      if (action === "prompt") throw new Error("model unavailable");
      throw new Error(`unexpected action ${action}`);
    });
    const adapter = createBridgeAdapter({
      bootstrap: vi.fn().mockResolvedValue(fixture.bootstrap),
      request: request as unknown as PadDesktopApi["request"],
      chooseProjectDirectory: vi.fn(),
      subscribe: vi.fn(() => () => undefined),
    });
    await adapter.loadSnapshot();
    request.mockClear();
    const events: unknown[] = [];
    adapter.subscribe((event) => events.push(event));

    await expect(adapter.sendMessage({
      taskId: "personal-task-1",
      accountId: "personal",
      fullAccess: false,
      text: "不要丢失",
      attachmentPaths: ["/tmp/retry.md"],
      provider: "openai",
      model: "missing-model",
      thinkingLevel: "high",
      fastMode: true,
    })).rejects.toThrow("model unavailable");

    expect(request.mock.calls.filter(([action]) => action === "prompt")).toHaveLength(1);
    expect(events).toEqual([]);
  });

  it("置顶视图无可见任务时先原子切回全部，再创建并发送普通任务", async () => {
    vi.useFakeTimers();
    try {
      const fixture = bridgeFixture(1);
      fixture.bootstrap.ui_state = { ...fixture.bootstrap.ui_state, sidebar_view: "pinned" };
      fixture.bootstrap.sidebar = { view: "pinned", active_profile_id: "personal", rows: [] };
      const created = taskRecord("created-from-pinned", null, { title: "新任务", updated_at: 10 });
      const allSidebar = () => ({
        view: "all",
        active_profile_id: "personal",
        rows: [
          row("task:created-from-pinned", "task", "created-from-pinned", 0, "新任务"),
          row("task:personal-task-1", "task", "personal-task-1", 0, "任务 1"),
        ],
      });
      const request = vi.fn(async (action: string, params: Record<string, unknown>) => {
        if (action === "hello") return { capabilities: ["history"] };
        if (action === "provider_status") return { provider_authentication: "authenticated", authenticated_providers: ["openai"], selected_provider: "openai" };
        if (action === "set_ui_state") return { state: params.state, sidebar: allSidebar() };
        if (action === "create_task") {
          fixture.records.tasks.push(created);
          return { task: created, records: fixture.records, sidebar: allSidebar() };
        }
        if (action === "prompt") return { accepted: true };
        throw new Error(`unexpected action ${action}`);
      });
      const adapter = createBridgeAdapter({
        bootstrap: vi.fn().mockResolvedValue(fixture.bootstrap),
        request: request as unknown as PadDesktopApi["request"],
        chooseProjectDirectory: vi.fn(),
        subscribe: vi.fn(() => () => undefined),
      });
      const initial = await adapter.loadSnapshot();
      expect(initial.tasks).toEqual([]);
      request.mockClear();

      const task = await adapter.createTask(null);
      await adapter.sendMessage({
        taskId: task.id,
        accountId: "personal",
        fullAccess: false,
        text: "从空视图发送",
        attachmentPaths: [],
        provider: "",
        model: "",
        thinkingLevel: "default",
        fastMode: true,
      });

      expect(request.mock.calls.map(([action]) => action)).toEqual([
        "set_ui_state",
        "create_task",
        "prompt",
      ]);
      expect(request.mock.calls[0]?.[1]).toEqual(expect.objectContaining({
        state: expect.objectContaining({ sidebar_view: "all" }),
      }));
      expect(request).toHaveBeenCalledWith("prompt", {
        task_id: "created-from-pinned",
        prompt: "从空视图发送",
        fast_mode: true,
      });
    } finally {
      vi.useRealTimers();
    }
  });

  it("临时空 history 不覆盖已经显示的 Pi 会话", async () => {
    const fixture = bridgeFixture(1);
    let historyReads = 0;
    const request = vi.fn(async (action: string) => {
      if (action === "hello") return { capabilities: ["history"] };
      if (action === "provider_status") return { provider_authentication: "authenticated" };
      if (action === "history") {
        historyReads += 1;
        return historyReads === 1
          ? { messages: [{ id: "kept", role: "assistant", content: "保留这条回复" }] }
          : { messages: [] };
      }
      throw new Error(`unexpected action ${action}`);
    });
    const adapter = createBridgeAdapter({
      bootstrap: vi.fn().mockResolvedValue(fixture.bootstrap),
      request: request as unknown as PadDesktopApi["request"],
      chooseProjectDirectory: vi.fn(),
      subscribe: vi.fn(() => () => undefined),
    });

    const initial = await adapter.loadSnapshot();
    expect(initial.turnsByTask["personal-task-1"]?.[0]?.body).toBe("保留这条回复");
    const refreshed = await adapter.loadTaskData("personal-task-1");
    expect(refreshed.turnsByTask["personal-task-1"]?.[0]?.body).toBe("保留这条回复");
  });
});

describe("archive/pinned 侧边栏 records 映射", () => {
  it("保留包含归档或置顶子任务的未归档、未置顶祖先项目", async () => {
    const fixture = bridgeFixture(0);
    const project = {
      id: "ancestor-project",
      name: "祖先项目",
      primary_root: "/work/ancestor",
      additional_roots: [],
      profile_id: "personal",
      pinned: false,
      archived: false,
      created_at: 1,
      updated_at: 1,
    };
    const archivedTask = taskRecord("archived-child", "ancestor-project", { archived: true });
    const pinnedTask = taskRecord("pinned-child", "ancestor-project", { pinned: true });
    const ordinaryTask = taskRecord("ordinary-child", "ancestor-project");
    const collisionProject = { ...project, id: "collision-project", name: "跨账号碰撞项目", primary_root: "/work/collision" };
    const otherProfileArchivedTask = { ...taskRecord("other-profile-child", "collision-project", { archived: true }), profile_id: "team" };
    fixture.records.projects = [project, collisionProject];
    fixture.records.tasks = [archivedTask, pinnedTask, ordinaryTask, otherProfileArchivedTask];
    fixture.bootstrap.ui_state = { ...fixture.bootstrap.ui_state, sidebar_view: "archive", selected_task_id: archivedTask.id };
    const sidebarFor = (view: "archive" | "pinned") => ({
      view,
      active_profile_id: "personal",
      rows: [
        row("project:ancestor-project", "project", "ancestor-project", 0, "祖先项目"),
        row(`task:${view === "archive" ? archivedTask.id : pinnedTask.id}`, "task", view === "archive" ? archivedTask.id : pinnedTask.id, 1, view === "archive" ? "归档子任务" : "置顶子任务"),
      ],
    });
    fixture.bootstrap.sidebar = sidebarFor("archive");
    const request = vi.fn(async (action: string, params: Record<string, unknown>) => {
      if (action === "hello") return { capabilities: ["history"] };
      if (action === "provider_status") return { provider_authentication: "authenticated", authenticated_providers: ["openai"], selected_provider: "openai" };
      if (action === "history") return { messages: [] };
      if (action === "set_ui_state") {
        const state = params.state as DesktopUiStateDto;
        return { state, sidebar: sidebarFor(state.sidebar_view as "archive" | "pinned") };
      }
      throw new Error(`unexpected action ${action}`);
    });
    const adapter = createBridgeAdapter({
      bootstrap: vi.fn().mockResolvedValue(fixture.bootstrap),
      request: request as unknown as PadDesktopApi["request"],
      chooseProjectDirectory: vi.fn(),
      subscribe: vi.fn(() => () => undefined),
    });

    const archived = await adapter.loadSnapshot();
    expect(archived.uiState.sidebarView).toBe("archive");
    expect(archived.projects.map((item) => item.id)).toEqual(["ancestor-project"]);
    expect(archived.tasks.map((item) => [item.id, item.archived])).toEqual([["archived-child", true]]);
    expect(archived.sidebar.rows.map((item) => item.id).filter(Boolean)).toEqual(["ancestor-project", "archived-child"]);

    await adapter.updateUiState({ sidebarView: "pinned" });
    const pinned = await adapter.loadTaskData(pinnedTask.id);
    expect(pinned.uiState.sidebarView).toBe("pinned");
    expect(pinned.projects.map((item) => item.id)).toEqual(["ancestor-project"]);
    expect(pinned.tasks.map((item) => item.id)).toEqual(["pinned-child"]);
    expect(pinned.sidebar.rows.map((item) => item.id).filter(Boolean)).toEqual(["ancestor-project", "pinned-child"]);
    expect(request).toHaveBeenCalledWith("set_ui_state", {
      state: expect.objectContaining({ sidebar_view: "pinned", selected_task_id: "pinned-child" }),
    });
  });
});

describe("terminal adapter", () => {
  it("使用 v2 白名单动作并确保 close 只请求一次", async () => {
    const request = vi.fn(async (action: string, params: Record<string, unknown>) => {
      if (action === "terminal_open") return {
        pane_id: "pane-1", task_id: "task-1", epoch: 3, status: "opening", size: { columns: params.columns, rows: params.rows },
      };
      if (action === "terminal_snapshot") return {
        pane_id: "pane-1",
        task_id: "task-1",
        epoch: 3,
        revision: 4,
        status: "running",
        is_open: true,
        size: { columns: 240, rows: 80 },
        lines: Array.from({ length: 90 }, (_, index) => `line-${index}`),
        cursor: null,
        mode: { alternate_screen: false, bracketed_paste: false, mouse_reporting: false, sgr_mouse: false, application_cursor: false },
        viewport: { display_offset: 0, history_size: 0 },
      };
      return { pane_id: "pane-1", accepted: true };
    });
    const adapter = createBridgeAdapter({
      bootstrap: vi.fn(),
      request: request as unknown as PadDesktopApi["request"],
      chooseProjectDirectory: vi.fn(),
      subscribe: vi.fn(() => () => undefined),
    });

    const opened = await adapter.openTerminal("task-1", { columns: 999, rows: 999 });
    await adapter.writeTerminal(opened.paneId, "中文\r");
    await adapter.resizeTerminal(opened.paneId, { columns: 999, rows: 999 });
    const terminal = await adapter.getTerminalSnapshot(opened.paneId);
    await adapter.closeTerminal(opened.paneId);

    expect(request).toHaveBeenCalledWith("terminal_open", expect.objectContaining({ task_id: "task-1", columns: 240, rows: 80 }));
    expect(request).toHaveBeenCalledWith("terminal_input", { pane_id: "pane-1", data: "中文\r" });
    expect(request).toHaveBeenCalledWith("terminal_resize", { pane_id: "pane-1", columns: 240, rows: 80 });
    expect(terminal.lines).toHaveLength(80);
    expect(request.mock.calls.filter(([action]) => action === "terminal_close")).toHaveLength(1);
  });
});

describe("local UI state adapter", () => {
  it("启动读取本地状态，并串行合并完整文档避免并发覆盖", async () => {
    let persisted: DesktopUiStateDto = {
      active_profile_id: "personal",
      selected_task_id: "personal-task",
      collapsed_section_ids: [],
      collapsed_project_ids: [],
      sidebar_width: 300,
      sidebar_view: "all",
      theme: "light",
      right_panel_open: false,
      bottom_panel_open: false,
      sidebar_open: true,
    };
    const saved: DesktopUiStateDto[] = [];
    const profile = (id: string): ProfileDto => ({
      id,
      name: id,
      default_provider: "openai",
      policy: { mode: "guarded", unattended: false },
      created_at: 1,
      updated_at: 1,
    });
    const records: DesktopBootstrapResult["records"] = {
      profiles: [profile("personal"), profile("team")],
      projects: [],
      tasks: [{
        id: "personal-task",
        project_id: null,
        profile_id: "personal",
        title: "New task",
        summary: "",
        cwd: "/work",
        environment: "local",
        status: "idle",
        unread: false,
        pinned: false,
        archived: false,
        created_at: 1,
        updated_at: 1,
      }],
    };
    const bootstrap: DesktopBootstrapResult = {
      protocol_version: 2,
      backend: { status: "ready", provider_authentication: "authenticated", authenticated_providers: ["openai"], selected_provider: "openai", selected_model: null },
      profile: records.profiles[0]!,
      capabilities: [],
      sidebar: { rows: [] },
      ui_state: persisted,
      records,
    };
    const request = vi.fn(async (action: string, params: Record<string, unknown>) => {
      if (action === "hello") return { capabilities: [] };
      if (action === "provider_status") return { provider_authentication: "authenticated", authenticated_providers: ["openai"], selected_provider: "openai" };
      if (action === "history") return { messages: [] };
      if (action === "set_ui_state") {
        persisted = (params.state as DesktopUiStateDto);
        saved.push(structuredClone(persisted));
        return { state: persisted, sidebar: { rows: [] } };
      }
      if (action === "set_task") {
        records.tasks[0] = { ...records.tasks[0]!, pinned: params.pinned === true };
        return { task: records.tasks[0], records, sidebar: { rows: [] } };
      }
      if (action === "create_profile") {
        const created = profile("new-account");
        records.profiles.push(created);
        return { profile: created, records, sidebar: { rows: [] } };
      }
      if (action === "list_sidebar") return { records, sidebar: { rows: [] }, ui_state: persisted };
      throw new Error(`unexpected action ${action}`);
    });
    const adapter = createBridgeAdapter({
      bootstrap: vi.fn().mockResolvedValue(bootstrap),
      request: request as unknown as PadDesktopApi["request"],
      chooseProjectDirectory: vi.fn(),
      subscribe: vi.fn(() => () => undefined),
    });

    const loaded = await adapter.loadSnapshot();
    expect(loaded.uiState).toMatchObject({ activeProfileId: "personal", selectedTaskId: "personal-task", sidebarWidth: 300 });
    expect(loaded.tasks[0]?.title).toBe("新任务");

    await Promise.all([
      adapter.updateUiState({ theme: "dark", collapsedProjectIds: ["project:p"] }),
      adapter.updateUiState({ sidebarWidth: 333, bottomPanelOpen: true }),
    ]);
    expect(saved).toHaveLength(2);
    expect(saved[1]).toMatchObject({ theme: "dark", sidebar_width: 333, bottom_panel_open: true, collapsed_project_ids: ["project:p"] });

    const updated = await adapter.updateTask("personal-task", { pinned: true });
    expect(request).toHaveBeenCalledWith("set_task", {
      task_id: "personal-task",
      pinned: true,
      archived: undefined,
      unread: undefined,
    });
    expect(updated.tasks[0]?.pinned).toBe(true);

    await adapter.createAccount("新账号", "anthropic");
    const createCall = request.mock.calls.find(([action]) => action === "create_profile");
    expect(createCall?.[1]).toEqual({ name: "新账号", default_provider: "anthropic" });
    expect(createCall?.[1]).not.toHaveProperty("permission_mode");
    expect(createCall?.[1]).not.toHaveProperty("unattended");

    await adapter.switchAccount("team");
    expect(saved.at(-1)).toMatchObject({ active_profile_id: "team", selected_task_id: null });
  });
});

function task(id: string, projectId: string | null, title: string): TaskSummary {
  return { id, projectId, profileId: "p", title, updatedAt: "刚刚", status: "idle", rawStatus: "idle" };
}

function row(key: string, kind: string, id: string | undefined, depth: number, title: string) {
  return {
    key,
    node: id ? { kind, id } : { kind },
    depth,
    title,
    status: "none",
    unread: false,
    pinned: false,
    archived: false,
    missing_reference: false,
  };
}

function bridgeFixture(taskCount: number, includeTeam = false) {
  const profile = (id: string): ProfileDto => ({
    id,
    name: id,
    default_provider: "openai",
    policy: { mode: "guarded", unattended: false },
    created_at: 1,
    updated_at: 1,
  });
  const profiles = [profile("personal"), ...(includeTeam ? [profile("team")] : [])];
  const tasks: DesktopBootstrapResult["records"]["tasks"] = Array.from({ length: taskCount }, (_, index) => ({
    id: `personal-task-${index + 1}`,
    project_id: null,
    profile_id: "personal",
    title: `任务 ${index + 1}`,
    summary: "",
    cwd: "/work",
    environment: "local",
    status: "idle",
    unread: false,
    pinned: false,
    archived: false,
    created_at: 1,
    updated_at: taskCount - index,
  }));
  const records: DesktopBootstrapResult["records"] = { profiles, projects: [], tasks };
  const uiState: DesktopUiStateDto = {
    active_profile_id: "personal",
    selected_task_id: tasks[0]?.id ?? null,
    collapsed_section_ids: [],
    collapsed_project_ids: [],
    sidebar_width: 275,
    sidebar_view: "all",
    theme: "system",
    right_panel_open: false,
    bottom_panel_open: false,
    sidebar_open: true,
  };
  const bootstrap: DesktopBootstrapResult = {
    protocol_version: 2,
    backend: {
      status: "ready",
      provider_authentication: "authenticated",
      authenticated_providers: ["openai"],
      selected_provider: "openai",
      selected_model: null,
    },
    profile: profiles[0]!,
    capabilities: ["history"],
    sidebar: { rows: [] },
    ui_state: uiState,
    records,
  };
  return { bootstrap, records };
}

function taskRecord(
  id: string,
  projectId: string | null,
  patch: Partial<DesktopBootstrapResult["records"]["tasks"][number]> = {},
): DesktopBootstrapResult["records"]["tasks"][number] {
  return {
    id,
    project_id: projectId,
    profile_id: "personal",
    title: id,
    summary: "",
    cwd: "/work",
    environment: "local",
    status: "idle",
    unread: false,
    pinned: false,
    archived: false,
    created_at: 1,
    updated_at: 1,
    ...patch,
  };
}
