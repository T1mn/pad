#if os(macOS)
import Foundation

/// Streaming state for one in-flight assistant message on one task.
struct PADStreamState {
    var messageId: String
    var textBlocks: [Int: String] = [:]
}

/// Outcome of reducing one raw Pi event.
struct PADReduceResult {
    var immediateFlush = false
    var settled = false
    var statusHint: String?
    var errorText: String?
}

/// Normalizes raw Pi JSON events and `history` payloads into `PADChatItem`
/// values. Pure and stateless apart from the caller-owned message id, so the
/// model can keep a per-task transcript and stream state without hidden caches.
enum PADMessageReducer {
    static let maxItems = 400
    static let maxTextLength = 20_000
    static let maxToolSummaryLength = 800

    // MARK: - History

    static func parseHistory(_ data: PADJSONValue) -> (items: [PADChatItem], truncated: Bool) {
        guard let object = data.objectValue else { return ([], false) }
        let truncated = object["truncated"]?.boolValue ?? false
        guard let messages = object["messages"]?.arrayValue else { return ([], truncated) }

        var toolResults: [String: [String: PADJSONValue]] = [:]
        for value in messages {
            guard let message = value.objectValue,
                  message["role"]?.stringValue == "toolResult",
                  let callId = message["toolCallId"]?.stringValue else { continue }
            toolResults[callId] = message
        }

        var items: [PADChatItem] = []
        for (index, value) in messages.enumerated() {
            guard let message = value.objectValue else { continue }
            switch message["role"]?.stringValue ?? "" {
            case "user":
                let text = textContent(message["content"])
                guard !text.isEmpty else { continue }
                items.append(PADChatItem(
                    id: "history-user-\(index)", role: "user", text: capped(text),
                    toolName: nil, status: nil
                ))
            case "assistant":
                let content = textContent(message["content"])
                let text = content.isEmpty ? (message["errorMessage"]?.stringValue ?? "") : content
                if !text.isEmpty {
                    items.append(PADChatItem(
                        id: "history-assistant-\(index)", role: "assistant", text: capped(text),
                        toolName: nil, status: nil, isError: isErrorMessage(message)
                    ))
                }
                for call in toolCalls(in: message["content"]) {
                    let callId = call.id
                    let result = toolResults[callId]
                    let isError = result?["isError"]?.boolValue ?? false
                    let summary: String
                    if let result {
                        let resultText = textContent(result["content"])
                        summary = resultText.isEmpty ? compactJSON(call.arguments) : resultText
                    } else {
                        summary = compactJSON(call.arguments)
                    }
                    items.append(PADChatItem(
                        id: "tool-\(callId)", role: "tool", text: capped(summary, limit: maxToolSummaryLength),
                        toolName: call.name, status: isError ? "error" : "done", isError: isError
                    ))
                }
            default:
                continue
            }
        }
        return (bound(items), truncated)
    }

    // MARK: - Live events

    static func applyPiEvent(
        _ data: PADJSONValue,
        taskId: String,
        transcript: inout [PADChatItem],
        stream: inout PADStreamState?
    ) -> PADReduceResult {
        var result = PADReduceResult()
        guard let event = data.objectValue, let type = event["type"]?.stringValue else { return result }

        switch type {
        case "message_start":
            guard let message = event["message"]?.objectValue else { break }
            switch message["role"]?.stringValue ?? "" {
            case "assistant":
                ensureStream(taskId: taskId, transcript: &transcript, stream: &stream)
                replaceStreamText(stream: stream, transcript: &transcript, text: textContent(message["content"]))
            case "user":
                appendUser(text: textContent(message["content"]), to: &transcript)
            default:
                break
            }

        case "message_update":
            guard let update = event["assistantMessageEvent"]?.objectValue,
                  let updateType = update["type"]?.stringValue else { break }
            switch updateType {
            case "text_start":
                ensureStream(taskId: taskId, transcript: &transcript, stream: &stream)
            case "text_delta":
                guard let delta = update["delta"]?.stringValue, !delta.isEmpty else { break }
                ensureStream(taskId: taskId, transcript: &transcript, stream: &stream)
                let index = Int(update["contentIndex"]?.numberValue ?? 0)
                let previous = stream?.textBlocks[index] ?? ""
                updateTextBlock(index: index, text: previous + delta, stream: &stream, transcript: &transcript)
            case "text_end":
                guard let content = update["content"]?.stringValue else { break }
                ensureStream(taskId: taskId, transcript: &transcript, stream: &stream)
                let index = Int(update["contentIndex"]?.numberValue ?? 0)
                updateTextBlock(index: index, text: content, stream: &stream, transcript: &transcript)
            case "toolcall_start":
                let callId = update["id"]?.stringValue
                    ?? update["contentIndex"]?.numberValue.map { String(Int($0)) }
                    ?? UUID().uuidString
                upsertTool(
                    id: "tool-\(callId)", toolName: update["toolName"]?.stringValue,
                    text: nil, status: "running", isError: false, in: &transcript
                )
            case "toolcall_end":
                if let call = update["toolCall"]?.objectValue, let callId = call["id"]?.stringValue {
                    upsertTool(
                        id: "tool-\(callId)", toolName: call["name"]?.stringValue,
                        text: compactJSON(call["arguments"]), status: "running", isError: false, in: &transcript
                    )
                }
            case "done":
                finishAssistant(message: update["message"], taskId: taskId, transcript: &transcript,
                                stream: &stream, result: &result)
            case "error":
                finishAssistant(message: update["error"] ?? update["message"], taskId: taskId,
                                transcript: &transcript, stream: &stream, result: &result)
            default:
                break
            }

        case "message_end":
            guard let message = event["message"]?.objectValue else { break }
            if message["role"]?.stringValue == "assistant" {
                finishAssistant(message: .object(message), taskId: taskId, transcript: &transcript,
                                stream: &stream, result: &result)
            } else if message["role"]?.stringValue == "user" {
                appendUser(text: textContent(message["content"]), to: &transcript)
            }

        case "tool_execution_start":
            let callId = event["toolCallId"]?.stringValue ?? UUID().uuidString
            upsertTool(
                id: "tool-\(callId)", toolName: event["toolName"]?.stringValue,
                text: compactJSON(event["args"]), status: "running", isError: false, in: &transcript
            )
            result.statusHint = "running"

        case "tool_execution_update":
            let callId = event["toolCallId"]?.stringValue ?? ""
            upsertTool(
                id: "tool-\(callId)", toolName: event["toolName"]?.stringValue,
                text: compactJSON(event["partialResult"]), status: "running", isError: false, in: &transcript
            )

        case "tool_execution_end":
            let callId = event["toolCallId"]?.stringValue ?? ""
            let isError = event["isError"]?.boolValue ?? false
            upsertTool(
                id: "tool-\(callId)", toolName: event["toolName"]?.stringValue,
                text: compactJSON(event["result"]), status: isError ? "error" : "done",
                isError: isError, in: &transcript
            )
            result.statusHint = "running"

        case "agent_settled":
            settle(&transcript)
            stream = nil
            result.settled = true
            result.immediateFlush = true
            result.statusHint = "idle"

        case "agent_start", "turn_start", "turn_end", "compaction_start", "auto_retry_start":
            result.statusHint = "running"

        default:
            break
        }

        transcript = bound(transcript)
        return result
    }

    // MARK: - Merge authoritative history with optimistic/live state

    static func mergeHistory(
        _ parsed: [PADChatItem],
        existing: [PADChatItem],
        activeStreamId: String?,
        activeStreamText: String?
    ) -> [PADChatItem] {
        var merged = parsed
        let parsedUserTexts = Set(parsed.filter { $0.role == "user" }.map(\.text))
        for item in existing where item.id.hasPrefix("local-") {
            if !parsedUserTexts.contains(item.text) { merged.append(item) }
        }
        if let activeStreamId, let activeStreamText, !activeStreamText.isEmpty {
            let alreadyPresent = parsed.contains { $0.role == "assistant" && $0.text == activeStreamText }
            if !alreadyPresent {
                var live = existing.first { $0.id == activeStreamId }
                if live == nil {
                    live = PADChatItem(
                        id: activeStreamId, role: "assistant", text: activeStreamText,
                        toolName: nil, status: "running"
                    )
                }
                merged.append(live!)
            }
        }
        return bound(merged)
    }

    static func bound(_ items: [PADChatItem]) -> [PADChatItem] {
        items.count > maxItems ? Array(items.suffix(maxItems)) : items
    }

    // MARK: - Helpers

    private static func ensureStream(
        taskId: String,
        transcript: inout [PADChatItem],
        stream: inout PADStreamState?
    ) {
        // Rebuild if the tracked item is gone: bounded transcripts, a history
        // merge, or an interrupted run can drop the stream id while `stream`
        // still points at it, which would silently swallow later deltas.
        if let messageId = stream?.messageId,
           transcript.contains(where: { $0.id == messageId }) {
            return
        }
        let id = "live-\(taskId)-\(UUID().uuidString)"
        stream = PADStreamState(messageId: id)
        transcript.append(PADChatItem(
            id: id, role: "assistant", text: "", toolName: nil, status: "running"
        ))
    }

    private static func updateTextBlock(
        index: Int, text: String, stream: inout PADStreamState?, transcript: inout [PADChatItem]
    ) {
        stream?.textBlocks[index] = capped(text)
        guard let blocks = stream?.textBlocks else { return }
        let text = blocks.keys.sorted().compactMap { blocks[$0] }.joined(separator: "\n")
        replaceStreamText(stream: stream, transcript: &transcript, text: text)
    }

    private static func replaceStreamText(
        stream: PADStreamState?,
        transcript: inout [PADChatItem],
        text: String
    ) {
        guard let messageId = stream?.messageId,
              let index = transcript.lastIndex(where: { $0.id == messageId }) else { return }
        transcript[index].text = capped(text)
        transcript[index].status = "running"
    }

    private static func finishAssistant(
        message: PADJSONValue?,
        taskId: String,
        transcript: inout [PADChatItem],
        stream: inout PADStreamState?,
        result: inout PADReduceResult
    ) {
        let object = message?.objectValue
        let finalText = textContent(object?["content"] ?? object?["text"])
        let failed = isErrorMessage(object ?? [:])
        let errorText = object?["errorMessage"]?.stringValue

        if let messageId = stream?.messageId,
           let index = transcript.lastIndex(where: { $0.id == messageId }) {
            if finalText.isEmpty {
                if transcript[index].text.isEmpty {
                    transcript.remove(at: index)
                } else {
                    transcript[index].status = failed ? "error" : nil
                    transcript[index].isError = failed
                }
            } else {
                transcript[index].text = capped(finalText)
                transcript[index].status = failed ? "error" : nil
                transcript[index].isError = failed
            }
        } else if !finalText.isEmpty {
            transcript.append(PADChatItem(
                id: "final-\(taskId)-\(UUID().uuidString)", role: "assistant", text: capped(finalText),
                toolName: nil, status: failed ? "error" : nil, isError: failed
            ))
        }
        stream = nil
        result.immediateFlush = true
        result.statusHint = failed ? "error" : "running"
        if failed { result.errorText = errorText.map { capped($0, limit: 1_000) } ?? "模型返回错误。" }
    }

    private static func settle(_ transcript: inout [PADChatItem]) {
        for index in transcript.indices {
            guard transcript[index].status == "running" else { continue }
            transcript[index].status = transcript[index].role == "tool" ? "done" : nil
        }
    }

    private static func appendUser(text: String, to transcript: inout [PADChatItem]) {
        guard !text.isEmpty else { return }
        guard !transcript.contains(where: { $0.role == "user" && $0.text == text }) else { return }
        transcript.append(PADChatItem(
            id: "live-user-\(UUID().uuidString)", role: "user", text: capped(text), toolName: nil, status: nil
        ))
    }

    private static func upsertTool(
        id: String,
        toolName: String?,
        text: String?,
        status: String,
        isError: Bool,
        in transcript: inout [PADChatItem]
    ) {
        if let index = transcript.lastIndex(where: { $0.id == id }) {
            if let toolName { transcript[index].toolName = toolName }
            if let text, !text.isEmpty { transcript[index].text = capped(text, limit: maxToolSummaryLength) }
            transcript[index].status = status
            transcript[index].isError = isError
        } else {
            transcript.append(PADChatItem(
                id: id, role: "tool", text: capped(text ?? "", limit: maxToolSummaryLength),
                toolName: toolName, status: status, isError: isError
            ))
        }
    }

    private struct ToolCall {
        let id: String
        let name: String?
        let arguments: PADJSONValue?
    }

    private static func toolCalls(in content: PADJSONValue?) -> [ToolCall] {
        guard let parts = content?.arrayValue else { return [] }
        return parts.compactMap { part in
            guard let object = part.objectValue,
                  object["type"]?.stringValue == "toolCall",
                  let id = object["id"]?.stringValue else { return nil }
            return ToolCall(id: id, name: object["name"]?.stringValue, arguments: object["arguments"])
        }
    }

    private static func isErrorMessage(_ message: [String: PADJSONValue]) -> Bool {
        if let stopReason = message["stopReason"]?.stringValue, stopReason == "error" { return true }
        if let error = message["errorMessage"]?.stringValue, !error.isEmpty { return true }
        return false
    }

    static func textContent(_ value: PADJSONValue?) -> String {
        guard let value else { return "" }
        switch value {
        case let .string(text):
            return text
        case let .array(parts):
            return parts.compactMap { textContent($0) }.filter { !$0.isEmpty }.joined(separator: "\n")
        case let .object(object):
            if let type = object["type"]?.stringValue,
               ["thinking", "toolCall", "image"].contains(type) {
                return ""
            }
            for key in ["text", "content", "message"] {
                let text = textContent(object[key])
                if !text.isEmpty { return text }
            }
            return ""
        default:
            return ""
        }
    }

    private static func compactJSON(_ value: PADJSONValue?) -> String {
        guard let value, let data = try? JSONEncoder().encode(value),
              let text = String(data: data, encoding: .utf8) else { return "" }
        return text
    }

    private static func capped(_ text: String, limit: Int = maxTextLength) -> String {
        text.count > limit ? String(text.prefix(limit)) : text
    }
}
#endif
