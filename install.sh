#!/usr/bin/env bash
set -euo pipefail

REPO="T1mn/pad"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"
VERSION_INPUT="${VERSION:-latest}"
ASSUME_YES="${PAD_INSTALL_ASSUME_YES:-0}"
FORCE_SOURCE="${PAD_INSTALL_FORCE_SOURCE:-0}"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]:-$0}")" && pwd 2>/dev/null || pwd)"
TEMP_DIRS=()

cleanup_temp_dirs() {
  if [ "${#TEMP_DIRS[@]}" -eq 0 ]; then
    return 0
  fi
  local dir
  for dir in "${TEMP_DIRS[@]}"; do
    [ -n "${dir}" ] && rm -rf "${dir}"
  done
}

trap cleanup_temp_dirs EXIT

say() {
  printf "%b\n" "$*"
}

warn() {
  say "${YELLOW}$*${NC}"
}

ok() {
  say "${GREEN}$*${NC}"
}

err() {
  say "${RED}$*${NC}" >&2
}

get_arch() {
  case "$(uname -m)" in
    x86_64|amd64) echo "x86_64" ;;
    aarch64|arm64) echo "aarch64" ;;
    *) echo "unsupported" ;;
  esac
}

get_os() {
  case "$(uname -s | tr '[:upper:]' '[:lower:]')" in
    linux) echo "linux" ;;
    darwin) echo "macos" ;;
    *) echo "unsupported" ;;
  esac
}

normalize_version() {
  local value="$1"
  if [ "$value" = "latest" ]; then
    echo "latest"
  elif [[ "$value" == v* ]]; then
    echo "$value"
  else
    echo "v$value"
  fi
}

check_tmux() {
  command -v tmux >/dev/null 2>&1
}

check_rust() {
  command -v cargo >/dev/null 2>&1
}

check_command() {
  command -v "$1" >/dev/null 2>&1
}

resolved_release_version() {
  if [ -n "${PAD_RESOLVED_RELEASE_VERSION:-}" ]; then
    echo "${PAD_RESOLVED_RELEASE_VERSION}"
    return 0
  fi

  local normalized
  normalized="$(normalize_version "$VERSION_INPUT")"
  if [ "$normalized" != "latest" ]; then
    PAD_RESOLVED_RELEASE_VERSION="$normalized"
    export PAD_RESOLVED_RELEASE_VERSION
    echo "$normalized"
    return 0
  fi

  local version
  version="$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" 2>/dev/null | sed -n 's/.*"tag_name":[[:space:]]*"\([^"]*\)".*/\1/p' | head -n1)"
  if [ -z "$version" ]; then
    return 1
  fi

  PAD_RESOLVED_RELEASE_VERSION="$version"
  export PAD_RESOLVED_RELEASE_VERSION
  echo "$version"
}

find_local_repo_root() {
  if [ -f "${SCRIPT_DIR}/rust-tui/Cargo.toml" ]; then
    echo "${SCRIPT_DIR}"
    return 0
  fi

  if [ -f "${PWD}/rust-tui/Cargo.toml" ]; then
    echo "${PWD}"
    return 0
  fi

  return 1
}

prompt_yes() {
  local prompt="$1"
  if [ "${ASSUME_YES}" = "1" ]; then
    return 0
  fi

  if [ ! -t 0 ]; then
    return 1
  fi

  printf "%s [y/N]\n> " "$prompt" >&2
  local answer=""
  IFS= read -r answer || true
  case "$(printf '%s' "$answer" | tr '[:upper:]' '[:lower:]')" in
    y|yes) return 0 ;;
    *) return 1 ;;
  esac
}

detect_tmux_install_plan() {
  case "$(get_os)" in
    macos)
      if check_command brew; then
        echo "brew"
        return 0
      fi
      ;;
    linux)
      for tool in apt-get dnf pacman zypper brew; do
        if check_command "$tool"; then
          echo "$tool"
          return 0
        fi
      done
      ;;
  esac
  return 1
}

tmux_manual_hint() {
  case "$1" in
    brew) echo "brew install tmux" ;;
    apt-get) echo "sudo apt-get update && sudo apt-get install -y tmux" ;;
    dnf) echo "sudo dnf install -y tmux" ;;
    pacman) echo "sudo pacman -Sy --noconfirm tmux" ;;
    zypper) echo "sudo zypper --non-interactive install tmux" ;;
    *) echo "install tmux with your system package manager" ;;
  esac
}

run_as_root_or_sudo() {
  if [ "$(id -u)" -eq 0 ]; then
    "$@"
  elif check_command sudo; then
    sudo "$@"
  else
    "$@"
  fi
}

install_tmux() {
  if check_tmux; then
    ok "✓ tmux already installed: $(tmux -V 2>/dev/null || echo tmux)"
    return 0
  fi

  local plan
  if ! plan="$(detect_tmux_install_plan)"; then
    warn "! tmux is required at runtime, but no supported package manager was detected"
    warn "  Install tmux manually, then run pad in the same environment"
    return 0
  fi

  warn "! tmux is not installed"
  if ! prompt_yes "PAD requires tmux at runtime. Install tmux now?"; then
    warn "  Skipped tmux installation"
    warn "  Manual command: $(tmux_manual_hint "$plan")"
    return 0
  fi

  say "${BLUE}Installing tmux via ${plan}...${NC}"
  case "$plan" in
    brew)
      brew install tmux
      ;;
    apt-get)
      run_as_root_or_sudo apt-get update
      run_as_root_or_sudo apt-get install -y tmux
      ;;
    dnf)
      run_as_root_or_sudo dnf install -y tmux
      ;;
    pacman)
      run_as_root_or_sudo pacman -Sy --noconfirm tmux
      ;;
    zypper)
      run_as_root_or_sudo zypper --non-interactive install tmux
      ;;
  esac

  if check_tmux; then
    ok "✓ tmux installed: $(tmux -V 2>/dev/null || echo tmux)"
  else
    err "✗ tmux install command completed but tmux is still missing"
    exit 1
  fi
}

install_from_binary() {
  if [ "${FORCE_SOURCE}" = "1" ]; then
    return 1
  fi

  local os arch version filename url tmp_dir
  os="$(get_os)"
  arch="$(get_arch)"

  if [ "$arch" = "unsupported" ] || [ "$os" = "unsupported" ]; then
    return 1
  fi

  if [ "$os" = "macos" ]; then
    arch="universal"
  fi

  if ! version="$(resolved_release_version)"; then
    warn "  Could not resolve latest release version"
    return 1
  fi

  filename="pad-${version}-${os}-${arch}.tar.gz"
  url="https://github.com/${REPO}/releases/download/${version}/${filename}"

  say "${BLUE}Trying to download pre-built binary...${NC}"
  say "  Platform: ${os}/${arch}"
  say "  Version:  ${version}"
  say "  URL:      ${url}"

  tmp_dir="$(mktemp -d)"
  TEMP_DIRS+=("${tmp_dir}")

  if ! curl -fsSL "$url" -o "${tmp_dir}/pad.tar.gz" 2>/dev/null; then
    warn "  Download failed (artifact may not exist for this platform/version)"
    return 1
  fi

  tar -xzf "${tmp_dir}/pad.tar.gz" -C "${tmp_dir}"
  mkdir -p "${INSTALL_DIR}"
  mv "${tmp_dir}/pad" "${INSTALL_DIR}/pad"
  chmod +x "${INSTALL_DIR}/pad"
  ok "✓ Installed binary to ${INSTALL_DIR}/pad"
}

download_source_tree() {
  local version="$1"
  local tmp_dir archive_url root_dir
  tmp_dir="$(mktemp -d)"
  TEMP_DIRS+=("${tmp_dir}")

  if [ "$version" = "latest" ]; then
    archive_url="https://github.com/${REPO}/archive/refs/heads/main.tar.gz"
  else
    archive_url="https://github.com/${REPO}/archive/refs/tags/${version}.tar.gz"
  fi

  say "${BLUE}Downloading source archive...${NC}"
  say "  URL: ${archive_url}"

  curl -fsSL "${archive_url}" -o "${tmp_dir}/source.tar.gz"
  tar -xzf "${tmp_dir}/source.tar.gz" -C "${tmp_dir}"
  root_dir="$(find "${tmp_dir}" -maxdepth 1 -mindepth 1 -type d | head -n1)"
  if [ -z "${root_dir}" ] || [ ! -f "${root_dir}/rust-tui/Cargo.toml" ]; then
    err "✗ Downloaded source archive does not contain rust-tui/Cargo.toml"
    exit 1
  fi

  echo "${root_dir}"
}

install_from_source() {
  local repo_root version

  say ""
  say "${BLUE}Building from source...${NC}"

  if ! check_rust; then
    err "✗ Rust/Cargo not found"
    say ""
    say "Install Rust:"
    say "  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    say ""
    say "Or download a pre-built binary from:"
    say "  https://github.com/${REPO}/releases"
    exit 1
  fi

  if repo_root="$(find_local_repo_root)"; then
    say "  Source:   local checkout (${repo_root})"
  else
    version="$(resolved_release_version || echo latest)"
    repo_root="$(download_source_tree "${version}")"
    say "  Source:   downloaded archive (${repo_root})"
  fi

  (
    cd "${repo_root}/rust-tui"
    cargo build --profile dist
  )

  mkdir -p "${INSTALL_DIR}"
  cp "${repo_root}/rust-tui/target/dist/pad" "${INSTALL_DIR}/pad"
  chmod +x "${INSTALL_DIR}/pad"
  ok "✓ Installed build to ${INSTALL_DIR}/pad"
}

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
  say "  1. Start an AI agent inside tmux"
  say "  2. Run: pad"
  say "  3. Use j/k to navigate, Enter to attach"
  say "  4. F12 or Ctrl+Q to return to PAD"
}

show_help() {
  cat <<'EOF'
PAD Installer

Usage:
  ./install.sh
  curl -fsSL https://raw.githubusercontent.com/T1mn/pad/main/install.sh | bash

What this script does:
  1. Try to download a matching release binary
  2. Fall back to building from source if needed
  3. Offer to install tmux if it is missing

Environment variables:
  INSTALL_DIR            Install destination (default: ~/.local/bin)
  VERSION                Release tag to install, e.g. v0.6.0 (default: latest)
  PAD_INSTALL_ASSUME_YES Auto-confirm tmux install prompt when set to 1
  PAD_INSTALL_FORCE_SOURCE
                         Skip binary download and build from source when set to 1

Notes:
  - PAD requires tmux at runtime.
  - On WSL2, install and run both tmux and pad inside WSL.
EOF
}

main() {
  if [ "${1:-}" = "--help" ] || [ "${1:-}" = "-h" ]; then
    show_help
    return 0
  fi

  say "=============================================="
  say "  PAD - Panel for Agent Development"
  say "=============================================="
  say ""

  if ! install_from_binary; then
    warn "Falling back to source build..."
    install_from_source
  fi

  install_tmux
  show_path_hint
  show_success
}

main "$@"
