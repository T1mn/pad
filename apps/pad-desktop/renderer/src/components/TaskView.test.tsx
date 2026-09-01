import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { snapshot } from "../test/fixtures";
import type { AccountSummary, ComposerMessageInput, PendingInteraction, TaskSummary, TerminalSnapshot, TurnEntry } from "../types";
import { BottomPanel, RightPanel, TaskView, sanitizeToolText } from "./TaskView";

describe("TaskView", () => {
  it("只对工具输出隐藏 Pi 内部会话路径与标识", () => {
    const secretPath = "/Users/tim/.pad/sessions/private.jsonl";
    const secretSession = "pi-session-secret-123";
    renderTaskView({
      turns: [{
        id: "tool-1",
        kind: "tool",
        title: "history",
        body: `PI_SESSION_FILE=${secretPath}\npi_session_id=${secretSession}`,
        state: "complete",
      }],
    });

    expect(screen.queryByText(new RegExp(secretPath))).not.toBeInTheDocument();
    expect(screen.queryByText(new RegExp(secretSession))).not.toBeInTheDocument();
    expect(screen.getByText(/已隐藏/)).toBeInTheDocument();
  });

  it("不会改写普通用户文本", () => {
    expect(sanitizeToolText("请解释普通项目路径 src/main.ts")).toBe("请解释普通项目路径 src/main.ts");
  });

  it("隐藏 Pi、Codex 与 ChatGPT 的私有目录和环境变量值", () => {
    const safe = sanitizeToolText([
      "PI_HOME=/Users/tim/.pi/private",
      "CODEX_HOME=/Users/tim/.codex/private",
      "CHATGPT_TOKEN=secret-token",
      "log /Users/tim/.chatgpt/session.json",
      "log /Users/tim/Library/Application Support/Pi/account.json",
    ].join("\n"));

    expect(safe).not.toContain("/Users/tim");
    expect(safe).not.toContain("secret-token");
    expect(safe).not.toContain(".pi");
    expect(safe).not.toContain(".codex");
    expect(safe).not.toContain(".chatgpt");
    expect(safe).toContain("<已隐藏>");
    expect(safe).toContain("<PAD 私有路径已隐藏>");
  });

  it("工具失败状态不会误显示为完成", () => {
    renderTaskView({
      turns: [{ id: "failed-tool", kind: "tool", title: "命令执行", body: "exit 1", state: "failed" }],
    });

    expect(screen.getByText("失败")).toBeInTheDocument();
    expect(screen.queryByText("完成")).not.toBeInTheDocument();
  });

  it("分别显示 reasoning、error、status、final 与 activity 时间线角色", () => {
    renderTaskView({
      turns: [
        { id: "reasoning", kind: "reasoning", title: "分析依赖", body: "检查调用关系", state: "running" },
        { id: "error", kind: "error", title: "构建失败", body: "类型不匹配", state: "failed" },
        { id: "status", kind: "status", title: "测试进度", body: "8 / 10", state: "running" },
        { id: "activity", kind: "activity", title: "索引项目", body: "完成扫描", state: "complete" },
        { id: "final", kind: "final", body: "所有检查均已完成。", state: "complete" },
      ],
    });

    expect(screen.getByText("分析依赖")).toBeInTheDocument();
    expect(screen.getByText("构建失败")).toBeInTheDocument();
    expect(screen.getByText("测试进度")).toBeInTheDocument();
    expect(screen.getByText("索引项目")).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "最终答复" })).toBeInTheDocument();
    expect(screen.getByText("所有检查均已完成。")).toBeInTheDocument();
  });

  it("安全渲染 assistant GFM，并拒绝 raw HTML 与非 HTTPS 链接", () => {
    renderTaskView({
      turns: [{
        id: "assistant-1",
        kind: "assistant",
        body: "## 结果\n\n- 一项\n\n`inline`\n\n```ts\nconst ok = true\n```\n\n|列|值|\n|-|-|\n|A|1|\n\n[安全](https://example.com) [不安全](http://example.com)\n\n<script>window.bad=true</script>",
      }],
    });

    expect(screen.getByRole("heading", { name: "结果" })).toBeInTheDocument();
    expect(screen.getByRole("list")).toBeInTheDocument();
    expect(screen.getByRole("table")).toBeInTheDocument();
    expect(screen.getByText("const ok = true")).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "安全" })).toHaveAttribute("href", "https://example.com");
    expect(screen.getByRole("link", { name: "安全" })).toHaveAttribute("rel", "noopener noreferrer");
    expect(screen.queryByRole("link", { name: "不安全" })).not.toBeInTheDocument();
    expect(screen.queryByText(/window\.bad/)).not.toBeInTheDocument();
  });

  it("复制按钮调用真实 Clipboard API", async () => {
    renderTaskView({ turns: [{ id: "a", kind: "assistant", body: "可复制正文" }] });
    const user = userEvent.setup();
    const writeText = vi.spyOn(navigator.clipboard, "writeText").mockResolvedValue(undefined);

    await user.click(screen.getByRole("button", { name: "复制" }));
    expect(writeText).toHaveBeenCalledWith("可复制正文");
    expect(screen.getByRole("button", { name: "已复制" })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /反馈/ })).not.toBeInTheDocument();
  });

  it("idle 和 failed 都像原生 Pi 一样直接发送，只有 running 才停止", async () => {
    const user = userEvent.setup();
    const onSend = vi.fn().mockResolvedValue(undefined);
    const idle = renderTaskView({ onSend });
    await user.type(screen.getByLabelText("任务输入"), "执行测试");
    await user.click(screen.getByRole("button", { name: "发送" }));
    expect(onSend).toHaveBeenCalledWith(expect.objectContaining({
      text: "执行测试",
      attachmentPaths: [],
      provider: "openai",
      model: "gpt-5.4",
      thinkingLevel: "default",
      fastMode: true,
    }));
    idle.unmount();

    const onStop = vi.fn().mockResolvedValue(undefined);
    const runningTask = { ...snapshot().tasks[0]!, status: "running" as const, rawStatus: "running" };
    const running = renderTaskView({ task: runningTask, onStop });
    await user.click(screen.getByRole("button", { name: "停止任务" }));
    expect(onStop).toHaveBeenCalledOnce();
    running.unmount();

    const failedSend = vi.fn().mockResolvedValue(undefined);
    const failedTask = { ...snapshot().tasks[0]!, status: "attention" as const, rawStatus: "failed" };
    renderTaskView({ task: failedTask, onSend: failedSend });
    await user.type(screen.getByLabelText("任务输入"), "继续");
    await user.click(screen.getByRole("button", { name: "发送" }));
    expect(failedSend).toHaveBeenCalledWith(expect.objectContaining({ text: "继续" }));
  });

  it("error 状态显示中文任务状态并保持原生 Pi 输入框可用", async () => {
    const errorTask = { ...snapshot().tasks[0]!, status: "attention" as const, rawStatus: "error" };
    renderTaskView({ task: errorTask });

    expect(screen.getByText("需要处理", { selector: ".task-status" })).toBeInTheDocument();
    expect(screen.getByLabelText("任务输入")).toBeEnabled();
    expect(screen.getByRole("button", { name: "发送" })).toBeInTheDocument();
  });

  it("发送失败时保留完整草稿且不产生未处理 rejection", async () => {
    const onSend = vi.fn().mockRejectedValue(new Error("send failed"));
    const onChooseAttachments = vi.fn().mockResolvedValue(["/tmp/失败时保留.md"]);
    renderTaskView({ onSend, onChooseAttachments });
    const user = userEvent.setup();
    const input = screen.getByLabelText("任务输入");

    await user.click(screen.getByRole("button", { name: "添加附件" }));
    expect(await screen.findByText("失败时保留.md")).toBeInTheDocument();
    await user.type(input, "这段内容不能丢");
    await user.click(screen.getByRole("button", { name: "发送" }));
    await waitFor(() => expect(onSend).toHaveBeenCalledWith(expect.objectContaining({
      text: "这段内容不能丢",
      attachmentPaths: ["/tmp/失败时保留.md"],
    })));
    expect(input).toHaveValue("这段内容不能丢");
    expect(screen.getByText("失败时保留.md")).toBeInTheDocument();
  });

  it("账号只有已登录 provider、没有 default_model 时使用 Pi 默认模型直接发送", async () => {
    const data = snapshot();
    const activeAccount = { ...data.accounts[0]!, selectedProvider: "openai", selectedModel: null };
    const onSend = vi.fn().mockResolvedValue(undefined);
    renderTaskView({ activeAccount, onSend });
    const user = userEvent.setup();

    expect(screen.getByRole("button", { name: /openai \/ Pi 默认模型/ })).toBeInTheDocument();
    await user.type(screen.getByLabelText("任务输入"), "使用默认模型");
    await user.click(screen.getByRole("button", { name: "发送" }));

    await waitFor(() => expect(onSend).toHaveBeenCalledWith(expect.objectContaining({
      text: "使用默认模型",
      provider: "",
      model: "",
      thinkingLevel: "default",
      fastMode: true,
    })));
    expect(screen.queryByText("请同时填写模型提供商和模型名称。")).not.toBeInTheDocument();
  });

  it("附件、模型与推理控件真实可用，成功发送后仅清空草稿和附件", async () => {
    const onSend = vi.fn().mockResolvedValue(undefined);
    const onChooseAttachments = vi.fn().mockResolvedValue(["/tmp/spec.md", "/tmp/screenshot.png"]);
    renderTaskView({ onSend, onChooseAttachments });
    const user = userEvent.setup();

    const attachmentButton = screen.getByRole("button", { name: "添加附件" });
    expect(attachmentButton).toBeEnabled();
    await user.click(attachmentButton);
    expect(onChooseAttachments).toHaveBeenCalledOnce();
    expect(await screen.findByText("spec.md")).toBeInTheDocument();
    expect(screen.getByText("screenshot.png")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "移除附件 /tmp/screenshot.png" }));
    expect(screen.queryByText("screenshot.png")).not.toBeInTheDocument();

    const modelButton = screen.getByRole("button", { name: /选择 Pi 模型/ });
    expect(modelButton).toHaveAttribute("aria-haspopup", "dialog");
    await user.click(modelButton);
    const modelDialog = screen.getByRole("dialog", { name: "选择 Pi 模型" });
    const provider = within(modelDialog).getByLabelText("模型提供商");
    const model = within(modelDialog).getByLabelText("模型名称");
    expect(provider).toHaveFocus();
    await user.clear(provider);
    await user.type(provider, "custom-provider");
    await user.clear(model);
    await user.type(model, "custom-model");
    await user.click(within(modelDialog).getByRole("button", { name: "完成" }));
    await user.selectOptions(screen.getByRole("combobox", { name: "推理强度" }), "xhigh");

    const input = screen.getByLabelText("任务输入");
    await user.type(input, "分析附件");
    await user.click(screen.getByRole("button", { name: "发送" }));

    await waitFor(() => expect(onSend).toHaveBeenCalledWith({
      text: "分析附件",
      attachmentPaths: ["/tmp/spec.md"],
      provider: "custom-provider",
      model: "custom-model",
      thinkingLevel: "xhigh",
      fastMode: true,
    }));
    expect(input).toHaveValue("");
    expect(screen.queryByText("spec.md")).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: /custom-provider \/ custom-model/ })).toBeInTheDocument();
    expect(screen.getByRole("combobox", { name: "推理强度" })).toHaveValue("xhigh");
    expect(screen.getByRole("checkbox", { name: "快速模式" })).toBeChecked();
    expect(screen.queryByText(/尚未开放/)).not.toBeInTheDocument();
  });

  it("快速模式默认开启并可在发送前关闭", async () => {
    const onSend = vi.fn().mockResolvedValue(undefined);
    renderTaskView({ onSend });
    const user = userEvent.setup();
    const toggle = screen.getByRole("checkbox", { name: "快速模式" });
    expect(toggle).toBeChecked();
    await user.click(toggle);
    await user.type(screen.getByLabelText("任务输入"), "普通服务等级");
    await user.click(screen.getByRole("button", { name: "发送" }));
    expect(onSend).toHaveBeenCalledWith(expect.objectContaining({ fastMode: false }));
  });

  it("模型选择弹层支持 Escape、焦点归还与外部点击关闭", async () => {
    renderTaskView();
    const user = userEvent.setup();
    const modelButton = screen.getByRole("button", { name: /选择 Pi 模型/ });

    await user.click(modelButton);
    expect(screen.getByRole("dialog", { name: "选择 Pi 模型" })).toBeInTheDocument();
    await user.keyboard("{Escape}");
    expect(screen.queryByRole("dialog", { name: "选择 Pi 模型" })).not.toBeInTheDocument();
    expect(modelButton).toHaveFocus();

    await user.click(modelButton);
    await user.click(screen.getByRole("region", { name: "当前任务" }));
    expect(screen.queryByRole("dialog", { name: "选择 Pi 模型" })).not.toBeInTheDocument();
  });

  it("附件去重、拒绝相对路径并严格限制为 20 个", async () => {
    const selected = Array.from({ length: 22 }, (_, index) => `/tmp/file-${index}.txt`);
    selected.push("relative.txt", "/tmp/file-0.txt");
    renderTaskView({ onChooseAttachments: vi.fn().mockResolvedValue(selected) });
    const user = userEvent.setup();

    await user.click(screen.getByRole("button", { name: "添加附件" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("最多只能添加 20 个附件");
    expect(screen.getAllByRole("button", { name: /移除附件/ })).toHaveLength(20);
    expect(screen.getByRole("button", { name: "添加附件" })).toBeDisabled();
  });

  it("渲染真实 confirm、select、input 与 editor 请求并提交对应 v2 值", async () => {
    const user = userEvent.setup();
    const onRespondInteraction = vi.fn().mockResolvedValue(undefined);
    const confirm = renderTaskView({
      interactions: [{
        id: "confirm-1", kind: "confirm", title: "允许执行命令？", message: "运行 npm test", options: [], requiresResponse: true,
      }],
      onRespondInteraction,
    });
    expect(screen.getByLabelText("任务输入")).toBeDisabled();
    await user.click(screen.getByRole("button", { name: "确认" }));
    expect(onRespondInteraction).toHaveBeenLastCalledWith("personal-task", "confirm-1", true);
    expect(await screen.findByText("已提交，Pi 正在继续执行。")).toBeInTheDocument();
    confirm.unmount();

    const select = renderTaskView({
      interactions: [{
        id: "select-1", kind: "select", title: "选择部署环境", options: ["测试环境", "生产环境"], defaultIndex: 0, requiresResponse: true,
      }],
      onRespondInteraction,
    });
    const testEnvironment = screen.getByRole("radio", { name: "测试环境" });
    testEnvironment.focus();
    await user.keyboard("{ArrowDown}");
    expect(screen.getByRole("radio", { name: "生产环境" })).toHaveFocus();
    expect(screen.getByRole("radio", { name: "生产环境" })).toHaveAttribute("aria-checked", "true");
    await user.click(screen.getByRole("button", { name: "提交选择" }));
    expect(onRespondInteraction).toHaveBeenLastCalledWith("personal-task", "select-1", 1);
    select.unmount();

    const input = renderTaskView({
      interactions: [{
        id: "input-1", kind: "input", title: "输入版本号", options: [], defaultValue: "1.0.0", requiresResponse: true,
      }],
      onRespondInteraction,
    });
    const inputField = screen.getByLabelText("输入内容");
    await user.clear(inputField);
    await user.type(inputField, "2.0.0");
    await user.click(screen.getByRole("button", { name: "提交" }));
    expect(onRespondInteraction).toHaveBeenLastCalledWith("personal-task", "input-1", "2.0.0");
    input.unmount();

    renderTaskView({
      interactions: [{
        id: "editor-1", kind: "editor", title: "编辑发布说明", options: [], defaultValue: "初稿", requiresResponse: true,
      }],
      onRespondInteraction,
    });
    const editor = screen.getByLabelText("编辑内容");
    await user.clear(editor);
    await user.type(editor, "最终内容");
    await user.click(screen.getByRole("button", { name: "提交" }));
    expect(onRespondInteraction).toHaveBeenLastCalledWith("personal-task", "editor-1", "最终内容");
  });

  it("交互提交期间锁定按钮，失败后显示中文错误并允许重试", async () => {
    let rejectResponse: ((reason?: unknown) => void) | undefined;
    const onRespondInteraction = vi.fn(() => new Promise<void>((_, reject) => { rejectResponse = reject; }));
    const user = userEvent.setup();
    renderTaskView({
      interactions: [{ id: "confirm-error", kind: "confirm", title: "继续？", options: [], requiresResponse: true }],
      onRespondInteraction,
    });

    await user.click(screen.getByRole("button", { name: "确认" }));
    expect(screen.getByRole("button", { name: "确认" })).toBeDisabled();
    expect(screen.getByText("正在提交…")).toBeInTheDocument();
    rejectResponse?.({ code: "interaction_failed", message: "raw respond_ui transport failed" });

    expect(await screen.findByRole("alert")).toHaveTextContent("提交响应失败，请重试。");
    expect(screen.getByText("诊断信息")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "确认" })).toBeEnabled();
  });

  it("建议卡片填入真实草稿，中文输入法确认候选词时不会误发送", async () => {
    const user = userEvent.setup();
    const onSend = vi.fn().mockResolvedValue(undefined);
    renderTaskView({ onSend });

    await user.click(screen.getByRole("button", { name: /理解项目/ }));
    const input = screen.getByLabelText("任务输入");
    expect(input).toHaveValue("请解释当前项目的代码结构与关键模块。");

    fireEvent.keyDown(input, { key: "Enter", isComposing: true, keyCode: 229 });
    expect(onSend).not.toHaveBeenCalled();

    fireEvent.keyDown(input, { key: "Enter" });
    await waitFor(() => expect(onSend).toHaveBeenCalledWith(expect.objectContaining({ text: "请解释当前项目的代码结构与关键模块。" })));
    expect(input).toHaveValue("");
  });

  it("更多菜单执行真实固定与归档动作", async () => {
    const user = userEvent.setup();
    const onUpdateTask = vi.fn().mockResolvedValue(undefined);
    renderTaskView({ onUpdateTask });

    await user.click(screen.getByRole("button", { name: "更多任务操作" }));
    await user.click(screen.getByRole("menuitem", { name: "固定任务" }));
    expect(onUpdateTask).toHaveBeenCalledWith({ pinned: true });

    await user.click(screen.getByRole("button", { name: "更多任务操作" }));
    await user.click(screen.getByRole("menuitem", { name: "归档任务" }));
    expect(onUpdateTask).toHaveBeenCalledWith({ archived: true });
  });

  it("归档任务在更多菜单中提供真实恢复动作", async () => {
    const user = userEvent.setup();
    const onUpdateTask = vi.fn().mockResolvedValue(undefined);
    const archivedTask = { ...snapshot().tasks[0]!, archived: true };
    renderTaskView({ task: archivedTask, onUpdateTask });

    await user.click(screen.getByRole("button", { name: "更多任务操作" }));
    await user.click(screen.getByRole("menuitem", { name: "恢复任务" }));
    expect(onUpdateTask).toHaveBeenCalledWith({ archived: false });
  });

  it("主任务与右侧面板保持独立焦点域，并支持方向键切换标签", async () => {
    const data = snapshot();
    const taskView = renderTaskView();
    expect(taskView.container.querySelector('[data-focus-domain="main"]')).toBeInTheDocument();
    expect(taskView.container.querySelector(".thread-content")).toHaveAttribute("data-thread-max-width", "768");
    taskView.unmount();

    const onClose = vi.fn();
    render(<RightPanel task={data.tasks[0]!} project={data.projects[0]!} turns={[]} onClose={onClose} />);
    expect(document.querySelector('[data-focus-domain="right"]')).toBeInTheDocument();
    const activity = screen.getByRole("tab", { name: "活动" });
    expect(activity).toHaveAttribute("aria-selected", "true");
    activity.focus();
    fireEvent.keyDown(activity, { key: "ArrowRight" });
    expect(screen.getByRole("tab", { name: "文件" })).toHaveAttribute("aria-selected", "true");
    expect(screen.getByRole("tabpanel")).toHaveAccessibleName("文件");
    fireEvent.keyDown(screen.getByRole("tab", { name: "文件" }), { key: "End" });
    expect(screen.getByRole("tab", { name: "更改" })).toHaveAttribute("aria-selected", "true");
    fireEvent.keyDown(screen.getByRole("tabpanel"), { key: "Escape" });
    expect(onClose).toHaveBeenCalledOnce();
  });

  it("只用 typed artifacts 分栏展示文件与更改，并支持选择文件", async () => {
    const data = snapshot();
    const turns: TurnEntry[] = [
      { id: "command", kind: "tool", title: "运行测试", body: "npm test 通过", state: "complete" },
      {
        id: "patch",
        kind: "tool",
        title: "应用补丁",
        body: "结构化补丁已应用",
        state: "complete",
        artifacts: [{
          id: "main-change",
          kind: "change",
          path: "src/main.ts",
          operation: "modified",
          diff: "@@ -1 +1 @@\n-const ready = false;\n+const ready = true;",
        }, {
          id: "worker-change",
          kind: "change",
          path: "src/worker.ts",
          operation: "created",
          diff: "@@ -0,0 +1 @@\n+export const worker = true;",
        }],
      },
      {
        id: "read",
        kind: "tool",
        title: "读取文件",
        body: "读取完成",
        state: "complete",
        artifacts: [{ id: "guide", kind: "file", path: "docs/guide.md", operation: "read" }],
      },
    ];
    render(<RightPanel task={data.tasks[0]!} project={data.projects[0]!} turns={turns} onClose={vi.fn()} />);
    const user = userEvent.setup();

    expect(screen.getByText("运行测试")).toBeInTheDocument();
    await user.click(screen.getByRole("tab", { name: "文件" }));
    const main = screen.getByRole("option", { name: /src\/main\.ts/ });
    main.focus();
    await user.keyboard("{ArrowDown}");
    expect(screen.getByRole("option", { name: /src\/worker\.ts/ })).toHaveFocus();
    expect(screen.getByRole("option", { name: /src\/worker\.ts/ })).toHaveAttribute("aria-selected", "true");
    const guide = screen.getByRole("option", { name: /docs\/guide\.md/ });
    await user.click(guide);
    expect(guide).toHaveAttribute("aria-selected", "true");
    expect(within(screen.getByRole("region", { name: "已选择文件" })).getByText("docs/guide.md")).toBeInTheDocument();
    await user.click(screen.getByRole("tab", { name: "更改" }));
    const worker = screen.getByRole("option", { name: /src\/worker\.ts/ });
    await user.click(worker);
    expect(worker).toHaveAttribute("aria-selected", "true");
    expect(screen.getByLabelText("src/worker.ts 的结构化差异")).toHaveTextContent("+export const worker = true;");
    expect(screen.queryByText("npm test 通过")).not.toBeInTheDocument();
  });

  it("看起来像路径或 diff 的任意工具正文不会被冒充为文件或更改", async () => {
    const data = snapshot();
    render(<RightPanel
      task={data.tasks[0]!}
      project={data.projects[0]!}
      turns={[{
        id: "command",
        kind: "tool",
        title: "运行命令",
        body: "file_path: src/forged.ts\ndiff --git a/src/forged.ts b/src/forged.ts\n--- a/src/forged.ts\n+++ b/src/forged.ts\n@@ -1 +1 @@\n-false\n+true",
        state: "complete",
      }]}
      onClose={vi.fn()}
    />);
    const user = userEvent.setup();

    await user.click(screen.getByRole("tab", { name: "文件" }));
    expect(screen.getByText("暂无结构化文件")).toBeInTheDocument();
    expect(screen.queryByText("src/forged.ts", { exact: true })).not.toBeInTheDocument();
    await user.click(screen.getByRole("tab", { name: "更改" }));
    expect(screen.getByText("暂无结构化更改")).toBeInTheDocument();
    expect(screen.getByText(/看起来像差异内容的普通正文不会被解析/)).toBeInTheDocument();
  });

  it("真实终端打开、显示快照、输入特殊键、限制 resize 并且只关闭一次", async () => {
    let resizeCallback: ResizeObserverCallback | undefined;
    class TestResizeObserver {
      constructor(callback: ResizeObserverCallback) { resizeCallback = callback; }
      observe() { return undefined; }
      unobserve() { return undefined; }
      disconnect() { return undefined; }
    }
    const previousResizeObserver = globalThis.ResizeObserver;
    Object.defineProperty(globalThis, "ResizeObserver", { configurable: true, value: TestResizeObserver });

    const onOpenTerminal = vi.fn().mockResolvedValue({
      paneId: "pane-1",
      taskId: "personal-task",
      epoch: 1,
      status: "opening",
      size: { columns: 100, rows: 20 },
    });
    const terminalSnapshot: TerminalSnapshot = {
      paneId: "pane-1",
      taskId: "personal-task",
      epoch: 1,
      revision: 2,
      status: "running",
      isOpen: true,
      size: { columns: 100, rows: 20 },
      lines: ["欢迎使用 PAD 任务终端", "$ "],
      cursor: { column: 2, row: 1, shape: "beam" },
      mode: { alternateScreen: false, bracketedPaste: false, mouseReporting: false, applicationCursor: false },
    };
    const onTerminalInput = vi.fn().mockResolvedValue(undefined);
    const onTerminalResize = vi.fn().mockResolvedValue(undefined);
    const onTerminalSnapshot = vi.fn().mockResolvedValue(terminalSnapshot);
    const onTerminalClose = vi.fn().mockResolvedValue(undefined);
    const rendered = render(
      <BottomPanel
        task={snapshot().tasks[0]!}
        onClose={vi.fn()}
        onOpenTerminal={onOpenTerminal}
        onTerminalInput={onTerminalInput}
        onTerminalResize={onTerminalResize}
        onTerminalSnapshot={onTerminalSnapshot}
        onTerminalClose={onTerminalClose}
      />,
    );
    const user = userEvent.setup();

    expect(await screen.findByText(/欢迎使用 PAD 任务终端/)).toBeInTheDocument();
    expect(document.querySelector('[data-focus-domain="bottom"]')).toBeInTheDocument();
    const heightSeparator = screen.getByRole("separator", { name: "调整终端高度" });
    expect(heightSeparator).toHaveAttribute("aria-valuenow", "220");
    fireEvent.keyDown(heightSeparator, { key: "ArrowUp" });
    expect(heightSeparator).toHaveAttribute("aria-valuenow", "236");
    fireEvent.keyDown(heightSeparator, { key: "End" });
    expect(heightSeparator).toHaveAttribute("aria-valuenow", "480");
    fireEvent.pointerDown(heightSeparator, { pointerId: 1, clientY: 200 });
    fireEvent.pointerMove(heightSeparator, { pointerId: 1, clientY: 600 });
    fireEvent.pointerUp(heightSeparator, { pointerId: 1, clientY: 600 });
    expect(heightSeparator).toHaveAttribute("aria-valuenow", "180");
    expect(onOpenTerminal).toHaveBeenCalledWith("personal-task", expect.objectContaining({ columns: expect.any(Number), rows: expect.any(Number) }));
    const input = screen.getByLabelText("终端输入");
    fireEvent.compositionStart(input);
    fireEvent.change(input, { target: { value: "z" } });
    fireEvent.change(input, { target: { value: "中文" } });
    expect(onTerminalInput).not.toHaveBeenCalled();
    fireEvent.compositionEnd(input, { data: "中文" });
    await waitFor(() => expect(onTerminalInput).toHaveBeenCalledTimes(1));
    expect(onTerminalInput).toHaveBeenLastCalledWith("pane-1", "中文");
    onTerminalInput.mockClear();
    await user.type(input, "中文");
    fireEvent.keyDown(input, { key: "Enter" });
    fireEvent.keyDown(input, { key: "Backspace" });
    fireEvent.keyDown(input, { key: "Tab" });
    fireEvent.keyDown(input, { key: "ArrowUp" });
    fireEvent.keyDown(input, { key: "c", ctrlKey: true });
    await waitFor(() => {
      expect(onTerminalInput).toHaveBeenCalledWith("pane-1", "中");
      expect(onTerminalInput).toHaveBeenCalledWith("pane-1", "文");
      expect(onTerminalInput).toHaveBeenCalledWith("pane-1", "\r");
      expect(onTerminalInput).toHaveBeenCalledWith("pane-1", "\u007f");
      expect(onTerminalInput).toHaveBeenCalledWith("pane-1", "\t");
      expect(onTerminalInput).toHaveBeenCalledWith("pane-1", "\u001b[A");
      expect(onTerminalInput).toHaveBeenCalledWith("pane-1", "\u0003");
    });

    const output = screen.getByRole("log", { name: "终端输出" });
    Object.defineProperty(output, "clientWidth", { configurable: true, value: 8_000 });
    Object.defineProperty(output, "clientHeight", { configurable: true, value: 8_000 });
    resizeCallback?.([], {} as ResizeObserver);
    await waitFor(() => expect(onTerminalResize).toHaveBeenCalledWith("pane-1", { columns: 240, rows: 80 }));

    await user.click(screen.getByRole("button", { name: "关闭终端" }));
    await waitFor(() => expect(onTerminalClose).toHaveBeenCalledTimes(1));
    rendered.unmount();
    expect(onTerminalClose).toHaveBeenCalledTimes(1);
    Object.defineProperty(globalThis, "ResizeObserver", { configurable: true, value: previousResizeObserver });
  });

  it("终端完整保留最近 80 行并显示中文退出状态与退出码", async () => {
    const terminalSnapshot: TerminalSnapshot = {
      paneId: "pane-exited",
      taskId: "personal-task",
      epoch: 1,
      revision: 4,
      status: "exited",
      isOpen: false,
      size: { columns: 100, rows: 20 },
      lines: Array.from({ length: 85 }, (_, index) => `LINE-${String(index + 1).padStart(3, "0")}::output`),
      mode: { alternateScreen: false, bracketedPaste: false, mouseReporting: false, applicationCursor: false },
      exit: { code: 23, signaled: false },
    };
    const onTerminalSnapshot = vi.fn().mockResolvedValue(terminalSnapshot);
    render(
      <BottomPanel
        task={snapshot().tasks[0]!}
        onClose={vi.fn()}
        onOpenTerminal={vi.fn().mockResolvedValue({
          paneId: "pane-exited", taskId: "personal-task", epoch: 1, status: "opening", size: { columns: 100, rows: 20 },
        })}
        onTerminalInput={vi.fn().mockResolvedValue(undefined)}
        onTerminalResize={vi.fn().mockResolvedValue(undefined)}
        onTerminalSnapshot={onTerminalSnapshot}
        onTerminalClose={vi.fn().mockResolvedValue(undefined)}
      />,
    );

    const output = screen.getByRole("log", { name: "终端输出" });
    await waitFor(() => expect(output).toHaveAttribute("data-terminal-line-count", "80"));
    const renderedLines = output.querySelector("pre")?.textContent?.split("\n") ?? [];
    expect(renderedLines).toHaveLength(80);
    expect(renderedLines[0]).toBe("LINE-006::output");
    expect(renderedLines.at(-1)).toBe("LINE-085::output");
    expect(output).toHaveAttribute("tabindex", "0");
    expect(screen.getByText("已退出")).toBeInTheDocument();
    expect(screen.getByText("退出码 23")).toBeInTheDocument();
    const completedPollCount = onTerminalSnapshot.mock.calls.length;
    await new Promise((resolve) => setTimeout(resolve, 420));
    expect(onTerminalSnapshot).toHaveBeenCalledTimes(completedPollCount);
  });

  it("关闭 RPC 失败时保留终端并允许重试，成功后才关闭面板", async () => {
    const onClose = vi.fn();
    const onTerminalClose = vi.fn()
      .mockRejectedValueOnce(new Error("close transport failed"))
      .mockResolvedValue(undefined);
    const terminalSnapshot: TerminalSnapshot = {
      paneId: "pane-retry",
      taskId: "personal-task",
      epoch: 1,
      revision: 1,
      status: "running",
      isOpen: true,
      size: { columns: 100, rows: 20 },
      lines: ["$ "],
      mode: { alternateScreen: false, bracketedPaste: false, mouseReporting: false, applicationCursor: false },
    };
    const rendered = render(
      <BottomPanel
        task={snapshot().tasks[0]!}
        onClose={onClose}
        onOpenTerminal={vi.fn().mockResolvedValue({
          paneId: "pane-retry", taskId: "personal-task", epoch: 1, status: "opening", size: { columns: 100, rows: 20 },
        })}
        onTerminalInput={vi.fn().mockResolvedValue(undefined)}
        onTerminalResize={vi.fn().mockResolvedValue(undefined)}
        onTerminalSnapshot={vi.fn().mockResolvedValue(terminalSnapshot)}
        onTerminalClose={onTerminalClose}
      />,
    );
    const user = userEvent.setup();

    await screen.findByText("运行中");
    await user.click(screen.getByRole("button", { name: "关闭终端" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("无法关闭任务终端，请重试");
    expect(onClose).not.toHaveBeenCalled();
    expect(screen.getByLabelText("终端输入")).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "关闭终端" }));
    await waitFor(() => expect(onClose).toHaveBeenCalledOnce());
    expect(onTerminalClose).toHaveBeenCalledTimes(2);
    rendered.unmount();
    expect(onTerminalClose).toHaveBeenCalledTimes(2);
  });
});

function renderTaskView(overrides: {
  task?: TaskSummary;
  activeAccount?: AccountSummary;
  turns?: TurnEntry[];
  interactions?: PendingInteraction[];
  onChooseAttachments?: () => Promise<string[]>;
  onSend?: (input: ComposerMessageInput) => Promise<void>;
  onStop?: () => Promise<void>;
  onRespondInteraction?: (taskId: string, interactionId: string, value: boolean | number | string) => Promise<void>;
  onUpdateTask?: (patch: { pinned?: boolean; archived?: boolean; unread?: boolean }) => Promise<void>;
} = {}) {
  const data = snapshot();
  return render(
    <TaskView
      task={overrides.task ?? data.tasks[0]!}
      project={data.projects[0]!}
      activeAccount={overrides.activeAccount ?? data.accounts[0]!}
      turns={overrides.turns ?? []}
      interactions={overrides.interactions ?? []}
      fullAccess={false}
      rightPanelOpen={false}
      bottomPanelOpen={false}
      onFullAccessChange={vi.fn()}
      onRightPanelToggle={vi.fn()}
      onBottomPanelToggle={vi.fn()}
      onChooseAttachments={overrides.onChooseAttachments ?? vi.fn().mockResolvedValue([])}
      onSend={overrides.onSend ?? vi.fn().mockResolvedValue(undefined)}
      onStop={overrides.onStop ?? vi.fn().mockResolvedValue(undefined)}
      onRespondInteraction={overrides.onRespondInteraction ?? vi.fn().mockResolvedValue(undefined)}
      onUpdateTask={overrides.onUpdateTask ?? vi.fn().mockResolvedValue(undefined)}
    />,
  );
}
