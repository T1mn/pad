show_path_hint() {
  case ":$PATH:" in
    *":${INSTALL_DIR}:"*) ;;
    *)
      say ""
      warn "! Add this to your shell config:"
      say "  export PATH=\"${INSTALL_DIR}:\$PATH\""
      ;;
  esac
}

show_success() {
  say ""
  ok "✓ Installation successful"
  say ""
  say "Usage:"
  say "  pad            Launch interactive TUI"
  say "  pad --help     Show help"
  say ""
  say "Quick start:"
  say "  1. Run: pad"
  say "  2. Press Tab to focus PAD's native terminal"
  say "  3. Press F12 to return to PAD controls"
  say "  4. Optional legacy mode: pad --tmux"
}

show_help() {
  cat <<'EOF'
PAD Installer

Usage:
  ./install.sh
  curl -fsSL https://raw.githubusercontent.com/T1mn/pad/master/install.sh | bash

What this script does:
  1. Try to download a matching release binary
  2. Verify it against the release SHA256SUMS before extracting
  3. Fall back to building from source if needed
  4. Optionally install tmux compatibility when explicitly requested

Environment variables:
  INSTALL_DIR            Install destination (default: ~/.local/bin)
  VERSION                Release tag to install, e.g. v0.6.0 (default: latest)
  PAD_INSTALL_ASSUME_YES Auto-confirm installer prompts when set to 1
  PAD_INSTALL_TMUX_COMPAT
                         Install optional tmux support for pad --tmux when set to 1
  PAD_INSTALL_ALLOW_UNVERIFIED
                         Install even when the archive cannot be checked against
                         SHA256SUMS (unsafe; only for sources you fully trust)
  PAD_INSTALL_REQUIRE_CHECKSUM
                         Require SHA256SUMS even for a custom
                         PAD_RELEASE_BASE_URL (always required for GitHub releases)
  PAD_INSTALL_DISABLE_SOURCE_FALLBACK
                         Fail instead of building from source when no compatible
                         prebuilt binary can be installed
  PAD_INSTALL_FORCE_SOURCE
                         Skip binary download and build from source when set to 1
  PAD_RELEASE_BASE_URL   Override the release download base URL
                         Useful for CI and local installer smoke tests

Notes:
  - Native mode does not require tmux.
  - On WSL2, run PAD inside WSL; tmux is needed there only for pad --tmux.
EOF
}

main() {
  if [ "${1:-}" = "--help" ] || [ "${1:-}" = "-h" ]; then
    show_help
    return 0
  fi

  require_command curl "Install curl first, then run the installer again."
  require_command tar "Install tar first, then run the installer again."

  say "=============================================="
  say "  PAD - Panel for Agent Development"
  say "=============================================="
  say ""

  if ! install_from_binary; then
    if [ "${DISABLE_SOURCE_FALLBACK}" = "1" ]; then
      err "✗ No compatible prebuilt binary could be installed and source fallback is disabled"
      exit 1
    fi
    if ! confirm_source_build_fallback; then
      err "✗ Local source build was declined"
      exit 1
    fi
    warn "Falling back to source build..."
    install_from_source
  fi

  if [ "${INSTALL_TMUX_COMPAT}" = "1" ]; then
    install_tmux
  elif check_tmux; then
    ok "✓ Optional tmux compatibility available: $(tmux -V 2>/dev/null || echo tmux)"
  else
    say "Native mode is ready; tmux compatibility was not requested."
  fi
  ensure_default_codex_jailbreak_prompt
  ensure_default_codex_index_prompt
  show_path_hint
  show_success
}

main "$@"
