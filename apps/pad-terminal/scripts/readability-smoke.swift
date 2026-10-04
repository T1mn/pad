// Offline: provider search/DTO/argv regression. No app, auth, or agent launch.
// Compile with native/{PADWorkbenchTypes,PADHostTransport,PADMessageReducer,
// PADWorkbenchModel,PADLocalSessionsModel,PADProviderSearch}.swift.
import Foundation

@main
struct ReadabilitySmoke {
    static func main() throws {
        let providers = [
            PADProvider(id: "nvidia", name: "NVIDIA", authTypes: ["api_key"], authenticated: false),
            PADProvider(id: "anthropic", name: "Anthropic", authTypes: ["oauth"], authenticated: true),
            PADProvider(id: "openai", name: "OpenAI", authTypes: ["api_key", "oauth"], authenticated: false),
        ]
        precondition(PADProviderSearch.filter(providers, query: " cLaUdE ", authenticatedOnly: false).map(\.id) == ["anthropic"])
        precondition(PADProviderSearch.filter(providers, query: "chatgpt", authenticatedOnly: false).map(\.id) == ["openai"])
        precondition(PADProviderSearch.filter(providers, query: "openai", authenticatedOnly: true).isEmpty)
        precondition(PADProviderSearch.filter(providers, query: "", authenticatedOnly: false).first?.id == "anthropic")
        let json = """
        {"sessions":[{"id":"hash","tool":"pi","sessionId":"id","title":"中文","cwd":"/tmp","file":"/tmp/a.jsonl","updatedAt":"2026-10-04T00:00:00Z"}],"roots":[{"tool":"pi","path":"/tmp","status":"ready"}],"truncated":false,"scannedAt":"2026-10-04T00:00:00Z"}
        """
        let list = try JSONDecoder().decode(PADLocalSessionList.self, from: Data(json.utf8))
        precondition(list.sessions.first?.title == "中文")
        let value = "file's ; $(printf unexpected) 中文"
        let launch = PADTerminalLaunch(cwd: "/tmp", executable: "/usr/bin/printf", args: ["%s", value])
        let process = Process(), pipe = Pipe()
        process.executableURL = URL(fileURLWithPath: "/bin/sh")
        process.arguments = ["-c", launch.shellCommand]
        process.standardOutput = pipe
        try process.run()
        let output = pipe.fileHandleForReading.readDataToEndOfFile()
        process.waitUntilExit()
        precondition(process.terminationStatus == 0 && String(data: output, encoding: .utf8) == value)
        print("PAD_SEARCH_LOCAL_DTO_AND_ARGV_OK; modelRequests=0; agentLaunches=0")
    }
}
