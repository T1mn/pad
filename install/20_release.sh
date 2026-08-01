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
  version="$(resolve_latest_release_from_api || true)"
  if [ -z "$version" ]; then
    version="$(resolve_latest_release_from_redirect || true)"
  fi
  if [ -z "$version" ]; then
    return 1
  fi

  PAD_RESOLVED_RELEASE_VERSION="$version"
  export PAD_RESOLVED_RELEASE_VERSION
  echo "$version"
}

resolve_latest_release_from_api() {
  curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" 2>/dev/null \
    | sed -n 's/.*"tag_name":[[:space:]]*"\([^"]*\)".*/\1/p' \
    | head -n1
}

resolve_latest_release_from_redirect() {
  local effective tag
  effective="$(curl -fsSIL -o /dev/null -w '%{url_effective}' "https://github.com/${REPO}/releases/latest" 2>/dev/null || true)"
  case "$effective" in
    */tag/*)
      tag="${effective##*/tag/}"
      tag="${tag%%[\?#]*}"
      [ -n "$tag" ] && printf '%s\n' "$tag"
      ;;
  esac
}

release_download_url() {
  local version="$1"
  local filename="$2"
  local base="${RELEASE_BASE_URL%/}"
  echo "${base}/${version}/${filename}"
}

checksum_tool() {
  if check_command sha256sum; then
    echo "sha256sum"
  elif check_command shasum; then
    echo "shasum"
  elif check_command openssl; then
    echo "openssl"
  else
    echo "none"
  fi
}

file_sha256() {
  local path="$1"
  local output=""

  case "$(checksum_tool)" in
    sha256sum)
      output="$(sha256sum "$path" 2>/dev/null || true)"
      output="${output%% *}"
      ;;
    shasum)
      output="$(shasum -a 256 "$path" 2>/dev/null || true)"
      output="${output%% *}"
      ;;
    openssl)
      output="$(openssl dgst -sha256 "$path" 2>/dev/null || true)"
      output="${output##* }"
      ;;
    *)
      return 1
      ;;
  esac

  [ -n "$output" ] || return 1
  printf '%s\n' "$output" | tr '[:upper:]' '[:lower:]'
}

# Look up the digest recorded for one asset in a `sha256sum`-style manifest.
expected_sha256_for() {
  local sums_file="$1"
  local filename="$2"
  local hash name

  [ -r "$sums_file" ] || return 1
  while read -r hash name || [ -n "$hash" ]; do
    [ -n "$hash" ] || continue
    case "$hash" in \#*) continue ;; esac
    name="${name#\*}"
    name="${name##*/}"
    if [ "$name" = "$filename" ]; then
      printf '%s\n' "$hash" | tr '[:upper:]' '[:lower:]'
      return 0
    fi
  done < "$sums_file"

  return 1
}

release_base_is_official() {
  [ "${RELEASE_BASE_URL%/}" = "${DEFAULT_RELEASE_BASE_URL%/}" ]
}

# Missing manifests are fatal for the official release host; a custom
# PAD_RELEASE_BASE_URL (mirror, CI fixture) only warns unless asked otherwise.
checksum_required() {
  case "$REQUIRE_CHECKSUM" in
    1) return 0 ;;
    0) return 1 ;;
  esac

  release_base_is_official
}

fetch_release_checksums() {
  local version="$1"
  local dest="$2"
  local url

  url="$(release_download_url "$version" "$CHECKSUM_FILE")"
  curl -fsSL "$url" -o "$dest" 2>/dev/null && [ -s "$dest" ]
}

prepare_release_checksums() {
  local version="$1"
  local dest="$2"
  local tool

  CHECKSUM_MODE="verify"

  tool="$(checksum_tool)"
  if [ "$tool" = "none" ]; then
    warn "! No SHA-256 tool found (tried sha256sum, shasum, openssl)"
    if [ "${ALLOW_UNVERIFIED}" = "1" ]; then
      warn "! PAD_INSTALL_ALLOW_UNVERIFIED=1: installing WITHOUT integrity verification"
      CHECKSUM_MODE="skip"
      return 0
    fi
    err "✗ Refusing to install a release archive that cannot be verified"
    say "  Install coreutils (sha256sum) or openssl, then run the installer again"
    say "  Or set PAD_INSTALL_ALLOW_UNVERIFIED=1 to accept the risk"
    exit 1
  fi

  if fetch_release_checksums "$version" "$dest"; then
    say "  Checksums: ${CHECKSUM_FILE} (${tool})"
    return 0
  fi

  warn "! ${CHECKSUM_FILE} is missing under ${RELEASE_BASE_URL%/}/${version}"
  if [ "${ALLOW_UNVERIFIED}" = "1" ]; then
    warn "! PAD_INSTALL_ALLOW_UNVERIFIED=1: installing WITHOUT integrity verification"
    CHECKSUM_MODE="skip"
    return 0
  fi
  if checksum_required; then
    err "✗ Refusing to install: this release publishes no ${CHECKSUM_FILE}"
    say "  Set PAD_INSTALL_ALLOW_UNVERIFIED=1 only if you fully trust this source"
    exit 1
  fi

  warn "! Continuing without ${CHECKSUM_FILE}: downloads will NOT be verified"
  warn "  Set PAD_INSTALL_REQUIRE_CHECKSUM=1 to turn this into a hard failure"
  CHECKSUM_MODE="skip"
  return 0
}

verify_release_archive() {
  local archive="$1"
  local filename="$2"
  local sums_file="$3"
  local expected actual

  if [ "$CHECKSUM_MODE" = "skip" ]; then
    return 0
  fi

  if ! expected="$(expected_sha256_for "$sums_file" "$filename")"; then
    err "✗ ${filename} is not listed in ${CHECKSUM_FILE}"
    say "  The release manifest does not cover this asset; aborting"
    exit 1
  fi

  if ! actual="$(file_sha256 "$archive")"; then
    err "✗ Could not compute the SHA-256 of ${filename}"
    exit 1
  fi

  if [ "$expected" != "$actual" ]; then
    err "✗ Checksum mismatch for ${filename}"
    say "  expected: ${expected}"
    say "  actual:   ${actual}"
    err "  The download does not match ${CHECKSUM_FILE}; aborting"
    exit 1
  fi

  say "  Verified: ${filename} (sha256 ok)"
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
