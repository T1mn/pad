#!/usr/bin/env bash
set -euo pipefail

REPO="T1mn/pad"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"
VERSION_INPUT="${VERSION:-latest}"
ASSUME_YES="${PAD_INSTALL_ASSUME_YES:-0}"
FORCE_SOURCE="${PAD_INSTALL_FORCE_SOURCE:-0}"
RELEASE_BASE_URL="${PAD_RELEASE_BASE_URL:-https://github.com/${REPO}/releases/download}"

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

get_linux_libc() {
  if [ "$(get_os)" != "linux" ]; then
    echo "unknown"
    return 0
  fi

  if check_command ldd; then
    local ldd_output
    ldd_output="$(ldd --version 2>&1 || true)"
    case "$ldd_output" in
      *musl*) echo "musl"; return 0 ;;
      *"GNU libc"*|*GLIBC*|*glibc*) echo "glibc"; return 0 ;;
    esac
  fi

  if getconf GNU_LIBC_VERSION >/dev/null 2>&1; then
    echo "glibc"
  else
    echo "unknown"
  fi
}

get_glibc_version() {
  local version=""

  version="$(getconf GNU_LIBC_VERSION 2>/dev/null | sed -n 's/.* \([0-9][0-9.]*\)$/\1/p' | head -n1 || true)"
  if [ -n "$version" ]; then
    echo "$version"
    return 0
  fi

  if check_command ldd; then
    version="$(ldd --version 2>&1 | sed -n '1{s/.* \([0-9][0-9.]*\)$/\1/p;q;}')"
    if [ -n "$version" ]; then
      echo "$version"
      return 0
    fi
  fi

  return 1
}

version_lt() {
  [ "$(printf '%s\n%s\n' "$1" "$2" | sort -V | head -n1)" = "$1" ] && [ "$1" != "$2" ]
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

require_command() {
  local cmd="$1"
  local hint="${2:-}"
  if check_command "$cmd"; then
    return 0
  fi

  err "✗ Required command not found: ${cmd}"
  if [ -n "$hint" ]; then
    say "  ${hint}"
  fi
  exit 1
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

release_download_url() {
  local version="$1"
  local filename="$2"
  local base="${RELEASE_BASE_URL%/}"
  echo "${base}/${version}/${filename}"
}

release_filenames_for_platform() {
  local version="$1"
  local os arch libc_family glibc_version

  os="$(get_os)"
  arch="$(get_arch)"

  if [ "$arch" = "unsupported" ] || [ "$os" = "unsupported" ]; then
    return 1
  fi

  if [ "$os" = "macos" ]; then
    printf '%s\n' "pad-${version}-macos-universal.tar.gz"
    return 0
  fi

  libc_family="$(get_linux_libc)"
  case "$libc_family" in
    musl)
      printf '%s\n' "pad-${version}-linux-${arch}-musl.tar.gz"
      printf '%s\n' "pad-${version}-linux-${arch}.tar.gz"
      return 0
      ;;
    glibc)
      glibc_version="$(get_glibc_version || true)"
      if [ -n "$glibc_version" ] && version_lt "$glibc_version" "2.35"; then
        printf '%s\n' "pad-${version}-linux-${arch}-musl.tar.gz"
        printf '%s\n' "pad-${version}-linux-${arch}-glibc-2.35.tar.gz"
        printf '%s\n' "pad-${version}-linux-${arch}.tar.gz"
        return 0
      fi
      printf '%s\n' "pad-${version}-linux-${arch}-glibc-2.35.tar.gz"
      printf '%s\n' "pad-${version}-linux-${arch}-musl.tar.gz"
      printf '%s\n' "pad-${version}-linux-${arch}.tar.gz"
      return 0
      ;;
    *)
      printf '%s\n' "pad-${version}-linux-${arch}-glibc-2.35.tar.gz"
      printf '%s\n' "pad-${version}-linux-${arch}-musl.tar.gz"
      printf '%s\n' "pad-${version}-linux-${arch}.tar.gz"
      return 0
      ;;
  esac
}

validate_installed_binary() {
  local binary_path="$1"
  local log_file
  log_file="$(mktemp)"
  TEMP_DIRS+=("${log_file}")

  if "${binary_path}" --version >"${log_file}" 2>&1; then
    return 0
  fi

  warn "  Installed binary failed self-check; falling back to source build"
  if grep -q 'GLIBC_[0-9]' "${log_file}"; then
    warn "  Detected glibc version mismatch on this system"
  fi
  sed -n '1,6{s/^/    /;p;}' "${log_file}"
  rm -f "${binary_path}"
  return 1
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

  if [ -r /dev/tty ] && [ -w /dev/tty ]; then
    printf "%s [y/N]\n> " "$prompt" >/dev/tty
    local answer=""
    IFS= read -r answer </dev/tty || true
    case "$(printf '%s' "$answer" | tr '[:upper:]' '[:lower:]')" in
      y|yes) return 0 ;;
      *) return 1 ;;
    esac
  fi

  warn "! ${prompt} (non-interactive install: proceeding automatically)"
  return 0
}

detect_build_install_plan() {
  case "$(get_os)" in
    macos)
      if check_command brew; then
        echo "brew"
        return 0
      fi
      ;;
    linux)
      for tool in apt-get dnf yum pacman zypper apk brew; do
        if check_command "$tool"; then
          echo "$tool"
          return 0
        fi
      done
      ;;
  esac
  return 1
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
      for tool in apt-get dnf yum pacman zypper apk brew; do
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
    yum) echo "sudo yum install -y tmux" ;;
    pacman) echo "sudo pacman -Sy --noconfirm tmux" ;;
    zypper) echo "sudo zypper --non-interactive install tmux" ;;
    apk) echo "sudo apk add tmux" ;;
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

ensure_root_or_sudo() {
  if [ "$(id -u)" -eq 0 ] || check_command sudo; then
    return 0
  fi

  err "✗ tmux installation requires root or sudo access"
  say "  Manual command: $(tmux_manual_hint "$1")"
  exit 1
}

install_tmux() {
  if check_tmux; then
    ok "✓ tmux already installed: $(tmux -V 2>/dev/null || echo tmux)"
    return 0
  fi

  local plan
  if ! plan="$(detect_tmux_install_plan)"; then
    err "✗ tmux is required at runtime, but no supported package manager was detected"
    say "  Install tmux manually, then run pad in the same environment"
    exit 1
  fi

  warn "! tmux is not installed"
  if ! prompt_yes "PAD requires tmux at runtime. Install tmux now?"; then
    err "✗ tmux installation was declined"
    say "  Manual command: $(tmux_manual_hint "$plan")"
    exit 1
  fi

  ensure_root_or_sudo "$plan"
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
    yum)
      run_as_root_or_sudo yum install -y tmux
      ;;
    pacman)
      run_as_root_or_sudo pacman -Sy --noconfirm tmux
      ;;
    zypper)
      run_as_root_or_sudo zypper --non-interactive install tmux
      ;;
    apk)
      run_as_root_or_sudo apk add tmux
      ;;
  esac

  if check_tmux; then
    ok "✓ tmux installed: $(tmux -V 2>/dev/null || echo tmux)"
  else
    err "✗ tmux install command completed but tmux is still missing"
    exit 1
  fi
}

build_tools_ready() {
  check_command cc && check_command pkg-config
}

build_tools_manual_hint() {
  case "$1" in
    brew) echo "brew install pkgconf" ;;
    apt-get) echo "sudo apt-get update && sudo apt-get install -y build-essential pkg-config" ;;
    dnf) echo "sudo dnf install -y gcc gcc-c++ make pkgconf-pkg-config" ;;
    yum) echo "sudo yum install -y gcc gcc-c++ make pkgconfig" ;;
    pacman) echo "sudo pacman -Sy --noconfirm base-devel pkgconf" ;;
    zypper) echo "sudo zypper --non-interactive install gcc gcc-c++ make pkg-config" ;;
    apk) echo "sudo apk add build-base pkgconf" ;;
    *) echo "install a C toolchain and pkg-config with your system package manager" ;;
  esac
}

confirm_source_build_fallback() {
  say ""
  say "No compatible prebuilt binary was found for this environment, or the downloaded binary could not run."
  say "この環境に対応する事前ビルド済みバイナリが見つからないか、ダウンロードしたバイナリを実行できませんでした。"
  say "当前环境没有可用的预编译二进制，或下载的二进制无法运行。"
  say ""
  prompt_yes "Continue with a local source build? / ローカルでソースビルドを続行しますか？ / 是否继续本地源码编译？"
}

install_build_tools() {
  if build_tools_ready; then
    return 0
  fi

  local plan
  if ! plan="$(detect_build_install_plan)"; then
    err "✗ A local source build requires a C toolchain and pkg-config, but no supported package manager was detected"
    say "  Manual command: install a C toolchain and pkg-config for your system"
    exit 1
  fi

  if ! prompt_yes "PAD needs local build tools for a source install. Install them now?"; then
    err "✗ Build tool installation was declined"
    say "  Manual command: $(build_tools_manual_hint "$plan")"
    exit 1
  fi

  ensure_root_or_sudo "$plan"
  say "${BLUE}Installing build tools via ${plan}...${NC}"
  case "$plan" in
    brew)
      brew install pkgconf
      ;;
    apt-get)
      run_as_root_or_sudo apt-get update
      run_as_root_or_sudo apt-get install -y build-essential pkg-config
      ;;
    dnf)
      run_as_root_or_sudo dnf install -y gcc gcc-c++ make pkgconf-pkg-config
      ;;
    yum)
      run_as_root_or_sudo yum install -y gcc gcc-c++ make pkgconfig
      ;;
    pacman)
      run_as_root_or_sudo pacman -Sy --noconfirm base-devel pkgconf
      ;;
    zypper)
      run_as_root_or_sudo zypper --non-interactive install gcc gcc-c++ make pkg-config
      ;;
    apk)
      run_as_root_or_sudo apk add build-base pkgconf
      ;;
  esac

  if ! build_tools_ready; then
    err "✗ Build tool installation completed but required commands are still missing"
    exit 1
  fi
}

install_rust() {
  if check_rust; then
    return 0
  fi

  if ! prompt_yes "Rust is required for a local source build. Install Rust now? / ローカルのソースビルドには Rust が必要です。今すぐ Rust をインストールしますか？ / 本地源码编译需要 Rust。现在安装 Rust 吗？"; then
    err "✗ Rust installation was declined"
    say "  Manual command: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    exit 1
  fi

  say "${BLUE}Installing Rust toolchain...${NC}"
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal

  if [ -f "${HOME}/.cargo/env" ]; then
    # shellcheck disable=SC1090
    . "${HOME}/.cargo/env"
  else
    export PATH="${HOME}/.cargo/bin:${PATH}"
  fi

  if ! check_rust; then
    err "✗ Rust installation completed but cargo is still unavailable"
    exit 1
  fi
}

install_from_binary() {
  if [ "${FORCE_SOURCE}" = "1" ]; then
    return 1
  fi

  local os arch version filename url tmp_dir libc_family runtime_note
  os="$(get_os)"
  arch="$(get_arch)"

  if [ "$arch" = "unsupported" ] || [ "$os" = "unsupported" ]; then
    return 1
  fi

  if ! version="$(resolved_release_version)"; then
    warn "  Could not resolve latest release version"
    return 1
  fi

  say "${BLUE}Trying to download pre-built binary...${NC}"
  if [ "$os" = "linux" ]; then
    libc_family="$(get_linux_libc)"
    runtime_note="${libc_family}"
    if [ "$libc_family" = "glibc" ]; then
      runtime_note="${runtime_note} $(get_glibc_version || echo unknown)"
    fi
    say "  Platform: ${os}/${arch}"
    say "  Runtime:  ${runtime_note}"
  else
    say "  Platform: ${os}/universal"
  fi
  say "  Version:  ${version}"

  while IFS= read -r filename; do
    [ -n "$filename" ] || continue
    url="$(release_download_url "${version}" "${filename}")"
    say "  Trying:   ${filename}"

    tmp_dir="$(mktemp -d)"
    TEMP_DIRS+=("${tmp_dir}")

    if ! curl -fsSL "$url" -o "${tmp_dir}/pad.tar.gz" 2>/dev/null; then
      warn "  Download failed for ${filename}"
      continue
    fi

    tar -xzf "${tmp_dir}/pad.tar.gz" -C "${tmp_dir}"
    mkdir -p "${INSTALL_DIR}"
    mv "${tmp_dir}/pad" "${INSTALL_DIR}/pad"
    chmod +x "${INSTALL_DIR}/pad"
    if ! validate_installed_binary "${INSTALL_DIR}/pad"; then
      continue
    fi
    ok "✓ Installed binary to ${INSTALL_DIR}/pad"
    return 0
  done < <(release_filenames_for_platform "${version}")

  return 1
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

  install_build_tools
  install_rust

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
  PAD_RELEASE_BASE_URL   Override the release download base URL
                         Useful for CI and local installer smoke tests

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

  require_command curl "Install curl first, then run the installer again."
  require_command tar "Install tar first, then run the installer again."

  say "=============================================="
  say "  PAD - Panel for Agent Development"
  say "=============================================="
  say ""

  if ! install_from_binary; then
    if ! confirm_source_build_fallback; then
      err "✗ Local source build was declined"
      exit 1
    fi
    warn "Falling back to source build..."
    install_from_source
  fi

  install_tmux
  if ! check_tmux; then
    err "✗ PAD was installed, but tmux is still unavailable"
    exit 1
  fi
  show_path_hint
  show_success
}

main "$@"
