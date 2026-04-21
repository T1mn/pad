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
