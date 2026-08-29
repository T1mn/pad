import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { snapshot } from "../test/fixtures";
import type { AuthSession } from "../types";
import { SettingsView } from "./SettingsView";

describe("SettingsView", () => {
  it("展示真实账号登录状态并按所选方式启动可视化登录", async () => {
    const data = snapshot();
    data.accounts[0] = { ...data.accounts[0]!, authentication: "missing", authenticatedProviders: [] };
    const onBeginLogin = vi.fn().mockResolvedValue(undefined);
    renderSettings({ accounts: data.accounts, initialSection: "accounts", onBeginLogin });
    const user = userEvent.setup();

    expect(screen.getByText("未登录")).toBeInTheDocument();
    await user.click(screen.getAllByRole("button", { name: "登录" })[0]!);
    expect(screen.getByRole("dialog", { name: "登录 个人账号" })).toBeInTheDocument();
    expect(screen.getByLabelText("模型提供商")).toHaveValue("openai");
    await user.click(screen.getByRole("button", { name: "API 密钥" }));
    await user.click(screen.getByRole("button", { name: "开始登录" }));

    expect(onBeginLogin).toHaveBeenCalledWith("personal", "openai", "api_key");
  });

  it("将 Pi options 显示为单选并提交真实 option id", async () => {
    const onRespondLogin = vi.fn().mockResolvedValue(undefined);
    const authSession: AuthSession = {
      attemptId: "attempt-options",
      promptId: "prompt-options",
      profileId: "personal",
      provider: "google",
      authType: "oauth",
      phase: "waiting_input",
      title: "登录模型账号",
      message: "请完成 Pi 提供的验证步骤。",
      promptKind: "select",
      promptMessage: "Choose an account",
      options: [
        { id: "account-personal", label: "个人账号" },
        { id: "account-team", label: "团队账号", description: "由组织管理" },
      ],
    };
    renderSettings({ initialSection: "accounts", authSession, onRespondLogin });
    const user = userEvent.setup();

    expect(screen.getByText("请选择一个选项。")).toBeInTheDocument();
    expect(screen.queryByText("Choose an account")).not.toBeInTheDocument();
    expect(screen.getByRole("radiogroup", { name: "Pi 登录选项" })).toBeInTheDocument();
    const personal = screen.getByRole("radio", { name: /个人账号/ });
    const team = screen.getByRole("radio", { name: /团队账号/ });
    const continueButton = screen.getByRole("button", { name: "继续" });

    expect(personal).toHaveAttribute("aria-checked", "true");
    expect(personal).toHaveAttribute("tabindex", "0");
    expect(team).toHaveAttribute("tabindex", "-1");
    expect(personal).toHaveFocus();

    await user.keyboard("{ArrowDown}");
    expect(team).toHaveAttribute("aria-checked", "true");
    expect(team).toHaveAttribute("tabindex", "0");
    expect(personal).toHaveAttribute("tabindex", "-1");
    expect(team).toHaveFocus();

    await user.keyboard("{ArrowRight}");
    expect(personal).toHaveFocus();
    await user.keyboard("{End}");
    expect(team).toHaveFocus();
    await user.keyboard("{Home}");
    expect(personal).toHaveFocus();
    await user.keyboard("{ArrowUp}");
    expect(team).toHaveFocus();
    await user.keyboard("{ArrowLeft}");
    expect(personal).toHaveFocus();
    await user.keyboard("{ArrowRight}");
    expect(team).toHaveFocus();

    expect(screen.getAllByRole("radio").filter((radio) => radio.tabIndex === 0)).toEqual([team]);
    await user.click(continueButton);

    expect(onRespondLogin).toHaveBeenCalledWith("personal", "account-team");
    expect(onRespondLogin).not.toHaveBeenCalledWith("personal", "团队账号");
  });

  it("在原生风格登录 Sheet 中提交 Pi challenge", async () => {
    const onRespondLogin = vi.fn().mockResolvedValue(undefined);
    const authSession: AuthSession = {
      attemptId: "attempt-1",
      promptId: "prompt-1",
      profileId: "personal",
      provider: "openai",
      phase: "waiting_input",
      title: "验证 OpenAI 登录",
      message: "输入浏览器显示的验证码",
      inputLabel: "验证码",
      inputSecret: true,
    };
    renderSettings({ initialSection: "accounts", authSession, onRespondLogin });
    const user = userEvent.setup();

    expect(screen.getByRole("dialog", { name: "验证 OpenAI 登录" })).toBeInTheDocument();
    await user.type(screen.getByLabelText("验证码"), "123456");
    await user.click(screen.getByRole("button", { name: "继续" }));

    expect(onRespondLogin).toHaveBeenCalledWith("personal", "123456");
  });

  it("完全访问开关由真实账号 Policy 驱动", async () => {
    const onFullAccessChange = vi.fn();
    renderSettings({ initialSection: "permissions", onFullAccessChange });
    const user = userEvent.setup();

    expect(screen.getByText("受保护")).toBeInTheDocument();
    expect(screen.queryByText("guarded")).not.toBeInTheDocument();
    await user.click(screen.getByLabelText("完全访问"));
    expect(onFullAccessChange).toHaveBeenCalledWith(true);
  });

  it("登录终态的完成只关闭 Sheet，不调用 auth_cancel", async () => {
    const onCancelLogin = vi.fn().mockResolvedValue(undefined);
    const onDismissLogin = vi.fn();
    renderSettings({
      initialSection: "accounts",
      authSession: { profileId: "personal", provider: "openai", phase: "authenticated", title: "登录成功", message: "已登录" },
      onCancelLogin,
      onDismissLogin,
    });
    const user = userEvent.setup();

    await user.click(screen.getByRole("button", { name: "完成" }));
    expect(onDismissLogin).toHaveBeenCalledOnce();
    expect(onCancelLogin).not.toHaveBeenCalled();
  });

  it("没有后端能力的设置明确禁用", () => {
    renderSettings({ backend: { status: "ready", capabilities: [], providerAuthentication: "unknown" } });
    expect(screen.getByText("登录时启动").closest(".settings-row")).toHaveClass("is-disabled");
    expect(screen.getAllByText("尚未开放").length).toBeGreaterThan(0);
    expect(document.querySelector('[data-focus-domain="settings"]')).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "浅色" })).toHaveAttribute("aria-pressed", "true");
  });

  it("通过可视化 Sheet 创建第二个 Profile", async () => {
    const onCreateAccount = vi.fn().mockResolvedValue(undefined);
    renderSettings({ initialSection: "accounts", onCreateAccount });
    const user = userEvent.setup();

    await user.click(screen.getByRole("button", { name: "新增账号" }));
    expect(screen.getByRole("dialog", { name: "新增 Pi 账号" })).toBeInTheDocument();
    await user.type(screen.getByLabelText("账号名称"), "第二账号");
    await user.selectOptions(screen.getByLabelText("模型提供商"), "anthropic");
    await user.click(screen.getByRole("button", { name: "创建账号" }));

    expect(onCreateAccount).toHaveBeenCalledWith("第二账号", "anthropic");
  });

  it("创建账号失败时 Sheet 保留输入、显示中文错误并可原地重试", async () => {
    const onCreateAccount = vi.fn()
      .mockRejectedValueOnce({ code: "profile_create_failed", message: "raw create failure" })
      .mockResolvedValueOnce(undefined);
    renderSettings({ initialSection: "accounts", onCreateAccount });
    const user = userEvent.setup();

    await user.click(screen.getByRole("button", { name: "新增账号" }));
    await user.type(screen.getByLabelText("账号名称"), "保留账号名");
    await user.selectOptions(screen.getByLabelText("模型提供商"), "anthropic");
    await user.click(screen.getByRole("button", { name: "创建账号" }));

    const dialog = screen.getByRole("dialog", { name: "新增 Pi 账号" });
    expect(dialog).toBeInTheDocument();
    expect(screen.getByLabelText("账号名称")).toHaveValue("保留账号名");
    expect(screen.getByLabelText("模型提供商")).toHaveValue("anthropic");
    expect(await screen.findByRole("alert")).toHaveTextContent("创建账号失败，请保留当前输入并重试。");

    await user.click(screen.getByRole("button", { name: "创建账号" }));
    await waitFor(() => expect(screen.queryByRole("dialog", { name: "新增 Pi 账号" })).not.toBeInTheDocument());
    expect(onCreateAccount).toHaveBeenCalledTimes(2);
  });

  it("开始登录失败时保留提供商和登录方式并显示 Sheet 内错误", async () => {
    const data = snapshot();
    data.accounts[0] = { ...data.accounts[0]!, authentication: "missing", authenticatedProviders: [] };
    const onBeginLogin = vi.fn()
      .mockRejectedValueOnce({ code: "auth_failed", message: "provider rejected login" })
      .mockResolvedValueOnce(undefined);
    renderSettings({ accounts: data.accounts, initialSection: "accounts", onBeginLogin });
    const user = userEvent.setup();

    await user.click(screen.getAllByRole("button", { name: "登录" })[0]!);
    await user.selectOptions(screen.getByLabelText("模型提供商"), "anthropic");
    await user.click(screen.getByRole("button", { name: "API 密钥" }));
    await user.click(screen.getByRole("button", { name: "开始登录" }));

    expect(screen.getByRole("dialog", { name: "登录 个人账号" })).toBeInTheDocument();
    expect(screen.getByLabelText("模型提供商")).toHaveValue("anthropic");
    expect(screen.getByRole("button", { name: "API 密钥" })).toHaveAttribute("aria-pressed", "true");
    expect(await screen.findByRole("alert")).toHaveTextContent("模型账号登录失败，请重新登录后再试。");

    await user.click(screen.getByRole("button", { name: "开始登录" }));
    await waitFor(() => expect(screen.queryByRole("dialog", { name: "登录 个人账号" })).not.toBeInTheDocument());
    expect(onBeginLogin).toHaveBeenCalledTimes(2);
  });

  it("账号创建始终保留自定义 provider 入口", async () => {
    const onCreateAccount = vi.fn().mockResolvedValue(undefined);
    renderSettings({ initialSection: "accounts", onCreateAccount });
    const user = userEvent.setup();

    await user.click(screen.getByRole("button", { name: "新增账号" }));
    await user.type(screen.getByLabelText("账号名称"), "自建账号");
    await user.selectOptions(screen.getByLabelText("模型提供商"), "__custom__");
    await user.type(screen.getByLabelText("自定义提供商 ID"), "company-gateway");
    await user.click(screen.getByRole("button", { name: "创建账号" }));

    expect(onCreateAccount).toHaveBeenCalledWith("自建账号", "company-gateway");
  });

  it("外观可选择跟随系统", async () => {
    const onThemeChange = vi.fn();
    renderSettings({ theme: "system", onThemeChange });
    const user = userEvent.setup();

    expect(screen.getByRole("button", { name: "跟随系统" })).toHaveAttribute("aria-pressed", "true");
    await user.click(screen.getByRole("button", { name: "深色" }));
    expect(onThemeChange).toHaveBeenCalledWith("dark");
  });

  it("设置导航提供独立远程连接页并显示真实状态", async () => {
    renderSettings({
      backend: {
        status: "ready",
        capabilities: ["remote_gateway_v1", "remote_pairing", "remote_device_management"],
        providerAuthentication: "authenticated",
      },
      remote: {
        enabled: true,
        state: "ready",
        displayName: "Tim 的 Mac",
        activeConnections: 1,
        devices: [],
        updatedAt: 1_800_000_000,
      },
    });
    const user = userEvent.setup();
    await user.click(screen.getByRole("button", { name: "远程连接" }));

    expect(screen.getByRole("heading", { name: "远程连接" })).toBeInTheDocument();
    expect(screen.getByText("Tim 的 Mac")).toBeInTheDocument();
    expect(screen.getByRole("checkbox", { name: "允许远程连接" })).toBeChecked();
    expect(screen.getByText("1 台在线")).toBeInTheDocument();
  });

  it("非当前账号先切换，不能直接跨账号登录或退出", async () => {
    const onSwitchAccount = vi.fn().mockResolvedValue(undefined);
    const onBeginLogin = vi.fn().mockResolvedValue(undefined);
    const onLogout = vi.fn().mockResolvedValue(undefined);
    renderSettings({ initialSection: "accounts", onSwitchAccount, onBeginLogin, onLogout });
    const user = userEvent.setup();

    await user.click(screen.getByRole("button", { name: "切换使用" }));

    expect(onSwitchAccount).toHaveBeenCalledWith("team");
    expect(onBeginLogin).not.toHaveBeenCalled();
    expect(onLogout).not.toHaveBeenCalled();
  });

  it("模态登录流程可用 Escape 安全退出", async () => {
    const onCancelLogin = vi.fn().mockResolvedValue(undefined);
    const trigger = document.createElement("button");
    document.body.append(trigger);
    trigger.focus();
    renderSettings({
      initialSection: "accounts",
      authSession: { profileId: "personal", provider: "openai", phase: "waiting_browser", title: "登录模型账号", message: "等待浏览器授权" },
      onCancelLogin,
    });
    const user = userEvent.setup();

    await user.keyboard("{Escape}");
    expect(onCancelLogin).toHaveBeenCalledWith("personal");
    trigger.remove();
  });
});

function renderSettings(overrides: Partial<React.ComponentProps<typeof SettingsView>> = {}) {
  const data = snapshot();
  const props: React.ComponentProps<typeof SettingsView> = {
    accounts: data.accounts,
    backend: data.backend,
    theme: "light",
    fullAccess: false,
    initialSection: "general",
    authSession: null,
    remote: null,
    onThemeChange: vi.fn(),
    onFullAccessChange: vi.fn(),
    onCreateAccount: vi.fn().mockResolvedValue(undefined),
    onSwitchAccount: vi.fn().mockResolvedValue(undefined),
    onBeginLogin: vi.fn().mockResolvedValue(undefined),
    onRefreshLogin: vi.fn().mockResolvedValue(undefined),
    onRespondLogin: vi.fn().mockResolvedValue(undefined),
    onCancelLogin: vi.fn().mockResolvedValue(undefined),
    onDismissLogin: vi.fn(),
    onLogout: vi.fn().mockResolvedValue(undefined),
    onRemoteRefresh: vi.fn().mockResolvedValue(undefined),
    onRemoteEnabledChange: vi.fn().mockResolvedValue(undefined),
    onBeginRemotePairing: vi.fn().mockRejectedValue(new Error("not configured")),
    onCancelRemotePairing: vi.fn().mockResolvedValue(undefined),
    onRevokeRemoteDevice: vi.fn().mockResolvedValue(undefined),
    onBack: vi.fn(),
    ...overrides,
  };
  return render(<SettingsView {...props} />);
}
