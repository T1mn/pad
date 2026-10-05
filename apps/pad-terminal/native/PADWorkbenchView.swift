#if os(macOS)
import AppKit
import SwiftUI

/// PAD 原生工作台入口（UI 结构参照公开 cmux 截图独立实现，不变更 PAD 任务语义）。
/// 顶部 34pt 一体化 bar 预留真实红黄绿灯位；左侧全高深炭色侧栏；主体左右平铺
/// PAD Agent 与 Ghostty 终端。终端视图结构恒定、无 `.id`，隐藏侧栏只改宽度/透明度。
struct PADWorkbenchView<TerminalContent: View>: View {
    private enum Sheet: String, Identifiable {
        case account, newTask, localSessions
        var id: String { rawValue }
    }

    @ObservedObject private var model = PADWorkbenchModel.shared
    private let openTerminal: (String) -> Void
    private let splitTerminal: (Bool) -> Void
    private let openSession: (PADTerminalLaunch) -> Void
    private let terminalContent: () -> TerminalContent

    @State private var activeSheet: Sheet?
    @State private var didStart = false
    @State private var sidebarVisible = true
    @State private var showActivity = false

    init(
        openTerminal: @escaping (String) -> Void,
        splitTerminal: @escaping (Bool) -> Void = { _ in },
        openSession: @escaping (PADTerminalLaunch) -> Void = { _ in },
        @ViewBuilder terminal: @escaping () -> TerminalContent
    ) {
        self.openTerminal = openTerminal
        self.splitTerminal = splitTerminal
        self.openSession = openSession
        self.terminalContent = terminal
    }

    var body: some View {
        VStack(spacing: 0) {
            topBar
            hline
            HStack(spacing: 0) {
                sidebar
                    .frame(width: sidebarVisible ? PADWorkbenchStyle.sidebarWidth : 0)
                    .opacity(sidebarVisible ? 1 : 0)
                    .clipped()
                    .allowsHitTesting(sidebarVisible)
                vline
                    .frame(width: sidebarVisible ? 1 : 0)
                    .opacity(sidebarVisible ? 1 : 0)
                mainColumn
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity)
        }
        .background(PADWorkbenchStyle.canvas)
        .ignoresSafeArea(.container, edges: .top)
        .frame(minWidth: 1080, minHeight: 640)
        .onAppear {
            guard !didStart else { return }
            didStart = true
            model.start()
        }
        .sheet(item: $activeSheet, content: sheetContent)
    }

    @ViewBuilder
    private func sheetContent(_ sheet: Sheet) -> some View {
        switch sheet {
        case .account:
            PADAccountView(model: model)
        case .localSessions:
            PADLocalSessionsView(workbench: model, openSession: openSession)
        case .newTask:
            PADTextPromptSheet(
                title: "新建任务",
                message: "任务会绑定当前项目与默认账号，创建后不可更改。",
                placeholder: "任务名称（可留空）", confirmLabel: "创建", allowsEmpty: true
            ) { model.createTask(title: $0) }
        }
    }

    private var hline: some View { Rectangle().fill(PADWorkbenchStyle.border).frame(height: 1) }
    private var vline: some View { Rectangle().fill(PADWorkbenchStyle.border).frame(maxHeight: .infinity) }

    private var topBar: some View {
        HStack(spacing: 0) {
            HStack(spacing: 6) {
                // Genuine macOS traffic lights occupy this leading area.
                Color.clear.frame(width: 80)
                PADChromeButton(symbol: "sidebar.left",
                                help: sidebarVisible ? "隐藏侧栏" : "显示侧栏",
                                active: sidebarVisible) { sidebarVisible.toggle() }
                PADChromeButton(symbol: "waveform.path.ecg",
                                help: "任务活动（仅展示现有任务状态）",
                                active: showActivity) { showActivity.toggle() }
                    .popover(isPresented: $showActivity, arrowEdge: .bottom) { activityPopover }
                newMenu
                PADTextSizeMenu()
                Spacer(minLength: 0)
            }
            .padding(.horizontal, 8)
            .frame(width: PADWorkbenchStyle.sidebarWidth)
            vline.frame(width: 1)
            titleCluster
                .padding(.horizontal, 10)
                .frame(maxWidth: .infinity, alignment: .leading)
        }
        .frame(height: PADWorkbenchStyle.barHeight)
        .background(PADWorkbenchStyle.chrome)
    }

    private var newMenu: some View {
        Menu {
            Button("选择目录…") { chooseDirectory() }
            Button("本地会话…") { activeSheet = .localSessions }
            Button("新建任务…") { activeSheet = .newTask }
                .disabled(model.selectedWorkspace == nil || model.defaultProfileId == nil)
            Divider()
            Button("账号与登录…") { activeSheet = .account }
        } label: {
            Image(systemName: "plus")
                .font(.system(size: 12))
                .foregroundStyle(PADWorkbenchStyle.muted)
                .frame(width: 24, height: 24).contentShape(Rectangle())
        }
        .menuStyle(.borderlessButton).menuIndicator(.hidden).fixedSize()
        .help("新建：项目 / 任务 / 账号")
    }

    private var titleCluster: some View {
        HStack(spacing: 6) {
            Image(systemName: "folder").font(.system(size: 11))
                .foregroundStyle(PADWorkbenchStyle.muted)
            Text(PADWorkbenchUI.workspaceName(model.selectedWorkspaceId, in: model.snapshot.workspaces))
                .padFont(size: 11.5, weight: .medium)
                .foregroundStyle(PADWorkbenchStyle.text)
                .lineLimit(1).truncationMode(.middle)
            if let task = model.selectedTask {
                Text("/").font(.system(size: 11)).foregroundStyle(PADWorkbenchStyle.muted)
                Text(PADWorkbenchUI.taskTitle(task))
                    .padFont(size: 11.5).foregroundStyle(PADWorkbenchStyle.text)
                    .lineLimit(1).truncationMode(.middle)
            }
        }
        .help(model.selectedWorkspace?.path ?? "未选择项目")
    }

    private var activityPopover: some View {
        VStack(alignment: .leading, spacing: 0) {
            Text("任务活动")
                .padFont(size: 11, weight: .semibold)
                .foregroundStyle(PADWorkbenchStyle.text)
                .padding(.horizontal, 10).padding(.vertical, 6)
            hline
            if model.snapshot.tasks.isEmpty {
                Text("暂无任务").padFont(size: 11)
                    .foregroundStyle(PADWorkbenchStyle.muted).padding(10)
            } else {
                ScrollView {
                    VStack(alignment: .leading, spacing: 2) {
                        ForEach(sortedTasks.prefix(20)) { task in
                            Button { model.selectTask(task.id); showActivity = false } label: {
                                HStack(spacing: 6) {
                                    Circle().fill(PADWorkbenchUI.taskStatusColor(task.status))
                                        .frame(width: 6, height: 6)
                                    Text(PADWorkbenchUI.taskTitle(task))
                                        .padFont(size: 11)
                                        .foregroundStyle(PADWorkbenchStyle.text).lineLimit(1)
                                    Spacer(minLength: 8)
                                    Text(PADWorkbenchUI.taskStatusLabel(task.status))
                                        .padFont(size: 10)
                                        .foregroundStyle(PADWorkbenchStyle.muted)
                                }
                                .padding(.horizontal, 8).padding(.vertical, 4)
                                .contentShape(Rectangle())
                            }
                            .buttonStyle(.plain)
                        }
                    }
                    .padding(.vertical, 4)
                }
                .frame(maxHeight: 260)
            }
        }
        .frame(width: 280)
        .background(PADWorkbenchStyle.sidebar)
    }

    // MARK: - 左侧栏

    private var sidebar: some View {
        VStack(spacing: 0) {
            sidebarActionRow("本地会话 · Codex / Pi", systemImage: "clock.arrow.circlepath") {
                activeSheet = .localSessions
            }.padding(.horizontal, 6).padding(.vertical, 8)
            hline
            ScrollView {
                LazyVStack(alignment: .leading, spacing: 1) {
                    ForEach(model.snapshot.workspaces) { workspaceGroup($0) }
                    sidebarActionRow("选择目录…", systemImage: "folder.badge.plus",
                                     action: chooseDirectory)
                }
                .padding(.horizontal, 6).padding(.vertical, 6)
            }
            hline
            sidebarFooter
        }
        .background(PADWorkbenchStyle.sidebar)
    }

    @ViewBuilder
    private func workspaceGroup(_ workspace: PADWorkspace) -> some View {
        Button { model.selectWorkspace(workspace.id) } label: {
            HStack(spacing: 5) {
                Image(systemName: "folder").font(.system(size: 9.5))
                Text(workspace.name).padFont(size: 11, weight: .semibold)
                    .lineLimit(1).truncationMode(.middle)
                Spacer(minLength: 0)
            }
            .foregroundStyle(PADWorkbenchStyle.muted)
            .padding(.horizontal, 8).padding(.top, 8).padding(.bottom, 2)
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .help(workspace.path)

        ForEach(tasks(in: workspace)) { taskRow($0) }

        if model.selectedWorkspaceId == workspace.id {
            sidebarActionRow("新建任务…", systemImage: "plus") { activeSheet = .newTask }
        }
    }

    private func taskRow(_ task: PADTask) -> some View {
        let selected = model.selectedTaskId == task.id
        let profile = task.profileId == model.defaultProfileId ? "默认账号"
            : "历史账号 · \(PADWorkbenchUI.profileName(task.profileId, in: model.snapshot.profiles))"
        let path = workspacePath(task.workspaceId)
        return Button { model.selectTask(task.id) } label: {
            VStack(alignment: .leading, spacing: 1) {
                Text(PADWorkbenchUI.taskTitle(task))
                    .padFont(size: 12, weight: selected ? .semibold : .regular)
                    .foregroundStyle(selected ? Color.white : PADWorkbenchStyle.text)
                    .lineLimit(1).truncationMode(.middle)
                HStack(spacing: 4) {
                    Circle()
                        .fill(selected ? Color.white.opacity(0.85)
                                       : PADWorkbenchUI.taskStatusColor(task.status))
                        .frame(width: 5, height: 5)
                    Text("\(PADWorkbenchUI.taskStatusLabel(task.status)) · \(profile)")
                        .padFont(size: 10)
                        .foregroundStyle(selected ? Color.white.opacity(0.9) : PADWorkbenchStyle.muted)
                        .lineLimit(1).truncationMode(.middle)
                }
                Text(path)
                    .padFont(size: 10, design: .monospaced)
                    .foregroundStyle(selected ? Color.white.opacity(0.72)
                                              : PADWorkbenchStyle.muted.opacity(0.8))
                    .lineLimit(1).truncationMode(.middle)
            }
            .frame(maxWidth: .infinity, alignment: .leading)
            .padding(.horizontal, 8).padding(.vertical, 5)
            .background(RoundedRectangle(cornerRadius: 4, style: .continuous)
                .fill(selected ? PADWorkbenchStyle.accent : Color.clear))
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .help(path)
    }

    private func sidebarActionRow(
        _ title: String, systemImage: String, action: @escaping () -> Void
    ) -> some View {
        Button(action: action) {
            HStack(spacing: 6) {
                Image(systemName: systemImage).font(.system(size: 9.5))
                Text(title).padFont(size: 11).lineLimit(1)
                Spacer(minLength: 0)
            }
            .foregroundStyle(PADWorkbenchStyle.muted)
            .padding(.horizontal, 8).padding(.vertical, 4)
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
    }

    /// 底部窄状态行：宿主连接状态 + 当前账号上下文 + 账号入口。
    private var sidebarFooter: some View {
        HStack(spacing: 6) {
            Circle().fill(PADWorkbenchUI.connectionColor(model.connectionStatus))
                .frame(width: 6, height: 6)
                .help("宿主：\(PADWorkbenchUI.connectionLabel(model.connectionStatus))")
            Text(model.accountContextLabel)
                .padFont(size: 11, weight: .medium)
                .foregroundStyle(PADWorkbenchStyle.muted)
                .lineLimit(1).truncationMode(.middle)
                .help(model.isHistoricalAccount ? "当前任务保留原账号；新任务仍使用默认账号。" : "新任务使用默认账号。")
                .contextMenu {
                    Button("默认账号") { model.selectDefaultAccount() }
                        .disabled(model.defaultProfileId == nil)
                }
            Spacer(minLength: 0)
            PADChromeButton(symbol: "arrow.clockwise", help: "重新加载工作区、任务与历史") { model.reload() }
            PADChromeButton(symbol: "person.crop.circle", help: "账号与登录") { activeSheet = .account }
        }
        .padding(.horizontal, 8).frame(height: 36)
        .background(PADWorkbenchStyle.chrome)
    }

    // MARK: - 右侧主列（Agent | Terminal）

    private var mainColumn: some View {
        HSplitView {
            VStack(spacing: 0) {
                if model.isHistoricalAccount {
                    PADUIBanner(systemImage: "info.circle",
                                text: "\(model.accountContextLabel)：模型与登录仍使用该任务的原账号；新任务使用默认账号。",
                                tint: .secondary)
                }
                PADConversationView(model: model) { activeSheet = .account }
            }
            .frame(minWidth: 420, maxWidth: .infinity, maxHeight: .infinity)
            terminalPane
                .frame(minWidth: 320, maxWidth: .infinity, maxHeight: .infinity)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .background(PADWorkbenchStyle.canvas)
    }

    private var terminalPane: some View {
        VStack(spacing: 0) {
            terminalBar
            hline
            // 结构上始终存在、无 `.id`：隐藏侧栏或切换任务都不会销毁 / 重建终端。
            terminalContent().frame(maxWidth: .infinity, maxHeight: .infinity)
        }
        .background(PADWorkbenchStyle.canvas)
    }

    /// 34pt 终端 tab strip：明确这是独立 shell，不声明已切换工作目录。
    private var terminalBar: some View {
        HStack(spacing: 6) {
            HStack(spacing: 5) {
                Image(systemName: "terminal").font(.system(size: 10))
                Text("终端").padFont(size: 11, weight: .semibold)
                Text("独立 shell").padFont(size: 10)
                    .foregroundStyle(PADWorkbenchStyle.muted)
            }
            .foregroundStyle(PADWorkbenchStyle.text)
            .padding(.horizontal, 8)
            .frame(height: PADWorkbenchStyle.barHeight)
            .overlay(alignment: .top) {
                Rectangle().fill(PADWorkbenchStyle.accent).frame(height: 1.5)
            }
            Spacer(minLength: 8)
            PADChromeButton(
                symbol: "plus",
                help: model.selectedWorkspace == nil
                    ? "请先选择项目" : "在所选项目目录中新开终端标签（不改动当前终端）",
                enabled: model.selectedWorkspace != nil
            ) { if let workspace = model.selectedWorkspace { openTerminal(workspace.path) } }
            PADChromeButton(symbol: "rectangle.split.1x2", help: "在下方拆分终端") { splitTerminal(true) }
            PADChromeButton(symbol: "rectangle.split.2x1", help: "在右侧拆分终端") { splitTerminal(false) }
        }
        .padding(.horizontal, 6)
        .frame(height: PADWorkbenchStyle.barHeight)
        .background(PADWorkbenchStyle.chrome)
    }

    // MARK: - 数据派生与动作

    private var sortedTasks: [PADTask] { model.snapshot.tasks.sorted { $0.updatedAt > $1.updatedAt } }

    private func tasks(in w: PADWorkspace) -> [PADTask] {
        model.snapshot.tasks.filter { $0.workspaceId == w.id }.sorted { $0.updatedAt > $1.updatedAt }
    }

    private func workspacePath(_ id: String) -> String {
        model.snapshot.workspaces.first { $0.id == id }?.path ?? "未选择目录"
    }

    private func chooseDirectory() {
        let panel = NSOpenPanel()
        panel.canChooseFiles = false
        panel.canChooseDirectories = true
        panel.allowsMultipleSelection = false
        panel.canCreateDirectories = false
        panel.resolvesAliases = true
        panel.message = "选择要加入 PAD 工作台的项目目录"
        panel.prompt = "选择目录"
        if panel.runModal() == .OK, let url = panel.url {
            model.addWorkspace(path: url.path)
        }
    }
}
#endif
