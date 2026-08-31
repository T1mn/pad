import { useEffect, useMemo, useRef, useState } from "react";
import type { KeyboardEvent as ReactKeyboardEvent, PointerEvent as ReactPointerEvent } from "react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import type {
  AccountSummary,
  ComposerMessageInput,
  InteractionResponse,
  PendingInteraction,
  ProjectSummary,
  TaskSummary,
  TerminalPane,
  TerminalSize,
  TerminalSnapshot,
  TurnArtifact,
  TurnArtifactOperation,
  TurnEntry,
  TurnKind,
  ThinkingLevel,
} from "../types";
import type { ModelCatalogModel } from "../lib/model-catalog";
import { taskStatusLabel, toolStateLabel } from "../lib/labels";
import { toUserFacingError, type UserFacingError } from "../lib/errors";
import { Icon, type IconName } from "./Icons";

const sensitiveToolKeys = /\b(?:(?:PI|PAD|CODEX|CHATGPT)_[A-Z0-9_]+|pi_session_id|session_file|session_id|credential_ref|(?:access_|refresh_)?token|api_key)\b/gi;
const sensitiveToolAssignment = /(((?:(?:PI|PAD|CODEX|CHATGPT)_[A-Z0-9_]+|pi_session_id|session_file|session_id|credential_ref|(?:access_|refresh_)?token|api_key))["']?\s*[:=]\s*)[^,}\s]+/gi;
const privatePadPath = /(?:\/Users\/[^\s"']+\/)?(?:\.pad|\.pi|\.codex|\.chatgpt|Library\/Application Support\/(?:PAD Desktop|Pi|Codex|ChatGPT))[^\s"']*/gi;
const homePathPrefix = /\/Users\/[^/\s"']+/g;

export function sanitizeToolText(value: string): string {
  return value
    .split("\n")
    .map((line) => {
      if (sensitiveToolKeys.test(line)) {
        sensitiveToolKeys.lastIndex = 0;
        return line.replace(sensitiveToolAssignment, "$1<已隐藏>");
      }
      sensitiveToolKeys.lastIndex = 0;
      return line.replace(privatePadPath, "<PAD 私有路径已隐藏>").replace(homePathPrefix, "~");
    })
    .join("\n");
}

interface TaskViewProps {
  task: TaskSummary | null;
  project: ProjectSummary | null;
  activeAccount: AccountSummary | null;
  turns: TurnEntry[];
  interactions: PendingInteraction[];
  fullAccess: boolean;
  rightPanelOpen: boolean;
  bottomPanelOpen: boolean;
  onFullAccessChange(value: boolean): void;
  onRightPanelToggle(): void;
  onBottomPanelToggle(): void;
  onChooseAttachments(): Promise<string[]>;
  onSend(input: ComposerMessageInput): Promise<void>;
  onStop(): Promise<void>;
  onRespondInteraction(taskId: string, interactionId: string, value: InteractionResponse): Promise<void>;
  onUpdateTask(patch: { pinned?: boolean; archived?: boolean; unread?: boolean }): Promise<void>;
  onOpenSettings(): void;
}

function Turn({ turn, last }: { turn: TurnEntry; last: boolean }) {
  const [copyState, setCopyState] = useState<"idle" | "copied" | "failed">("idle");

  async function copyBody() {
    try {
      await navigator.clipboard.writeText(turn.body);
      setCopyState("copied");
    } catch {
      setCopyState("failed");
    }
  }
  if (turn.kind === "user") {
    return (
      <article className="turn turn-user">
        <div className="user-bubble">{turn.body}</div>
        <div className="turn-meta">{turn.meta}</div>
      </article>
    );
  }

  if (turn.kind === "tool") {
    const safeBody = sanitizeToolText(turn.body);
    return (
      <article className="turn turn-tool">
        <div className={`timeline-node ${turn.state === "running" ? "is-running" : ""}`}><Icon name="terminal" /></div>
        <div className="tool-card">
          <header><span>{turn.title}</span><span className={`tool-state state-${turn.state ?? "unknown"}`}>{toolStateLabel(turn.state)}</span></header>
          <p>{safeBody}</p>
          <footer><Icon name="code" />{turn.meta}</footer>
        </div>
      </article>
    );
  }

  if (turn.kind === "notice") {
    return <div className="turn-notice"><Icon name="sparkles" /><span>{turn.body}</span></div>;
  }

  if (["reasoning", "error", "status", "activity"].includes(turn.kind)) {
    const safeBody = sanitizeToolText(turn.body);
    return (
      <article className={`turn turn-structured turn-${turn.kind}`}>
        <div className={`timeline-node ${turn.state === "running" ? "is-running" : ""}`}>
          <Icon name={turnKindIcon(turn.kind)} />
        </div>
        <div className="structured-turn-card">
          <header>
            <div><span>{turnKindLabel(turn.kind)}</span><strong>{turn.title ?? turnKindLabel(turn.kind)}</strong></div>
            {turn.state && <span className={`tool-state state-${turn.state}`}>{toolStateLabel(turn.state)}</span>}
          </header>
          {safeBody && <p>{safeBody}</p>}
          {turn.meta && <footer>{turn.meta}</footer>}
        </div>
      </article>
    );
  }

  return (
    <article className={`turn turn-assistant ${turn.kind === "final" ? "turn-final" : ""}`}>
      <div className={`timeline-node ${last ? "is-current" : ""}`}><Icon name="sparkles" /></div>
      <div className="assistant-copy">
        {(turn.title || turn.kind === "final") && <h2>{turn.title ?? "最终答复"}</h2>}
        <MarkdownBody body={turn.body} />
        {(turn.model || turn.meta) && (
          <div className="turn-runtime-meta">
            {turn.model && <span>{formatModelLabel(turn.model)}</span>}
            {turn.meta && <span>{turn.meta}</span>}
          </div>
        )}
        <div className="turn-actions">
          <button onClick={() => void copyBody()}>{copyState === "copied" ? "已复制" : "复制"}</button>
          {copyState === "failed" && <span role="status">复制失败，请手动选择文本。</span>}
        </div>
      </div>
    </article>
  );
}

export function formatModelLabel(model: string): string {
  const match = /^gpt-(\d+(?:\.\d+)?)-(sol|terra|luna)$/i.exec(model.trim());
  if (!match) return model;
  return `GPT-${match[1]} ${match[2][0].toUpperCase()}${match[2].slice(1).toLowerCase()}`;
}

function turnKindLabel(kind: TurnKind): string {
  if (kind === "reasoning") return "推理";
  if (kind === "error") return "错误";
  if (kind === "status") return "状态";
  if (kind === "final") return "最终答复";
  if (kind === "activity") return "活动";
  if (kind === "tool") return "工具";
  if (kind === "user") return "用户";
  if (kind === "assistant") return "助手";
  return "通知";
}

function turnKindIcon(kind: TurnKind): IconName {
  if (kind === "error") return "x";
  if (kind === "status") return "check";
  if (kind === "activity" || kind === "tool") return "terminal";
  return "sparkles";
}

function safeMarkdownUrl(url: string): string {
  try {
    return new URL(url).protocol === "https:" ? url : "";
  } catch {
    return "";
  }
}

export function MarkdownBody({ body }: { body: string }) {
  return (
    <div className="markdown-body">
      <ReactMarkdown
        remarkPlugins={[remarkGfm]}
        skipHtml
        urlTransform={safeMarkdownUrl}
        components={{
          a: ({ href, children }) => href?.startsWith("https://")
            ? <a href={href} target="_blank" rel="noopener noreferrer">{children}</a>
            : <span>{children}</span>,
        }}
      >{body}</ReactMarkdown>
    </div>
  );
}

function EmptyTask({ onSuggestion }: { onSuggestion(prompt: string): void }) {
  return (
    <div className="empty-task">
      <div className="empty-logo"><Icon name="sparkles" /></div>
      <h1>从一个任务开始</h1>
      <p>让 Pi 帮你阅读代码、修改文件、运行命令，或者处理一个完整的工作目标。</p>
      <div className="suggestion-grid">
        <button onClick={() => onSuggestion("请解释当前项目的代码结构与关键模块。") }><Icon name="code" /><span><strong>理解项目</strong><small>解释当前代码结构与关键模块</small></span></button>
        <button onClick={() => onSuggestion("请定位当前问题，完成修复并运行相关测试。") }><Icon name="terminal" /><span><strong>修复问题</strong><small>定位错误并完成测试验证</small></span></button>
        <button onClick={() => onSuggestion("请为当前项目生成一份清晰的中文实现说明。") }><Icon name="file" /><span><strong>生成文档</strong><small>整理清晰的中文实现说明</small></span></button>
      </div>
    </div>
  );
}

function interactionTitle(interaction: PendingInteraction): string {
  if (interaction.title) return interaction.title;
  if (interaction.kind === "confirm") return "需要确认";
  if (interaction.kind === "select") return "请选择一项";
  if (interaction.kind === "editor") return "请编辑内容";
  if (interaction.kind === "input") return "需要你的输入";
  return "需要处理的 Pi 请求";
}

function InteractionCard({
  taskId,
  interaction,
  onRespond,
}: {
  taskId: string;
  interaction: PendingInteraction;
  onRespond(taskId: string, interactionId: string, value: InteractionResponse): Promise<void>;
}) {
  const [selectedIndex, setSelectedIndex] = useState<number | null>(
    interaction.defaultIndex ?? (interaction.options.length > 0 ? 0 : null),
  );
  const [value, setValue] = useState(interaction.defaultValue ?? "");
  const [busy, setBusy] = useState(false);
  const [submitted, setSubmitted] = useState(false);
  const [notice, setNotice] = useState<UserFacingError | null>(null);
  const optionRefs = useRef<Array<HTMLButtonElement | null>>([]);
  const titleId = `interaction-title-${interaction.id.replace(/[^a-zA-Z0-9_-]/g, "-")}`;

  function moveSelection(event: ReactKeyboardEvent<HTMLDivElement>) {
    if (busy || submitted || interaction.options.length === 0) return;
    let nextIndex: number | null = null;
    const currentIndex = selectedIndex ?? 0;
    if (event.key === "ArrowRight" || event.key === "ArrowDown") nextIndex = (currentIndex + 1) % interaction.options.length;
    if (event.key === "ArrowLeft" || event.key === "ArrowUp") nextIndex = (currentIndex - 1 + interaction.options.length) % interaction.options.length;
    if (event.key === "Home") nextIndex = 0;
    if (event.key === "End") nextIndex = interaction.options.length - 1;
    if (nextIndex === null) return;
    event.preventDefault();
    setSelectedIndex(nextIndex);
    optionRefs.current[nextIndex]?.focus();
  }

  async function respond(response: InteractionResponse) {
    if (busy || submitted || !interaction.requiresResponse) return;
    setBusy(true);
    setNotice(null);
    try {
      await onRespond(taskId, interaction.id, response);
      setSubmitted(true);
    } catch (error) {
      setNotice(toUserFacingError(error, "提交响应失败，请重试。"));
    } finally {
      setBusy(false);
    }
  }

  return (
    <article className="turn turn-interaction" aria-labelledby={titleId} aria-busy={busy}>
      <div className="timeline-node is-attention"><Icon name="check" /></div>
      <div className="interaction-card">
        <header>
          <div><span className="interaction-eyebrow">Pi 正在等待</span><strong id={titleId}>{interactionTitle(interaction)}</strong></div>
          <span className="interaction-state">需要操作</span>
        </header>
        {interaction.message && <p className="interaction-message">{sanitizeToolText(interaction.message)}</p>}

        {interaction.kind === "select" && (
          <div className="interaction-options" role="radiogroup" aria-label={interactionTitle(interaction)} onKeyDown={moveSelection}>
            {interaction.options.map((option, index) => (
              <button
                key={`${interaction.id}-${index}`}
                ref={(element) => { optionRefs.current[index] = element; }}
                type="button"
                role="radio"
                aria-checked={selectedIndex === index}
                tabIndex={selectedIndex === index ? 0 : -1}
                disabled={busy || submitted}
                onClick={() => setSelectedIndex(index)}
              ><span className="interaction-radio" />{option}</button>
            ))}
          </div>
        )}

        {(interaction.kind === "input" || interaction.kind === "editor") && (
          <label className="interaction-input">
            <span>{interaction.kind === "editor" ? "编辑内容" : "输入内容"}</span>
            {interaction.kind === "editor" ? (
              <textarea value={value} disabled={busy || submitted} onChange={(event) => setValue(event.target.value)} rows={5} />
            ) : (
              <input value={value} disabled={busy || submitted} onChange={(event) => setValue(event.target.value)} />
            )}
          </label>
        )}

        {interaction.kind === "unknown" && (
          <div className="interaction-unsupported" role="status">当前版本无法处理这项 Pi 请求，请升级 PAD 后重试。</div>
        )}

        {notice && <div className="interaction-error" role="alert">
          <span>{notice.message}</span>
          {notice.diagnostic && <details><summary>诊断信息</summary><code>{notice.diagnostic}</code></details>}
        </div>}
        {submitted && <div className="interaction-submitted" role="status">已提交，Pi 正在继续执行。</div>}

        {interaction.requiresResponse && !submitted && <footer className="interaction-actions">
          {interaction.kind === "confirm" && <>
            <button type="button" disabled={busy} onClick={() => void respond(false)}>拒绝</button>
            <button type="button" className="is-primary" disabled={busy} onClick={() => void respond(true)}>确认</button>
          </>}
          {interaction.kind === "select" && (
            <button type="button" className="is-primary" disabled={busy || selectedIndex === null} onClick={() => selectedIndex !== null && void respond(selectedIndex)}>提交选择</button>
          )}
          {(interaction.kind === "input" || interaction.kind === "editor") && (
            <button type="button" className="is-primary" disabled={busy} onClick={() => void respond(value)}>提交</button>
          )}
          {busy && <span role="status">正在提交…</span>}
        </footer>}
      </div>
    </article>
  );
}

function Composer({
  task,
  activeAccount,
  text,
  onTextChange,
  fullAccess,
  onFullAccessChange,
  onChooseAttachments,
  onSend,
  onStop,
  interactionPending,
}: Pick<TaskViewProps, "task" | "activeAccount" | "fullAccess" | "onFullAccessChange" | "onChooseAttachments" | "onSend" | "onStop"> & {
  text: string;
  onTextChange(value: string): void;
  interactionPending: boolean;
}) {
  const [sending, setSending] = useState(false);
  const [pickingAttachments, setPickingAttachments] = useState(false);
  const [attachmentPaths, setAttachmentPaths] = useState<string[]>([]);
  const [composerNotice, setComposerNotice] = useState<UserFacingError | null>(null);
  const [modelPickerOpen, setModelPickerOpen] = useState(false);
  const [provider, setProvider] = useState(() => accountProvider(activeAccount));
  const [model, setModel] = useState(() => accountModel(activeAccount));
  const [thinkingLevel, setThinkingLevel] = useState<ThinkingLevel>("default");
  const [fastMode, setFastMode] = useState(true);
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const modelPickerButtonRef = useRef<HTMLButtonElement>(null);
  const modelPickerRef = useRef<HTMLDivElement>(null);
  const modelCatalog = activeAccount?.modelCatalog;

  useEffect(() => {
    setAttachmentPaths([]);
    setComposerNotice(null);
    setThinkingLevel("default");
    setModelPickerOpen(false);
  }, [task?.id]);

  useEffect(() => {
    setProvider(accountProvider(activeAccount));
    setModel(accountModel(activeAccount));
    setModelPickerOpen(false);
  }, [activeAccount?.id]);

  useEffect(() => {
    const textarea = textareaRef.current;
    if (!textarea) return;
    textarea.style.height = "0px";
    textarea.style.height = `${Math.min(Math.max(textarea.scrollHeight, 28), 164)}px`;
  }, [text]);

  useEffect(() => {
    if (!modelPickerOpen) return;
    modelPickerRef.current?.querySelector<HTMLInputElement>("input")?.focus();
    const closeOnEscape = (event: KeyboardEvent) => {
      if (event.key !== "Escape") return;
      event.preventDefault();
      setModelPickerOpen(false);
      modelPickerButtonRef.current?.focus();
    };
    const closeOnOutsidePointer = (event: PointerEvent) => {
      const target = event.target;
      if (!(target instanceof Node)) return;
      if (modelPickerRef.current?.contains(target) || modelPickerButtonRef.current?.contains(target)) return;
      setModelPickerOpen(false);
    };
    document.addEventListener("keydown", closeOnEscape);
    document.addEventListener("pointerdown", closeOnOutsidePointer);
    return () => {
      document.removeEventListener("keydown", closeOnEscape);
      document.removeEventListener("pointerdown", closeOnOutsidePointer);
    };
  }, [modelPickerOpen]);

  // Match native Pi: after an error the editor remains available and the next
  // prompt is sent directly through the same session. PAD does not invent a
  // separate retry workflow or replay the user's previous prompt.
  const action = task?.status === "running" ? "stop" : "send";
  const sendBlockedByInteraction = action === "send" && interactionPending;

  async function chooseAttachments() {
    if (pickingAttachments || attachmentPaths.length >= 20) return;
    setPickingAttachments(true);
    setComposerNotice(null);
    try {
      const selected = await onChooseAttachments();
      const next = mergeAttachmentPaths(attachmentPaths, selected);
      setAttachmentPaths(next.paths);
      if (next.rejected > 0) {
        setComposerNotice({ message: next.limitReached ? "最多只能添加 20 个附件。" : "已忽略不是绝对路径的附件。" });
      }
    } catch (error) {
      setComposerNotice(toUserFacingError(error, "无法选择附件，请重试。"));
    } finally {
      setPickingAttachments(false);
    }
  }

  async function submit() {
    if (sending || sendBlockedByInteraction) return;
    if (action === "stop") {
      setSending(true);
      try { await onStop(); } finally { setSending(false); }
      return;
    }
    const value = text.trim();
    if (!value) return;
    if (!!provider.trim() !== !!model.trim()) {
      setComposerNotice({ message: "请同时填写模型提供商和模型名称。" });
      setModelPickerOpen(true);
      return;
    }
    setSending(true);
    setComposerNotice(null);
    try {
      await onSend({
        text: value,
        attachmentPaths,
        provider: provider.trim(),
        model: model.trim(),
        thinkingLevel,
        fastMode,
      });
      onTextChange("");
      setAttachmentPaths([]);
    } catch {
      // App owns the localized error banner. Draft and explicitly selected
      // attachments stay intact so a rejected transaction can be retried.
    } finally {
      setSending(false);
    }
  }

  return (
    <div className="composer-wrap" data-thread-scroll-footer="true">
      <div className="composer">
        {attachmentPaths.length > 0 && <div className="composer-attachments" aria-label="已选择附件">
          {attachmentPaths.map((path) => (
            <span className="attachment-chip" key={path} title={path}>
              <Icon name="file" />
              <span>{attachmentName(path)}</span>
              <button type="button" onClick={() => setAttachmentPaths((current) => current.filter((candidate) => candidate !== path))} aria-label={`移除附件 ${path}`}><Icon name="x" /></button>
            </span>
          ))}
        </div>}
        {composerNotice && <div className="composer-notice" role="alert">
          <span>{composerNotice.message}</span>
          {composerNotice.diagnostic && <details><summary>诊断信息</summary><code>{composerNotice.diagnostic}</code></details>}
        </div>}
        <textarea
          ref={textareaRef}
          value={text}
          disabled={action !== "send" || interactionPending}
          onChange={(event) => onTextChange(event.target.value)}
          onKeyDown={(event) => {
            if (event.nativeEvent.isComposing || event.keyCode === 229) return;
            if (event.key === "Enter" && !event.shiftKey) {
              event.preventDefault();
              void submit();
            }
          }}
          placeholder={interactionPending ? "请先完成上方交互" : "向 Pi 描述一个任务"}
          aria-label="任务输入"
          rows={1}
        />
        <div className="composer-footer">
          <div className="composer-tools">
            <button
              type="button"
              className="icon-button"
              aria-label="添加附件"
              title={attachmentPaths.length >= 20 ? "最多 20 个附件" : "添加附件"}
              disabled={action !== "send" || interactionPending || sending || pickingAttachments || attachmentPaths.length >= 20}
              onClick={() => void chooseAttachments()}
            ><Icon name="attachment" /></button>
            <div className="composer-picker-wrap">
              <button
                ref={modelPickerButtonRef}
                type="button"
                className="composer-select"
                aria-label={`选择 Pi 模型，当前 ${modelDisplayLabel(provider, model, activeAccount)}`}
                aria-haspopup="dialog"
                aria-expanded={modelPickerOpen}
                aria-controls="composer-model-picker"
                disabled={action !== "send" || interactionPending || sending}
                onClick={() => setModelPickerOpen((value) => !value)}
              ><span>{modelDisplayLabel(provider, model, activeAccount)}</span><Icon name="chevron-down" /></button>
              {modelPickerOpen && <div ref={modelPickerRef} id="composer-model-picker" className="composer-model-picker" role="dialog" aria-label="选择 Pi 模型">
                <ModelCatalogOptions catalogModels={modelCatalog?.models ?? []} provider={provider} model={model} onSelect={(nextProvider, nextModel) => {
                  setProvider(nextProvider);
                  setModel(nextModel);
                }} />
                <label><span>模型提供商</span><input value={provider} onChange={(event) => setProvider(event.target.value)} placeholder="例如 openai" /></label>
                <label><span>模型名称</span><input value={model} onChange={(event) => setModel(event.target.value)} placeholder="例如 gpt-5.4" /></label>
                <button type="button" onClick={() => {
                  setModelPickerOpen(false);
                  modelPickerButtonRef.current?.focus();
                }}>完成</button>
              </div>}
            </div>
            <label className="composer-thinking-picker">
              <span className="visually-hidden">推理强度</span>
              <select aria-label="推理强度" value={thinkingLevel} disabled={action !== "send" || interactionPending || sending} onChange={(event) => setThinkingLevel(event.target.value as ThinkingLevel)}>
                <option value="default">默认强度</option>
                <option value="off">关闭</option>
                <option value="minimal">极低</option>
                <option value="low">低</option>
                <option value="medium">中</option>
                <option value="high">高</option>
                <option value="xhigh">超高</option>
                <option value="max">最高</option>
              </select>
              <Icon name="chevron-down" />
            </label>
          </div>
          <div className="composer-actions">
            <label className="full-access-toggle" title="使用同一模型与推理强度，请求 OpenAI Codex Fast 服务等级">
              <input type="checkbox" checked={fastMode} onChange={(event) => setFastMode(event.target.checked)} />
              <span>快速模式</span>
            </label>
            <label className="full-access-toggle" title="普通操作可自动执行；PAD、Pi、Codex、ChatGPT 私有区域、系统保护区域及 macOS 隐私权限（TCC）仍不会自动放行">
              <input type="checkbox" checked={fullAccess} onChange={(event) => onFullAccessChange(event.target.checked)} />
              <span>完全访问</span>
            </label>
            <button
              className={`send-button action-${action}`}
              disabled={sendBlockedByInteraction || (action === "send" && !text.trim()) || sending}
              onClick={() => void submit()}
              aria-label={action === "stop" ? "停止任务" : "发送"}
            >
              <Icon name={action === "stop" ? "x" : "send"} />
            </button>
          </div>
        </div>
      </div>
      <p className="composer-disclaimer">Pi 可能会出错，请检查重要修改。PAD 数据与 Codex 会话相互独立。</p>
    </div>
  );
}

function accountProvider(account: AccountSummary | null): string {
  return account?.selectedProvider && account.selectedModel ? account.selectedProvider : "";
}

function accountModel(account: AccountSummary | null): string {
  return account?.selectedProvider && account.selectedModel ? account.selectedModel : "";
}

function ModelCatalogOptions({
  catalogModels,
  provider,
  model,
  onSelect,
}: {
  catalogModels: ModelCatalogModel[];
  provider: string;
  model: string;
  onSelect(nextProvider: string, nextModel: string): void;
}) {
  if (catalogModels.length === 0) {
    return <p className="model-catalog-empty" role="status">未读取到可用模型，将使用自动选择。</p>;
  }

  const groups = new Map<string, ModelCatalogModel[]>();
  catalogModels.forEach((item) => {
    const group = groups.get(item.provider) ?? [];
    group.push(item);
    groups.set(item.provider, group);
  });
  return (
    <div className="model-catalog-options" role="listbox" aria-label="可用模型">
      {[...groups.entries()].map(([groupProvider, models]) => (
        <div className="model-catalog-group" key={groupProvider}>
          <span className="model-catalog-provider">{groupProvider}</span>
          {models.map((item) => {
            const selected = provider === item.provider && model === item.id;
            return (
              <button
                type="button"
                role="option"
                aria-selected={selected}
                className={selected ? "is-selected" : ""}
                key={`${item.provider}/${item.id}`}
                onClick={() => onSelect(item.provider, item.id)}
              >
                <span>{item.name || item.id}</span>
                {item.name !== item.id && <small>{item.id}</small>}
              </button>
            );
          })}
        </div>
      ))}
    </div>
  );
}

function modelDisplayLabel(provider: string, model: string, account: AccountSummary | null): string {
  if (provider.trim() && model.trim()) {
    const displayName = account?.modelCatalog.models.find((item) => item.provider === provider.trim() && item.id === model.trim())?.name;
    return `${provider.trim()} / ${displayName || model.trim()}`;
  }
  if (provider.trim()) return `${provider.trim()} / 默认模型`;
  const accountDefaultProvider = account?.selectedProvider ?? account?.authenticatedProviders[0];
  if (accountDefaultProvider) return `${accountDefaultProvider} / Pi 默认模型`;
  return account ? "Pi 默认模型" : "选择模型";
}

function validAttachmentPath(value: string): string | null {
  const path = value.trim();
  return path.startsWith("/") && !path.includes("\n") && !path.includes("\r") && !path.includes("\0") ? path : null;
}

function mergeAttachmentPaths(current: readonly string[], selected: readonly string[]): {
  paths: string[];
  rejected: number;
  limitReached: boolean;
} {
  const paths = [...current];
  let rejected = 0;
  let limitReached = false;
  for (const value of selected) {
    const path = validAttachmentPath(value);
    if (!path) {
      rejected += 1;
      continue;
    }
    if (paths.includes(path)) continue;
    if (paths.length >= 20) {
      rejected += 1;
      limitReached = true;
      continue;
    }
    paths.push(path);
  }
  return { paths, rejected, limitReached };
}

function attachmentName(path: string): string {
  return path.split("/").filter(Boolean).at(-1) ?? path;
}

export function TaskView({
  task,
  project,
  activeAccount,
  turns,
  interactions,
  fullAccess,
  rightPanelOpen,
  bottomPanelOpen,
  onFullAccessChange,
  onRightPanelToggle,
  onBottomPanelToggle,
  onChooseAttachments,
  onSend,
  onStop,
  onRespondInteraction,
  onUpdateTask,
}: TaskViewProps) {
  const contentRef = useRef<HTMLDivElement>(null);
  const [draft, setDraft] = useState("");
  const [moreOpen, setMoreOpen] = useState(false);
  const [updatingTask, setUpdatingTask] = useState(false);

  useEffect(() => {
    setDraft("");
  }, [task?.id]);

  const latestTurn = turns.at(-1);
  useEffect(() => {
    contentRef.current?.scrollTo({ top: contentRef.current.scrollHeight, behavior: "smooth" });
  }, [interactions.length, latestTurn?.body, latestTurn?.id, latestTurn?.state, turns.length]);

  async function updateTask(patch: { pinned?: boolean; archived?: boolean; unread?: boolean }) {
    if (!task || updatingTask) return;
    setUpdatingTask(true);
    setMoreOpen(false);
    try {
      await onUpdateTask(patch);
    } finally {
      setUpdatingTask(false);
    }
  }

  return (
    <section className="task-pane" data-focus-domain="main" aria-label="当前任务">
      <header className="task-toolbar" role="toolbar" aria-label="任务工具栏">
        <div className="task-heading">
          <div className="task-heading-copy">
            <strong>{task?.title ?? "新任务"}</strong>
            <span className={`task-status status-${task?.status ?? "idle"}`}>{taskStatusLabel(task?.status ?? "idle")}</span>
          </div>
          <span className="task-project">{project?.name ?? "未归类"}</span>
        </div>
        <div className="task-toolbar-actions">
          <button className={rightPanelOpen ? "is-active" : ""} onClick={onRightPanelToggle} aria-label="切换右侧面板" aria-pressed={rightPanelOpen}><Icon name="panel-right" /></button>
          <button className={bottomPanelOpen ? "is-active" : ""} onClick={onBottomPanelToggle} aria-label="切换终端" aria-pressed={bottomPanelOpen}><Icon name="panel-bottom" /></button>
          <div className="task-more-wrap">
            <button
              className={moreOpen ? "is-active" : ""}
              disabled={!task || updatingTask}
              aria-label="更多任务操作"
              aria-haspopup="menu"
              aria-expanded={moreOpen}
              onClick={() => setMoreOpen((value) => !value)}
            ><Icon name="more" /></button>
            {moreOpen && task && <div className="task-more-menu" role="menu" aria-label="任务操作">
              <button role="menuitem" onClick={() => void updateTask({ pinned: !task.pinned })}>{task.pinned ? "取消固定" : "固定任务"}</button>
              <button role="menuitem" onClick={() => void updateTask({ unread: true })}>标为未读</button>
              <button className={task.archived ? "" : "is-destructive"} role="menuitem" onClick={() => void updateTask({ archived: !task.archived })}>{task.archived ? "恢复任务" : "归档任务"}</button>
            </div>}
          </div>
        </div>
      </header>
      <div className="thread-scroll" ref={contentRef} role="region" aria-label="任务时间线">
        <main className="thread-content" data-thread-max-width="768">
          {turns.length === 0 && interactions.length === 0 ? <EmptyTask onSuggestion={setDraft} /> : <>
            {turns.map((turn, index) => <Turn key={turn.id} turn={turn} last={index === turns.length - 1 && interactions.length === 0} />)}
            {task && interactions.map((interaction) => (
              <InteractionCard key={interaction.id} taskId={task.id} interaction={interaction} onRespond={onRespondInteraction} />
            ))}
          </>}
        </main>
        <Composer task={task} activeAccount={activeAccount} text={draft} onTextChange={setDraft} fullAccess={fullAccess} onFullAccessChange={onFullAccessChange} onChooseAttachments={onChooseAttachments} onSend={onSend} onStop={onStop} interactionPending={interactions.some((interaction) => interaction.requiresResponse)} />
      </div>
    </section>
  );
}

interface InspectorArtifact extends TurnArtifact {
  selectionId: string;
  turnId: string;
  turnTitle: string;
}

function safeInspectorArtifact(turn: TurnEntry, artifact: TurnArtifact, index: number): InspectorArtifact | null {
  const path = sanitizeToolText(artifact.path).trim();
  if (!path || path.includes("<PAD 私有路径已隐藏>") || path.includes("<已隐藏>") || path.includes("\n")) return null;
  const previousPath = artifact.previousPath ? sanitizeToolText(artifact.previousPath).trim() : undefined;
  const diff = artifact.diff ? sanitizeToolText(artifact.diff) : undefined;
  return {
    ...artifact,
    path,
    ...(previousPath ? { previousPath } : {}),
    ...(diff ? { diff } : {}),
    selectionId: `${turn.id}:${artifact.id}:${index}`,
    turnId: turn.id,
    turnTitle: turn.title ?? turnKindLabel(turn.kind),
  };
}

function isInspectorActivity(kind: TurnKind): boolean {
  return ["tool", "reasoning", "error", "status", "activity"].includes(kind);
}

function isChangeArtifact(artifact: TurnArtifact): boolean {
  return artifact.kind === "change"
    || artifact.diff !== undefined
    || ["created", "modified", "deleted", "renamed"].includes(artifact.operation);
}

function inspectorData(turns: TurnEntry[]): {
  activities: TurnEntry[];
  files: InspectorArtifact[];
  changes: InspectorArtifact[];
} {
  const activities = turns.filter((turn) => isInspectorActivity(turn.kind));
  const files = new Map<string, InspectorArtifact>();
  const changes: InspectorArtifact[] = [];
  for (const turn of turns) {
    for (const [index, rawArtifact] of (turn.artifacts ?? []).entries()) {
      const artifact = safeInspectorArtifact(turn, rawArtifact, index);
      if (!artifact) continue;
      files.set(artifact.path, artifact);
      if (isChangeArtifact(artifact)) changes.push(artifact);
    }
  }
  return { activities, files: [...files.values()], changes };
}

function artifactOperationLabel(operation: TurnArtifactOperation): string {
  if (operation === "read") return "已读取";
  if (operation === "created") return "已创建";
  if (operation === "modified") return "已修改";
  if (operation === "deleted") return "已删除";
  if (operation === "renamed") return "已重命名";
  return "已记录";
}

export function RightPanel({
  task,
  project,
  turns,
  onClose,
}: {
  task: TaskSummary | null;
  project: ProjectSummary | null;
  turns: TurnEntry[];
  onClose(): void;
}) {
  const [tab, setTab] = useState<"activity" | "files" | "changes">("activity");
  const [selectedArtifactId, setSelectedArtifactId] = useState<string | null>(null);
  const inspector = useMemo(() => inspectorData(turns), [turns]);
  const selectedFile = inspector.files.find((file) => file.selectionId === selectedArtifactId) ?? inspector.files[0] ?? null;
  const selectedChange = inspector.changes.find((change) => change.selectionId === selectedArtifactId) ?? inspector.changes[0] ?? null;

  function moveArtifactSelection(event: ReactKeyboardEvent<HTMLDivElement>) {
    const options = [...event.currentTarget.querySelectorAll<HTMLButtonElement>('[role="option"]')];
    if (options.length === 0) return;
    const focusedIndex = options.indexOf(document.activeElement as HTMLButtonElement);
    const currentIndex = focusedIndex >= 0
      ? focusedIndex
      : Math.max(0, options.findIndex((option) => option.getAttribute("aria-selected") === "true"));
    let nextIndex: number | null = null;
    if (event.key === "ArrowDown" || event.key === "ArrowRight") nextIndex = (currentIndex + 1) % options.length;
    if (event.key === "ArrowUp" || event.key === "ArrowLeft") nextIndex = (currentIndex - 1 + options.length) % options.length;
    if (event.key === "Home") nextIndex = 0;
    if (event.key === "End") nextIndex = options.length - 1;
    if (nextIndex === null) return;
    event.preventDefault();
    options[nextIndex]?.focus();
    options[nextIndex]?.click();
  }

  return (
    <aside
      className="right-panel"
      aria-label="任务详情面板"
      data-focus-domain="right"
      tabIndex={-1}
      onKeyDown={(event) => {
        if (event.key !== "Escape") return;
        event.preventDefault();
        onClose();
      }}
    >
      <header className="panel-toolbar">
        <div
          className="panel-tabs"
          role="tablist"
          aria-label="任务详情分类"
          onKeyDown={(event) => {
            const tabs = [...event.currentTarget.querySelectorAll<HTMLButtonElement>('[role="tab"]')];
            const current = tabs.indexOf(document.activeElement as HTMLButtonElement);
            if (current < 0) return;
            let nextIndex: number | null = null;
            if (event.key === "ArrowRight") nextIndex = (current + 1) % tabs.length;
            if (event.key === "ArrowLeft") nextIndex = (current - 1 + tabs.length) % tabs.length;
            if (event.key === "Home") nextIndex = 0;
            if (event.key === "End") nextIndex = tabs.length - 1;
            if (nextIndex === null) return;
            event.preventDefault();
            const next = tabs[nextIndex];
            next?.focus();
            next?.click();
          }}
        >
          <button id="right-panel-tab-activity" role="tab" aria-controls="right-panel-content" aria-selected={tab === "activity"} tabIndex={tab === "activity" ? 0 : -1} onClick={() => setTab("activity")}>活动</button>
          <button id="right-panel-tab-files" role="tab" aria-controls="right-panel-content" aria-selected={tab === "files"} tabIndex={tab === "files" ? 0 : -1} onClick={() => setTab("files")}>文件</button>
          <button id="right-panel-tab-changes" role="tab" aria-controls="right-panel-content" aria-selected={tab === "changes"} tabIndex={tab === "changes" ? 0 : -1} onClick={() => setTab("changes")}>更改</button>
        </div>
        <button className="panel-close" onClick={onClose} aria-label="关闭右侧面板"><Icon name="x" /></button>
      </header>
      <div
        className="panel-content"
        id="right-panel-content"
        role="tabpanel"
        aria-labelledby={`right-panel-tab-${tab}`}
        tabIndex={0}
      >
        {tab === "activity" && (inspector.activities.length > 0 ? (
          <div className="activity-list">
            {inspector.activities.map((turn) => (
              <article key={turn.id} className={`activity-row activity-${turn.kind}`}>
                <Icon name={turnKindIcon(turn.kind)} />
                <div><strong>{turn.title ?? turnKindLabel(turn.kind)}</strong><p>{sanitizeToolText(turn.body) || "没有可显示的活动详情。"}</p></div>
                <span className={`activity-kind ${turn.state ? `state-${turn.state}` : ""}`}>{turn.state ? toolStateLabel(turn.state) : turnKindLabel(turn.kind)}</span>
              </article>
            ))}
          </div>
        ) : <div className="panel-placeholder"><Icon name="terminal" /><strong>{task ? "暂无结构化活动" : "尚未选择任务"}</strong><p>工具、推理、状态和错误事件出现后会按时间显示在这里。</p></div>)}
        {tab === "files" && (inspector.files.length > 0 ? (
          <>
            <div className="inspector-file-list" role="listbox" aria-label="结构化文件" onKeyDown={moveArtifactSelection}>
              {inspector.files.map((file) => (
                <button
                  className="inspector-file-row"
                  role="option"
                  aria-selected={selectedFile?.selectionId === file.selectionId}
                  tabIndex={selectedFile?.selectionId === file.selectionId ? 0 : -1}
                  key={file.selectionId}
                  onClick={() => setSelectedArtifactId(file.selectionId)}
                >
                  <Icon name="file" /><div><strong>{file.path}</strong><small>{artifactOperationLabel(file.operation)} · 来自：{file.turnTitle}</small></div>
                </button>
              ))}
            </div>
            {selectedFile && <section className="inspector-selection" aria-label="已选择文件">
              <span>{artifactOperationLabel(selectedFile.operation)}</span>
              <strong>{selectedFile.path}</strong>
              {selectedFile.previousPath && <small>原路径：{selectedFile.previousPath}</small>}
              <small>结构化来源：{selectedFile.turnTitle}</small>
            </section>}
          </>
        ) : <div className="panel-placeholder"><Icon name="folder" /><strong>暂无结构化文件</strong><p>{project ? `${project.name} 的历史消息尚未提供文件记录。` : "请先选择任务并运行文件工具。"}</p></div>)}
        {tab === "changes" && (inspector.changes.length > 0 ? (
          <div className="inspector-diff-list">
            <div className="inspector-change-picker" role="listbox" aria-label="结构化更改文件" onKeyDown={moveArtifactSelection}>
              {inspector.changes.map((change) => (
                <button
                  type="button"
                  role="option"
                  aria-selected={selectedChange?.selectionId === change.selectionId}
                  tabIndex={selectedChange?.selectionId === change.selectionId ? 0 : -1}
                  key={change.selectionId}
                  onClick={() => setSelectedArtifactId(change.selectionId)}
                >
                  <Icon name="file" /><span>{change.path}</span><small>{artifactOperationLabel(change.operation)}</small>
                </button>
              ))}
            </div>
            {selectedChange && <article className="inspector-diff">
              <header><Icon name="code" /><strong>{selectedChange.path}</strong><small>{selectedChange.turnTitle}</small></header>
              {selectedChange.diff
                ? <pre aria-label={`${selectedChange.path} 的结构化差异`}>{selectedChange.diff}</pre>
                : <p className="inspector-no-diff">后端报告了这项结构化更改，但没有提供可显示的差异内容。</p>}
            </article>}
          </div>
        ) : <div className="panel-placeholder"><Icon name="code" /><strong>暂无结构化更改</strong><p>只有历史记录中的元数据或结构化记录明确提供更改时才会显示；看起来像差异内容的普通正文不会被解析。</p></div>)}
      </div>
    </aside>
  );
}

interface BottomPanelProps {
  task: TaskSummary | null;
  onClose(): void;
  onOpenTerminal(taskId: string, size: TerminalSize): Promise<TerminalPane>;
  onTerminalInput(paneId: string, data: string): Promise<void>;
  onTerminalResize(paneId: string, size: TerminalSize): Promise<void>;
  onTerminalSnapshot(paneId: string): Promise<TerminalSnapshot>;
  onTerminalClose(paneId: string): Promise<void>;
}

const terminalPanelMinHeight = 180;
const terminalPanelMaxHeight = 480;

function clampTerminalPanelHeight(height: number): number {
  return Math.min(terminalPanelMaxHeight, Math.max(terminalPanelMinHeight, Math.round(height)));
}

function terminalStatusLabel(status: TerminalSnapshot["status"] | "idle"): string {
  if (status === "opening") return "正在启动";
  if (status === "running") return "运行中";
  if (status === "exited") return "已退出";
  if (status === "failed") return "运行失败";
  return "未连接";
}

function terminalExitLabel(snapshot: TerminalSnapshot | null): string | null {
  if (!snapshot?.exit) return null;
  if (snapshot.exit.signaled) return "进程由信号终止";
  if (typeof snapshot.exit.code === "number") return `退出码 ${snapshot.exit.code}`;
  return "退出码未知";
}

function estimateTerminalSize(element: HTMLElement | null): TerminalSize {
  const width = element?.clientWidth || 800;
  const height = element?.clientHeight || 170;
  return {
    columns: Math.min(240, Math.max(20, Math.floor((width - 24) / 7.2))),
    rows: Math.min(80, Math.max(4, Math.floor((height - 18) / 17))),
  };
}

function terminalLines(snapshot: TerminalSnapshot | null): string {
  if (!snapshot) return "";
  const lines = snapshot.lines.slice(-80).map(sanitizeToolText);
  const cursor = snapshot.cursor;
  if (snapshot.isOpen && cursor && cursor.row >= 0 && cursor.row < lines.length) {
    const characters = Array.from(lines[cursor.row] ?? "");
    const column = Math.min(Math.max(0, cursor.column), characters.length);
    characters.splice(column, 0, cursor.shape === "underline" ? "▁" : cursor.shape === "beam" ? "▏" : "▌");
    lines[cursor.row] = characters.join("");
  }
  return lines.join("\n");
}

function terminalKeyData(key: string, ctrlKey: boolean, applicationCursor: boolean): string | null {
  if (ctrlKey && key.toLowerCase() === "c") return "\u0003";
  if (key === "Enter") return "\r";
  if (key === "Backspace") return "\u007f";
  if (key === "Tab") return "\t";
  const prefix = applicationCursor ? "\u001bO" : "\u001b[";
  if (key === "ArrowUp") return `${prefix}A`;
  if (key === "ArrowDown") return `${prefix}B`;
  if (key === "ArrowRight") return `${prefix}C`;
  if (key === "ArrowLeft") return `${prefix}D`;
  return null;
}

export function BottomPanel({
  task,
  onClose,
  onOpenTerminal,
  onTerminalInput,
  onTerminalResize,
  onTerminalSnapshot,
  onTerminalClose,
}: BottomPanelProps) {
  const viewportRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLTextAreaElement>(null);
  const paneIdRef = useRef<string | null>(null);
  const closedPaneIdsRef = useRef<Set<string>>(new Set());
  const closingPaneIdsRef = useRef<Map<string, Promise<void>>>(new Map());
  const composingRef = useRef(false);
  const writeQueueRef = useRef<Promise<void>>(Promise.resolve());
  const resizeDragRef = useRef<{ startY: number; startHeight: number } | null>(null);
  const [pane, setPane] = useState<TerminalPane | null>(null);
  const [snapshot, setSnapshot] = useState<TerminalSnapshot | null>(null);
  const [notice, setNotice] = useState<UserFacingError | null>(null);
  const [panelHeight, setPanelHeight] = useState(220);
  const [closingPanel, setClosingPanel] = useState(false);

  function requestPaneClose(paneId: string): Promise<void> {
    if (closedPaneIdsRef.current.has(paneId)) return Promise.resolve();
    const pending = closingPaneIdsRef.current.get(paneId);
    if (pending) return pending;
    const operation = onTerminalClose(paneId)
      .then(() => {
        closedPaneIdsRef.current.add(paneId);
      })
      .finally(() => {
        closingPaneIdsRef.current.delete(paneId);
      });
    closingPaneIdsRef.current.set(paneId, operation);
    return operation;
  }

  async function closeDetachedPane(paneId: string) {
    for (let attempt = 0; attempt < 3; attempt += 1) {
      try {
        await requestPaneClose(paneId);
        return;
      } catch {
        if (attempt < 2) await new Promise<void>((resolve) => window.setTimeout(resolve, 60 * (attempt + 1)));
      }
    }
  }

  useEffect(() => {
    if (!task?.id) return;
    const taskId: string = task.id;
    let active = true;
    let timer: number | undefined;
    let openedPaneId: string | null = null;

    async function poll() {
      timer = undefined;
      if (!openedPaneId || !active) return;
      let continuePolling = true;
      try {
        const next = await onTerminalSnapshot(openedPaneId);
        if (!active) return;
        setSnapshot(next);
        if (next.error) setNotice(toUserFacingError({ code: "terminal_failed", message: next.error }, "终端运行失败，请关闭后重试。"));
        continuePolling = next.status === "opening" || next.status === "running";
      } catch (error) {
        if (active) setNotice(toUserFacingError(error, "无法读取终端输出，请关闭后重试。"));
      } finally {
        if (active && continuePolling) timer = window.setTimeout(() => void poll(), 180);
      }
    }

    async function open() {
      try {
        const opened = await onOpenTerminal(taskId, estimateTerminalSize(viewportRef.current));
        openedPaneId = opened.paneId;
        if (!active) {
          await closeDetachedPane(opened.paneId);
          return;
        }
        paneIdRef.current = opened.paneId;
        setPane(opened);
        void poll();
        inputRef.current?.focus();
      } catch (error) {
        if (active) setNotice(toUserFacingError(error, "无法打开任务终端，请确认本地服务正在运行。"));
      }
    }

    setPane(null);
    setSnapshot(null);
    setNotice(null);
    void open();
    return () => {
      active = false;
      if (timer !== undefined) window.clearTimeout(timer);
      if (openedPaneId) {
        if (paneIdRef.current === openedPaneId) paneIdRef.current = null;
        void closeDetachedPane(openedPaneId);
      }
    };
  }, [task?.id, onOpenTerminal, onTerminalClose, onTerminalSnapshot]);

  useEffect(() => {
    const element = viewportRef.current;
    if (!element || !pane?.paneId || typeof ResizeObserver === "undefined") return;
    let previous = pane.size;
    const observer = new ResizeObserver(() => {
      const next = estimateTerminalSize(element);
      if (next.columns === previous.columns && next.rows === previous.rows) return;
      previous = next;
      void onTerminalResize(pane.paneId, next).catch((error: unknown) => {
        setNotice(toUserFacingError(error, "终端尺寸更新失败。"));
      });
    });
    observer.observe(element);
    return () => observer.disconnect();
  }, [onTerminalResize, pane?.paneId]);

  useEffect(() => {
    const element = viewportRef.current;
    if (element) element.scrollTop = element.scrollHeight;
  }, [snapshot?.revision]);

  function sendInput(data: string) {
    const paneId = paneIdRef.current;
    if (!paneId || !data) return;
    writeQueueRef.current = writeQueueRef.current
      .then(() => onTerminalInput(paneId, data))
      .catch((error: unknown) => {
        setNotice(toUserFacingError(error, "终端输入发送失败，请重试。"));
      });
  }

  async function closePanel() {
    const paneId = paneIdRef.current;
    if (closingPanel) return;
    setClosingPanel(true);
    setNotice(null);
    try {
      if (paneId) await requestPaneClose(paneId);
      if (paneIdRef.current === paneId) paneIdRef.current = null;
      onClose();
    } catch (error) {
      setNotice(toUserFacingError(error, "无法关闭任务终端，请重试。"));
    } finally {
      setClosingPanel(false);
    }
  }

  function startPanelResize(event: ReactPointerEvent<HTMLDivElement>) {
    event.preventDefault();
    resizeDragRef.current = { startY: event.clientY, startHeight: panelHeight };
    event.currentTarget.setPointerCapture?.(event.pointerId);
  }

  function movePanelResize(event: ReactPointerEvent<HTMLDivElement>) {
    const drag = resizeDragRef.current;
    if (!drag) return;
    event.preventDefault();
    setPanelHeight(clampTerminalPanelHeight(drag.startHeight + drag.startY - event.clientY));
  }

  function finishPanelResize(event: ReactPointerEvent<HTMLDivElement>) {
    resizeDragRef.current = null;
    if (event.currentTarget.hasPointerCapture?.(event.pointerId)) event.currentTarget.releasePointerCapture(event.pointerId);
  }

  const status = snapshot?.status ?? pane?.status ?? "idle";
  const output = terminalLines(snapshot);
  const exitLabel = terminalExitLabel(snapshot);
  return (
    <section
      className="bottom-panel"
      style={{ height: panelHeight }}
      aria-label="终端面板"
      data-focus-domain="bottom"
      data-terminal-pane-id={pane?.paneId ?? ""}
      data-terminal-status={status}
      tabIndex={-1}
    >
      <div
        className="bottom-panel-resize"
        role="separator"
        aria-label="调整终端高度"
        aria-orientation="horizontal"
        aria-valuemin={terminalPanelMinHeight}
        aria-valuemax={terminalPanelMaxHeight}
        aria-valuenow={panelHeight}
        aria-valuetext={`${panelHeight} 像素`}
        tabIndex={0}
        onPointerDown={startPanelResize}
        onPointerMove={movePanelResize}
        onPointerUp={finishPanelResize}
        onPointerCancel={finishPanelResize}
        onKeyDown={(event) => {
          const step = event.shiftKey ? 48 : 16;
          if (event.key === "ArrowUp") {
            event.preventDefault();
            setPanelHeight((height) => clampTerminalPanelHeight(height + step));
          } else if (event.key === "ArrowDown") {
            event.preventDefault();
            setPanelHeight((height) => clampTerminalPanelHeight(height - step));
          } else if (event.key === "Home") {
            event.preventDefault();
            setPanelHeight(terminalPanelMinHeight);
          } else if (event.key === "End") {
            event.preventDefault();
            setPanelHeight(terminalPanelMaxHeight);
          }
        }}
      />
      <header className="bottom-toolbar">
        <div><Icon name="terminal" /><strong>任务终端</strong><span role="status">{terminalStatusLabel(status)}</span>{exitLabel && <span className="terminal-exit">{exitLabel}</span>}</div>
        <button disabled={closingPanel} onClick={() => void closePanel()} aria-label="关闭终端"><Icon name="x" /></button>
      </header>
      {!task ? (
        <div className="terminal-empty"><Icon name="terminal" /><strong>请先选择任务</strong><p>终端只连接当前账号内选中的任务。</p></div>
      ) : (
        <div className="terminal-session" onClick={() => inputRef.current?.focus()}>
          <div
            className="terminal-output"
            ref={viewportRef}
            role="log"
            aria-label="终端输出"
            aria-live="polite"
            data-terminal-line-count={snapshot ? Math.min(snapshot.lines.length, 80) : 0}
            tabIndex={0}
          >
            <pre>{output || (status === "opening" ? "正在启动任务终端…" : "终端暂时没有输出。")}</pre>
          </div>
          <textarea
            ref={inputRef}
            className="terminal-input"
            aria-label="终端输入"
            spellCheck={false}
            autoCapitalize="off"
            autoCorrect="off"
            disabled={status !== "running"}
            onCompositionStart={() => { composingRef.current = true; }}
            onCompositionEnd={(event) => {
              composingRef.current = false;
              const value = event.currentTarget.value || event.data;
              sendInput(value);
              event.currentTarget.value = "";
            }}
            onChange={(event) => {
              if (composingRef.current) return;
              sendInput(event.currentTarget.value);
              event.currentTarget.value = "";
            }}
            onPaste={(event) => {
              event.preventDefault();
              const text = event.clipboardData.getData("text");
              sendInput(snapshot?.mode.bracketedPaste ? `\u001b[200~${text}\u001b[201~` : text);
            }}
            onKeyDown={(event) => {
              if (composingRef.current || event.nativeEvent.isComposing) return;
              const data = terminalKeyData(event.key, event.ctrlKey, snapshot?.mode.applicationCursor ?? false);
              if (!data) return;
              event.preventDefault();
              sendInput(data);
            }}
          />
          {notice && <div className="terminal-notice" role="alert">
            <span>{notice.message}</span>
            {notice.diagnostic && <details><summary>诊断信息</summary><code>{notice.diagnostic}</code></details>}
          </div>}
        </div>
      )}
    </section>
  );
}
