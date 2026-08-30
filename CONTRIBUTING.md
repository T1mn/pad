# Contributing

PAD currently focuses on one product: the macOS Desktop app in `apps/pad-desktop`, plus its iOS companion in `apps/pad-ios`.

## Before opening a change

```bash
cd apps/pad-desktop
npm ci
npm run typecheck
npm run test:ui -- --run
```

For changes to packaging or the Electron main process, also run:

```bash
./scripts/package-electron-app.sh
./scripts/install-electron-app.sh --check-only
```

Keep tests focused on user-visible product paths. Avoid adding compatibility layers for the removed Rust/TUI/SwiftUI implementations.

Update the nearest `index.md` when directory responsibilities change. Do not mix PAD data with Codex or ChatGPT session stores.
