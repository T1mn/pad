#if os(macOS)
import SwiftUI

/// 右上：PAD Agent 会话。cmux 风格窄 tab strip + 左对齐日志流 + 紧凑输入区。
///
/// 只通过共享 Model 工作：不持有进程、不读写凭据、不自动发送；所有请求都由
/// 用户点击「发送」或在输入区按 Enter 触发；Shift+Enter 换行，IME 确认不发送。
struct PADConversationView: View {
    @ObservedObject var model: PADWorkbenchModel
    var onOpenAccount: () -> Void

    @State private var dismissedError: String?
    @State private var modelPickerPresented = false
    @State private var capturedModelContext: ModelPickerContext?

    private struct ModelPickerContext: Equatable {
        let taskId: String?
        let profileId: String?
        let catalogData: Data?
        let loading: Bool
        let connection: String
    }

    private var modelPickerContext: ModelPickerContext {
        let encoder = JSONEncoder()
        encoder.outputFormatting = .sortedKeys
        return ModelPickerContext(taskId: model.selectedTaskId, profileId: model.activeProfileId,
                                  catalogData: try? encoder.encode(model.catalog),
                                  loading: model.catalogLoading, connection: model.connectionStatus)
    }

    var body: some View {
        VStack(spacing: 0) {
            tabStrip
            notices
            transcript
            composer
        }
        .background(PADWorkbenchStyle.canvas)
        .onChange(of: modelPickerContext) { _ in
            modelPickerPresented = false
            capturedModelContext = nil
        }
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
            Button { model.openSessionInfo() } label: {
                Label("会话信息", systemImage: "info.circle")
                    .padFont(size: 10.5)
            }
            .buttonStyle(.plain)
            .help("查看 Pi 会话 ID、PAD 任务 ID 与本地路径")
            .disabled(model.selectedTask == nil || model.connectionStatus != "ready")
            .padding(.horizontal, 10)
            .popover(isPresented: Binding(
                get: { model.sessionInfoPresented },
                set: { if !$0 { model.closeSessionInfo() } }
            )) {
                PADSessionInfoView(model: model)
            }
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
                message: "输入消息后按 Enter 或点击「发送」提交；Shift+Enter 换行。"
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
                PADComposerInput(
                    text: $model.draft,
                    isEditable: model.selectedTask != nil,
                    canSend: { model.canSend },
                    onSend: { model.send() }
                )
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
        PADComposerControlLayout(spacing: 6) {
            modelPicker
            thinkingPicker
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
                .fixedSize(horizontal: false, vertical: true)

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
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(.horizontal, 8)
        .padding(.vertical, 6)
    }

    // MARK: - 推理选择（只使用当前模型声明的能力）

    private func thinkingLabel(_ level: String) -> String {
        switch level {
        case "off": return "关闭"
        case "minimal": return "最低"
        case "low": return "低"
        case "medium": return "中"
        case "high": return "高"
        case "xhigh": return "超高"
        case "max": return "最大"
        default: return "未知（\(level)）"
        }
    }

    private var currentThinkingLabel: String {
        guard let task = model.selectedTask else { return "未选择任务" }
        guard task.modelId != nil else { return "未选择模型" }
        guard !model.thinkingLevels.isEmpty else { return "能力未知" }
        if model.thinkingLevels == ["off"] { return "不支持推理 · 关闭" }
        // nil is inherited / not yet confirmed, never an implicit medium.
        if let level = task.thinkingLevel { return thinkingLabel(level) }
        if model.thinkingLevels.count == 1, let level = model.thinkingLevels.first {
            return "\(thinkingLabel(level)) · 跟随会话"
        }
        return "跟随会话"
    }

    private var thinkingPicker: some View {
        Menu {
            ForEach(model.thinkingLevels, id: \.self) { level in
                Button {
                    model.setThinkingLevel(level)
                } label: {
                    if model.selectedTask?.thinkingLevel == level {
                        Label(thinkingLabel(level), systemImage: "checkmark")
                    } else {
                        Text(thinkingLabel(level))
                    }
                }
            }
        } label: {
            HStack(spacing: 4) {
                Text("推理：\(currentThinkingLabel)")
                    .padFont(size: 10.5)
                    .fixedSize(horizontal: false, vertical: true)
                if model.thinkingLevels.count > 1 {
                    Image(systemName: "chevron.down")
                        .font(.system(size: 7, weight: .semibold))
                }
            }
            .foregroundStyle(model.canConfigureThinking ? PADWorkbenchStyle.text : PADWorkbenchStyle.muted)
            .padding(.horizontal, 6)
            .padding(.vertical, 3)
            .background(RoundedRectangle(cornerRadius: 4).fill(PADWorkbenchStyle.input))
            .overlay(RoundedRectangle(cornerRadius: 4).stroke(PADWorkbenchStyle.border))
        }
        .menuStyle(.borderlessButton)
        .menuIndicator(.hidden)
        .disabled(!model.canConfigureThinking)
        .help(thinkingPickerHelp)
    }

    private var thinkingPickerHelp: String {
        if model.selectedTask == nil { return "请先选择任务" }
        if model.thinkingLevels.isEmpty { return "当前模型没有已确认的推理能力信息" }
        if model.thinkingLevels == ["off"] { return "当前模型不支持推理，推理关闭" }
        if model.thinkingLevels.count == 1 { return "当前模型仅支持固定推理等级" }
        if !model.canConfigureThinking { return "账号或任务仍有进行中的操作，暂时无法切换推理等级" }
        return "设置当前任务的推理等级；跟随会话表示尚未确认，不代表默认中。与 Fast / service_tier 无关。"
    }

    // MARK: - 模型选择

    private var modelPicker: some View {
        Button {
            capturedModelContext = modelPickerContext
            modelPickerPresented = true
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
        .buttonStyle(.plain)
        .help(modelPickerHelp)
        .popover(isPresented: $modelPickerPresented, arrowEdge: .bottom) {
            PADModelPicker(
                catalog: model.catalog,
                selectedProvider: model.selectedTask?.provider,
                selectedModelId: model.selectedTask?.modelId,
                canSelect: model.selectedTask != nil && model.canConfigureModels,
                loading: model.catalogLoading,
                directoryStatus: openaiDirectoryStatus,
                showsOpenAISync: model.hasOpenAIDiscovery,
                canSyncOpenAI: model.canSyncOpenAI,
                onSyncOpenAI: {
                    guard capturedModelContext == modelPickerContext else { return }
                    model.syncOpenAIModels()
                },
                onSelect: { info in
                    guard capturedModelContext == modelPickerContext,
                          capturedModelContext?.taskId != nil,
                          model.canConfigureModels, info.isSelectable else { return }
                    modelPickerPresented = false
                    model.setModel(provider: info.provider, modelId: info.id)
                }
            )
        }
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

    private var canPickModel: Bool {
        // Browsing and explicit sync remain reachable without a selected task.
        model.selectedTask != nil && model.canConfigureModels
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
        .help(title == "发送" ? "Enter / ⌘Enter 发送；Shift+Enter 换行；输入法确认不发送" : "清空排队消息并中止当前执行，历史保留")
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
        return "输入消息…（Enter 发送，Shift+Enter 换行）"
    }

    private var canSendNow: Bool {
        guard model.selectedTask != nil else { return false }
        guard !PADWorkbenchUI.trimmed(model.draft).isEmpty else { return false }
        return model.canSend
    }

    private var sendDisabledReason: String {
        if model.selectedTask == nil { return "请先选择任务" }
        if model.isBusy { return "任务执行中，请先停止或等待完成" }
        if model.isConfiguringTask { return "正在更新模型 / 推理设置" }
        if PADWorkbenchUI.trimmed(model.draft).isEmpty { return "输入内容后才能发送" }
        if model.openaiRefreshing { return "正在同步 OpenAI 模型" }
        if model.hasOpenAIDiscovery && model.catalog.openaiDiscovery?.state == "not_loaded" {
            return "账号已登录，请先同步 OpenAI 模型"
        }
        if let task = model.selectedTask,
           let info = model.catalog.models.first(where: { $0.provider == task.provider && $0.id == task.modelId }),
           !info.isSelectable { return "当前版本暂不支持所选模型" }
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
        if model.isConfiguringTask { return .blocked }
        if model.canSend { return .ready }
        if PADWorkbenchUI.trimmed(model.draft).isEmpty { return .ready }
        return .blocked
    }

    private var statusText: String {
        switch composerStatus {
        case .noTask: return "未选择任务"
        case .offline: return "宿主离线 · 请重新加载"
        case .busy: return "执行中 · 可停止"
        case .ready: return "Enter 发送 · Shift+Enter 换行"
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

/// Wrap whole controls rather than squeezing a growing horizontal toolbar.
/// Intrinsic labels remain readable at all supported text sizes and pane widths.
private struct PADComposerControlLayout: Layout {
    let spacing: CGFloat

    private func arrange(_ subviews: Subviews, width: CGFloat) -> (size: CGSize, origins: [CGPoint], sizes: [CGSize]) {
        var origins: [CGPoint] = []
        var sizes: [CGSize] = []
        var x: CGFloat = 0
        var y: CGFloat = 0
        var rowHeight: CGFloat = 0
        var usedWidth: CGFloat = 0
        for subview in subviews {
            let intrinsic = subview.sizeThatFits(.unspecified)
            let size = subview.sizeThatFits(ProposedViewSize(width: min(intrinsic.width, width), height: nil))
            if x > 0 && x + size.width > width {
                x = 0
                y += rowHeight + spacing
                rowHeight = 0
            }
            origins.append(CGPoint(x: x, y: y))
            sizes.append(size)
            usedWidth = max(usedWidth, x + size.width)
            x += size.width + spacing
            rowHeight = max(rowHeight, size.height)
        }
        return (CGSize(width: usedWidth, height: y + rowHeight), origins, sizes)
    }

    func sizeThatFits(proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) -> CGSize {
        arrange(subviews, width: proposal.width ?? .infinity).size
    }

    func placeSubviews(in bounds: CGRect, proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) {
        let layout = arrange(subviews, width: bounds.width)
        for index in subviews.indices {
            let origin = layout.origins[index]
            subviews[index].place(
                at: CGPoint(x: bounds.minX + origin.x, y: bounds.minY + origin.y),
                anchor: .topLeading,
                proposal: ProposedViewSize(width: layout.sizes[index].width, height: layout.sizes[index].height)
            )
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
