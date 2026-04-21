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
    say "  Selected: ${filename}"
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
