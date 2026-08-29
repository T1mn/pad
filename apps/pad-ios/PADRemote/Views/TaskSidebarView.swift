import SwiftUI

struct TaskSidebarView: View {
    @EnvironmentObject private var model: RemoteAppModel
    @State private var confirmsForget = false
    var compact = false

    var body: some View {
        List {
            Section {
                ConnectionStatusRow()
            }

            Section("任务") {
                if model.tasks.isEmpty {
                    ContentUnavailableView(
                        "还没有任务",
                        systemImage: "text.bubble",
                        description: Text("点右上角的加号，在 Mac 上创建一个新任务。")
                    )
                    .listRowBackground(Color.clear)
                } else {
                    ForEach(model.tasks) { task in
                        if compact {
                            NavigationLink(value: task.id) { TaskRow(task: task) }
                        } else {
                            Button {
                                model.selectTask(task.id)
                            } label: {
                                TaskRow(task: task)
                            }
                            .buttonStyle(.plain)
                            .listRowBackground(
                                task.id == model.cached.selectedTaskID
                                    ? Color.accentColor.opacity(0.12)
                                    : Color.clear
                            )
                        }
                    }
                }
            }
        }
        .listStyle(.sidebar)
        .navigationTitle("PAD Remote")
        .toolbar {
            ToolbarItem(placement: .primaryAction) {
                Button { model.createTask() } label: {
                    Label("新任务", systemImage: "square.and.pencil")
                }
            }
            ToolbarItem(placement: .secondaryAction) {
                Button("刷新", systemImage: "arrow.clockwise") {
                    model.refreshContent()
                }
            }
            ToolbarItem(placement: .secondaryAction) {
                Button("取消配对", systemImage: "rectangle.portrait.and.arrow.right", role: .destructive) {
                    confirmsForget = true
                }
            }
        }
        .confirmationDialog(
            "取消与这台 Mac 的配对？",
            isPresented: $confirmsForget,
            titleVisibility: .visible
        ) {
            Button("取消配对并清除本机缓存", role: .destructive) { model.disconnectAndForget() }
            Button("保留配对", role: .cancel) {}
        } message: {
            Text("设备凭据、离线缓存和待发送操作都会从这台 iPhone 或 iPad 删除。")
        }
    }
}

private struct ConnectionStatusRow: View {
    @EnvironmentObject private var model: RemoteAppModel

    var body: some View {
        VStack(alignment: .leading, spacing: 5) {
            Label(model.connectionState.title, systemImage: model.connectionState.symbol)
                .font(.subheadline.weight(.semibold))
                .foregroundStyle(model.connectionState == .online ? Color.green : Color.secondary)
            if let host = model.pairedHost {
                Text(host.displayName)
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .lineLimit(1)
            }
            if !model.profileAvailable {
                Label("Mac 上的当前工作区暂不可用", systemImage: "exclamationmark.circle")
                    .font(.caption)
                    .foregroundStyle(.orange)
            }
        }
        .accessibilityElement(children: .combine)
    }
}

private struct TaskRow: View {
    let task: RemoteTaskSummary

    var body: some View {
        HStack(spacing: 10) {
            Image(systemName: statusSymbol)
                .foregroundStyle(statusColor)
                .frame(width: 18)
                .accessibilityLabel(task.status.localizedTitle)
            VStack(alignment: .leading, spacing: 3) {
                Text(task.title)
                    .font(.body.weight(.medium))
                    .foregroundStyle(.primary)
                    .lineLimit(2)
                HStack(spacing: 6) {
                    if let subtitle = task.subtitle, !subtitle.isEmpty {
                        Text(subtitle).lineLimit(1)
                    }
                    Text(task.updatedAt, style: .relative)
                }
                .font(.caption)
                .foregroundStyle(.secondary)
            }
        }
        .padding(.vertical, 3)
        .contentShape(Rectangle())
        .accessibilityElement(children: .combine)
        .accessibilityLabel("\(task.title)，\(task.status.localizedTitle)")
    }

    private var statusSymbol: String {
        switch task.status {
        case .running: return "circle.dotted"
        case .attention: return "exclamationmark.circle.fill"
        case .failed: return "xmark.circle.fill"
        case .completed: return "checkmark.circle.fill"
        case .idle: return "circle"
        }
    }

    private var statusColor: Color {
        switch task.status {
        case .running: return .blue
        case .attention: return .orange
        case .failed: return .red
        case .completed: return .green
        case .idle: return .secondary
        }
    }
}
