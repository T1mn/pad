import Foundation

/// Local, stable filtering only: every query word must partially match a field.
/// Punctuation separates words, so `gpt 6.1` also finds `GPT-6.1-Sol`.
enum PADModelSearch {
    static func filter<Item>(_ items: [Item], query: String, fields: (Item) -> [String]) -> [Item] {
        let words = normalized(query).split(separator: " ").map(String.init)
        guard !words.isEmpty else { return items }
        return items.filter { item in
            let values = fields(item).map(normalized)
            return words.allSatisfy { word in values.contains { $0.contains(word) } }
        }
    }

    private static func normalized(_ value: String) -> String {
        value.lowercased().components(separatedBy: CharacterSet.alphanumerics.inverted)
            .filter { !$0.isEmpty }.joined(separator: " ")
    }
}
