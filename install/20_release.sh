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
  local os arch libc_family glibc_version distro_id

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
  distro_id="$(get_linux_distro_id || true)"
  case "$libc_family" in
    musl)
      printf '%s\n' "pad-${version}-linux-${arch}-musl.tar.gz"
      printf '%s\n' "pad-${version}-linux-${arch}.tar.gz"
      return 0
      ;;
    glibc)
      if [ "$distro_id" = "nixos" ]; then
        printf '%s\n' "pad-${version}-linux-${arch}-musl.tar.gz"
        printf '%s\n' "pad-${version}-linux-${arch}-glibc-2.35.tar.gz"
        printf '%s\n' "pad-${version}-linux-${arch}.tar.gz"
        return 0
      fi
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
      printf '%s\n' "pad-${version}-linux-${arch}-musl.tar.gz"
      printf '%s\n' "pad-${version}-linux-${arch}-glibc-2.35.tar.gz"
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

  warn "  Installed binary failed self-check; trying the next compatible artifact"
  if grep -q 'GLIBC_[0-9]' "${log_file}"; then
    warn "  Detected glibc version mismatch on this system"
  fi
  sed -n '1,6{s/^/    /;p;}' "${log_file}"
  rm -f "${binary_path}"
  return 1
}
