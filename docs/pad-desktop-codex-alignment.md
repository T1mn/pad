# PAD Desktop Codex alignment baseline

This document is the clean-room implementation and acceptance baseline for the
macOS PAD Desktop renderer. It describes observable geometry and behavior from
the locally installed Codex `26.825.41651` (`build 7345`). It must not be used to
copy Codex source code, bundled CSS/JavaScript, icons, branding, authentication,
or private service behavior.

公开产品语义以 OpenAI 的 [Codex App 介绍](https://openai.com/index/introducing-the-codex-app/)
为准：Agent 以独立 thread 运行并按 project 组织。PAD 因此把 Project/Task 作为侧边栏主体，
Profile 切换只保留在底部账号入口，不额外制造一层重复的账号树节点。

## Product boundary

- PAD owns its renderer, icons, Chinese copy, accounts, database, and sessions.
- Rust `pad __internal desktop-server` remains the only control plane.
- Pi remains the agent runtime and session journal owner.
- The Electron renderer cannot read SQLite, credentials, provider directories,
  or spawn a process.
- PAD never imports from or writes to `.codex`, ChatGPT containers, or a user's
  independent Pi root.

## Window and surface hierarchy

```text
BrowserWindow
├── macOS hidden-inset titlebar and traffic lights
└── AppShell
    ├── GlobalTitlebar
    ├── LeftSidebar
    ├── MainContentSurface
    │   ├── TaskToolbar
    │   └── TaskViewport
    │       ├── TurnTimeline
    │       └── ThreadScrollFooter / Composer
    ├── RightPanel (optional)
    ├── BottomPanel (optional)
    └── OverlayRoot
```

`main`, `right-panel`, and `bottom-panel` are separate focus areas. Right and
bottom panels must never be inserted into the conversation document tree.

## Stable design tokens

| Token | Baseline |
| --- | ---: |
| Default window | `1280px × 820px` |
| Minimum window | `480px × 600px` |
| Global titlebar | `46px` |
| Pane toolbar | `40px` |
| Sidebar default | `275px` |
| Sidebar range | `240px–520px` |
| Navigation row | `30px` |
| Settings row | `64px` |
| Thread body maximum | `768px` (`48rem`) |
| UI text | `14px` |
| Conversation text | `14px` |
| Code text | `12px` |
| Composer minimum | `44px` |
| Composer radius | `20px–24px` |
| Navigation radius | `10px` |
| Dialog radius | `16px` |
| Hairline border | `0.5px` |

Spacing uses a `4px` grid: `4, 8, 12, 16, 20, 24, 32`. Production components
must consume named CSS custom properties; page-local geometry constants are not
accepted.

The UI font stack is `-apple-system-body, -apple-system, BlinkMacSystemFont,
"Segoe UI", sans-serif`. Code uses `SFMono-Regular, SF Mono, Menlo, monospace`.

## Responsive modes

- Wider than `960px`: persistent tiled sidebar.
- `721px–960px`: collapsible sidebar; the main surface keeps priority.
- `720px` or narrower: overlay sidebar.
- Below `475px`: composer utility labels may collapse to icons.
- Sidebar maximum width is `min(520px, viewport width - 320px)`.

A normal `1280px` desktop window must not render the sidebar as a floating card.

## Sidebar hierarchy

```text
New task
Search
Needs attention
Pinned
Custom sections
Projects
  Project tasks
Unscoped tasks
Account / Profile
Settings
```

The renderer consumes the Rust sidebar snapshot as the canonical hierarchy. It
must not independently rebuild ordering or nesting from flat project/task
arrays. Search retains matched ancestors; selection, collapsed nodes, panel
width, and active profile survive a restart.

## Task surface

- The task toolbar is fixed while the timeline scrolls independently.
- Normal message content stays within `768px`; wide diff, table, image, terminal,
  and artifact blocks may expand within the main panel.
- A turn can contain user text, assistant text, reasoning/activity, tool calls,
  approval/input, diffs, artifacts, errors, and final status.
- Composer is an interaction layer in `ThreadScrollFooter`, not a normal final
  timeline node.
- Send changes to Stop while a task is active and to Retry after a retryable
  failure.

## Security and access behavior

- `contextIsolation` is enabled and `nodeIntegration` is disabled.
- Preload exposes an explicit typed allowlist only.
- One Electron app instance owns one Rust host; all windows share it.
- Full Access is evaluated in Rust after Profile → Project → Task policy merge.
- Full Access may auto-approve ordinary workspace operations, but never PAD/Pi
  auth and session roots, provider credentials, macOS TCC prompts, or destructive
  product actions.
- Private Profile directories use mode `0700`; credential, settings, and session
  files use mode `0600` on Unix systems.

## Golden matrix

Capture light and dark variants at:

- `1280 × 820`
- `1440 × 900`
- `960 × 720`
- `720 × 700`
- `480 × 600`

Exercise empty, long, running, approval, input, failed, and completed tasks;
single-line and multiline composer; attachments; sidebar drag/collapse; right
and bottom panels; Settings; login; account switch; Full Access; hover, pressed,
focus-visible, disabled, and selected states; Chinese/English/emoji titles; and
Retina `2×` plus external-display `1×` rendering.

Acceptance thresholds:

- Major geometry differs by no more than `1 CSS px`.
- Text baselines differ by no more than `1–2 CSS px`.
- Screenshot SSIM is at least `0.985` after excluding font antialiasing and
  native traffic lights.
- Critical Chinese actions never truncate at a Golden size.
- Keyboard focus order and VoiceOver labels cover every action.

Evaluation honesty is part of the gate. `SSIM >= 0.985` may be reported only
when an authorized, version-matched Codex image exists for the same macOS scale,
theme, viewport, content state, and traffic-light mask. Without that source the
Golden comparison is `NOT_EVALUATED`, never a self-comparison. PAD still has to
pass the local five-size geometry, clipping, focus, ARIA, Chinese truncation,
light/dark, and interaction-state matrix.

## Functional release gates

- Profile credentials and sessions are mutually inaccessible.
- Cross-Profile session paths are rejected.
- Renderer reload restores active tasks and pending interactions.
- Renderer, Rust, or Pi failure cannot leave duplicate session writers.
- Existing v1 Profile, Project, Task, and Pi journals survive migration.
- Packaged arm64 app starts, signs in, runs a task, and restores history on a
  clean macOS machine without Homebrew, Node, Bun, or Pi installed.
- A before/after filesystem audit proves Codex, ChatGPT, and independent Pi data
  were unchanged.
