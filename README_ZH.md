<div align="center">
  <h1>PAD</h1>
  <p><strong>别再来回找 pane 了。在一个地方跑完整个 AI 工作流。</strong></p>
  <p><code>pad</code> = Panel for Agent Development。</p>
  <p><a href="README.md">English</a> | 中文</p>
</div>

如果你平时会在 tmux 里同时开两个以上 agent session，PAD 基本很快就能派上用场。

## 一句话总结（太长不看）

- 纯 Rust 打造，原生基于 tmux，专为终端智能体而生。
- 看清谁动了，先读 preview，再进对的 pane。
- 当前 macOS 实测：dist 二进制约 3.7 MB，空闲运行时约 12 MB RSS。

## 安装

运行时依赖：`tmux`

支持的运行环境：

- macOS
- Linux
- WSL2

```bash
# 一行安装
curl -fsSL https://raw.githubusercontent.com/T1mn/pad/master/install.sh | bash

# 或者从本地 clone 安装
git clone https://github.com/T1mn/pad.git
cd pad
./install.sh
```

安装脚本会优先尝试下载预编译 release，并在 Linux 上按运行时环境优先选择匹配的 glibc 或 musl 包；下载后还会验证二进制能否在当前机器上运行。只有在没有可用预编译包时，才会提示后回退到本地源码构建。系统里缺少 `tmux` 时，它也会提示你自动安装；进入源码构建路径时，还会按需补齐 Rust 和常见构建依赖。

安装器源码已经拆到 `install/` 目录。修改这些模块后，请重新生成仓库里提交的单文件 `install.sh`：

```bash
bash scripts/build_installer.sh
```

手动源码构建：

```bash
cd pad/rust-tui
cargo build --profile dist
cp target/dist/pad ~/.local/bin/
```

PAD 是 tmux-first 的工具。`pad` 和 `tmux` 需要运行在同一个环境里。  
macOS 下推荐使用 [Ghostty](https://ghostty.org/)，它和 tmux/TUI 的刷新体验通常更顺。  
如果你在 WSL2 下使用，请确保两者都安装并运行在 WSL 内部。

## 演示

<video src="https://github.com/user-attachments/assets/773baf57-c25f-41d4-a30a-3c38e702d2d8" controls muted loop playsinline width="960"></video>

实际用起来大概就是这样：

- 在左侧树里扫一眼 live sessions，不用再回 tmux 到处找 pane
- 在右侧先读最新 preview，再决定要不要 attach
- 按 `Tab` 进入最新 detail，再用 `Shift+J` / `Shift+K` 在 Q/A 间切换
- 按 `F2` 改 thread 标题，按 `T` 编辑标签，不用离开 PAD
- 用 `c` 新建一个 session，发出任务后按 `F12` 回到 PAD
- 通过活动指示看出哪个 session 还在后台工作

如果你的 Markdown 查看器不支持内联视频，可以直接打开这个[演示视频](https://github.com/user-attachments/assets/773baf57-c25f-41d4-a30a-3c38e702d2d8)。

## 为什么要有 PAD

tmux 里同时跑多个 agent 之后，最烦的往往不是“不会用”，而是这些很碎的事：

- 哪个 pane 刚刚动过？
- 哪个 session 现在还在工作？
- 我到底要不要 attach，还是 preview 里已经有答案了？
- 我 archive 这个 thread，到底只是隐藏，还是会真的删掉数据？

PAD 把这些动作收进一个地方：扫描、预览、attach、archive，然后再快速退回来继续看全局。

## 30 秒上手路径

1. 运行 `pad`
2. 在左侧 sidebar 找到刚刚有变化的 session
3. 在 agent pane 里按 `F10`，需要时打开项目侧边浏览
4. 先在 preview 里看最近几轮对话，再决定要不要 attach
5. 用 `Enter` 进入，用 `F12` 或 `Ctrl+Q` 回来

## 核心能力

- 左侧一栏同时看 live pane 和最近的 session history
- attach 之前先把最近几轮对话读一眼
- 纯 Rust TUI，体积小，session-aware preview 响应快
- 当前 macOS 实测：dist 二进制约 3.7 MB，空闲 RSS 约 12 MB
- session 级监听，活动追踪更聚焦，也更省资源
- `Enter` 进去，`F12` / `Ctrl+Q` 退回 PAD
- `F10` 打开 pad-sider，在 agent 旁边看 tree、index map、changes 和文件预览
- archive 只动 PAD 自己的索引，不碰上游原始 session 数据
- 支持的 agent relay / proxy 配置
- 在支持的平台上，在 agent 完成任务后发送桌面通知
- Telegram Bot 守护进程，可用于远程查看更新和快速进入 session
- 全键盘操作的 search、settings、tree 和 session 创建

## PAD 不做什么

- 它不替代 tmux
- 它不在 tmux 上面伪造一层 tabs
- 它不会在 archive 时删除上游 agent 的原始历史
- 它不接管 agent runtime，本质上是让你更快地看清、跳转、返回

## 界面导览

### 首页 Overview

<img src="docs/media/first-annotated.png" alt="PAD 首页结构说明" width="960">

这里是你进入 PAD 后最先看的地方，也是最快的扫描视图。

1. `LIVE 6`：顶部 live inbox 和当前在线 session 数量
2. 高亮 session 行：左侧当前选中的 session，可直接 preview 或 attach
3. Preview 头部：一眼看到 agent、状态、PID、分支、路径、SID
4. Preview turns：先读最近几轮 Q/A，再决定要不要进入 pane

### 设置 Settings

<img src="docs/media/settings-annotated.png" alt="PAD 设置界面说明" width="960">

Settings 是保持在主流程里的。用 `/` 打开，改完后 `Esc` 退出。

1. `/` 入口：设置页沿用了终端工具常见的 slash flow
2. 设置项列表：可以全键盘移动和打开不同配置项
3. 行内当前值：不用逐个点进去，也能直接扫当前配置状态
4. 底部提示：当前可用操作键始终显示在底部，包含可用时的 Codex CLI 检查与升级操作

### 归档 Archive

<img src="docs/media/archive-annotated.png" alt="PAD 归档说明" width="960">

PAD 的 archive 语义是刻意收窄的，和 Codex 的那套心智模型保持一致：从 PAD 里隐藏，但不碰原始 session 数据。

1. 确认弹窗：archive 是显式且可恢复的，不是 delete
2. 目标 thread：你能明确看到这次归档的是哪一个 thread
3. Live pane 提示：如果这个 thread 仍然绑定 live pane，PAD 会明确告诉你它只会在 PAD 中隐藏，并更新 PAD 本地索引
4. 与 Codex 对齐：PAD 只更新自己的 tracking layer，不会动上游原始 session 数据。对 Claude 来说，这意味着 PAD 更新的是自己的 Claude sqlite 索引，而不是修改 `~/.claude` 的原始 session 源

### 文件树 Tree

<img src="docs/media/tree-annotated.png" alt="PAD 文件树说明" width="960">

Tree 模式适合在不离开 PAD 的情况下浏览代码、预览文件，或者直接从一个目录创建 session。

1. 根路径：顶部始终显示当前 workspace 路径
2. 文件树：快速展开、折叠并移动目录与文件
3. 文件预览：右侧即时显示当前文件内容
4. Tree 底栏：tree 模式下的按键提示会固定显示，包括导航、展开、attach、create 和 help

### Pad Sider

`F10` 会在当前 agent pane 旁边打开一个辅助栏。它的作用很直接：不离开对话，也能顺手看代码。

1. 左侧：tree 或 index map，用来快速找项目结构
2. 右侧：文件预览，Markdown 会用更紧凑的方式渲染，并带行号
3. `II`：在 tree 和 index map 间切换
4. `[` / `]`：三档调整 sider 宽度

### 帮助 Help

<img src="docs/media/help-annotated.png" alt="PAD 帮助界面说明" width="960">

Help 把键盘模型直接放在 UI 里，不需要你切出去翻文档。

1. Help 头部：明确告诉你这里是 PAD 内建的键位说明
2. Navigation 区：移动、跳转、搜索等全局导航键集中在一起
3. Actions 区：attach、create、delete、refresh、focus 切换、preview 控制等核心操作集中展示
4. 退出提示：底栏始终显示如何最快返回主界面

## 其他能力

- Preview 头部直接看 Git 信息：分支、提交、变更数
- Live agent pane 的 busy / waiting 状态提示
- 文件树浏览和文件预览
- 主题切换
- 从目录树直接启动 agent session
- 按 session 保存的 thread 自定义标题与标签

## 线程标题与标签

- 按 `F2` 可编辑当前 thread 标题
- 按 `T` 可编辑当前 thread 标签
- 编辑标题时，按 `Shift+Delete` 可快速清空整个输入框
- 自定义标题按 session 保存在 PAD 中；清空后会回退到生成标题或上游标题
- 这些修改只影响 PAD 的本地元数据层，不会改动上游原始 session 历史

## 使用

```bash
pad              # 启动 TUI
pad --help       # 查看帮助
pad --version    # 查看版本
pad telegram-bot # 启动 Telegram Bot 守护进程
```

发布与平台说明：

- [平台支持说明](docs/platform-support.md)
- [发布检查清单](docs/release-checklist.md)

Linux 发布产物现在按运行时家族分开：

- `pad-*-linux-x86_64-glibc-2.35.tar.gz`
- `pad-*-linux-aarch64-glibc-2.35.tar.gz`
- `pad-*-linux-x86_64-musl.tar.gz`
- `pad-*-linux-aarch64-musl.tar.gz`

## 快捷键

| 按键 | 作用 |
|-----|------|
| `j/k` 或 `↑/↓` | 面板导航 |
| `J/K` 或 `Shift+J/K` | 在 preview turns 中快速移动 |
| `1-9` | 快速跳转到面板 |
| `Enter` | attach 到 pane |
| `F12` / `Ctrl+Q` | 返回 PAD |
| `Tab` | 切换 panel / preview 焦点 |
| `Tab` 双击 | 进入最新 preview detail，或从 detail 返回 turns list |
| `?` | 打开帮助 |
| `t` | 切换文件树 |
| `Ctrl+T` | 从 `~/` 打开文件树 |
| `F2` | 编辑 thread 标题 |
| `T` | 编辑 thread 标签 |
| `Shift+Delete` | 编辑标题时清空输入 |
| `Space` | 展开 / 折叠目录 |
| `Space` 双击 | 展开 / 折叠全部 session 文件夹 |
| `F10` | 在当前 pane 旁切换 pad-sider |
| `[` / `]` | 调整 pad-sider 宽度 |
| `II` | 在 pad-sider 中切换 tree / index map |
| `pad-sider 中 /` | 模糊搜索文件 |
| `pad-sider 中对 .md 按 Space` | 打开全屏 Markdown 预览 |
| `c` | 创建新 session |
| `d` | 删除 pane |
| `A` / `U` | 归档 / 恢复选中 session |
| `Z` | 切换归档 session 视图 |
| `E` / `S` | 导出选中的 OpenCode session 原始 JSON / 脱敏 JSON 到 `~/.pad/opencode-exports/` |
| `r` | 刷新 |
| `Ctrl+F` | 搜索 panel |
| `/` | 打开设置 |
| `F1` | 设置 |
| `q` | 退出 |

## Agent 支持情况

完整 session 工作流支持：

- 🟣 Claude (`claude`)
- 🔵 Codex (`codex`)
- 🔷 Gemini (`gemini-cli`)

增强 session / history 支持：

- 🟠 OpenCode (`opencode`)：launcher 与 pane attach、relay/model 配置、SQLite history、session preview、usage/share 元数据、archive/unarchive、`opencode export` / `--sanitize` 导出，以及通过 `opencode --session` 恢复会话

基础 launcher / pane 工作流支持：

- 🟢 Kimi (`kimi-cli`)

PAD 仍然可以识别并 attach 到其他终端 agent。OpenCode 现在已经有可用的 session 工作流，但 hook 驱动的实时事件深度仍然是 Claude、Codex 和 Gemini 更完整。

## 致谢

感谢更广泛的终端工具社区在早期提供的反馈与测试，也感谢我一路上在 [linux.do](https://linux.do) 学到的很多对这个项目有帮助的东西。

## License

MIT
