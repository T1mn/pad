#if os(macOS)
import SwiftUI

/// 右上：PAD Agent 会话。cmux 风格窄 tab strip + 左对齐日志流 + 紧凑输入区。
///
/// 只通过共享 Model 工作：不持有进程、不读写凭据、不自动发送；所有请求都由
/// 用户显式点击「发送」触发。普通 Return 只换行，不新增任何快捷键。
struct PADConversationView: View {
    @ObservedObject var model: PADWorkbenchModel
    var onOpenAccount: () -> Void

    @State private var dismissedError: String?

    var body: some View {
        VStack(spacing: 0) {
            tabStrip
            notices
            transcript
            composer
        }
        .background(PADWorkbenchStyle.canvas)
    }

    // MARK: - 窄 tab strip（30px）

    private var tabStrip: some View {
        HStack(spacing: 0) {
            HStack(spacing: 6) {
                Image(systemName: "terminal")
                    .font(.system(size: 10.5, weight: .medium))
                    .foregroundStyle(PADWorkbenchStyle.accent)
                Text("PAD Agent")
                    .padFont(size: 11.5, weight: .medium, design: .monospaced)
                    .foregroundStyle(PADWorkbenchStyle.text)
                if model.selectedTask != nil {
                    Text(currentTitle)
                        .padFont(size: 11, design: .monospaced)
                        .foregroundStyle(PADWorkbenchStyle.muted)
                        .lineLimit(1)
                        .truncationMode(.middle)
                }
                if model.isBusy {
                    ProgressView().controlSize(.mini)
                }
            }
            .padding(.horizontal, 10)
            .frame(height: PADWorkbenchStyle.barHeight)
            .overlay(alignment: .top) {
                Rectangle()
                    .fill(PADWorkbenchStyle.accent)
                    .frame(height: 1.5)
            }
            .contentShape(Rectangle())

            Spacer(minLength: 0)
        }
        .frame(height: PADWorkbenchStyle.barHeight)
        .background(PADWorkbenchStyle.chrome)
        .overlay(alignment: .bottom) {
            Rectangle().fill(PADWorkbenchStyle.border).frame(height: 1)
        }
    }

    private var currentTitle: String {
        guard let task = model.selectedTask else { return "PAD 会话" }
        return PADWorkbenchUI.taskTitle(task)
    }

    // MARK: - 紧凑提示（登录 / 离线 / 错误合并为单行真实文本）

    private struct NoticeItem: Identifiable {
        let id: String
        let icon: String
        let text: String
        let tint: Color
        var actionTitle: String?
        var action: (() -> Void)?
        var dismiss: (() -> Void)?
    }

    private var noticeItems: [NoticeItem] {
        var items: [NoticeItem] = []

        if PADWorkbenchUI.isOffline(model.connectionStatus) {
            items.append(NoticeItem(
                id: "offline",
                icon: "bolt.horizontal.circle",
                text: "宿主未连接（\(PADWorkbenchUI.connectionLabel(model.connectionStatus))）。确认宿主进程仍在运行后重新加载。",
                tint: .red,
                actionTitle: "重试",
                action: { model.reload() }
            ))
        }

        if let warning = model.proxyWarning {
            items.append(NoticeItem(
                id: "system-proxy-warning",
                icon: "exclamationmark.triangle.fill",
                text: warning,
                tint: .orange
            ))
        }

        if let errorText = model.errorMessage, !errorText.isEmpty, errorText != dismissedError {
            items.append(NoticeItem(
                id: "error",
                icon: "exclamationmark.triangle.fill",
                text: errorText,
                tint: .red,
                dismiss: { dismissedError = errorText }
            ))
        }

        if let task = model.selectedTask, task.status.lowercased() == "error" {
            items.append(NoticeItem(
                id: "task-error",
                icon: "exclamationmark.octagon.fill",
                text: "任务处于错误状态；历史仍可查看，可停止后重试或新建任务。",
                tint: .orange
            ))
        }

        if model.hasOpenAIDiscovery {
            items.append(NoticeItem(
                id: "openai-directory",
                icon: "list.bullet.rectangle",
                text: openaiDirectoryStatus,
                tint: model.catalog.openaiDiscovery?.state == "error" ? .orange : PADWorkbenchStyle.muted
            ))
        }

        if needsLogin {
            items.append(NoticeItem(
                id: "login",
                icon: "person.badge.key",
                text: "当前账号没有已认证的 Provider；PAD 不使用系统已有凭据。",
                tint: PADWorkbenchStyle.accent,
                actionTitle: "账号",
                action: onOpenAccount
            ))
        }

        return items
    }

    @ViewBuilder
    private var notices: some View {
        let items = noticeItems
        if !items.isEmpty {
            VStack(alignment: .leading, spacing: 0) {
                ForEach(items) { item in
                    HStack(alignment: .firstTextBaseline, spacing: 6) {
                        Image(systemName: item.icon)
                            .font(.system(size: 10, weight: .semibold))
                        Text(item.text)
                            .padFont(size: 10.5, design: .monospaced)
                            .lineLimit(item.id == "system-proxy-warning" ? nil : 2)
                            .fixedSize(horizontal: false, vertical: true)
                        Spacer(minLength: 4)
                        if let actionTitle = item.actionTitle, let action = item.action {
                            Button(actionTitle, action: action)
                                .buttonStyle(.plain)
                                .padFont(size: 10, weight: .semibold)
                        }
                        if let dismiss = item.dismiss {
                            Button(action: dismiss) {
                                Image(systemName: "xmark")
                                    .font(.system(size: 8, weight: .bold))
                            }
                            .buttonStyle(.plain)
                        }
                    }
                    .foregroundStyle(item.tint)
                    .padding(.horizontal, 10)
                    .padding(.vertical, 4)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .background(item.tint.opacity(0.07))
                    .overlay(alignment: .bottom) {
                        Rectangle().fill(PADWorkbenchStyle.border).frame(height: 1)
                    }
                }
            }
        }
    }

    private var catalogMatchesProfile: Bool {
        model.catalog.profileId.isEmpty || model.catalog.profileId == model.activeProfileId
    }

    private var needsLogin: Bool {
        guard model.selectedTask != nil, !model.catalogLoading, catalogMatchesProfile else { return false }
        let providers = model.catalog.providers
        guard !providers.isEmpty else { return false }
        return !providers.contains(where: { $0.authenticated })
    }

    // MARK: - 日志流

    private var transcript: some View {
        ScrollViewReader { proxy in
            ScrollView {
                transcriptContent
            }
            .onChange(of: model.messages.count) { _ in scrollToLatest(proxy) }
            .onChange(of: model.messages.last?.text) { _ in scrollToLatest(proxy) }
            .onChange(of: model.selectedTaskId) { _ in scrollToLatest(proxy) }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .background(PADWorkbenchStyle.canvas)
    }

    @ViewBuilder
    private var transcriptContent: some View {
        if model.selectedTask == nil {
            logPlaceholder(
                icon: "square.on.square.dashed",
                title: "未选择任务",
                message: "在左侧选择或新建任务；输入消息后显式点击「发送」，PAD 才会请求模型。"
            )
        } else if model.historyLoading && model.messages.isEmpty {
            HStack(spacing: 6) {
                ProgressView().controlSize(.mini)
                Text("正在加载历史…")
                    .padFont(size: 11, design: .monospaced)
                    .foregroundStyle(PADWorkbenchStyle.muted)
            }
            .frame(maxWidth: .infinity, alignment: .leading)
            .padding(10)
        } else if model.messages.isEmpty {
            logPlaceholder(
                icon: "chevron.right",
                title: "还没有对话",
                message: "输入消息后点击「发送」提交；多行输入按 Return 只换行。"
            )
        } else {
            LazyVStack(alignment: .leading, spacing: 3) {
                ForEach(model.messages) { item in
                    PADConversationRow(item: item)
                        .id(item.id)
                }
            }
            .padding(.horizontal, 8)
            .padding(.vertical, 6)
            .frame(maxWidth: .infinity, alignment: .leading)
        }
    }

    private func logPlaceholder(icon: String, title: String, message: String) -> some View {
        VStack(alignment: .leading, spacing: 4) {
            HStack(spacing: 6) {
                Image(systemName: icon).font(.system(size: 11))
                Text(title).padFont(size: 12, weight: .semibold, design: .monospaced)
            }
            .foregroundStyle(PADWorkbenchStyle.muted)
            Text(message)
                .padFont(size: 11.5, design: .monospaced)
                .foregroundStyle(PADWorkbenchStyle.muted)
                .fixedSize(horizontal: false, vertical: true)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(12)
    }

    private func scrollToLatest(_ proxy: ScrollViewProxy) {
        guard let last = model.messages.last else { return }
        withAnimation(.easeOut(duration: 0.15)) {
            proxy.scrollTo(last.id, anchor: .bottom)
        }
    }

    // MARK: - 紧凑输入区（约 56–84pt）

    private var composer: some View {
        VStack(spacing: 0) {
            ZStack(alignment: .topLeading) {
                TextEditor(text: $model.draft)
                    .padFont(size: 12, design: .monospaced)
                    .foregroundStyle(PADWorkbenchStyle.text)
                    .scrollContentBackground(.hidden)
                    .padding(.horizontal, 4)
                    .padding(.vertical, 3)
                    .frame(minHeight: 76, maxHeight: 110)
                if model.draft.isEmpty {
                    Text(draftPlaceholder)
                        .padFont(size: 12, design: .monospaced)
                        .foregroundStyle(PADWorkbenchStyle.muted.opacity(0.85))
                        .padding(.top, 7)
                        .padding(.leading, 9)
                        .allowsHitTesting(false)
                }
            }
            .background(PADWorkbenchStyle.input)
            .clipShape(RoundedRectangle(cornerRadius: 4, style: .continuous))
            .overlay(
                RoundedRectangle(cornerRadius: 4, style: .continuous)
                    .stroke(PADWorkbenchStyle.border)
            )
            .disabled(model.selectedTask == nil)
            .padding(.horizontal, 8)
            .padding(.top, 6)
            .padding(.bottom, 4)

            statusRow
        }
        .background(PADWorkbenchStyle.chrome)
        .overlay(alignment: .top) {
            Rectangle().fill(PADWorkbenchStyle.border).frame(height: 1)
        }
    }

    private var statusRow: some View {
        HStack(spacing: 6) {
            modelPicker
            if model.hasOpenAIDiscovery {
                Button(model.openaiRefreshing ? "同步中…" : "同步 OpenAI 模型") {
                    model.syncOpenAIModels()
                }
                .buttonStyle(.plain)
                .padFont(size: 10, weight: .medium)
                .foregroundStyle(model.canSyncOpenAI ? PADWorkbenchStyle.accent : PADWorkbenchStyle.muted)
                .disabled(!model.canSyncOpenAI)
                .help("仅同步当前账号的官方模型元数据；不会登录或发送模型请求")
            }
            PADChromeButton(symbol: "person.crop.circle", help: "账号与登录", action: onOpenAccount)
            PADChromeButton(symbol: "arrow.clockwise", help: "重新连接并加载工作区、任务与历史", action: { model.reload() })

            Text(statusText)
                .padFont(size: 10, design: .monospaced)
                .foregroundStyle(statusColor)
                .lineLimit(1)
                .truncationMode(.middle)

            Spacer(minLength: 4)

            if model.isBusy {
                ProgressView().controlSize(.mini)
                composerButton("停止", systemImage: "stop.fill", style: .destructive, enabled: true) {
                    model.abort()
                }
            }
            composerButton("发送", systemImage: "paperplane.fill", style: .primary, enabled: canSendNow) {
                model.send()
            }
        }
        .padding(.horizontal, 8)
        .padding(.vertical, 6)
    }

    // MARK: - 模型选择

    private var modelPicker: some View {
        Menu {
            if model.hasOpenAIDiscovery {
                Text(openaiDirectoryStatus)
                Button("同步 OpenAI 模型") { model.syncOpenAIModels() }
                    .disabled(!model.canSyncOpenAI)
                Divider()
            }
            if model.catalogLoading && model.catalog.models.isEmpty {
                Text("正在加载模型…")
            } else if model.catalog.models.isEmpty {
                Text("没有可用模型")
            } else {
                ForEach(modelProviderIds, id: \.self) { providerId in
                    Section(modelProviderLabel(providerId)) {
                        ForEach(model.catalog.models.filter { $0.provider == providerId }, id: \.selectionKey) { info in
                            Button {
                                model.setModel(provider: info.provider, modelId: info.id)
                            } label: {
                                if isCurrent(info) {
                                    Label(modelRowLabel(info), systemImage: "checkmark")
                                } else {
                                    Text(modelRowLabel(info))
                                }
                            }
                            .disabled(!info.isSelectable || !model.canConfigureModels)
                        }
                    }
                }
            }
        } label: {
            HStack(spacing: 4) {
                Image(systemName: "cpu")
                    .font(.system(size: 10))
                Text(currentModelLabel)
                    .padFont(size: 10.5, design: .monospaced)
                    .lineLimit(1)
                    .frame(maxWidth: 150, alignment: .leading)
                if model.catalogLoading {
                    ProgressView().controlSize(.mini)
                }
                Image(systemName: "chevron.down")
                    .font(.system(size: 7, weight: .semibold))
            }
            .foregroundStyle(canPickModel ? PADWorkbenchStyle.text : PADWorkbenchStyle.muted)
            .padding(.horizontal, 6)
            .padding(.vertical, 3)
            .background(
                RoundedRectangle(cornerRadius: 4, style: .continuous)
                    .fill(PADWorkbenchStyle.input)
            )
            .overlay(
                RoundedRectangle(cornerRadius: 4, style: .continuous)
                    .stroke(PADWorkbenchStyle.border)
            )
        }
        .menuStyle(.borderlessButton)
        .fixedSize()
        .disabled(!canPickModel)
        .help(modelPickerHelp)
    }

    // Group providers without sorting the upstream account visibility list.
    private var modelProviderIds: [String] {
        model.catalog.models.reduce(into: [String]()) { ids, info in
            if !ids.contains(info.provider) { ids.append(info.provider) }
        }
    }

    private func modelProviderLabel(_ providerId: String) -> String {
        let name = model.catalog.providers.first { $0.id == providerId }?.name ?? providerId
        let accountModels = model.catalog.models.filter { $0.provider == providerId }
        if providerId == "openai", model.hasOpenAIDiscovery,
           accountModels.allSatisfy({ $0.source == "openai_account" }) {
            return "OpenAI 官方账号目录"
        }
        return "\(name) · Pi SDK 目录（非账号权限验证）"
    }

    private func modelRowLabel(_ info: PADModelInfo) -> String {
        let source = info.source == "openai_account" ? "官方账号目录" : "Pi SDK"
        let unsupported = info.isSelectable ? "" : " · 当前 Pi 版本暂不支持"
        return "\(info.name) · \(source)\(unsupported)"
    }

    private var openaiDirectoryStatus: String {
        guard let discovery = model.catalog.openaiDiscovery else { return "" }
        let status: String
        if model.openaiRefreshing {
            status = "正在同步模型元数据…"
        } else {
            switch discovery.state {
            case "fresh": status = "已同步"
            case "stale": status = "缓存已过期，请重新同步"
            case "error": status = "同步失败，请重试"
            default: status = "账号已登录，模型目录尚未同步，请点击「同步 OpenAI 模型」"
            }
        }
        let update = discovery.updatedAt.map { " · 上次更新：\($0)" } ?? ""
        return "OpenAI 官方账号目录 · \(status)\(update)"
    }

    private var currentModelLabel: String {
        guard let task = model.selectedTask else { return "选择模型" }
        if let name = PADWorkbenchUI.modelDisplay(provider: task.provider, modelId: task.modelId, in: model.catalog) {
            return name
        }
        return task.modelId ?? "选择模型"
    }

    private func isCurrent(_ info: PADModelInfo) -> Bool {
        guard let task = model.selectedTask else { return false }
        return task.provider == info.provider && task.modelId == info.id
    }

    private var canPickModel: Bool {
        // Keep the menu readable even when every upstream row is unsupported.
        model.selectedTask != nil && model.canConfigureModels &&
            (!model.catalog.models.isEmpty || model.hasOpenAIDiscovery)
    }

    private var modelPickerHelp: String {
        if model.selectedTask == nil { return "请先选择任务" }
        if model.isBusy { return "任务执行中，停止后可切换模型" }
        if model.hasOpenAIDiscovery { return openaiDirectoryStatus }
        if model.catalogLoading { return "正在加载模型目录" }
        if !model.canConfigureModels { return "账号或任务仍有进行中的操作，暂时无法切换模型" }
        if model.catalog.models.isEmpty { return "当前账号没有可用模型，请检查账号认证与模型目录" }
        return "选择当前任务使用的模型（不会自动替换）"
    }

    // MARK: - 发送 / 状态

    private enum ComposerButtonStyle {
        case primary
        case destructive
    }

    private func composerButton(
        _ title: String,
        systemImage: String,
        style: ComposerButtonStyle,
        enabled: Bool,
        action: @escaping () -> Void
    ) -> some View {
        Button(action: action) {
            HStack(spacing: 4) {
                Image(systemName: systemImage).font(.system(size: 9.5, weight: .semibold))
                Text(title).padFont(size: 11, weight: .medium)
            }
            .foregroundStyle(buttonForeground(style: style, enabled: enabled))
            .padding(.horizontal, 8)
            .padding(.vertical, 3)
            .background(
                RoundedRectangle(cornerRadius: 4, style: .continuous)
                    .fill(buttonBackground(style: style, enabled: enabled))
            )
            .overlay(
                RoundedRectangle(cornerRadius: 4, style: .continuous)
                    .stroke(buttonBorder(style: style, enabled: enabled))
            )
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .disabled(!enabled)
        .help(title == "发送" ? "点击才会请求模型；Return 只换行" : "清空排队消息并中止当前执行，历史保留")
    }

    private func buttonForeground(style: ComposerButtonStyle, enabled: Bool) -> Color {
        guard enabled else { return PADWorkbenchStyle.muted }
        switch style {
        case .primary: return .white
        case .destructive: return .red
        }
    }

    private func buttonBackground(style: ComposerButtonStyle, enabled: Bool) -> Color {
        guard enabled else { return PADWorkbenchStyle.input }
        switch style {
        case .primary: return PADWorkbenchStyle.accent
        case .destructive: return Color.red.opacity(0.14)
        }
    }

    private func buttonBorder(style: ComposerButtonStyle, enabled: Bool) -> Color {
        guard enabled else { return PADWorkbenchStyle.border }
        switch style {
        case .primary: return Color.clear
        case .destructive: return Color.red.opacity(0.45)
        }
    }

    private var draftPlaceholder: String {
        if model.selectedTask == nil { return "请先选择或新建任务" }
        return "输入消息…（Return 换行，点击「发送」提交）"
    }

    private var canSendNow: Bool {
        guard model.selectedTask != nil else { return false }
        guard !PADWorkbenchUI.trimmed(model.draft).isEmpty else { return false }
        return model.canSend
    }

    private var sendDisabledReason: String {
        if model.selectedTask == nil { return "请先选择任务" }
        if PADWorkbenchUI.trimmed(model.draft).isEmpty { return "输入内容后才能发送" }
        if model.isBusy { return "任务执行中，请先停止或等待完成" }
        if model.openaiRefreshing { return "正在同步 OpenAI 模型" }
        if model.hasOpenAIDiscovery && model.catalog.openaiDiscovery?.state == "not_loaded" {
            return "账号已登录，请先同步 OpenAI 模型"
        }
        if let task = model.selectedTask,
           let info = model.catalog.models.first(where: { $0.provider == task.provider && $0.id == task.modelId }),
           !info.isSelectable { return "当前 Pi 版本暂不支持所选模型" }
        return "不可发送：请检查账号状态并选择可用模型"
    }

    private enum ComposerStatus {
        case noTask
        case offline
        case busy
        case ready
        case blocked
    }

    private var composerStatus: ComposerStatus {
        if model.selectedTask == nil { return .noTask }
        if PADWorkbenchUI.isOffline(model.connectionStatus) { return .offline }
        if model.isBusy { return .busy }
        if model.canSend { return .ready }
        if PADWorkbenchUI.trimmed(model.draft).isEmpty { return .ready }
        return .blocked
    }

    private var statusText: String {
        switch composerStatus {
        case .noTask: return "未选择任务"
        case .offline: return "宿主离线 · 请重新加载"
        case .busy: return "执行中 · 可停止"
        case .ready: return "Return 换行 · 点击发送提交"
        case .blocked: return sendDisabledReason
        }
    }

    private var statusColor: Color {
        switch composerStatus {
        case .noTask, .ready: return PADWorkbenchStyle.muted
        case .offline: return .red
        case .busy: return PADWorkbenchStyle.accent
        case .blocked: return .orange
        }
    }
}

// MARK: - 日志行

/// 密集左对齐日志行：统一左侧角色 / 状态标记；用户与工具行仅用薄底色矩形，
/// 不使用左右气泡或大圆角。
private struct PADConversationRow: View {
    let item: PADChatItem

    private enum Kind {
        case user
        case assistant
        case tool
        case notice
    }

    private var kind: Kind {
        switch item.role.lowercased() {
        case "user", "human": return .user
        case "tool", "tool_call", "tool_result", "toolcall", "tool_use": return .tool
        case "system", "error", "notice", "status": return .notice
        default: return .assistant
        }
    }

    var body: some View {
        HStack(alignment: .top, spacing: 6) {
            marker
                .frame(width: 12, alignment: .leading)
                .padding(.top, 1)
            content
            Spacer(minLength: 0)
        }
        .padding(.horizontal, 8)
        .padding(.vertical, kind == .notice ? 2 : 5)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(rowBackground)
    }

    @ViewBuilder
    private var marker: some View {
        switch kind {
        case .user:
            Image(systemName: "chevron.right")
                .font(.system(size: 10, weight: .bold))
                .foregroundStyle(PADWorkbenchStyle.accent)
        case .assistant:
            Image(systemName: "circle.fill")
                .font(.system(size: 5))
                .foregroundStyle(PADWorkbenchStyle.muted)
        case .tool:
            Image(systemName: "wrench.and.screwdriver")
                .font(.system(size: 10))
                .foregroundStyle(
                    PADWorkbenchUI.isToolStatusActive(item.status)
                        ? PADWorkbenchStyle.accent
                        : PADWorkbenchStyle.muted
                )
        case .notice:
            Image(systemName: item.isError ? "exclamationmark.triangle.fill" : "info.circle")
                .font(.system(size: 9.5))
                .foregroundStyle(item.isError ? Color.red : PADWorkbenchStyle.muted)
        }
    }

    @ViewBuilder
    private var content: some View {
        switch kind {
        case .user:
            Text(item.text.isEmpty ? "（空消息）" : item.text)
                .padFont(size: 12, design: .monospaced)
                .foregroundStyle(PADWorkbenchStyle.text)
                .textSelection(.enabled)
                .fixedSize(horizontal: false, vertical: true)
                .frame(maxWidth: .infinity, alignment: .leading)

        case .assistant:
            VStack(alignment: .leading, spacing: 2) {
                if item.text.isEmpty {
                    HStack(spacing: 5) {
                        if PADWorkbenchUI.isToolStatusActive(item.status) {
                            ProgressView().controlSize(.mini)
                        }
                        Text(item.isError ? "模型返回错误" : "等待模型输出…")
                            .padFont(size: 11.5, design: .monospaced)
                            .foregroundStyle(PADWorkbenchStyle.muted)
                    }
                } else {
                    Text(item.text)
                        .padFont(size: 12, design: .monospaced)
                        .foregroundStyle(item.isError ? Color.red : PADWorkbenchStyle.text)
                        .textSelection(.enabled)
                        .fixedSize(horizontal: false, vertical: true)
                        .frame(maxWidth: .infinity, alignment: .leading)
                }
                if let status = PADWorkbenchUI.toolStatusLabel(item.status), !item.text.isEmpty {
                    Text(status)
                        .padFont(size: 10, design: .monospaced)
                        .foregroundStyle(PADWorkbenchUI.toolStatusTint(item.status))
                }
            }

        case .tool:
            VStack(alignment: .leading, spacing: 2) {
                HStack(spacing: 5) {
                    Text(item.toolName ?? "工具调用")
                        .padFont(size: 11.5, weight: .semibold, design: .monospaced)
                        .foregroundStyle(PADWorkbenchStyle.text)
                    if let label = PADWorkbenchUI.toolStatusLabel(item.status) {
                        Text(label)
                            .padFont(size: 10, design: .monospaced)
                            .foregroundStyle(PADWorkbenchUI.toolStatusTint(item.status))
                    }
                    if PADWorkbenchUI.isToolStatusActive(item.status) {
                        ProgressView().controlSize(.mini)
                    }
                }
                if !item.text.isEmpty {
                    Text(item.text)
                        .padFont(size: 11.5, design: .monospaced)
                        .foregroundStyle(PADWorkbenchStyle.muted)
                        .textSelection(.enabled)
                        .lineLimit(14)
                        .fixedSize(horizontal: false, vertical: true)
                        .frame(maxWidth: .infinity, alignment: .leading)
                }
            }

        case .notice:
            Text(item.text.isEmpty ? "状态更新" : item.text)
                .padFont(size: 11, design: .monospaced)
                .foregroundStyle(item.isError ? Color.red : PADWorkbenchStyle.muted)
                .fixedSize(horizontal: false, vertical: true)
                .frame(maxWidth: .infinity, alignment: .leading)
        }
    }

    @ViewBuilder
    private var rowBackground: some View {
        switch kind {
        case .user:
            RoundedRectangle(cornerRadius: 3, style: .continuous)
                .fill(PADWorkbenchStyle.input)
        case .tool:
            RoundedRectangle(cornerRadius: 3, style: .continuous)
                .fill(PADWorkbenchStyle.input.opacity(0.55))
        case .assistant, .notice:
            Color.clear
        }
    }
}
#endif
