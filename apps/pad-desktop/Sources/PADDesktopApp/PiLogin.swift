import AppKit
import Foundation
import SwiftUI

struct PiLoginOption: Identifiable, Hashable {
    let id: String
    let label: String
    let description: String?
}

struct PiLoginPrompt: Identifiable, Hashable {
    let id: String
    let kind: String
    let message: String
    let placeholder: String?
    let options: [PiLoginOption]
}

struct PiLoginEvent: Identifiable, Hashable {
    let id = UUID()
    let kind: String
    let message: String
    let url: URL?
    let userCode: String?
}

enum PiLoginPhase: Equatable {
    case idle
    case running
    case succeeded
    case failed(String)
    case cancelled
}

@MainActor
final class PiLoginCoordinator: NSObject, ObservableObject {
    @Published var provider = "openai-codex"
    @Published var authType = "oauth"
    @Published private(set) var phase: PiLoginPhase = .idle
    @Published private(set) var prompt: PiLoginPrompt?
    @Published private(set) var events: [PiLoginEvent] = []

    private var process: Process?
    private var input: FileHandle?
    private var output: FileHandle?
    private var errorOutput: FileHandle?
    private var outputBuffer = Data()
    private var preparedProfileID: String?
    private var stderrBuffer = Data()

    /// The login helper uses Pi's public SDK instead of automating the Pi TUI.
    /// Its stdin/stdout are JSONL so the Swift sheet can render every OAuth,
    /// API-key, selection and device-code prompt as a native macOS control.
    private static let loginScript = #"""
    import { createInterface } from "node:readline";
    import { randomUUID } from "node:crypto";
    import path from "node:path";
    import { ModelRuntime } from "@earendil-works/pi-coding-agent";

    const send = (value) => process.stdout.write(JSON.stringify(value) + "\n");
    const pending = new Map();
    const readline = createInterface({ input: process.stdin });
    readline.on("line", (line) => {
      try {
        const value = JSON.parse(line);
        if (value.type === "response" && pending.has(value.id)) {
          pending.get(value.id)(value);
          pending.delete(value.id);
        }
      } catch {
        // Ignore malformed renderer responses; the Swift side only sends JSONL.
      }
    });

    const waitForResponse = (id) => new Promise((resolve) => pending.set(id, resolve));
    const interaction = {
      prompt: async (value) => {
        const id = randomUUID();
        send({ type: "prompt", id, kind: value.type, message: value.message,
          placeholder: value.placeholder, options: value.options ?? [] });
        const response = await waitForResponse(id);
        if (response.cancelled) throw new Error("用户取消了登录");
        return String(response.value ?? "");
      },
      notify: (event) => send({ type: "event", event }),
    };

    try {
      const agentDir = process.env.PAD_LOGIN_AGENT_DIR;
      const runtime = await ModelRuntime.create({
        authPath: path.join(agentDir, "auth.json"),
        modelsPath: path.join(agentDir, "models.json"),
        refreshOnCreate: false,
      });
      await runtime.login(process.env.PAD_LOGIN_PROVIDER, process.env.PAD_LOGIN_TYPE, interaction);
      send({ type: "success", provider: process.env.PAD_LOGIN_PROVIDER });
    } catch (error) {
      send({ type: "error", message: error instanceof Error ? error.message : String(error) });
      process.exitCode = 1;
    } finally {
      readline.close();
    }
    """#

    func prepare(profile: Profile) {
        // Re-opening the same Profile after a completed/cancelled attempt must
        // start from a clean sheet. Keep the guard only while an attempt is
        // actively running so SwiftUI re-renders do not restart the process.
        if preparedProfileID == profile.id && phase == .running { return }
        cancel()
        preparedProfileID = profile.id
        provider = profile.subtitle.hasPrefix("默认服务商：")
            ? String(profile.subtitle.dropFirst("默认服务商：".count))
            : "openai-codex"
        authType = "oauth"
        phase = .idle
        prompt = nil
        events = []
    }

    func start(profile: Profile) {
        guard phase != .running else { return }
        guard let agentDirectory = profile.agentDirectory,
              !agentDirectory.isEmpty else {
            phase = .failed("当前 Profile 没有可用的 Pi agent 目录。")
            return
        }
        guard let node = findNode(), let packageRoot = findPiPackageRoot() else {
            phase = .failed("未找到 Pi SDK。请先安装 Pi，再重试。")
            return
        }

        let process = Process()
        let stdin = Pipe()
        let stdout = Pipe()
        let stderr = Pipe()
        process.executableURL = URL(fileURLWithPath: node)
        process.arguments = ["--input-type=module", "-e", Self.loginScript]
        process.currentDirectoryURL = URL(fileURLWithPath: packageRoot)
        process.standardInput = stdin
        process.standardOutput = stdout
        process.standardError = stderr

        var environment = ProcessInfo.processInfo.environment
        environment["PAD_LOGIN_PROVIDER"] = provider.trimmingCharacters(in: .whitespacesAndNewlines)
        environment["PAD_LOGIN_TYPE"] = authType
        environment["PAD_LOGIN_AGENT_DIR"] = agentDirectory
        environment["PI_CODING_AGENT_DIR"] = agentDirectory
        if let bundledLibraries = Bundle.main.resourceURL?.appendingPathComponent("lib").path,
           FileManager.default.fileExists(atPath: bundledLibraries) {
            let inheritedLibraries = environment["DYLD_LIBRARY_PATH"]
            environment["DYLD_LIBRARY_PATH"] = [bundledLibraries, inheritedLibraries]
                .compactMap { $0 }
                .joined(separator: ":")
        }
        process.environment = environment
        process.terminationHandler = { [weak self] terminated in
            Task { @MainActor in
                guard let self, self.process === process else { return }
                if self.phase == .running {
                    self.phase = terminated.terminationStatus == 0 ? .succeeded : .failed(self.stderrMessage())
                }
                self.cleanupProcess()
            }
        }

        phase = .running
        prompt = nil
        events = []
        outputBuffer = Data()
        stderrBuffer = Data()
        do {
            try process.run()
            self.process = process
            input = stdin.fileHandleForWriting
            output = stdout.fileHandleForReading
            errorOutput = stderr.fileHandleForReading
            output?.readabilityHandler = { [weak self] handle in
                let data = handle.availableData
                guard !data.isEmpty else { return }
                Task { @MainActor in self?.consume(data: data) }
            }
            errorOutput?.readabilityHandler = { [weak self] handle in
                let data = handle.availableData
                guard !data.isEmpty else { return }
                Task { @MainActor in self?.stderrBuffer.append(data) }
            }
        } catch {
            phase = .failed("Pi 登录进程启动失败：\(error.localizedDescription)")
            cleanupProcess()
        }
    }

    func respond(value: String) {
        guard let prompt, let input else { return }
        let response: [String: Any] = ["type": "response", "id": prompt.id, "value": value]
        write(response, to: input)
        self.prompt = nil
    }

    func cancel() {
        guard phase == .running || process != nil else { return }
        process?.terminate()
        phase = .cancelled
        cleanupProcess()
    }

    private func consume(data: Data) {
        outputBuffer.append(data)
        while let newline = outputBuffer.firstIndex(of: 0x0A) {
            let line = outputBuffer.prefix(upTo: newline)
            outputBuffer.removeSubrange(...newline)
            guard let object = try? JSONSerialization.jsonObject(with: line) as? [String: Any],
                  let type = object["type"] as? String else { continue }
            switch type {
            case "prompt":
                let options = (object["options"] as? [[String: Any]] ?? []).compactMap { item -> PiLoginOption? in
                    guard let id = item["id"] as? String, let label = item["label"] as? String else { return nil }
                    return PiLoginOption(id: id, label: label, description: item["description"] as? String)
                }
                prompt = PiLoginPrompt(id: object["id"] as? String ?? UUID().uuidString,
                                       kind: object["kind"] as? String ?? "text",
                                       message: object["message"] as? String ?? "请输入",
                                       placeholder: object["placeholder"] as? String,
                                       options: options)
            case "event":
                guard let event = object["event"] as? [String: Any] else { continue }
                let kind = event["type"] as? String ?? "info"
                let url = (event["url"] as? String).flatMap(URL.init(string:))
                    ?? (event["verificationUri"] as? String).flatMap(URL.init(string:))
                let message = event["message"] as? String
                    ?? event["instructions"] as? String
                    ?? (event["verificationUri"] as? String ?? "")
                events.append(PiLoginEvent(kind: kind, message: message, url: url, userCode: event["userCode"] as? String))
            case "success":
                phase = .succeeded
                prompt = nil
            case "error":
                phase = .failed(object["message"] as? String ?? "Pi 登录失败")
                prompt = nil
            default:
                continue
            }
        }
    }

    private func write(_ object: [String: Any], to handle: FileHandle) {
        guard let data = try? JSONSerialization.data(withJSONObject: object) else { return }
        try? handle.write(contentsOf: data + Data([0x0A]))
    }

    private func stderrMessage() -> String {
        let value = String(data: stderrBuffer, encoding: .utf8)?.trimmingCharacters(in: .whitespacesAndNewlines)
        return value?.isEmpty == false ? value! : "Pi 登录失败，请检查服务商和网络配置。"
    }

    private func cleanupProcess() {
        output?.readabilityHandler = nil
        errorOutput?.readabilityHandler = nil
        process = nil
        input = nil
        output = nil
        errorOutput = nil
    }

    private func findNode() -> String? {
        [
            Bundle.main.resourceURL?.appendingPathComponent("bin/node").path,
            "/opt/homebrew/bin/node",
            "/usr/local/bin/node",
            "/usr/bin/node",
        ]
            .compactMap { $0 }
            .first(where: { FileManager.default.isExecutableFile(atPath: $0) && nodeCanRun(at: $0) })
    }

    private func nodeCanRun(at path: String) -> Bool {
        let probe = Process()
        probe.executableURL = URL(fileURLWithPath: path)
        probe.arguments = ["--version"]
        let sink = Pipe()
        probe.standardOutput = sink
        probe.standardError = sink
        do {
            try probe.run()
            probe.waitUntilExit()
            return probe.terminationStatus == 0
        } catch {
            return false
        }
    }

    private func findPiPackageRoot() -> String? {
        let candidates = [
            Bundle.main.resourceURL?.appendingPathComponent("pi").path,
            "/opt/homebrew/lib/node_modules/@earendil-works/pi-coding-agent",
            "/usr/local/lib/node_modules/@earendil-works/pi-coding-agent",
        ].compactMap { $0 }
        return candidates.first(where: { FileManager.default.fileExists(atPath: $0 + "/package.json") })
    }

    deinit {
        output?.readabilityHandler = nil
        errorOutput?.readabilityHandler = nil
        process?.terminate()
    }
}

struct PiLoginSheet: View {
    let profile: Profile
    @ObservedObject var coordinator: PiLoginCoordinator
    let onDone: () -> Void
    @State private var promptValue = ""

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            HStack(spacing: 12) {
                Image(systemName: "person.crop.circle.badge.checkmark")
                    .font(.system(size: 27))
                    .foregroundStyle(Color.accentColor)
                VStack(alignment: .leading, spacing: 3) {
                    Text("登录 Pi 账号")
                        .font(.title3.weight(.semibold))
                    Text(profile.name)
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
                Spacer()
                Button("取消") {
                    coordinator.cancel()
                    onDone()
                }
                .keyboardShortcut(.cancelAction)
            }
            .padding(22)
            Divider()

            ScrollView {
                VStack(alignment: .leading, spacing: 16) {
                    if coordinator.phase == .idle {
                        Text("凭据只会保存到当前 Profile 的 Pi agent 目录，不会写入 Codex 或 ChatGPT 会话。")
                            .font(.callout)
                            .foregroundStyle(.secondary)
                            .fixedSize(horizontal: false, vertical: true)
                        TextField("服务商，例如 openai-codex、anthropic", text: $coordinator.provider)
                            .textFieldStyle(.roundedBorder)
                        Picker("登录方式", selection: $coordinator.authType) {
                            Text("订阅 / OAuth").tag("oauth")
                            Text("API Key").tag("api_key")
                        }
                        .pickerStyle(.segmented)
                        Button {
                            coordinator.start(profile: profile)
                        } label: {
                            Label("开始登录", systemImage: "arrow.right.circle.fill")
                                .frame(maxWidth: .infinity)
                        }
                        .buttonStyle(.borderedProminent)
                    } else if coordinator.phase == .running {
                        Text("正在通过 Pi 完成登录…")
                            .font(.callout.weight(.medium))
                        ForEach(coordinator.events) { event in
                            VStack(alignment: .leading, spacing: 7) {
                                Text(event.message)
                                    .font(.callout)
                                    .textSelection(.enabled)
                                if let code = event.userCode {
                                    Text("设备验证码：\(code)")
                                        .font(.headline.monospaced())
                                }
                                if let url = event.url {
                                    Link("打开授权页面", destination: url)
                                        .font(.callout.weight(.medium))
                                }
                            }
                            .padding(11)
                            .background(Color.accentColor.opacity(0.08), in: RoundedRectangle(cornerRadius: 9))
                        }
                        if let prompt = coordinator.prompt {
                            LoginPromptView(prompt: prompt, value: $promptValue) { value in
                                coordinator.respond(value: value)
                                promptValue = ""
                            }
                        }
                        Button("停止登录") { coordinator.cancel() }
                            .buttonStyle(.bordered)
                    } else {
                        switch coordinator.phase {
                        case .succeeded:
                            ResultView(icon: "checkmark.circle.fill", color: .green, title: "Pi 账号登录成功", detail: "凭据已保存到当前 Profile。", actionTitle: "完成", action: onDone)
                        case .failed(let message):
                            ResultView(icon: "xmark.octagon.fill", color: .red, title: "登录失败", detail: message, actionTitle: "关闭", action: onDone)
                        case .cancelled:
                            ResultView(icon: "pause.circle.fill", color: .orange, title: "已取消登录", detail: "你可以稍后再次尝试。", actionTitle: "关闭", action: onDone)
                        default:
                            EmptyView()
                        }
                    }
                }
                .padding(22)
            }
        }
        .frame(width: 520, height: 520)
        .onAppear { coordinator.prepare(profile: profile) }
        .onChange(of: coordinator.prompt?.id) { _ in promptValue = "" }
    }
}

struct LoginPromptView: View {
    let prompt: PiLoginPrompt
    @Binding var value: String
    let submit: (String) -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 9) {
            Text(prompt.message)
                .font(.callout.weight(.medium))
            if prompt.kind == "select" {
                ForEach(prompt.options) { option in
                    Button {
                        submit(option.id)
                    } label: {
                        VStack(alignment: .leading, spacing: 2) {
                            Text(option.label)
                            if let description = option.description {
                                Text(description)
                                    .font(.caption)
                                    .foregroundStyle(.secondary)
                            }
                        }
                        .frame(maxWidth: .infinity, alignment: .leading)
                    }
                    .buttonStyle(.bordered)
                }
            } else {
                HStack(spacing: 8) {
                    if prompt.kind == "secret" {
                        SecureField(prompt.placeholder ?? "请输入", text: $value)
                            .textFieldStyle(.roundedBorder)
                    } else {
                        TextField(prompt.placeholder ?? "请输入", text: $value)
                            .textFieldStyle(.roundedBorder)
                    }
                    Button("提交") { submit(value) }
                        .buttonStyle(.borderedProminent)
                        .disabled(value.isEmpty)
                }
                .onSubmit { submit(value) }
            }
        }
        .padding(12)
        .background(Color.secondary.opacity(0.08), in: RoundedRectangle(cornerRadius: 9))
    }
}

struct ResultView: View {
    let icon: String
    let color: Color
    let title: String
    let detail: String
    let actionTitle: String
    let action: () -> Void

    var body: some View {
        VStack(spacing: 13) {
            Image(systemName: icon)
                .font(.system(size: 42))
                .foregroundStyle(color)
            Text(title)
                .font(.title3.weight(.semibold))
            Text(detail)
                .font(.callout)
                .foregroundStyle(.secondary)
                .multilineTextAlignment(.center)
                .textSelection(.enabled)
            Button(actionTitle, action: action)
                .buttonStyle(.borderedProminent)
        }
        .frame(maxWidth: .infinity, minHeight: 260)
    }
}
