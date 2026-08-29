# PAD Desktop（macOS）

PAD Desktop 是只面向 Apple Silicon、macOS 13 及以上版本的 Pi 桌面工作台。当前主应用
使用 Electron + React/TypeScript，实现接近 macOS Codex 的平铺侧边栏、任务时间线、
Composer、右侧检查器、底部终端和两栏设置；Rust `pad __internal desktop-server` 是唯一
可信控制面，Pi 是唯一 Agent runtime。旧 SwiftUI 版本只保留作回滚壳，不再是发布入口。

界面、菜单、登录流程、错误与权限说明均为简体中文。Pi 返回的模型内容保持原样。

## 安装后怎么用

1. 打开 `PAD Desktop.app`。首次启动会建立一个默认 Pi Profile 和 Workspace。
2. 点击左下角账号，再点“管理账号”；可以新增 Profile、选择 provider，并在 macOS
   Sheet 中完成 OAuth、设备码或输入式登录。整个登录流程由 Rust 启动受管 Pi 进程，
   不会弹出 Terminal。
3. 点击“新任务”或按 `⌘N`，在底部输入任务；`Enter` 发送，`Shift+Enter` 换行。
4. 运行中发送键会变成“停止”，失败后会变成“重试”。右上角可打开任务检查器和真实
   NativePty 终端；终端支持中文输入、方向键、resize 和退出回收。
5. 任务右上角的“更多”菜单可固定、标为未读或归档。`⌘K` 搜索当前账号，`⌘B`
   显示/隐藏侧边栏，`⌘,` 打开设置。
6. 每个 Profile 的任务、登录、Pi root、Session 与 UI 状态相互隔离；切换账号时旧任务树
   会先清空，再载入目标账号。
7. 打开“设置 → 远程连接”，开启网关并点击“连接 iPhone”；用 PAD iOS 扫描短期二维码后，
   可以实时查看和继续这台 Mac 上的对话。网络短暂切换时客户端会自动续接，已配对设备也可
   随时从 Mac 端撤销。

“完全访问”默认开启，用于让普通任务自动执行，减少逐次确认。它不会自动放行 PAD/Pi/
Codex/ChatGPT 私有目录、Provider 凭据、跨 Profile 路径、macOS TCC 和系统保护区域；可在
Composer 或“设置 → 权限与访问”中关闭。

附件、任务级模型和推理强度都通过真实 Pi RPC 生效。登录项、系统通知和安全清理入口仍会
明确显示为尚未开放，不会伪装成已经可用。

## 数据边界

PAD Desktop 只使用自己的数据根：

```text
~/Library/Application Support/PAD Desktop/v1/store/pad.sqlite
~/Library/Application Support/PAD Desktop/v1/profiles/<profile-id>/pi-agent
~/Library/Application Support/PAD Desktop/v1/profiles/<profile-id>/pi-sessions
```

它不会导入、迁移或写入 `~/.codex`、ChatGPT container，也不会复用用户独立的 Pi root。
同一 PAD 数据根只允许一个 desktop-server writer，第二个实例会在打开 SQLite/Pi journal
之前失败。

## 本地开发

```bash
cd /path/to/pad/rust-tui
cargo build --locked

cd ../apps/pad-desktop
npm ci
npm run dev
```

开发模式可以用 `PAD_BIN=/absolute/path/to/pad npm run dev` 指定 Rust host。Renderer 不会
直接读取 SQLite、环境变量或凭据，也不能自行启动命令。

远程二维码中的短期配对票据只存在于当前配对 Sheet 的组件内存；它不会进入任务记录、
Electron 日志、localStorage、辅助功能文本或 `remote_changed` 事件。远程状态只向 renderer
暴露设备显示名、平台、在线状态与安全错误码，不暴露监听地址、证书指纹或本地路径。

## 构建完整 App

构建机使用固定的 Node **24.20.0**、arm64 Bun **1.3.14** 和
`@earendil-works/pi-coding-agent` **0.84.4**；
脚本会精确校验版本，不接受“任意兼容版本”。目标 Mac 不需要 Homebrew、Node、Bun 或预装
Pi。打包时还会校验关键 Mach-O 的 deployment target 均不高于 `Info.plist` 声明的 macOS
13.0。

```bash
cd /path/to/pad/apps/pad-desktop
./scripts/package-electron-app.sh
```

完整 App 输出到：

```text
out/PAD Desktop-darwin-arm64/PAD Desktop.app
```

App 内的 `Contents/Resources/release-evidence/` 同时包含：

- `runtime-manifest.json`：目标平台、最低系统、签名模式及固定 runtime 版本；
- `runtime-sbom.spdx.json`：SPDX 2.3 runtime SBOM；
- `runtime-SHA256SUMS.txt`：PAD、Pi、Bun、启动包装器和 SBOM 的 SHA-256，可在
  `Contents/Resources` 下执行 `shasum -a 256 -c release-evidence/runtime-SHA256SUMS.txt`
  复核。

生成 ZIP、DMG 和 SHA-256 清单：

```bash
./scripts/release-electron-app.sh /absolute/output/directory
```

本机验收包使用 ad-hoc 签名。面向其他 Mac 分发前仍需要 Developer ID Application
签名、公证与 staple；未完成这些步骤时不能声称已通过外部 Gatekeeper。

有 Developer ID 证书时，使用显式的外部分发路径。先用 `notarytool store-credentials`
把凭据保存到 Keychain Profile，再执行：

```bash
PAD_DESKTOP_SIGN_IDENTITY='Developer ID Application: Example Corp (TEAMID)' \
PAD_DESKTOP_NOTARY_PROFILE='pad-desktop-notary' \
./scripts/release-electron-app.sh /absolute/output/directory
```

如 Profile 位于非默认 Keychain，可额外设置
`PAD_DESKTOP_NOTARY_KEYCHAIN=/absolute/path/to/keychain-db`。此路径会严格执行 Developer ID
hardened-runtime 签名、App 公证与 staple、DMG 公证与 staple，以及 `spctl` 评估；任一步失败
都不会生成“完成”的发布结果。未设置证书时输出会明确标记 `LOCAL-ONLY`，不会把 ad-hoc 包
描述为已通过 Gatekeeper。发布目录还会得到独立 runtime manifest、SPDX SBOM、runtime
checksums、发布证据和制品 SHA-256；Developer ID 路径另含 Apple 公证 JSON 回执。

安装到 `/Applications`：

```bash
./scripts/install-electron-app.sh --source \
  '/absolute/path/to/PAD Desktop.app' --launch
```

安装脚本先备份旧版本，再用隔离的 HOME、PAD data root 和 Electron user-data 启动新版本；
只有 renderer 最终 ready、Rust backend ready、protocol v2 可用、无可见 fatal alert 且整个
进程组可干净退出时才打印 `INSTALL_COMPLETE`。探针失败会保存失败的新包并恢复旧版本。

## 发布门禁

```bash
cd /path/to/pad/rust-tui
cargo fmt --all -- --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked

cd ../apps/pad-desktop
npm run typecheck
npm run test:ui
npm audit --omit=dev --audit-level=high
npm audit --audit-level=high

cd ../..
python3 scripts/ci/pad_desktop_electron_e2e.py \
  --artifact-dir /tmp/pad-desktop-e2e
python3 scripts/ci/pad_desktop_visual.py \
  --output-dir /tmp/pad-desktop-visual
python3 scripts/ci/pad_desktop_perf.py \
  --artifact-dir /tmp/pad-desktop-perf
python3 scripts/ci/check_index.py
git diff --check
```

视觉脚本在没有 `--baseline-dir` 时只验证布局矩阵、溢出、中文化、键盘焦点与辅助功能，
顶层结果会标记为 `PARTIAL`，Golden 相似度覆盖项标记为 `NOT_EVALUATED`，不会把它冒充为
Codex Golden/SSIM 对齐结果。若有经授权、
版本匹配的 Codex Golden，可额外传入 `--baseline-dir /absolute/path/to/golden`。
