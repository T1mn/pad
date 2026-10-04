#if os(macOS)
import AppKit
import SwiftUI

/// 账号 sheet：当前任务/默认账号的 Provider 登录方式、认证提示响应与退出登录。
/// 输入只存在内存 @State；不写入 UserDefaults / 文件 / 日志，提交或取消后清空。
/// 未知 promptKind 不自动批准；URL 仅允许 http(s)，且只在用户点击后打开。
struct PADAccountView: View {
    @ObservedObject var model: PADWorkbenchModel

    @Environment(\.dismiss) private var dismiss

    @State private var pendingProvider: String?
    @State private var pendingMethod: String?
    @State private var promptValue: String = ""
    @State private var pendingLogoutProvider: String?
    @State private var providerQuery = ""
    @State private var authenticatedOnly = false
    @FocusState private var searchFocused: Bool

    var body: some View {
        VStack(spacing: 0) {
            header
            horizontalSeparator
            authPanel
            if model.isHistoricalAccount {
                PADUIBanner(systemImage: "info.circle",
                            text: "历史任务保留原账号。此处登录与模型目录仅作用于该账号，不会迁移凭据；新任务和 Pi 副本使用默认账号。",
                            tint: .secondary)
            }
            providerColumn
                .frame(maxWidth: .infinity, maxHeight: .infinity)
            horizontalSeparator
            footer
        }
        .frame(width: 880, height: min(680, max(480, (NSScreen.main?.visibleFrame.height ?? 840) - 160)))
        .padFont()
        .background(PADWorkbenchStyle.canvas)
        .onAppear { if !authRunning { searchFocused = true } }
        .onChange(of: model.activeProfileId) { _ in
            resetLocalAuthInput()
            pendingLogoutProvider = nil
        }
        .onChange(of: model.authState?.promptId) { _ in
            promptValue = ""
        }
        .onChange(of: model.authState?.phase) { phase in
            if phase != "running" {
                resetLocalAuthInput()
            }
        }
        .confirmationDialog(
            "退出登录？",
            isPresented: logoutDialogBinding,
            titleVisibility: .visible,
            presenting: pendingLogoutProvider
        ) { providerId in
            Button("退出登录", role: .destructive) {
                model.logout(provider: providerId)
                pendingLogoutProvider = nil
            }
            Button("取消", role: .cancel) {
                pendingLogoutProvider = nil
            }
        } message: { providerId in
            Text("将退出「\(PADWorkbenchUI.providerDisplayName(providerId, in: model.catalog))」并移除当前账号的登录凭据；已有任务不会被迁移或删除。")
        }
    }

    private var horizontalSeparator: some View {
        Rectangle().fill(PADWorkbenchStyle.border).frame(height: 1)
    }

    // MARK: - 头部

    private var header: some View {
        HStack(alignment: .center, spacing: 8) {
            Image(systemName: "person.badge.key.fill")
                .font(.system(size: 12))
                .foregroundStyle(PADWorkbenchStyle.accent)
            Text("PAD 账号")
                .padFont(size: 13, weight: .semibold)
                .foregroundStyle(PADWorkbenchStyle.text)
            Text(model.accountContextLabel)
                .padFont(size: 11, design: .monospaced)
                .foregroundStyle(PADWorkbenchStyle.muted)
                .lineLimit(1)
                .truncationMode(.middle)
                .contextMenu {
                    Button("默认账号") { model.selectDefaultAccount() }
                        .disabled(model.defaultProfileId == nil)
                }
            Spacer(minLength: 8)
            if model.catalogLoading {
                ProgressView().controlSize(.small)
            }
            Button("完成") { dismiss() }
                .buttonStyle(.borderless)
        }
        .padding(.horizontal, 12)
        .frame(height: PADWorkbenchStyle.barHeight + 6)
        .background(PADWorkbenchStyle.chrome)
    }

    // MARK: - 认证进行面板

    private var authRunning: Bool {
        model.authState?.phase == "running"
    }

    private var panelProvider: String? {
        if let state = model.authState, state.phase == "running", !state.provider.isEmpty {
            return state.provider
        }
        return pendingProvider
    }

    @ViewBuilder
    private var authPanel: some View {
        if let provider = panelProvider {
            VStack(alignment: .leading, spacing: 8) {
                HStack(spacing: 8) {
                    ProgressView().controlSize(.small)
                    Text("正在登录 \(PADWorkbenchUI.providerDisplayName(provider, in: model.catalog))")
                        .padFont(size: 12, weight: .semibold)
                    if let state = model.authState, state.phase == "running" {
                        PADUIBadge(text: PADWorkbenchUI.authMethodLabel(state.method), tint: PADWorkbenchStyle.accent)
                        if state.profileId != model.activeProfileId, !state.profileId.isEmpty {
                            Text("（Profile：\(PADWorkbenchUI.profileName(state.profileId, in: model.snapshot.profiles))）")
                                .padFont(size: 10.5)
                                .foregroundStyle(PADWorkbenchStyle.muted)
                        }
                    } else {
                        Text("正在准备…")
                            .padFont(size: 10.5)
                            .foregroundStyle(PADWorkbenchStyle.muted)
                        if let method = pendingMethod {
                            PADUIBadge(text: PADWorkbenchUI.authMethodLabel(method), tint: .secondary)
                        }
                    }
                    Spacer(minLength: 0)
                    Button("取消") { cancelAuth() }
                        .controlSize(.small)
                }
                if let message = model.authState?.message, !message.isEmpty {
                    Text(message)
                        .padFont(size: 11)
                        .foregroundStyle(PADWorkbenchStyle.muted)
                        .fixedSize(horizontal: false, vertical: true)
                }
                promptControls
                externalRows
            }
            .padding(10)
            .frame(maxWidth: .infinity, alignment: .leading)
            .background(PADWorkbenchStyle.input.opacity(0.6))
            .overlay(alignment: .bottom) {
                Rectangle().fill(PADWorkbenchStyle.accent.opacity(0.35)).frame(height: 1)
            }
        } else if let state = model.authState, state.phase != "running" {
            PADUIBanner(
                systemImage: authResultIcon(state.phase),
                text: authResultText(state),
                tint: authResultTint(state.phase)
            )
        }
    }

    private func authResultIcon(_ phase: String) -> String {
        switch phase.lowercased() {
        case "succeeded": return "checkmark.circle.fill"
        case "failed": return "exclamationmark.triangle.fill"
        default: return "info.circle"
        }
    }

    private func authResultText(_ state: PADAuthState) -> String {
        let provider = PADWorkbenchUI.providerDisplayName(state.provider, in: model.catalog)
        switch state.phase.lowercased() {
        case "succeeded":
            return "「\(provider)」登录成功，正在刷新账号目录。"
        case "failed":
            let detail = (state.message?.isEmpty == false) ? state.message! : "登录失败，请重试或改用其他方式。"
            return "「\(provider)」登录失败：\(detail)"
        case "cancelled":
            return "已取消「\(provider)」的登录流程；若授权已完成，请检查账号状态。"
        default:
            return "「\(provider)」\(PADWorkbenchUI.authPhaseLabel(state.phase))"
        }
    }

    private func authResultTint(_ phase: String) -> Color {
        switch phase.lowercased() {
        case "succeeded": return .green
        case "failed": return .red
        default: return .secondary
        }
    }

    // MARK: - 认证提示响应

    @ViewBuilder
    private var promptControls: some View {
        Group {
            switch model.authState?.promptKind {
            case "text":
                promptField(secure: false, placeholder: "请输入")
            case "secret":
                promptField(secure: true, placeholder: "请输入密钥")
            case "manual_code":
                promptField(secure: true, placeholder: "请输入验证码")
            case "select":
                selectPrompt
            case "device_code":
                HStack(spacing: 8) {
                    Text("请在浏览器完成设备授权，完成后回到此窗口。")
                        .padFont(size: 11)
                        .foregroundStyle(PADWorkbenchStyle.muted)
                    Spacer(minLength: 0)
                }
            case nil:
                Text("等待宿主返回登录提示…")
                    .padFont(size: 11)
                    .foregroundStyle(PADWorkbenchStyle.muted)
            default:
                unknownPrompt
            }
        }
    }

    private func promptField(secure: Bool, placeholder: String) -> some View {
        HStack(spacing: 8) {
            Group {
                if secure {
                    SecureField(model.authState?.placeholder ?? placeholder, text: $promptValue)
                } else {
                    TextField(model.authState?.placeholder ?? placeholder, text: $promptValue)
                }
            }
            .textFieldStyle(.roundedBorder)
            .frame(maxWidth: 320)
            .onSubmit { submitPrompt() }

            Button("提交") { submitPrompt() }
                .controlSize(.small)
                .disabled(model.authState?.promptId == nil || PADWorkbenchUI.trimmed(promptValue).isEmpty)

            if secure {
                Text("仅保存在内存，提交后立即清空。")
                    .padFont(size: 10)
                    .foregroundStyle(PADWorkbenchStyle.muted)
            }
            Spacer(minLength: 0)
        }
    }

    private var selectPrompt: some View {
        VStack(alignment: .leading, spacing: 6) {
            Text("请选择一个选项（点击后立即提交）：")
                .padFont(size: 11)
                .foregroundStyle(PADWorkbenchStyle.muted)
            ForEach(model.authState?.options ?? []) { option in
                Button {
                    guard let promptId = model.authState?.promptId else { return }
                    model.respondAuth(promptId: promptId, value: option.id)
                    promptValue = ""
                } label: {
                    HStack(spacing: 6) {
                        Text(option.label)
                            .padFont(size: 11.5)
                        Spacer(minLength: 0)
                    }
                    .contentShape(Rectangle())
                }
                .buttonStyle(.bordered)
                .controlSize(.small)
                .disabled(model.authState?.promptId == nil)
            }
        }
    }

    private var unknownPrompt: some View {
        HStack(alignment: .top, spacing: 8) {
            Image(systemName: "questionmark.circle")
                .foregroundStyle(.orange)
            VStack(alignment: .leading, spacing: 2) {
                Text("暂不支持的登录步骤（\(model.authState?.promptKind ?? "未知")）")
                    .padFont(size: 11.5, weight: .semibold)
                Text("PAD 不会自动批准该请求。请点击「取消」，改用其他登录方式。")
                    .padFont(size: 10.5)
                    .foregroundStyle(PADWorkbenchStyle.muted)
            }
            Spacer(minLength: 0)
        }
    }

    @ViewBuilder
    private var externalRows: some View {
        if let url = PADWorkbenchUI.externalURL(model.authState?.url) {
            HStack(spacing: 8) {
                Image(systemName: "link")
                    .font(.system(size: 11))
                Text(url.absoluteString)
                    .padFont(size: 10.5, design: .monospaced)
                    .lineLimit(1)
                    .truncationMode(.middle)
                    .textSelection(.enabled)
                Spacer(minLength: 4)
                Button("打开授权链接") {
                    NSWorkspace.shared.open(url)
                }
                .controlSize(.small)
            }
            Text("链接不会自动打开；仅支持 http/https。")
                .padFont(size: 10)
                .foregroundStyle(PADWorkbenchStyle.muted)
        } else if let raw = model.authState?.url, !raw.isEmpty {
            Text("授权链接无效（仅支持 http/https），已忽略：\(raw)")
                .padFont(size: 10)
                .foregroundStyle(.orange)
                .lineLimit(2)
                .truncationMode(.middle)
        }

        if let code = model.authState?.userCode, !code.isEmpty {
            HStack(spacing: 8) {
                Image(systemName: "number")
                    .font(.system(size: 11))
                Text(code)
                    .padFont(size: 12, weight: .semibold, design: .monospaced)
                    .textSelection(.enabled)
                Button("复制") {
                    NSPasteboard.general.clearContents()
                    NSPasteboard.general.setString(code, forType: .string)
                }
                .controlSize(.small)
                Text("在浏览器中输入此设备码")
                    .padFont(size: 10)
                    .foregroundStyle(PADWorkbenchStyle.muted)
                Spacer(minLength: 0)
            }
        }
    }

    // MARK: - Provider 列

    private var catalogMatchesProfile: Bool {
        model.catalog.profileId.isEmpty || model.catalog.profileId == model.activeProfileId
    }

    private var visibleProviders: [PADProvider] {
        PADProviderSearch.filter(model.catalog.providers, query: providerQuery, authenticatedOnly: authenticatedOnly)
    }

    private var providerColumn: some View {
        VStack(alignment: .leading, spacing: 0) {
            HStack(spacing: 8) {
                Text("Provider · \(visibleProviders.count)")
                    .padFont(size: 10, weight: .semibold, design: .monospaced)
                    .foregroundStyle(PADWorkbenchStyle.muted)
                Spacer(minLength: 0)
                if !catalogMatchesProfile {
                    Text("账号上下文已切换")
                        .padFont(size: 10)
                        .foregroundStyle(.orange)
                }
            }
            .padding(.horizontal, 12)
            .padding(.top, 8)
            .padding(.bottom, 4)

            VStack(spacing: 8) {
                HStack(spacing: 8) {
                    Image(systemName: "magnifyingglass").foregroundStyle(PADWorkbenchStyle.muted)
                    TextField("搜索名称：OpenAI、ChatGPT、Claude…", text: $providerQuery)
                        .textFieldStyle(.plain).focused($searchFocused)
                        .onSubmit { } // Search filters live; Return must not dismiss or log in.
                        .onExitCommand { providerQuery = "" }
                    if !providerQuery.isEmpty {
                        Button { providerQuery = ""; searchFocused = true } label: {
                            Image(systemName: "xmark.circle.fill")
                        }.buttonStyle(.plain).help("清空搜索")
                    }
                }
                .padding(9).background(PADWorkbenchStyle.input)
                .clipShape(RoundedRectangle(cornerRadius: 5))
                HStack {
                    ForEach(["OpenAI", "Claude", "Gemini"], id: \.self) { name in
                        Button(name) { providerQuery = name }.buttonStyle(.bordered)
                    }
                    Spacer()
                    Toggle("已登录", isOn: $authenticatedOnly).toggleStyle(.checkbox)
                }
            }
            .padding(.horizontal, 12).padding(.vertical, 8)

            ScrollView {
                LazyVStack(alignment: .leading, spacing: 8) {
                    if model.catalogLoading && model.catalog.providers.isEmpty {
                        HStack(spacing: 8) {
                            ProgressView().controlSize(.small)
                            Text("正在加载当前账号的 Provider…")
                                .padFont(size: 11)
                                .foregroundStyle(PADWorkbenchStyle.muted)
                        }
                        .padding(.vertical, 12)
                    } else if !catalogMatchesProfile && model.catalog.providers.isEmpty {
                        Text("正在切换到当前账号…")
                            .padFont(size: 11)
                            .foregroundStyle(PADWorkbenchStyle.muted)
                            .padding(.vertical, 12)
                    } else if model.catalog.providers.isEmpty {
                        VStack(alignment: .leading, spacing: 4) {
                            Text("没有可用的 Provider")
                                .padFont(size: 12, weight: .semibold)
                            Text("宿主只使用当前账号已配置/已认证的 Pi Provider，不会读取系统里已有的凭据。请先在宿主侧安装或配置 Provider。")
                                .padFont(size: 11)
                                .foregroundStyle(PADWorkbenchStyle.muted)
                                .fixedSize(horizontal: false, vertical: true)
                        }
                        .padding(.vertical, 12)
                    } else if visibleProviders.isEmpty {
                        Text("没有匹配的 Provider；请修改搜索或关闭“已登录”筛选。")
                            .foregroundStyle(PADWorkbenchStyle.muted).padding(.vertical, 16)
                    } else {
                        ForEach(visibleProviders) { provider in
                            providerRow(provider)
                                .disabled(!catalogMatchesProfile || model.catalogLoading || model.activeProfileId == nil)
                        }
                    }
                }
                .padding(.horizontal, 12)
                .padding(.vertical, 8)
                .frame(maxWidth: .infinity, alignment: .leading)
            }
            .frame(maxHeight: .infinity)
            .id(providerQuery + "|" + String(authenticatedOnly))

            horizontalSeparator

            Text("登录只在你点击后发起；密钥由当前账号的 Pi 目录保存。登录方式以 SDK 目录为准，部分 API Provider 需额外端点或云配置；不提供 Claude / Gemini 订阅 OAuth。")
                .padFont(size: 10)
                .foregroundStyle(PADWorkbenchStyle.muted)
                .fixedSize(horizontal: false, vertical: true)
                .padding(.horizontal, 12)
                .padding(.vertical, 8)
        }
        .frame(maxHeight: .infinity, alignment: .top)
    }

    private func providerRow(_ provider: PADProvider) -> some View {
        VStack(alignment: .leading, spacing: 6) {
            HStack(spacing: 8) {
                Text(provider.name)
                    .padFont(size: 12, weight: .semibold)
                    .foregroundStyle(PADWorkbenchStyle.text)
                PADUIBadge(
                    text: provider.authenticated ? "已登录" : "未登录",
                    tint: provider.authenticated ? .green : .secondary
                )
                Spacer(minLength: 0)
                if provider.authenticated {
                    Button("退出登录") {
                        pendingLogoutProvider = provider.id
                    }
                    .controlSize(.small)
                }
            }
            HStack(spacing: 6) {
                if provider.authTypes.isEmpty {
                    Text("此 Provider 未提供可用登录方式")
                        .padFont(size: 11)
                        .foregroundStyle(PADWorkbenchStyle.muted)
                } else {
                    ForEach(provider.authTypes, id: \.self) { method in
                        Button {
                            beginAuth(provider: provider.id, method: method)
                        } label: {
                            Label(
                                PADWorkbenchUI.authMethodLabel(method),
                                systemImage: PADWorkbenchUI.authMethodIcon(method)
                            )
                        }
                        .controlSize(.regular)
                        .disabled(authRunning)
                    }
                }
                Spacer(minLength: 0)
            }
        }
        .padding(.vertical, 7)
        .overlay(alignment: .bottom) {
            Rectangle().fill(PADWorkbenchStyle.border).frame(height: 1)
        }
    }

    // MARK: - 动作

    private func beginAuth(provider: String, method: String) {
        promptValue = ""
        pendingProvider = provider
        pendingMethod = method
        model.beginAuth(provider: provider, method: method)
    }

    private func submitPrompt() {
        guard let promptId = model.authState?.promptId else { return }
        guard !PADWorkbenchUI.trimmed(promptValue).isEmpty else { return }
        let value = promptValue
        model.respondAuth(promptId: promptId, value: value)
        promptValue = ""
    }

    private func cancelAuth() {
        model.cancelAuth()
        resetLocalAuthInput()
    }

    private func resetLocalAuthInput() {
        promptValue = ""
        pendingProvider = nil
        pendingMethod = nil
    }

    private var logoutDialogBinding: Binding<Bool> {
        Binding(
            get: { pendingLogoutProvider != nil },
            set: { presented in
                if !presented { pendingLogoutProvider = nil }
            }
        )
    }

    // MARK: - 底部

    private var footer: some View {
        HStack(alignment: .center, spacing: 10) {
            Image(systemName: "info.circle")
                .font(.system(size: 10))
                .foregroundStyle(PADWorkbenchStyle.muted)
            Text("新任务与 Pi 副本使用默认账号；已有任务保留创建时的项目与账号，不会迁移或删除。")
                .padFont(size: 10.5)
                .foregroundStyle(PADWorkbenchStyle.muted)
                .fixedSize(horizontal: false, vertical: true)
            Spacer(minLength: 8)
            if let error = model.errorMessage, !error.isEmpty {
                Text(error)
                    .padFont(size: 10.5)
                    .foregroundStyle(.red)
                    .lineLimit(2)
                    .truncationMode(.middle)
            }
        }
        .padding(.horizontal, 12)
        .padding(.vertical, 6)
        .background(PADWorkbenchStyle.chrome)
    }
}
#endif
