import { useEffect, useState } from "react";
import type { KeyboardEvent as ReactKeyboardEvent, ReactNode } from "react";
import type { AccountSummary, AuthSession, AuthType, BackendSummary, RemoteHostStatus, RemotePairing } from "../types";
import { toUserFacingError, type UserFacingError } from "../lib/errors";
import { backendStatusLabel, localizePiAuthOption, localizePiAuthPrompt, permissionModeLabel } from "../lib/labels";
import { Icon, type IconName } from "./Icons";
import { ModalSheet } from "./ModalSheet";
import { RemoteSettingsSection } from "./RemoteSettingsSection";

export type SettingsSection = "general" | "accounts" | "pi" | "remote" | "permissions" | "data" | "about";

interface SettingsViewProps {
  accounts: AccountSummary[];
  backend: BackendSummary;
  theme: "light" | "dark" | "system";
  fullAccess: boolean;
  initialSection?: SettingsSection;
  authSession: AuthSession | null;
  remote: RemoteHostStatus | null;
  onThemeChange(theme: "light" | "dark" | "system"): void;
  onFullAccessChange(value: boolean): void;
  onCreateAccount(name: string, provider?: string): Promise<void>;
  onSwitchAccount(accountId: string): Promise<void>;
  onBeginLogin(accountId: string, provider: string, authType: AuthType): Promise<void>;
  onRefreshLogin(accountId: string): Promise<void>;
  onRespondLogin(accountId: string, value: string): Promise<void>;
  onCancelLogin(accountId: string): Promise<void>;
  onDismissLogin(): void;
  onLogout(accountId: string, provider?: string): Promise<void>;
  onRemoteRefresh(): Promise<void>;
  onRemoteEnabledChange(enabled: boolean): Promise<void>;
  onBeginRemotePairing(): Promise<RemotePairing>;
  onCancelRemotePairing(pairingId: string): Promise<void>;
  onRevokeRemoteDevice(deviceId: string): Promise<void>;
  onBack(): void;
}

const sections: Array<{ id: SettingsSection; label: string; icon: IconName }> = [
  { id: "general", label: "通用", icon: "settings" },
  { id: "accounts", label: "账号", icon: "layout" },
  { id: "pi", label: "Pi 运行时", icon: "sparkles" },
  { id: "remote", label: "远程连接", icon: "layout" },
  { id: "permissions", label: "权限与访问", icon: "check" },
  { id: "data", label: "数据与存储", icon: "archive" },
  { id: "about", label: "关于", icon: "code" },
];

const authCapabilities = ["auth_begin", "auth_status", "auth_respond", "auth_cancel", "logout"];
const customProviderId = "__custom__";
const commonProviders = [
  { id: "openai", label: "OpenAI" },
  { id: "anthropic", label: "Anthropic" },
  { id: "google", label: "Google" },
  { id: "openrouter", label: "OpenRouter" },
] as const;

function ProviderPicker({ value, onChange }: { value: string; onChange(value: string): void }) {
  const selected = commonProviders.some((provider) => provider.id === value) ? value : customProviderId;
  return (
    <>
      <label className="auth-input">
        <span>模型提供商 <small>常用快捷项，也可自定义</small></span>
        <select
          className="settings-select"
          aria-label="模型提供商"
          value={selected}
          onChange={(event) => onChange(event.target.value === customProviderId ? "" : event.target.value)}
        >
          {commonProviders.map((provider) => <option key={provider.id} value={provider.id}>{provider.label}</option>)}
          <option value={customProviderId}>自定义提供商…</option>
        </select>
      </label>
      {selected === customProviderId && (
        <label className="auth-input">
          <span>自定义提供商 ID</span>
          <input
            autoFocus
            aria-label="自定义提供商 ID"
            value={value}
            onChange={(event) => onChange(event.target.value)}
            placeholder="例如：company-gateway"
          />
        </label>
      )}
    </>
  );
}

function SettingsRow({ title, description, children, disabled = false }: { title: string; description?: string; children: ReactNode; disabled?: boolean }) {
  return (
    <div className={`settings-row${disabled ? " is-disabled" : ""}`}>
      <div><strong>{title}</strong>{description && <p>{description}</p>}</div>
      <div className="settings-control">{children}</div>
    </div>
  );
}

function Toggle({ checked, onChange, label, disabled = false }: { checked: boolean; onChange(value: boolean): void; label: string; disabled?: boolean }) {
  return (
    <label className={`switch${disabled ? " is-disabled" : ""}`} aria-label={label}>
      <input type="checkbox" checked={checked} disabled={disabled} onChange={(event) => onChange(event.target.checked)} />
      <span />
    </label>
  );
}

function authenticationLabel(account: AccountSummary): string {
  if (account.authentication === "authenticated") return "已登录";
  if (account.authentication === "partial") return "部分可用";
  if (account.authentication === "missing") return "未登录";
  return "状态未知";
}

function SheetOperationError({ error }: { error: UserFacingError | null }) {
  if (!error) return null;
  return <div className="interaction-error" role="alert">
    <span>{error.message}</span>
    {error.diagnostic && <details><summary>诊断信息</summary><code>{error.diagnostic}</code></details>}
  </div>;
}

function AuthSheet({
  session,
  busy,
  onRefresh,
  onRespond,
  onCancel,
  onDismiss,
}: {
  session: AuthSession;
  busy: boolean;
  onRefresh(): Promise<void>;
  onRespond(value: string): Promise<void>;
  onCancel(): Promise<void>;
  onDismiss(): void;
}) {
  const [value, setValue] = useState("");
  const [selectedOptionId, setSelectedOptionId] = useState(session.options?.[0]?.id ?? "");
  const [operationError, setOperationError] = useState<UserFacingError | null>(null);
  const optionIds = session.options?.map((option) => option.id).join("\u0000") ?? "";
  const terminal = ["authenticated", "failed", "cancelled"].includes(session.phase);
  const authError = session.error
    ? toUserFacingError({ code: "auth_failed", message: session.error }, "登录失败，请重新尝试。")
    : null;

  useEffect(() => {
    setValue("");
    setSelectedOptionId(session.options?.[0]?.id ?? "");
    setOperationError(null);
  }, [session.attemptId, session.promptId, optionIds]);

  async function runOperation(action: () => Promise<void>, fallback: string) {
    setOperationError(null);
    try {
      await action();
    } catch (error) {
      setOperationError(toUserFacingError(error, fallback));
    }
  }

  function handleOptionKeyDown(event: ReactKeyboardEvent<HTMLDivElement>, optionIndex: number) {
    const options = session.options ?? [];
    if (options.length === 0) return;

    let nextIndex: number | null = null;
    if (event.key === "ArrowDown" || event.key === "ArrowRight") {
      nextIndex = (optionIndex + 1) % options.length;
    } else if (event.key === "ArrowUp" || event.key === "ArrowLeft") {
      nextIndex = (optionIndex - 1 + options.length) % options.length;
    } else if (event.key === "Home") {
      nextIndex = 0;
    } else if (event.key === "End") {
      nextIndex = options.length - 1;
    } else if (event.key === " " || event.key === "Enter") {
      event.preventDefault();
      setSelectedOptionId(options[optionIndex]!.id);
      return;
    }

    if (nextIndex === null) return;
    event.preventDefault();
    const option = options[nextIndex]!;
    setSelectedOptionId(option.id);
    const radios = event.currentTarget.parentElement?.querySelectorAll<HTMLElement>('[role="radio"]');
    radios?.[nextIndex]?.focus();
  }

  return (
    <ModalSheet
      labelledBy="auth-sheet-title"
      describedBy="auth-sheet-description"
      busy={busy}
      onDismiss={() => {
        if (terminal) onDismiss();
        else void runOperation(onCancel, "取消登录失败，请稍后重试。");
      }}
    >
        <div className={`auth-sheet-icon phase-${session.phase}`}><Icon name={session.phase === "authenticated" ? "check" : "sparkles"} /></div>
        <h2 id="auth-sheet-title">{session.title}</h2>
        <p id="auth-sheet-description">{authError?.message ?? session.message}</p>
        {authError?.diagnostic && <details className="auth-diagnostic"><summary>诊断信息</summary><code>{authError.diagnostic}</code></details>}
        {session.provider && <span className="auth-provider-badge">{session.provider}</span>}
        {session.authType && <span className="auth-provider-badge">{session.authType === "oauth" ? "网页登录" : "API 密钥"}</span>}
        {session.verificationUrl && (
          <button className="settings-primary-button auth-open-browser" onClick={() => window.open(session.verificationUrl, "_blank", "noopener,noreferrer")}>
            在浏览器中继续
          </button>
        )}
        {session.phase === "waiting_input" && session.promptMessage && (
          <p className="interaction-message">{localizePiAuthPrompt(session.promptMessage, (session.options?.length ?? 0) > 0)}</p>
        )}
        {session.phase === "waiting_input" && (session.options?.length ?? 0) > 0 && (
          <div className="interaction-options auth-login-options" role="radiogroup" aria-label="Pi 登录选项">
            {session.options!.map((option, optionIndex) => (
              <div
                key={option.id}
                role="radio"
                aria-checked={selectedOptionId === option.id}
                tabIndex={selectedOptionId === option.id ? 0 : -1}
                onClick={(event) => {
                  setSelectedOptionId(option.id);
                  event.currentTarget.focus();
                }}
                onKeyDown={(event) => handleOptionKeyDown(event, optionIndex)}
              >
                <span className="interaction-radio" />
                <span><strong>{localizePiAuthOption(option.label)}</strong>{option.description && <small>{localizePiAuthOption(option.description)}</small>}</span>
              </div>
            ))}
          </div>
        )}
        {session.phase === "waiting_input" && (session.options?.length ?? 0) === 0 && (
          <label className="auth-input">
            <span>{session.inputLabel ?? "验证码"}</span>
            <input
              autoFocus
              type={session.inputSecret ? "password" : "text"}
              value={value}
              onChange={(event) => setValue(event.target.value)}
              onKeyDown={(event) => {
                if (event.key === "Enter" && value.trim()) {
                  void runOperation(() => onRespond(value.trim()), "登录验证失败，请保留当前输入并重试。");
                }
              }}
            />
          </label>
        )}
        <SheetOperationError error={operationError} />
        <div className="auth-sheet-actions">
          {!terminal && <button disabled={busy} onClick={() => void runOperation(onCancel, "取消登录失败，请稍后重试。")}>取消</button>}
          {(session.phase === "waiting_browser" || session.phase === "starting") && <button className="is-primary" disabled={busy} onClick={() => void runOperation(onRefresh, "无法刷新登录状态，请稍后重试。")}>我已完成</button>}
          {session.phase === "waiting_input" && (session.options?.length ?? 0) > 0 && (
            <button className="is-primary" disabled={busy || !selectedOptionId} onClick={() => void runOperation(() => onRespond(selectedOptionId), "登录验证失败，请保留当前选择并重试。")}>继续</button>
          )}
          {session.phase === "waiting_input" && (session.options?.length ?? 0) === 0 && (
            <button className="is-primary" disabled={busy || !value.trim()} onClick={() => void runOperation(() => onRespond(value.trim()), "登录验证失败，请保留当前输入并重试。")}>继续</button>
          )}
          {terminal && <button className="is-primary" onClick={onDismiss}>完成</button>}
        </div>
    </ModalSheet>
  );
}

function LoginSetupSheet({
  account,
  busy,
  onLogin,
  onCancel,
}: {
  account: AccountSummary;
  busy: boolean;
  onLogin(provider: string, authType: AuthType): Promise<void>;
  onCancel(): void;
}) {
  const [provider, setProvider] = useState(account.selectedProvider ?? "openai");
  const [authType, setAuthType] = useState<AuthType>("oauth");
  const [operationError, setOperationError] = useState<UserFacingError | null>(null);

  async function submit() {
    if (!provider.trim() || busy) return;
    setOperationError(null);
    try {
      await onLogin(provider.trim(), authType);
    } catch (error) {
      setOperationError(toUserFacingError(error, "无法启动登录流程，请保留当前选择并重试。"));
    }
  }

  return (
    <ModalSheet
      labelledBy="login-setup-title"
      describedBy="login-setup-description"
      busy={busy}
      onDismiss={onCancel}
    >
      <div className="auth-sheet-icon"><Icon name="sparkles" /></div>
      <h2 id="login-setup-title">登录 {account.name}</h2>
      <p id="login-setup-description">选择 Pi 登录所用的提供商与方式。提供商列表仅是常用快捷项，不代表后端支持范围。</p>
      <ProviderPicker value={provider} onChange={setProvider} />
      <div className="auth-input">
        <span>登录方式</span>
        <div className="segmented-control" role="group" aria-label="登录方式">
          <button type="button" className={authType === "oauth" ? "is-active" : ""} aria-pressed={authType === "oauth"} onClick={() => setAuthType("oauth")}>网页登录</button>
          <button type="button" className={authType === "api_key" ? "is-active" : ""} aria-pressed={authType === "api_key"} onClick={() => setAuthType("api_key")}>API 密钥</button>
        </div>
      </div>
      <SheetOperationError error={operationError} />
      <div className="auth-sheet-actions">
        <button disabled={busy} onClick={onCancel}>取消</button>
        <button className="is-primary" disabled={busy || !provider.trim()} onClick={() => void submit()}>开始登录</button>
      </div>
    </ModalSheet>
  );
}

function CreateAccountSheet({ busy, onCreate, onCancel }: { busy: boolean; onCreate(name: string, provider: string): Promise<void>; onCancel(): void }) {
  const [name, setName] = useState("");
  const [provider, setProvider] = useState("openai");
  const [operationError, setOperationError] = useState<UserFacingError | null>(null);

  async function submit() {
    if (!name.trim() || !provider.trim() || busy) return;
    setOperationError(null);
    try {
      await onCreate(name.trim(), provider.trim());
    } catch (error) {
      setOperationError(toUserFacingError(error, "创建账号失败，请保留当前输入并重试。"));
    }
  }

  return (
    <ModalSheet
      labelledBy="create-account-title"
      describedBy="create-account-description"
      className="account-create-sheet"
      busy={busy}
      onDismiss={onCancel}
    >
        <div className="auth-sheet-icon"><Icon name="plus" /></div>
        <h2 id="create-account-title">新增 Pi 账号</h2>
        <p id="create-account-description">创建独立的 Pi 账号配置。新账号沿用本地服务的完全访问默认策略，之后可在设置中关闭。</p>
        <label className="auth-input"><span>账号名称</span><input autoFocus value={name} onChange={(event) => setName(event.target.value)} placeholder="例如：个人账号" /></label>
        <ProviderPicker value={provider} onChange={setProvider} />
        <SheetOperationError error={operationError} />
        <div className="auth-sheet-actions">
          <button disabled={busy} onClick={onCancel}>取消</button>
          <button className="is-primary" disabled={busy || !name.trim() || !provider.trim()} onClick={() => void submit()}>创建账号</button>
        </div>
    </ModalSheet>
  );
}

export function SettingsView({
  accounts,
  backend,
  theme,
  fullAccess,
  initialSection = "general",
  authSession,
  remote,
  onThemeChange,
  onFullAccessChange,
  onCreateAccount,
  onSwitchAccount,
  onBeginLogin,
  onRefreshLogin,
  onRespondLogin,
  onCancelLogin,
  onDismissLogin,
  onLogout,
  onRemoteRefresh,
  onRemoteEnabledChange,
  onBeginRemotePairing,
  onCancelRemotePairing,
  onRevokeRemoteDevice,
  onBack,
}: SettingsViewProps) {
  const [section, setSection] = useState<SettingsSection>(initialSection);
  const [busyAccountId, setBusyAccountId] = useState<string | null>(null);
  const [createAccountOpen, setCreateAccountOpen] = useState(false);
  const [loginAccountId, setLoginAccountId] = useState<string | null>(null);
  const activeAccount = accounts.find((account) => account.active) ?? accounts[0];
  const loginAccount = accounts.find((account) => account.id === loginAccountId) ?? null;
  const authSupported = backend.capabilities.includes("pi_auth_control_plane")
    || authCapabilities.every((capability) => backend.capabilities.includes(capability));

  useEffect(() => setSection(initialSection), [initialSection]);

  async function withAccount(accountId: string, action: () => Promise<void>) {
    setBusyAccountId(accountId);
    try {
      await action();
    } finally {
      setBusyAccountId(null);
    }
  }

  return (
    <section className="settings-page" data-focus-domain="settings" aria-label="设置">
      <header className="settings-toolbar">
        <button onClick={onBack} aria-label="返回任务"><Icon name="arrow-left" /></button>
        <strong>设置</strong>
      </header>
      <div className="settings-layout">
        <nav className="settings-nav" aria-label="设置分类">
          {sections.map((item) => (
            <button
              className={section === item.id ? "is-active" : ""}
              key={item.id}
              title={item.label}
              aria-current={section === item.id ? "page" : undefined}
              onClick={() => setSection(item.id)}
            >
              <Icon name={item.icon} /><span>{item.label}</span>
            </button>
          ))}
        </nav>
        <div className="settings-content">
          {section === "general" && <>
            <div className="settings-heading"><h1>通用</h1><p>本地界面设置立即生效；尚未接入系统能力的选项不会伪装成可用。</p></div>
            <section className="settings-card">
              <SettingsRow title="外观" description="保存在本机 PAD Desktop 界面配置中。">
                <div className="segmented-control" role="group" aria-label="外观">
                  <button className={theme === "light" ? "is-active" : ""} aria-pressed={theme === "light"} onClick={() => onThemeChange("light")}>浅色</button>
                  <button className={theme === "dark" ? "is-active" : ""} aria-pressed={theme === "dark"} onClick={() => onThemeChange("dark")}>深色</button>
                  <button className={theme === "system" ? "is-active" : ""} aria-pressed={theme === "system"} onClick={() => onThemeChange("system")}>跟随系统</button>
                </div>
              </SettingsRow>
              <SettingsRow title="登录时启动" description="等待接入 macOS 登录项管理。" disabled><span className="settings-unavailable">尚未开放</span></SettingsRow>
              <SettingsRow title="任务通知" description="等待接入 macOS 通知权限与通知中心。" disabled><span className="settings-unavailable">尚未开放</span></SettingsRow>
            </section>
          </>}

          {section === "accounts" && <>
            <div className="settings-heading"><h1>账号</h1><p>每个 Pi 账号配置独立保存模型登录状态、任务与权限；切换后不会展示其他账号的任务。</p></div>
            <section className="account-card-list">
              {accounts.map((account) => (
                <article className={`account-card${account.active ? " is-active" : ""}`} key={account.id} data-account-id={account.id}>
                  <span className="avatar account-card-avatar">{account.initials}</span>
                  <div className="account-card-copy">
                    <div><strong>{account.name}</strong>{account.active && <span className="settings-badge">当前账号</span>}</div>
                    <p>{account.selectedProvider ?? "未选择提供商"}{account.selectedModel ? ` · ${account.selectedModel}` : ""}</p>
                    <span className={`account-state auth-${account.authentication}`}><i />{authenticationLabel(account)}</span>
                  </div>
                  <div className="account-card-actions">
                    {!account.active ? (
                      <button
                        className="settings-secondary-button"
                        disabled={busyAccountId === account.id}
                        onClick={() => void withAccount(account.id, () => onSwitchAccount(account.id))}
                      >切换使用</button>
                    ) : account.authentication === "authenticated" ? (
                      <button
                        className="settings-secondary-button is-danger"
                        disabled={!authSupported || busyAccountId === account.id}
                        onClick={() => void withAccount(account.id, () => onLogout(account.id, account.selectedProvider ?? undefined))}
                      >退出登录</button>
                    ) : (
                      <button
                        className="settings-primary-button"
                        disabled={!authSupported || busyAccountId === account.id}
                        onClick={() => setLoginAccountId(account.id)}
                      >登录</button>
                    )}
                  </div>
                </article>
              ))}
            </section>
            {!authSupported && <div className="settings-inline-notice"><Icon name="archive" /><span>当前 PAD 控制面尚未提供完整登录协议；账号状态只读，升级本地服务后会自动启用登录按钮。</span></div>}
            <button className="settings-secondary-button" onClick={() => setCreateAccountOpen(true)}><Icon name="plus" />新增账号</button>
          </>}

          {section === "pi" && <>
            <div className="settings-heading"><h1>Pi 运行时</h1><p>以下信息来自当前 PAD 本地服务，不使用演示状态。</p></div>
            <section className="settings-card">
              <SettingsRow title="控制面状态"><span className={`healthy-state status-${backend.status}`}><span />{backendStatusLabel(backend.status)}</span></SettingsRow>
              <SettingsRow title="当前提供商"><code className="settings-code">{activeAccount?.selectedProvider ?? "未选择"}</code></SettingsRow>
              <SettingsRow title="当前模型"><code className="settings-code">{activeAccount?.selectedModel ?? "未选择"}</code></SettingsRow>
              <SettingsRow title="推理强度" description="在每个任务的输入框中选择；发送前会下发给 Pi。"><span className="settings-badge">任务级设置</span></SettingsRow>
              <SettingsRow title="控制面能力"><span className="settings-badge">{backend.capabilities.length} 项</span></SettingsRow>
            </section>
          </>}

          {section === "remote" && (
            <RemoteSettingsSection
              capabilities={backend.capabilities}
              status={remote}
              onRefresh={onRemoteRefresh}
              onEnabledChange={onRemoteEnabledChange}
              onBeginPairing={onBeginRemotePairing}
              onCancelPairing={onCancelRemotePairing}
              onRevokeDevice={onRevokeRemoteDevice}
            />
          )}

          {section === "permissions" && <>
            <div className="settings-heading"><h1>权限与访问</h1><p>权限直接读取并写回当前 Pi 账号的持久化权限策略。</p></div>
            <section className="settings-card">
              <SettingsRow title="完全访问" description="启用系统完全访问与无人值守；普通操作可自动执行，但 PAD、Pi、Codex、ChatGPT 私有区域、系统保护区域及 macOS 隐私权限（TCC）仍不会自动放行。">
                <Toggle checked={fullAccess} onChange={onFullAccessChange} label="完全访问" />
              </SettingsRow>
              <SettingsRow title="当前模式"><span className="settings-badge">{permissionModeLabel(activeAccount?.policy.mode ?? null)}</span></SettingsRow>
              <SettingsRow title="无人值守"><span className="settings-badge">{activeAccount?.policy.unattended ? "已启用" : "已关闭"}</span></SettingsRow>
              <SettingsRow title="工作区根目录"><span className="settings-badge">{activeAccount?.policy.workspaceRootCount ?? 0} 个</span></SettingsRow>
              <SettingsRow title="受保护命名空间"><span className="settings-badge">{activeAccount?.policy.protectedNamespaceNames.length ?? 0} 个</span></SettingsRow>
            </section>
            <div className="settings-callout"><Icon name="check" /><p><strong>账号级隔离</strong><span>此开关只更新当前账号，不会修改其他账号或 Codex/ChatGPT 的会话。</span></p></div>
          </>}

          {section === "data" && <>
            <div className="settings-heading"><h1>数据与存储</h1><p>PAD 本地服务管理独立的账号配置、任务和 Pi 会话记录。</p></div>
            <section className="settings-card">
              <SettingsRow title="任务历史"><span className="settings-badge">{backend.capabilities.includes("history") ? "本地启用" : "不可用"}</span></SettingsRow>
              <SettingsRow title="存储位置" description="为避免向渲染层暴露敏感路径，目录由本地服务私有管理。"><span className="settings-badge">本地服务托管</span></SettingsRow>
              <SettingsRow title="清理数据" description="安全清理与可恢复策略尚未接入。" disabled><span className="settings-unavailable">尚未开放</span></SettingsRow>
            </section>
          </>}

          {section === "about" && <>
            <div className="settings-heading"><h1>关于 PAD Desktop</h1><p>面向 macOS 的 Pi 原生任务工作台。</p></div>
            <section className="about-card">
              <div className="about-logo"><Icon name="sparkles" /></div>
              <h2>PAD Desktop</h2>
              <p>Electron + React · Rust PAD 本地服务 · Pi 运行内核</p>
              <span>0.7.6</span>
            </section>
          </>}
        </div>
      </div>
      {authSession && (
        <AuthSheet
          session={authSession}
          busy={busyAccountId === authSession.profileId}
          onRefresh={() => withAccount(authSession.profileId, () => onRefreshLogin(authSession.profileId))}
          onRespond={(value) => withAccount(authSession.profileId, () => onRespondLogin(authSession.profileId, value))}
          onCancel={() => withAccount(authSession.profileId, () => onCancelLogin(authSession.profileId))}
          onDismiss={onDismissLogin}
        />
      )}
      {loginAccount && !authSession && (
        <LoginSetupSheet
          account={loginAccount}
          busy={busyAccountId === loginAccount.id}
          onLogin={async (provider, authType) => {
            await withAccount(loginAccount.id, () => onBeginLogin(loginAccount.id, provider, authType));
            setLoginAccountId(null);
          }}
          onCancel={() => setLoginAccountId(null)}
        />
      )}
      {createAccountOpen && (
        <CreateAccountSheet
          busy={busyAccountId === "new-account"}
          onCreate={async (name, provider) => {
            await withAccount("new-account", () => onCreateAccount(name, provider));
            setCreateAccountOpen(false);
          }}
          onCancel={() => setCreateAccountOpen(false)}
        />
      )}
    </section>
  );
}
