#if os(macOS)
import AppKit
import SwiftUI

/// 纯 UI 辅助：状态文案、颜色、URL 校验与小尺寸通用视图。
/// 不持有任何凭据，不做持久化，不发网络请求。
enum PADWorkbenchUI {
    static func trimmed(_ value: String) -> String {
        value.trimmingCharacters(in: .whitespacesAndNewlines)
    }

    // MARK: - 名称解析

    static func profileName(_ id: String?, in profiles: [PADProfile]) -> String {
        guard let id, let profile = profiles.first(where: { $0.id == id }) else { return "未选择账号" }
        return profile.name
    }

    static func workspaceName(_ id: String?, in workspaces: [PADWorkspace]) -> String {
        guard let id, let workspace = workspaces.first(where: { $0.id == id }) else { return "未选择工作区" }
        return workspace.name
    }

    static func taskTitle(_ task: PADTask) -> String {
        let title = task.title.trimmingCharacters(in: .whitespacesAndNewlines)
        return title.isEmpty ? "未命名任务" : title
    }

    /// 任务上记录的 provider/modelId 对应的模型显示名；找不到时回退为 modelId。
    static func modelDisplay(provider: String?, modelId: String?, in catalog: PADCatalog) -> String? {
        guard let modelId, !modelId.isEmpty else { return nil }
        if let info = catalog.models.first(where: { $0.id == modelId && (provider == nil || $0.provider == provider) }) {
            return info.name
        }
        return modelId
    }

    static func providerDisplayName(_ provider: String, in catalog: PADCatalog) -> String {
        catalog.providers.first(where: { $0.id == provider })?.name ?? provider
    }

    /// 按 catalog 顺序分组，保留每个 provider 的模型顺序。
    static func groupedModels(_ models: [PADModelInfo], providers: [PADProvider]) -> [PADModelGroup] {
        var order: [String] = []
        var buckets: [String: [PADModelInfo]] = [:]
        for model in models {
            if buckets[model.provider] == nil {
                order.append(model.provider)
                buckets[model.provider] = []
            }
            buckets[model.provider]?.append(model)
        }
        return order.map { provider in
            let name = providers.first(where: { $0.id == provider })?.name ?? provider
            return PADModelGroup(provider: provider, name: name, models: buckets[provider] ?? [])
        }
    }

    // MARK: - 连接状态

    static func connectionLabel(_ status: String) -> String {
        switch status.lowercased() {
        case "": return "未知"
        case "ready", "connected", "online", "ok": return "已连接"
        case "connecting", "starting", "handshake", "restarting": return "正在连接…"
        case "offline", "disconnected", "stopped", "closed": return "离线"
        case "error", "failed", "crashed": return "连接异常"
        default: return status
        }
    }

    static func isOffline(_ status: String) -> Bool {
        ["offline", "disconnected", "stopped", "closed", "error", "failed", "crashed"].contains(status.lowercased())
    }

    static func isConnected(_ status: String) -> Bool {
        ["ready", "connected", "online", "ok"].contains(status.lowercased())
    }

    static func connectionColor(_ status: String) -> Color {
        switch status.lowercased() {
        case "ready", "connected", "online", "ok": return .green
        case "connecting", "starting", "handshake", "restarting": return .orange
        case "offline", "disconnected", "stopped", "closed": return .secondary
        case "error", "failed", "crashed": return .red
        default: return .secondary
        }
    }

    // MARK: - 任务状态

    static func taskStatusLabel(_ status: String) -> String {
        switch status.lowercased() {
        case "idle": return "空闲"
        case "starting": return "启动中"
        case "running": return "运行中"
        case "error": return "错误"
        default: return status.isEmpty ? "未知" : status
        }
    }

    static func taskStatusColor(_ status: String) -> Color {
        switch status.lowercased() {
        case "idle": return .secondary
        case "starting": return .orange
        case "running": return .accentColor
        case "error": return .red
        default: return .secondary
        }
    }

    // MARK: - 工具状态

    static func toolStatusLabel(_ status: String?) -> String? {
        guard let status, !status.isEmpty else { return nil }
        switch status.lowercased() {
        case "running", "in_progress", "inprogress", "started", "pending", "streaming":
            return "运行中"
        case "completed", "complete", "success", "succeeded", "done", "ok", "finished":
            return "已完成"
        case "error", "failed", "failure":
            return "失败"
        case "cancelled", "canceled", "aborted":
            return "已取消"
        default:
            return status
        }
    }

    static func isToolStatusActive(_ status: String?) -> Bool {
        guard let status else { return false }
        return ["running", "in_progress", "inprogress", "started", "pending", "streaming"].contains(status.lowercased())
    }

    static func toolStatusTint(_ status: String?) -> Color {
        guard let status else { return .secondary }
        switch status.lowercased() {
        case "error", "failed", "failure": return .red
        case "cancelled", "canceled", "aborted": return .secondary
        case "completed", "complete", "success", "succeeded", "done", "ok", "finished": return .green
        default: return .accentColor
        }
    }

    // MARK: - 认证状态

    static func authMethodLabel(_ method: String) -> String {
        switch method.lowercased() {
        case "api_key": return "API Key"
        case "oauth": return "OAuth"
        default: return method
        }
    }

    static func authMethodIcon(_ method: String) -> String {
        switch method.lowercased() {
        case "oauth": return "globe"
        default: return "key"
        }
    }

    static func authPhaseLabel(_ phase: String) -> String {
        switch phase.lowercased() {
        case "running": return "进行中"
        case "succeeded": return "成功"
        case "failed": return "失败"
        case "cancelled": return "已取消"
        default: return phase
        }
    }

    // MARK: - 外链校验

    /// 只接受 http/https，且必须有主机名；其余一律忽略，绝不自动打开。
    static func externalURL(_ raw: String?) -> URL? {
        guard let raw = raw?.trimmingCharacters(in: .whitespacesAndNewlines), !raw.isEmpty else { return nil }
        guard let components = URLComponents(string: raw),
              let scheme = components.scheme?.lowercased(),
              scheme == "http" || scheme == "https",
              let host = components.host,
              !host.isEmpty else {
            return nil
        }
        return components.url
    }
}

struct PADModelGroup: Identifiable {
    let provider: String
    let name: String
    let models: [PADModelInfo]
    var id: String { provider }
}

// MARK: - 小尺寸通用视图

struct PADUIBadge: View {
    let text: String
    var tint: Color = .secondary

    var body: some View {
        Text(text)
            .padFont(size: 10, weight: .semibold)
            .padding(.horizontal, 6)
            .padding(.vertical, 2)
            .background(RoundedRectangle(cornerRadius: 3).fill(tint.opacity(0.12)))
            .foregroundStyle(tint)
            .lineLimit(1)
    }
}

struct PADUIBanner: View {
    let systemImage: String
    let text: String
    var tint: Color = .orange
    var actionTitle: String? = nil
    var action: (() -> Void)? = nil

    var body: some View {
        HStack(alignment: .firstTextBaseline, spacing: 8) {
            Image(systemName: systemImage)
                .font(.system(size: 11, weight: .semibold))
            Text(text)
                .padFont(size: 11)
                .fixedSize(horizontal: false, vertical: true)
            Spacer(minLength: 8)
            if let actionTitle, let action {
                Button(actionTitle, action: action)
                    .controlSize(.small)
            }
        }
        .foregroundStyle(tint)
        .padding(.horizontal, 12)
        .padding(.vertical, 7)
        .background(tint.opacity(0.10))
        .overlay(alignment: .bottom) {
            Rectangle().fill(tint.opacity(0.22)).frame(height: 1)
        }
    }
}

struct PADUIEmptyState: View {
    let systemImage: String
    let title: String
    let message: String
    var actionTitle: String? = nil
    var action: (() -> Void)? = nil

    var body: some View {
        VStack(spacing: 8) {
            Image(systemName: systemImage)
                .font(.system(size: 26, weight: .regular))
                .foregroundStyle(.tertiary)
            Text(title)
                .padFont(size: 13, weight: .semibold)
            Text(message)
                .padFont(size: 11)
                .foregroundStyle(.secondary)
                .multilineTextAlignment(.center)
                .fixedSize(horizontal: false, vertical: true)
                .frame(maxWidth: 420)
            if let actionTitle, let action {
                Button(actionTitle, action: action)
                    .controlSize(.small)
                    .padding(.top, 2)
            }
        }
        .frame(maxWidth: .infinity, minHeight: 220, alignment: .center)
        .padding(24)
    }
}

/// 仅持有内存中的输入文本，取消/确认后清空；不做任何持久化。
struct PADTextPromptSheet: View {
    let title: String
    var message: String? = nil
    var placeholder: String = ""
    var confirmLabel: String = "确定"
    var allowsEmpty: Bool = false
    let onSubmit: (String) -> Void

    @Environment(\.dismiss) private var dismiss
    @State private var text: String = ""

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text(title)
                .padFont(size: 13, weight: .semibold)
            if let message {
                Text(message)
                    .padFont(size: 11)
                    .foregroundStyle(.secondary)
                    .fixedSize(horizontal: false, vertical: true)
            }
            TextField(placeholder, text: $text)
                .textFieldStyle(.roundedBorder)
                .onSubmit(commit)
            HStack(spacing: 8) {
                Spacer(minLength: 0)
                Button("取消") { cancel() }
                    .keyboardShortcut(.cancelAction)
                Button(confirmLabel, action: commit)
                    .keyboardShortcut(.defaultAction)
                    .disabled(!allowsEmpty && PADWorkbenchUI.trimmed(text).isEmpty)
            }
        }
        .padFont()
        .padding(18)
        .frame(width: 420)
        .foregroundStyle(PADWorkbenchStyle.text)
        .background(PADWorkbenchStyle.canvas)
    }

    private func commit() {
        let value = PADWorkbenchUI.trimmed(text)
        guard allowsEmpty || !value.isEmpty else { return }
        text = ""
        onSubmit(value)
        dismiss()
    }

    private func cancel() {
        text = ""
        dismiss()
    }
}
#endif
