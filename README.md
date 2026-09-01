# PAD

PAD is a macOS desktop client powered directly by [Pi](https://github.com/badlogic/pi-mono). It combines a Codex-style task sidebar, Pi conversations, multiple account profiles, model selection, Fast mode, Full Access, a local terminal, and iPhone remote access in one app.

中文说明见 [README_ZH.md](README_ZH.md).

## Architecture

- Electron main process: TypeScript local backend, SQLite storage, Pi process lifecycle, authentication, proxy inheritance, terminal, and remote gateway.
- React renderer: macOS/Codex-style interface. It receives renderer-safe DTOs only.
- Pi: the only agent runtime. PAD launches bundled Pi in RPC mode and keeps each PAD profile and session in PAD-owned directories.
- No Rust sidecar, CLI TUI, or SwiftUI fallback is part of the product.

PAD data lives under `~/Library/Application Support/PAD Desktop`. It does not read or overwrite the Codex or ChatGPT sidebar/session store.

## Develop

Requirements: Apple Silicon Mac, Node.js/npm, and `@earendil-works/pi-coding-agent` 0.84.4 installed locally for packaging. The app bundles a pinned Node.js runtime; Bun is not shipped.

```bash
cd apps/pad-desktop
npm ci
npm run dev
```

## Verify and package

```bash
cd apps/pad-desktop
npm run typecheck
npm run test:ui -- --run
./scripts/package-electron-app.sh
./scripts/install-electron-app.sh --check-only
./scripts/install-electron-app.sh --launch
```

The final app is generated at `apps/pad-desktop/out/PAD Desktop-darwin-arm64/PAD Desktop.app` and installed to `/Applications/PAD Desktop.app`.

## iPhone remote

`apps/pad-ios` contains the native iOS companion. Enable Remote in PAD Desktop, scan the one-time QR code, then continue a task from the same LAN.

## License

MIT
