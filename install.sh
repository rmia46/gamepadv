#!/usr/bin/env bash
set -euo pipefail

REPO="rmia46/gamepadv"
TMP_DIR="$(mktemp -d)"
trap 'rm -rf "${TMP_DIR}"' EXIT

echo "==> Installing GamepadV from GitHub Releases (${REPO})..."

TARBALL_URL="$(
  curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" \
    | grep -Eo 'https://[^"]+x86_64-linux-gnu\.tar\.gz' \
    | head -n 1
)"

if [[ -z "${TARBALL_URL}" ]]; then
  echo "Error: Could not find release tarball in ${REPO} releases." >&2
  exit 1
fi

curl -fsSL "${TARBALL_URL}" -o "${TMP_DIR}/gamepadv.tar.gz"
tar -xzf "${TMP_DIR}/gamepadv.tar.gz" -C "${TMP_DIR}"

if command -v sudo >/dev/null 2>&1; then
  sudo install -Dm755 "${TMP_DIR}/gamepadv" "/usr/local/bin/gamepadv"
  if [[ -f "${TMP_DIR}/gamepadv.desktop" ]]; then
    sudo install -Dm644 "${TMP_DIR}/gamepadv.desktop" "/usr/share/applications/gamepadv.desktop"
  fi
  echo "==> Installed to /usr/local/bin/gamepadv"
else
  mkdir -p "${HOME}/.local/bin" "${HOME}/.local/share/applications"
  install -m 755 "${TMP_DIR}/gamepadv" "${HOME}/.local/bin/gamepadv"
  if [[ -f "${TMP_DIR}/gamepadv.desktop" ]]; then
    install -m 644 "${TMP_DIR}/gamepadv.desktop" "${HOME}/.local/share/applications/gamepadv.desktop"
  fi
  echo "==> Installed to ${HOME}/.local/bin/gamepadv"
fi

echo "==> Done! Launch 'gamepadv' from your app menu or terminal."
