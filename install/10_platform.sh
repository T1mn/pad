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

get_linux_distro_id() {
  if [ "$(get_os)" != "linux" ] || [ ! -r /etc/os-release ]; then
    return 1
  fi

  sed -n 's/^ID=//p' /etc/os-release | head -n1 | tr -d '"'
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
