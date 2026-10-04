// Compile with native/{PADWorkbenchTypes,PADHostTransport,PADMessageReducer}.swift.
// Run after workbench-smoke.mjs with PAD_TERMINAL_HOST and its isolated data root.
import Foundation

@main
struct PADNativeSmoke {
    @MainActor
    static func main() async throws {
        let host = PADHostTransport()
        try host.start()
        func request(_ command: String, _ fields: [String: PADJSONValue] = [:]) async throws -> PADJSONValue {
            try await withCheckedThrowingContinuation { continuation in
                host.send(command: command, fields: fields) { continuation.resume(with: $0) }
            }
        }
        let snapshot = try await request("snapshot").decoded(PADSnapshot.self)!
        precondition(snapshot.tasks.count == 1 && snapshot.profiles.count == 2)
        let task = snapshot.tasks[0]
        let catalog = try await request("catalog", ["profileId": .string(task.profileId)]).decoded(PADCatalog.self)!
        precondition(!catalog.providers.isEmpty && catalog.models.isEmpty)
        let history = try await request("history", ["taskId": .string(task.id)])
        precondition(PADMessageReducer.parseHistory(history).items.first?.text.contains("离线历史夹具") == true)
        _ = try await request("shutdown")
        try await Task.sleep(nanoseconds: 300_000_000)
        precondition(!host.isRunning, "Host must exit after shutdown")

        // Recorded-shape fixtures, not real model responses. Test block-wise
        // reconstruction, authoritative completion, UTF-8, and tool identity.
        var items: [PADChatItem] = []
        var stream: PADStreamState?
        @discardableResult
        func reduce(_ json: String) throws -> PADReduceResult {
            let event = try JSONDecoder().decode(PADJSONValue.self, from: Data(json.utf8))
            return PADMessageReducer.applyPiEvent(event, taskId: "fixture", transcript: &items, stream: &stream)
        }
        try reduce(#"{"type":"message_start","message":{"role":"assistant","content":[]}}"#)
        try reduce(#"{"type":"message_update","assistantMessageEvent":{"type":"text_delta","contentIndex":0,"delta":"中文\u2028OK"}}"#)
        try reduce(#"{"type":"message_update","assistantMessageEvent":{"type":"text_end","contentIndex":0,"content":"中文\u2028OK"}}"#)
        try reduce(#"{"type":"message_update","assistantMessageEvent":{"type":"text_delta","contentIndex":2,"delta":"second block"}}"#)
        try reduce(#"{"type":"message_update","assistantMessageEvent":{"type":"text_end","contentIndex":2,"content":"second block"}}"#)
        precondition(items[0].text == "中文\u{2028}OK\nsecond block")
        try reduce(#"{"type":"message_end","message":{"role":"assistant","content":[{"type":"text","text":"authoritative final"}],"stopReason":"stop"}}"#)
        precondition(items.filter { $0.role == "assistant" }.count == 1 && items[0].text == "authoritative final")
        try reduce(#"{"type":"tool_execution_start","toolCallId":"fixture-tool","toolName":"bash","args":{"command":"printf fixture"}}"#)
        try reduce(#"{"type":"tool_execution_end","toolCallId":"fixture-tool","toolName":"bash","result":{"content":[{"type":"text","text":"fixture"}]},"isError":false}"#)
        precondition(items.filter { $0.role == "tool" }.count == 1 && items.last?.status == "done")
        let settled = try reduce(#"{"type":"agent_settled"}"#)
        precondition(settled.settled && stream == nil)
        print("PAD_NATIVE_TRANSPORT_AND_REDUCER_OK; modelRequests=0; liveModelReplyTested=false")
    }
}
