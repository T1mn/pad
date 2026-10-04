#if os(macOS)
import Foundation

/// Search is local-only; typing never starts authentication or sends a request.
enum PADProviderSearch {
    static func filter(_ providers: [PADProvider], query: String, authenticatedOnly: Bool) -> [PADProvider] {
        let terms = query.lowercased().split(whereSeparator: \.isWhitespace)
        return providers.filter { provider in
            guard !authenticatedOnly || provider.authenticated else { return false }
            var aliases = "\(provider.id) \(provider.name)".lowercased()
            if provider.id == "openai" { aliases += " chatgpt gpt codex" }
            if provider.id == "anthropic" { aliases += " claude 克劳德" }
            if provider.id == "google" { aliases += " gemini 谷歌" }
            return terms.allSatisfy { aliases.localizedStandardContains(String($0)) }
        }.sorted { a, b in
            if a.authenticated != b.authenticated { return a.authenticated }
            let common = ["openai", "anthropic", "google", "openai-codex"]
            let ar = common.firstIndex(of: a.id) ?? common.count
            let br = common.firstIndex(of: b.id) ?? common.count
            if ar != br { return ar < br }
            return a.name.localizedStandardCompare(b.name) == .orderedAscending
        }
    }
}
#endif
