# PAD Desktop

- `Package.swift`：Swift Package Manager 构建入口，macOS 13+、零第三方依赖。
- `Sources/PADDesktopApp/PADDesktopApp.swift`：SwiftUI 应用、Codex 风格 Sidebar、对话区、Composer，以及 PAD `desktop-server` JSONL bridge。
- `Sources/PADDesktopApp/PiLogin.swift`：原生 macOS Pi 登录窗口与 Pi SDK JSONL 交互桥。
- `Resources/Info.plist`：应用包元数据模板。
- `scripts/package-app.sh`：构建 Swift release executable、Rust `pad` host，并生成内含控制面的可双击 `.app` bundle。
- `README.md`：开发、运行、打包和 Pi 配置说明。
