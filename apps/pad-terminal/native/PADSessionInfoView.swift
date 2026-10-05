#if os(macOS)
import AppKit
import SwiftUI

/// Explicit inspection only; the clipboard changes only on a copy-button click.
struct PADSessionInfoView: View {
    @ObservedObject var model: PADWorkbenchModel

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack {
                Text("会话信息").font(.headline)
                Spacer()
                Button { model.closeSessionInfo() } label: {
                    Image(systemName: "xmark")
                }
                .buttonStyle(.plain)
                .help("关闭会话信息")
            }
            if model.sessionInfoLoading {
                ProgressView("正在读取…").controlSize(.small)
            } else if let info = model.sessionInfo, info.taskId == model.selectedTaskId {
                Text("引擎：Pi").font(.subheadline)
                if model.isHistoricalAccount, let name = info.profileName {
                    Text("历史任务配置：\(name)").font(.subheadline)
                }
                if info.state == "not_created" {
                    Text("尚未创建 Pi 会话").foregroundStyle(.secondary)
                } else if info.state == "unavailable" {
                    Text("Pi 会话信息暂不可用，未读取到有效 ID。").foregroundStyle(.secondary)
                }
                if info.state == "available", let id = info.sessionId {
                    copyRow("Pi 会话 ID", value: id)
                }
                copyRow("PAD 任务 ID", value: info.taskId)
                if let file = info.sessionFile { copyRow("会话文件", value: file) }
                copyRow("工作目录", value: info.cwd)
                Text(fileLabel(info.fileState))
                    .font(.caption).foregroundStyle(.secondary)
                if info.source == "runtime" {
                    Text("ID 来自现有 Pi 进程的状态；文件路径不代表已持久化。")
                        .font(.caption).foregroundStyle(.secondary)
                }
            } else {
                Text(model.sessionInfoError ?? "会话信息暂不可用。")
                    .foregroundStyle(.secondary)
            }
        }
        .padding(16)
        .frame(width: 420, alignment: .leading)
        .onDisappear { model.closeSessionInfo() }
    }

    private func fileLabel(_ state: String) -> String {
        switch state {
        case "present": return "会话文件已存在"
        case "absent": return "会话文件尚未持久化或已不存在"
        default: return "会话文件不可用"
        }
    }

    private func copyRow(_ label: String, value: String) -> some View {
        HStack(alignment: .top, spacing: 8) {
            VStack(alignment: .leading, spacing: 3) {
                Text(label).font(.caption).foregroundStyle(.secondary)
                Text(value).font(.system(.body, design: .monospaced))
                    .textSelection(.enabled)
                    .fixedSize(horizontal: false, vertical: true)
            }
            Spacer(minLength: 0)
            Button {
                NSPasteboard.general.clearContents()
                NSPasteboard.general.setString(value, forType: .string)
            } label: { Image(systemName: "doc.on.doc") }
            .buttonStyle(.plain)
            .help("复制\(label)")
            .accessibilityLabel("复制\(label)")
        }
    }
}
#endif
