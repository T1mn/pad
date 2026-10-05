#if os(macOS)
import AppKit
import SwiftUI

/// Explicit local-history entry point. Closing stops refreshes; scans never run
/// merely because the workbench launched. Source files are always read-only.
struct PADLocalSessionsView: View {
    @ObservedObject var workbench: PADWorkbenchModel
    @StateObject private var local: PADLocalSessionsModel
    let openSession: (PADTerminalLaunch) -> Void
    @Environment(\.dismiss) private var dismiss
    @AppStorage("pad.terminal.localSessionAutoRefresh") private var autoRefresh = true
    @State private var query = ""
    @State private var source = "all"
    @State private var currentProjectOnly = false
    @State private var pending: PADLocalSession?
    @State private var importing = false
    @FocusState private var searchFocused: Bool
    private let timer = Timer.publish(every: 15, on: .main, in: .common).autoconnect()

    init(workbench: PADWorkbenchModel, openSession: @escaping (PADTerminalLaunch) -> Void) {
        self.workbench = workbench
        self.openSession = openSession
        _local = StateObject(wrappedValue: PADLocalSessionsModel(workbench: workbench))
    }

    private var filtered: [PADLocalSession] {
        let terms = query.split(whereSeparator: \.isWhitespace)
        return local.sessions.filter { session in
            (source == "all" || session.tool == source)
                && (!currentProjectOnly || session.cwd == workbench.selectedWorkspace?.path)
                && terms.allSatisfy { term in
                    "\(session.title) \(session.cwd) \(session.sessionId) \(session.toolName)".localizedStandardContains(String(term))
                }
        }
    }

    var body: some View {
        VStack(spacing: 0) {
            HStack {
                Label("本地会话 · Codex / Pi", systemImage: "clock.arrow.circlepath").padFont(size: 13, weight: .semibold)
                Spacer()
                PADTextSizeMenu()
                Button("完成") { dismiss() }.keyboardShortcut(.cancelAction)
            }.padding(12)
            Divider()
            searchBar
            Divider()
            if let error = local.error {
                Text(error).foregroundStyle(.red).textSelection(.enabled)
                    .frame(maxWidth: .infinity, alignment: .leading).padding(10)
                Divider()
            }
            HSplitView {
                sessionList.frame(minWidth: 300, idealWidth: 340, maxWidth: 420)
                detail.frame(minWidth: 460, maxWidth: .infinity, maxHeight: .infinity)
            }
            Divider()
            footer
        }
        .padFont()
        .foregroundStyle(PADWorkbenchStyle.text)
        .background(PADWorkbenchStyle.canvas)
        .frame(width: 1060, height: min(720, max(480, (NSScreen.main?.visibleFrame.height ?? 880) - 160)))
        .onAppear { local.refresh(); searchFocused = true }
        .onDisappear { local.close() }
        .onReceive(timer) { _ in if autoRefresh && NSApp.isActive { local.refresh() } }
        .onReceive(NotificationCenter.default.publisher(for: NSApplication.didBecomeActiveNotification)) { _ in
            if autoRefresh { local.refresh() }
        }
        .onChange(of: query) { _ in clearHiddenSelection() }
        .onChange(of: source) { _ in clearHiddenSelection() }
        .onChange(of: currentProjectOnly) { _ in clearHiddenSelection() }
        .confirmationDialog(importing ? "导入 Pi 历史副本？" : "用原工具继续会话？",
                            isPresented: Binding(get: { pending != nil }, set: { if !$0 { pending = nil } }),
                            titleVisibility: .visible) {
            if let session = pending {
                Button(importing ? "导入到默认账号" : "在新终端中继续") {
                    if importing { local.importPi(session) { dismiss() } }
                    else { local.resume(session) { launch in openSession(launch); dismiss() } }
                    pending = nil
                }
            }
            Button("取消", role: .cancel) { pending = nil }
        } message: {
            Text(importing
                 ? "复制完整 Pi 会话树到 PAD，不修改原文件，不导入凭据。副本不是双向同步；副本使用默认账号，继续前需在默认账号登录并选模型。建议先停止原会话写入。"
                 : "请先停止其他终端中的同一会话，避免并发写入。将以原工具及其本地账号/配置运行（可能加载项目扩展），不自动发送消息，不向已有终端注入命令。")
        }
    }

    private var searchBar: some View {
        VStack(spacing: 10) {
            HStack {
                Image(systemName: "magnifyingglass")
                TextField("搜索会话标题、项目路径或 Session ID", text: $query)
                    .textFieldStyle(.plain).focused($searchFocused)
                if !query.isEmpty { Button { query = "" } label: { Image(systemName: "xmark.circle.fill") }.buttonStyle(.plain) }
            }.padding(9).background(PADWorkbenchStyle.input).cornerRadius(5)
            HStack {
                Picker("来源", selection: $source) {
                    Text("全部").tag("all"); Text("Codex").tag("codex")
                    Text("Pi").tag("pi")
                }.pickerStyle(.segmented).frame(width: 360)
                Toggle("当前项目", isOn: $currentProjectOnly).toggleStyle(.checkbox)
                    .disabled(workbench.selectedWorkspace == nil)
                Spacer()
                if local.refreshing { ProgressView().controlSize(.small) }
                Button("刷新") { local.refresh() }.disabled(local.refreshing)
            }
        }.padding(12)
    }

    private var sessionList: some View {
        ScrollView {
            LazyVStack(alignment: .leading, spacing: 4) {
                Text("\(filtered.count) 个会话").padFont(size: 10).foregroundStyle(PADWorkbenchStyle.muted)
                if filtered.isEmpty {
                    Text(local.refreshing ? "正在读取本地会话索引…" : "没有匹配的会话。可清空搜索或检查底部来源目录状态。")
                        .padding(.vertical, 14).foregroundStyle(PADWorkbenchStyle.muted)
                }
                ForEach(filtered) { session in
                    Button { local.select(session.id) } label: {
                        VStack(alignment: .leading, spacing: 5) {
                            Text(session.title).padFont(size: 12, weight: .medium).lineLimit(2)
                            HStack {
                                Text(session.toolName)
                                Spacer()
                                Text(session.updatedAt.prefix(16).replacingOccurrences(of: "T", with: " ") + " UTC")
                            }.padFont(size: 10)
                            Text(session.cwd.isEmpty ? "未记录项目目录" : session.cwd)
                                .padFont(size: 10).lineLimit(1).truncationMode(.middle)
                        }
                        .foregroundStyle(local.selectedId == session.id ? .white : PADWorkbenchStyle.text)
                        .padding(10).frame(maxWidth: .infinity, alignment: .leading)
                        .background(local.selectedId == session.id ? PADWorkbenchStyle.accent : Color.clear)
                        .cornerRadius(5).contentShape(Rectangle())
                    }.buttonStyle(.plain)
                }
            }.padding(8)
        }.background(PADWorkbenchStyle.sidebar)
    }

    @ViewBuilder private var detail: some View {
        if let session = local.selected {
            VStack(alignment: .leading, spacing: 10) {
                Text(session.title).padFont(size: 13, weight: .semibold).lineLimit(2)
                Text("\(session.toolName) · \(session.sessionId)").padFont(size: 10, design: .monospaced)
                    .foregroundStyle(PADWorkbenchStyle.muted).textSelection(.enabled)
                HStack {
                    Button("在原工具中继续…") { importing = false; pending = session }
                    if session.tool == "pi" {
                        Button("导入到 PAD 原生面板…") { importing = true; pending = session }
                            .disabled(workbench.defaultProfileId == nil)
                    }
                }.disabled(local.acting || local.loadingHistory)
                Text("只读同步 Codex / Pi 源会话；不会把 Codex 的执行状态转换成 Pi。")
                    .padFont(size: 10).foregroundStyle(PADWorkbenchStyle.muted)
                Divider()
                if local.loadingHistory { ProgressView("读取历史…") }
                if local.historyTruncated {
                    Text("历史较长，仅预览最近 200 条 / 8 MiB；完整记录仍在原工具中。")
                        .padFont(size: 10).foregroundStyle(.orange)
                }
                ScrollView {
                    LazyVStack(alignment: .leading, spacing: 14) {
                        ForEach(local.messages) { message in
                            VStack(alignment: .leading, spacing: 5) {
                                Text(message.role).padFont(size: 10, weight: .semibold)
                                    .foregroundStyle(PADWorkbenchStyle.accent)
                                Text(message.text).padFont(size: 12, design: .monospaced)
                                    .textSelection(.enabled).fixedSize(horizontal: false, vertical: true)
                            }.frame(maxWidth: .infinity, alignment: .leading)
                        }
                    }.padding(.vertical, 8)
                }
            }.padding(12)
        } else {
            VStack(alignment: .leading, spacing: 12) {
                Text("继续你的本地工作").padFont(size: 16, weight: .semibold)
                Text("左侧选择一个会话即可预览历史，无需先在 PAD 登录。")
                Text("原工具续接使用它原来的会话和账号；Pi 也可以复制到 PAD 的原生对话面板。浏览与刷新不会调用模型，不读取或复制账号密钥。")
                    .foregroundStyle(PADWorkbenchStyle.muted)
            }.padding(24).frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        }
    }

    private var footer: some View {
        VStack(alignment: .leading, spacing: 6) {
            HStack {
                Toggle("此面板打开时自动同步（15 秒）", isOn: $autoRefresh).toggleStyle(.checkbox)
                Spacer()
                if let date = local.lastSync { Text("上次同步 \(date.formatted(date: .omitted, time: .standard))") }
            }
            Text(local.roots.map { "\($0.tool): \($0.status == "ready" ? "已读取" : $0.status == "missing" ? "目录不存在" : "部分不可读")" }.joined(separator: "   ·   "))
                .help(local.roots.map { $0.path }.joined(separator: "\n"))
            if local.listTruncated { Text("本地会话很多，扫描/显示已达上限；部分历史未列出。可用原 CLI 的 resume 查看全部。 ").foregroundStyle(.orange) }
        }.padFont(size: 10).foregroundStyle(PADWorkbenchStyle.muted).padding(12)
    }

    private func clearHiddenSelection() {
        if let id = local.selectedId, !filtered.contains(where: { $0.id == id }) { local.clearSelection() }
    }
}
#endif
