import Foundation

struct TaskOutputReduction: Equatable {
    let taskID: String?
    let shouldRefreshHistory: Bool
}

enum RemoteContentReducer {
    static func addOptimisticMessage(
        _ message: RemoteMessage,
        taskID: String,
        to state: inout CachedRemoteState
    ) {
        state.messagesByTask[taskID, default: []].append(message)
        var ids = state.pendingLocalMessageIDsByTask[taskID] ?? []
        if !ids.contains(message.id) { ids.append(message.id) }
        state.pendingLocalMessageIDsByTask[taskID] = ids
    }

    static func removeOptimisticMessage(
        id: String,
        taskID: String,
        from state: inout CachedRemoteState
    ) {
        state.messagesByTask[taskID]?.removeAll { $0.id == id }
        state.pendingLocalMessageIDsByTask[taskID]?.removeAll { $0 == id }
        if state.pendingLocalMessageIDsByTask[taskID]?.isEmpty == true {
            state.pendingLocalMessageIDsByTask.removeValue(forKey: taskID)
        }
    }

    static func discardLiveStreamsForAuthoritativeRecovery(_ state: inout CachedRemoteState) {
        for (taskID, stream) in state.liveStreamsByTask {
            state.messagesByTask[taskID]?.removeAll { $0.id == stream.messageID }
        }
        state.liveStreamsByTask.removeAll()
    }

    static func apply(result: JSONValue, action: RemoteAction, to state: inout CachedRemoteState) {
        guard let object = result.objectValue else { return }
        let isFullTaskSnapshot = action == .bootstrap || action == .listSidebar
        if isFullTaskSnapshot, (object["sidebar"] != nil || object["records"] != nil) {
            var authoritativeTasks: [RemoteTaskSummary] = []
            if let sidebar = object["sidebar"] { authoritativeTasks += taskSummaries(in: sidebar) }
            if let records = object["records"] { authoritativeTasks += recordTaskSummaries(in: records) }
            replaceTasks(with: authoritativeTasks, state: &state)
        } else {
            if let sidebar = object["sidebar"] {
                let tasks = taskSummaries(in: sidebar)
                if !tasks.isEmpty { merge(tasks: tasks, into: &state.tasks) }
            }
            if let records = object["records"] {
                let tasks = recordTaskSummaries(in: records)
                if !tasks.isEmpty { merge(tasks: tasks, into: &state.tasks) }
            }
        }
        if let taskValue = object["task"], let task = taskSummary(from: taskValue) {
            merge(tasks: [task], into: &state.tasks)
            if action == .createTask { state.selectedTaskID = task.id }
        }
        if action == .createTask, let taskID = object["task_id"]?.stringValue {
            state.selectedTaskID = taskID
        }
        let isPendingEmptyHistory: Bool = {
            guard action == .history, object["pending"] == .bool(true) else { return false }
            guard case let .array(values) = object["messages"] else { return true }
            return values.isEmpty
        }()
        if !isPendingEmptyHistory,
           let messages = object["messages"],
           let taskID = object["task_id"]?.stringValue ?? state.selectedTaskID {
            var parsed = messageValues(in: messages)
            if !parsed.isEmpty || action == .history {
                let pendingIDs = state.pendingLocalMessageIDsByTask[taskID] ?? []
                let pendingMessages = pendingIDs.compactMap { id in
                    state.messagesByTask[taskID]?.first { $0.id == id }
                }
                var unresolvedIDs: [String] = []
                for pending in pendingMessages {
                    if parsed.contains(where: { $0.role == .user && $0.text == pending.text }) { continue }
                    unresolvedIDs.append(pending.id)
                    if !parsed.contains(where: { $0.id == pending.id }) { parsed.append(pending) }
                }
                if unresolvedIDs.isEmpty {
                    state.pendingLocalMessageIDsByTask.removeValue(forKey: taskID)
                } else {
                    state.pendingLocalMessageIDsByTask[taskID] = unresolvedIDs
                }
                let liveMessage = state.liveStreamsByTask[taskID].flatMap { stream in
                    state.messagesByTask[taskID]?.first { $0.id == stream.messageID }
                }
                if let liveMessage,
                   let last = parsed.last,
                   last.role == .assistant,
                   !last.text.isEmpty,
                   liveMessage.text.hasPrefix(last.text) {
                    parsed.removeLast()
                }
                state.messagesByTask[taskID] = parsed
                if let liveMessage,
                   !parsed.contains(where: { $0.id == liveMessage.id }) {
                    state.messagesByTask[taskID, default: []].append(liveMessage)
                }
            }
        }
        if action == .bootstrap, state.selectedTaskID == nil {
            state.selectedTaskID = state.tasks.first?.id
        }
        applyPendingSnapshots(from: object, action: action, to: &state)
        state.updatedAt = Date()
    }

    static func applyTaskOutput(_ payload: JSONValue, to state: inout CachedRemoteState) -> TaskOutputReduction {
        guard let object = payload.objectValue else {
            return TaskOutputReduction(taskID: nil, shouldRefreshHistory: false)
        }
        let taskID = object["task_id"]?.stringValue
            ?? object["task"]?.objectValue?["id"]?.stringValue
        guard let taskID else {
            return TaskOutputReduction(taskID: nil, shouldRefreshHistory: false)
        }
        let selectedBeforeUpdate = state.selectedTaskID

        if var task = object["task"]?.objectValue {
            task["id"] = task["id"] ?? .string(taskID)
            if let runtimeStatus = object["runtime"]?.objectValue?["status"] {
                task["status"] = runtimeStatus
            }
            let snapshot: [String: JSONValue] = ["task": .object(task), "task_id": .string(taskID)]
            apply(result: .object(snapshot), action: .runtimeSnapshot, to: &state)
        } else if let runtimeStatus = object["runtime"]?.objectValue?["status"] {
            let existing = state.tasks.first { $0.id == taskID }
            let task: JSONValue = .object([
                "id": .string(taskID),
                "title": .string(existing?.title ?? "未命名任务"),
                "status": runtimeStatus,
            ])
            apply(result: .object(["task": task, "task_id": .string(taskID)]), action: .runtimeSnapshot, to: &state)
        }

        var shouldRefreshHistory = false
        if let authoritativePending = object["pending_ui_requests"] {
            state.pendingUIRequestsByTask[taskID] = uiRequests(in: authoritativePending)
        }
        if let poll = object["poll"]?.objectValue {
            if object["pending_ui_requests"] == nil, let pending = poll["pending_ui_requests"] {
                // Older gateways exposed only requests newly observed by this
                // poll. Empty is not authoritative and must not clear a card.
                let incoming = uiRequests(in: pending)
                if !incoming.isEmpty {
                    var merged = state.pendingUIRequestsByTask[taskID] ?? []
                    for request in incoming {
                        merged.removeAll { $0.id == request.id }
                        merged.append(request)
                    }
                    state.pendingUIRequestsByTask[taskID] = merged
                }
            }
            if case let .array(controlMessages) = poll["messages"] {
                for controlMessage in controlMessages {
                    guard let envelope = controlMessage.objectValue,
                          envelope["type"]?.stringValue == "response",
                          let value = envelope["value"]?.objectValue,
                          value["command"]?.stringValue == "get_messages",
                          value["success"] == .bool(true),
                          let messages = value["data"]?.objectValue?["messages"] else { continue }
                    apply(
                        result: .object(["task_id": .string(taskID), "messages": messages]),
                        action: .history,
                        to: &state
                    )
                }
            }
            if case let .array(events) = poll["events"] {
                for event in events {
                    shouldRefreshHistory = applyRuntimeEvent(event, taskID: taskID, to: &state) || shouldRefreshHistory
                }
            }
        }
        state.selectedTaskID = selectedBeforeUpdate
        state.updatedAt = Date()
        return TaskOutputReduction(taskID: taskID, shouldRefreshHistory: shouldRefreshHistory)
    }

    private static func applyRuntimeEvent(
        _ value: JSONValue,
        taskID: String,
        to state: inout CachedRemoteState
    ) -> Bool {
        guard let event = value.objectValue, let type = event["type"]?.stringValue else { return false }
        switch type {
        case "message_start":
            guard let message = event["message"]?.objectValue,
                  message["role"]?.stringValue == "assistant" else { return false }
            startLiveStream(taskID: taskID, message: message, state: &state)
        case "message_update":
            guard let update = event["assistantMessageEvent"]?.objectValue,
                  let updateType = update["type"]?.stringValue else { return false }
            switch updateType {
            case "text_start":
                guard let index = contentIndex(update["contentIndex"]) else { return false }
                ensureLiveStream(taskID: taskID, state: &state)
                state.liveStreamsByTask[taskID]?.textBlocks[index] = ""
                updateLiveMessage(taskID: taskID, state: &state)
            case "text_delta":
                guard let index = contentIndex(update["contentIndex"]),
                      let delta = update["delta"]?.stringValue,
                      !delta.isEmpty else { return false }
                ensureLiveStream(taskID: taskID, state: &state)
                state.liveStreamsByTask[taskID]?.textBlocks[index, default: ""] += delta
                updateLiveMessage(taskID: taskID, state: &state)
            case "text_end":
                guard let index = contentIndex(update["contentIndex"]),
                      let content = update["content"]?.stringValue else { return false }
                ensureLiveStream(taskID: taskID, state: &state)
                state.liveStreamsByTask[taskID]?.textBlocks[index] = content
                updateLiveMessage(taskID: taskID, state: &state)
            default:
                break
            }
        case "message_end":
            guard let message = event["message"]?.objectValue,
                  message["role"]?.stringValue == "assistant" else { return false }
            finishLiveStream(taskID: taskID, message: message, state: &state)
            return true
        case "agent_settled":
            settleLiveStream(taskID: taskID, state: &state)
            return true
        default:
            break
        }
        return false
    }

    private static func startLiveStream(
        taskID: String,
        message: [String: JSONValue],
        state: inout CachedRemoteState
    ) {
        settleLiveStream(taskID: taskID, state: &state)
        let startedAt = parseDate(message["timestamp"] ?? message["created_at"]) ?? Date()
        let stream = RemoteLiveStreamState(
            messageID: "live-\(taskID)-\(UUID().uuidString)",
            textBlocks: textBlocks(in: message["content"] ?? message["text"]),
            startedAt: startedAt
        )
        state.liveStreamsByTask[taskID] = stream
        state.messagesByTask[taskID, default: []].append(RemoteMessage(
            id: stream.messageID,
            role: .assistant,
            text: renderedText(stream.textBlocks),
            createdAt: startedAt,
            isStreaming: true
        ))
    }

    private static func ensureLiveStream(taskID: String, state: inout CachedRemoteState) {
        guard state.liveStreamsByTask[taskID] == nil else { return }
        let stream = RemoteLiveStreamState(
            messageID: "live-\(taskID)-\(UUID().uuidString)",
            textBlocks: [:],
            startedAt: Date()
        )
        state.liveStreamsByTask[taskID] = stream
        state.messagesByTask[taskID, default: []].append(RemoteMessage(
            id: stream.messageID,
            role: .assistant,
            text: "",
            createdAt: stream.startedAt,
            isStreaming: true
        ))
    }

    private static func updateLiveMessage(taskID: String, state: inout CachedRemoteState) {
        guard let stream = state.liveStreamsByTask[taskID] else { return }
        let text = renderedText(stream.textBlocks)
        if let index = state.messagesByTask[taskID]?.firstIndex(where: { $0.id == stream.messageID }) {
            state.messagesByTask[taskID]?[index].text = text
            state.messagesByTask[taskID]?[index].isStreaming = true
        } else {
            state.messagesByTask[taskID, default: []].append(RemoteMessage(
                id: stream.messageID,
                role: .assistant,
                text: text,
                createdAt: stream.startedAt,
                isStreaming: true
            ))
        }
    }

    private static func finishLiveStream(
        taskID: String,
        message: [String: JSONValue],
        state: inout CachedRemoteState
    ) {
        let finalText = textContent(message["content"] ?? message["text"] ?? .object(message))
        if let stream = state.liveStreamsByTask.removeValue(forKey: taskID),
           let index = state.messagesByTask[taskID]?.firstIndex(where: { $0.id == stream.messageID }) {
            let fallback = state.messagesByTask[taskID]?[index].text ?? ""
            let authoritative = finalText.isEmpty ? fallback : finalText
            if authoritative.isEmpty {
                state.messagesByTask[taskID]?.remove(at: index)
            } else {
                state.messagesByTask[taskID]?[index].text = authoritative
                state.messagesByTask[taskID]?[index].isStreaming = false
            }
            return
        }
        guard !finalText.isEmpty,
              state.messagesByTask[taskID]?.last?.text != finalText else { return }
        state.messagesByTask[taskID, default: []].append(RemoteMessage(
            id: "final-\(taskID)-\(UUID().uuidString)",
            role: .assistant,
            text: finalText,
            createdAt: parseDate(message["timestamp"] ?? message["created_at"]) ?? Date(),
            isStreaming: false
        ))
    }

    private static func settleLiveStream(taskID: String, state: inout CachedRemoteState) {
        guard let stream = state.liveStreamsByTask.removeValue(forKey: taskID),
              let index = state.messagesByTask[taskID]?.firstIndex(where: { $0.id == stream.messageID }) else { return }
        state.messagesByTask[taskID]?[index].isStreaming = false
    }

    private static func contentIndex(_ value: JSONValue?) -> Int? {
        guard case let .number(raw) = value,
              raw.rounded() == raw,
              raw >= 0,
              raw <= 10_000 else { return nil }
        return Int(raw)
    }

    private static func textBlocks(in value: JSONValue?) -> [Int: String] {
        guard let value else { return [:] }
        switch value {
        case let .array(parts):
            var blocks: [Int: String] = [:]
            for (index, part) in parts.enumerated() {
                let text = textContent(part)
                if !text.isEmpty { blocks[index] = text }
            }
            return blocks
        default:
            let text = textContent(value)
            return text.isEmpty ? [:] : [0: text]
        }
    }

    private static func renderedText(_ blocks: [Int: String]) -> String {
        blocks.keys.sorted().compactMap { blocks[$0] }.joined()
    }

    static func uiRequests(in value: JSONValue) -> [RemoteUIRequest] {
        guard case let .array(values) = value else { return [] }
        var seen = Set<String>()
        return values.compactMap { value in
            guard let object = value.objectValue,
                  object["response_action"]?.stringValue == "respond_ui",
                  let id = object["id"]?.stringValue,
                  !id.isEmpty,
                  id.utf8.count <= 256,
                  seen.insert(id).inserted else { return nil }
            let kind = RemoteUIRequestKind(rawValue: object["kind"]?.stringValue ?? "unknown") ?? .unknown
            let options: [String]
            if case let .array(values) = object["options"] {
                options = values.compactMap(\.stringValue).prefix(100).map { String($0.prefix(2_000)) }
            } else { options = [] }
            let defaultIndex: Int?
            if case let .number(index) = object["default_index"],
               index.rounded() == index,
               index >= 0,
               Int(index) < options.count {
                defaultIndex = Int(index)
            } else { defaultIndex = nil }
            let requiresResponse: Bool
            if case let .bool(value) = object["requires_response"] {
                requiresResponse = value && kind != .unknown
            } else { requiresResponse = false }
            return RemoteUIRequest(
                id: id,
                kind: kind,
                title: object["title"]?.stringValue.map { String($0.prefix(500)) },
                message: object["message"]?.stringValue.map { String($0.prefix(4_000)) },
                options: options,
                defaultIndex: defaultIndex,
                defaultValue: (object["prefill"] ?? object["default"])?.stringValue.map {
                    String($0.prefix(10_000))
                },
                requiresResponse: requiresResponse,
                placeholder: object["placeholder"]?.stringValue.map { String($0.prefix(500)) }
            )
        }
    }

    private static func merge(tasks incoming: [RemoteTaskSummary], into existing: inout [RemoteTaskSummary]) {
        var byID = Dictionary(uniqueKeysWithValues: existing.map { ($0.id, $0) })
        incoming.forEach { byID[$0.id] = $0 }
        existing = byID.values.sorted { lhs, rhs in
            if lhs.updatedAt != rhs.updatedAt { return lhs.updatedAt > rhs.updatedAt }
            return lhs.title.localizedStandardCompare(rhs.title) == .orderedAscending
        }
    }

    private static func replaceTasks(with incoming: [RemoteTaskSummary], state: inout CachedRemoteState) {
        var unique: [RemoteTaskSummary] = []
        var seen = Set<String>()
        for task in incoming where seen.insert(task.id).inserted { unique.append(task) }
        unique.sort { lhs, rhs in
            if lhs.updatedAt != rhs.updatedAt { return lhs.updatedAt > rhs.updatedAt }
            return lhs.title.localizedStandardCompare(rhs.title) == .orderedAscending
        }
        let validIDs = Set(unique.map(\.id))
        state.tasks = unique
        state.messagesByTask = state.messagesByTask.filter { validIDs.contains($0.key) }
        state.liveStreamsByTask = state.liveStreamsByTask.filter { validIDs.contains($0.key) }
        state.pendingUIRequestsByTask = state.pendingUIRequestsByTask.filter { validIDs.contains($0.key) }
        state.pendingLocalMessageIDsByTask = state.pendingLocalMessageIDsByTask.filter { validIDs.contains($0.key) }
        if let selected = state.selectedTaskID, !validIDs.contains(selected) {
            state.selectedTaskID = unique.first?.id
        }
    }

    private static func applyPendingSnapshots(
        from root: [String: JSONValue],
        action: RemoteAction,
        to state: inout CachedRemoteState
    ) {
        var collected: [String: [RemoteUIRequest]] = [:]
        var foundAnyField = false

        func visit(_ value: JSONValue, inheritedTaskID: String?) {
            switch value {
            case let .array(values):
                values.forEach { visit($0, inheritedTaskID: inheritedTaskID) }
            case let .object(object):
                let taskID = object["task_id"]?.stringValue
                    ?? object["id"]?.stringValue
                    ?? inheritedTaskID
                if let pending = object["pending_ui_requests"], let taskID {
                    foundAnyField = true
                    collected[taskID] = uiRequests(in: pending)
                }
                if let byTask = object["pending_ui_requests_by_task"]?.objectValue {
                    foundAnyField = true
                    for (taskID, pending) in byTask { collected[taskID] = uiRequests(in: pending) }
                }
                for key in ["task", "tasks", "items", "records", "sidebar"] {
                    if let nested = object[key] { visit(nested, inheritedTaskID: taskID) }
                }
            case .string, .number, .bool, .null:
                break
            }
        }

        let explicitTaskID = root["task_id"]?.stringValue ?? root["task"]?.objectValue?["id"]?.stringValue
        visit(.object(root), inheritedTaskID: explicitTaskID)
        if action == .bootstrap {
            state.pendingUIRequestsByTask = collected
        } else if foundAnyField {
            for (taskID, requests) in collected { state.pendingUIRequestsByTask[taskID] = requests }
            if let explicitTaskID, collected[explicitTaskID] == nil,
               root["pending_ui_requests"] != nil {
                state.pendingUIRequestsByTask[explicitTaskID] = []
            }
        }
    }

    private static func taskSummaries(in value: JSONValue) -> [RemoteTaskSummary] {
        switch value {
        case let .array(values):
            return values.flatMap { taskSummary(from: $0).map { [$0] } ?? taskSummaries(in: $0) }
        case let .object(object):
            if let task = taskSummary(from: value) { return [task] }
            return ["tasks", "items", "recent", "rows", "records"].flatMap { key in
                object[key].map(taskSummaries(in:)) ?? []
            }
        default:
            return []
        }
    }

    private static func recordTaskSummaries(in value: JSONValue) -> [RemoteTaskSummary] {
        guard let object = value.objectValue else { return taskSummaries(in: value) }
        if let tasks = object["tasks"] { return taskSummaries(in: tasks) }
        if let records = object["records"] { return recordTaskSummaries(in: records) }
        return []
    }

    private static func taskSummary(from value: JSONValue) -> RemoteTaskSummary? {
        guard let object = value.objectValue else { return nil }
        if ["project", "profile", "section", "header"].contains(
            object["kind"]?.stringValue ?? object["entity_type"]?.stringValue ?? ""
        ) {
            return nil
        }
        guard let id = (object["id"] ?? object["task_id"] ?? object["thread_id"])?.stringValue,
              !id.isEmpty,
              id.utf8.count <= 512 else {
            return nil
        }
        let title = (object["title"] ?? object["name"])?.stringValue
            .map { String($0.prefix(500)) } ?? "未命名任务"
        let subtitle = (object["subtitle"] ?? object["project"] ?? object["workspace_name"])?.stringValue
            .map { String($0.prefix(1_000)) }
        let statusText = object["status"]?.stringValue ?? "idle"
        let status: RemoteTaskStatus
        switch statusText {
        case "running", "in_progress", "starting", "streaming", "tool_running", "compacting", "retrying": status = .running
        case "attention", "needs_attention", "needs_approval", "needs_input", "waiting": status = .attention
        case "failed", "error", "disconnected": status = .failed
        case "completed", "done": status = .completed
        default: status = .idle
        }
        let updatedAt = parseDate(object["updated_at"] ?? object["updatedAt"]) ?? Date()
        return RemoteTaskSummary(id: id, title: title, subtitle: subtitle, status: status, updatedAt: updatedAt)
    }

    private static func messageValues(in value: JSONValue) -> [RemoteMessage] {
        guard case let .array(values) = value else { return [] }
        return values.compactMap { value in
            guard let object = value.objectValue else { return nil }
            let text = textContent(object["text"] ?? object["content"] ?? object["message"])
            guard !text.isEmpty else { return nil }
            let role = role(from: object)
            let id = (object["id"] ?? object["message_id"])?.stringValue ?? UUID().uuidString
            let createdAt = parseDate(object["created_at"] ?? object["createdAt"]) ?? Date()
            let isStreaming: Bool
            if case let .bool(value) = object["is_streaming"] { isStreaming = value } else { isStreaming = false }
            return RemoteMessage(id: id, role: role, text: text, createdAt: createdAt, isStreaming: isStreaming)
        }
    }

    private static func parseDate(_ value: JSONValue?) -> Date? {
        switch value {
        case let .string(text):
            return ISO8601DateFormatter().date(from: text)
        case let .number(epoch):
            return Date(timeIntervalSince1970: epoch > 100_000_000_000 ? epoch / 1_000 : epoch)
        default:
            return nil
        }
    }

    private static func role(from object: [String: JSONValue]) -> RemoteRole {
        if let role = object["role"]?.stringValue, let mapped = RemoteRole(rawValue: role) { return mapped }
        switch object["kind"]?.stringValue ?? object["type"]?.stringValue {
        case "user_message", "input", "input_text": return .user
        case "system", "system_message": return .system
        default: return .assistant
        }
    }

    private static func textContent(_ value: JSONValue?) -> String {
        guard let value else { return "" }
        switch value {
        case let .string(text): return text
        case let .array(parts):
            return parts.map(textContent).filter { !$0.isEmpty }.joined(separator: "\n")
        case let .object(object):
            for key in ["text", "content", "message", "value"] {
                let text = textContent(object[key])
                if !text.isEmpty { return text }
            }
            return ""
        case .number, .bool, .null: return ""
        }
    }
}
