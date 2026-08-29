import SwiftUI

struct ConversationView: View {
    @EnvironmentObject private var model: RemoteAppModel
    @FocusState private var composerFocused: Bool

    var body: some View {
        Group {
            if let task = model.selectedTask {
                VStack(spacing: 0) {
                    if model.connectionState != .online {
                        OfflineBanner()
                    }
                    MessageTimeline(messages: model.selectedMessages)
                    if !model.selectedUIRequests.isEmpty {
                        Divider()
                        PendingInteractionsView(taskID: task.id, requests: model.selectedUIRequests)
                    }
                    Divider()
                    ComposerView(composerFocused: $composerFocused)
                }
                .navigationTitle(task.title)
                .navigationBarTitleDisplayMode(.inline)
                .toolbar {
                    if task.status == .failed {
                        ToolbarItem(placement: .primaryAction) {
                            Button("重试", systemImage: "arrow.clockwise") { model.retrySelectedTask() }
                        }
                    }
                }
            } else {
                ContentUnavailableView(
                    "选择一个任务",
                    systemImage: "sidebar.left",
                    description: Text("从侧边栏选择任务，或创建一个新任务。")
                )
            }
        }
        .background(Color(.systemBackground))
        .alert("连接提示", isPresented: Binding(
            get: { model.lastError != nil },
            set: { if !$0 { model.clearError() } }
        )) {
            if model.settingsRecoveryAvailable {
                Button("打开系统设置") { model.openSystemSettings() }
            }
            Button("知道了") {}
        } message: {
            Text(model.lastError ?? "")
        }
    }
}

private struct PendingInteractionsView: View {
    @EnvironmentObject private var model: RemoteAppModel
    let taskID: String
    let requests: [RemoteUIRequest]

    var body: some View {
        ScrollView(.vertical) {
            VStack(spacing: 10) {
                ForEach(requests) { request in
                    InteractionCard(
                        request: request,
                        respond: { value in
                            model.respondToUI(taskID: taskID, requestID: request.id, value: value)
                        },
                        cancel: {
                            model.cancelUI(taskID: taskID, requestID: request.id)
                        }
                    )
                }
            }
            .frame(maxWidth: 760)
            .padding(.horizontal, 14)
            .padding(.vertical, 10)
            .frame(maxWidth: .infinity)
        }
        .frame(maxHeight: 310)
        .background(Color.orange.opacity(0.07))
        .accessibilityElement(children: .contain)
        .accessibilityLabel("PAD 需要你的响应")
    }
}

private struct InteractionCard: View {
    let request: RemoteUIRequest
    let respond: (JSONValue) -> Void
    let cancel: () -> Void
    @State private var text: String
    @State private var selectedIndex: Int

    init(
        request: RemoteUIRequest,
        respond: @escaping (JSONValue) -> Void,
        cancel: @escaping () -> Void
    ) {
        self.request = request
        self.respond = respond
        self.cancel = cancel
        _text = State(initialValue: request.defaultValue ?? "")
        _selectedIndex = State(initialValue: request.defaultIndex ?? 0)
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            Label(request.title ?? "PAD 需要你的响应", systemImage: "questionmark.bubble.fill")
                .font(.headline)
                .foregroundStyle(.primary)
            if let message = request.message, !message.isEmpty {
                Text(message)
                    .font(.callout)
                    .foregroundStyle(.secondary)
            }

            switch request.kind {
            case .confirm:
                HStack {
                    Button("取消") { cancel() }
                        .buttonStyle(.bordered)
                    Button("拒绝", role: .destructive) { respond(.bool(false)) }
                        .buttonStyle(.bordered)
                    Spacer()
                    Button("允许") { respond(.bool(true)) }
                        .buttonStyle(.borderedProminent)
                }
                .disabled(!request.requiresResponse)
            case .select:
                if request.options.isEmpty {
                    Text("Mac 没有提供可选项，请等待任务刷新。")
                        .font(.footnote)
                        .foregroundStyle(.secondary)
                } else {
                    Picker("选择", selection: $selectedIndex) {
                        ForEach(Array(request.options.enumerated()), id: \.offset) { index, option in
                            Text(option).tag(index)
                        }
                    }
                    .pickerStyle(.menu)
                    Button("提交选择") {
                        guard request.options.indices.contains(selectedIndex) else { return }
                        respond(.string(request.options[selectedIndex]))
                    }
                        .buttonStyle(.borderedProminent)
                        .disabled(!request.requiresResponse || !request.options.indices.contains(selectedIndex))
                        .frame(maxWidth: .infinity, alignment: .trailing)
                    Button("取消交互") { cancel() }
                        .buttonStyle(.borderless)
                        .disabled(!request.requiresResponse)
                }
            case .input:
                TextField(request.placeholder ?? "输入回复", text: $text)
                    .textFieldStyle(.roundedBorder)
                responseButtons
            case .editor:
                ZStack(alignment: .topLeading) {
                    TextEditor(text: $text)
                        .frame(minHeight: 90)
                        .accessibilityLabel("编辑回复")
                    if text.isEmpty, let placeholder = request.placeholder, !placeholder.isEmpty {
                        Text(placeholder)
                            .foregroundStyle(.tertiary)
                            .padding(.horizontal, 5)
                            .padding(.vertical, 8)
                            .allowsHitTesting(false)
                    }
                }
                .padding(6)
                .background(Color(.secondarySystemBackground), in: RoundedRectangle(cornerRadius: 10))
                responseButtons
            case .unknown:
                Label("当前版本无法安全处理这种交互，请升级 PAD Remote。", systemImage: "exclamationmark.triangle")
                    .font(.footnote)
                    .foregroundStyle(.orange)
            }
        }
        .padding(14)
        .background(Color(.secondarySystemBackground), in: RoundedRectangle(cornerRadius: 14, style: .continuous))
        .accessibilityElement(children: .contain)
    }

    private var responseButtons: some View {
        HStack {
            Button("取消交互") { cancel() }
                .buttonStyle(.bordered)
                .disabled(!request.requiresResponse)
            Spacer()
            Button("提交") { respond(.string(text)) }
                .buttonStyle(.borderedProminent)
                .disabled(!request.requiresResponse || text.isEmpty)
        }
    }
}

private struct OfflineBanner: View {
    @EnvironmentObject private var model: RemoteAppModel

    var body: some View {
        Label(model.connectionState.title, systemImage: model.connectionState.symbol)
            .font(.footnote.weight(.medium))
            .foregroundStyle(.secondary)
            .frame(maxWidth: .infinity)
            .padding(.vertical, 7)
            .background(Color(.secondarySystemBackground))
    }
}

private struct MessageTimeline: View {
    let messages: [RemoteMessage]
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    var body: some View {
        ScrollViewReader { proxy in
            ScrollView {
                LazyVStack(alignment: .leading, spacing: 16) {
                    if messages.isEmpty {
                        ContentUnavailableView(
                            "开始对话",
                            systemImage: "sparkles",
                            description: Text("从这里向 Mac 上的 PAD 任务发送消息。")
                        )
                        .frame(maxWidth: .infinity, minHeight: 280)
                    } else {
                        ForEach(messages) { message in
                            MessageBubble(message: message)
                                .id(message.id)
                        }
                    }
                }
                .frame(maxWidth: 760)
                .padding(.horizontal, 18)
                .padding(.vertical, 24)
                .frame(maxWidth: .infinity)
            }
            .onChange(of: scrollSignal) { _, signal in
                guard let signal else { return }
                if reduceMotion {
                    proxy.scrollTo(signal.id, anchor: .bottom)
                } else {
                    withAnimation(.easeOut(duration: 0.2)) { proxy.scrollTo(signal.id, anchor: .bottom) }
                }
            }
        }
    }

    private var scrollSignal: MessageScrollSignal? {
        messages.last.map { MessageScrollSignal(id: $0.id, textLength: $0.text.utf8.count) }
    }
}

struct MessageScrollSignal: Equatable {
    let id: String
    let textLength: Int
}

private struct MessageBubble: View {
    let message: RemoteMessage

    var body: some View {
        HStack {
            if message.role == .user { Spacer(minLength: 42) }
            VStack(alignment: .leading, spacing: 7) {
                Text(roleTitle)
                    .font(.caption.weight(.semibold))
                    .foregroundStyle(.secondary)
                Text(message.text)
                    .font(.body)
                    .textSelection(.enabled)
                if message.isStreaming {
                    ProgressView()
                        .controlSize(.small)
                        .accessibilityLabel("正在生成回复")
                }
            }
            .padding(13)
            .background(background, in: RoundedRectangle(cornerRadius: 16, style: .continuous))
            if message.role != .user { Spacer(minLength: 42) }
        }
        .accessibilityElement(children: .combine)
        .accessibilityLabel("\(roleTitle)：\(message.text)")
    }

    private var roleTitle: String {
        switch message.role {
        case .user: return "你"
        case .assistant: return "PAD"
        case .system: return "系统"
        }
    }

    private var background: Color {
        message.role == .user ? Color.accentColor.opacity(0.14) : Color(.secondarySystemBackground)
    }
}

private struct ComposerView: View {
    @EnvironmentObject private var model: RemoteAppModel
    var composerFocused: FocusState<Bool>.Binding

    var body: some View {
        VStack(spacing: 10) {
            if model.connectionState != .online, !model.composerText.isEmpty {
                Text("当前离线：发送后会安全保存在发件箱，连接恢复后自动送达。")
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .frame(maxWidth: .infinity, alignment: .leading)
            }
            HStack(alignment: .bottom, spacing: 10) {
                TextField("向 PAD 发送消息", text: $model.composerText, axis: .vertical)
                    .lineLimit(1 ... 8)
                    .focused(composerFocused)
                    .textFieldStyle(.plain)
                    .padding(.horizontal, 13)
                    .padding(.vertical, 11)
                    .background(Color(.secondarySystemBackground), in: RoundedRectangle(cornerRadius: 15))
                    .submitLabel(.send)
                    .onSubmit {
                        if model.canSend { model.sendPrompt() }
                    }

                if model.selectedTask?.status == .running {
                    Button(role: .destructive) { model.stopSelectedTask() } label: {
                        Image(systemName: "stop.fill")
                            .frame(width: 32, height: 32)
                    }
                    .buttonStyle(.bordered)
                    .clipShape(Circle())
                    .accessibilityLabel("停止任务")
                }

                Button { model.sendPrompt() } label: {
                    Group {
                        if model.isSavingPrompt {
                            ProgressView().controlSize(.small)
                        } else {
                            Image(systemName: "arrow.up").font(.headline)
                        }
                    }
                    .frame(width: 32, height: 32)
                }
                .buttonStyle(.borderedProminent)
                .clipShape(Circle())
                .disabled(!model.canSend)
                .accessibilityLabel("发送")
            }
        }
        .frame(maxWidth: 760)
        .padding(.horizontal, 14)
        .padding(.vertical, 12)
        .frame(maxWidth: .infinity)
        .background(.ultraThinMaterial)
    }
}
