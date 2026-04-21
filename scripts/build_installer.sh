#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
OUTPUT_PATH="${REPO_ROOT}/install.sh"
PARTS=(
  "install/00_core.sh"
  "install/10_platform.sh"
  "install/20_release.sh"
  "install/30_dependencies.sh"
  "install/40_installers.sh"
  "install/50_prompt.sh"
  "install/90_main.sh"
)

tmp_file="$(mktemp)"
trap 'rm -f "${tmp_file}"' EXIT

for part in "${PARTS[@]}"; do
  part_path="${REPO_ROOT}/${part}"
  if [ ! -f "${part_path}" ]; then
    echo "missing installer part: ${part}" >&2
    exit 1
  fi
  cat "${part_path}" >>"${tmp_file}"
  printf '\n' >>"${tmp_file}"
done

mv "${tmp_file}" "${OUTPUT_PATH}"
chmod +x "${OUTPUT_PATH}"
echo "wrote ${OUTPUT_PATH}"
