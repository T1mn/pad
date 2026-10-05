// Offline pure filtering only; no UI, host, login, or model requests.
// swiftc apps/pad-terminal/native/PADModelSearch.swift \
//   apps/pad-terminal/scripts/model-search-smoke.swift -o /tmp/pad-model-search-smoke
// /tmp/pad-model-search-smoke
@main
struct PADModelSearchSmoke {
    static func main() {
        let rows = [
            ["GPT-6.1 Sol", "gpt-6.1-sol", "openai-codex", "OpenAI Codex"],
            ["DeepSeek Chat", "deepseek-v3", "deepseek", "DeepSeek"],
            ["Fast Model", "fast-42", "vendor-id", "Example Provider"]
        ]
        func matches(_ query: String) -> [[String]] {
            PADModelSearch.filter(rows, query: query) { $0 }
        }
        precondition(matches("  ") == rows, "Empty query preserves upstream order")
        precondition(matches("GpT 6.1 sOL") == [rows[0]], "Mixed-case name and punctuation")
        precondition(matches("V3") == [rows[1]], "Partial model ID")
        precondition(matches("vendor-id example") == [rows[2]], "Provider ID and display name across words")
        precondition(matches("missing model") == [], "No match")
        print("PAD_MODEL_SEARCH_OK; modelRequests=0; nativePickerInteractionTested=false")
    }
}
