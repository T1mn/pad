# Release checklist

1. `apps/pad-desktop/package.json` version matches the tag.
2. Run `npm ci`, `npm run typecheck`, and `npm run test:ui -- --run`.
3. Run `./scripts/package-electron-app.sh`.
4. Confirm the bundle contains `app.asar`, Pi and pinned Node.js, but neither Bun nor `Contents/Resources/pad`; locale directories are limited to English, Simplified Chinese, and Traditional Chinese.
5. Run `./scripts/install-electron-app.sh --check-only`.
6. Install with `./scripts/install-electron-app.sh --launch` and verify account, model list, one conversation, sidebar restore, system proxy, and remote pairing.
7. Confirm no Rust sidecar process exists while the app is running.
8. For public distribution, use Developer ID signing/notarization with `release-electron-app.sh`.
