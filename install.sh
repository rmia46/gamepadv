#!/usr/bin/env bash
set -euo pipefail

REPO="rmia46/gamepadv"
INSTALL_DIR="${HOME}/.local/bin"
DESKTOP_DIR="${HOME}/.local/share/applications"
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

mkdir -p "${INSTALL_DIR}" "${DESKTOP_DIR}"
install -m 755 "${TMP_DIR}/gamepadv" "${INSTALL_DIR}/gamepadv"
if [[ -f "${TMP_DIR}/gamepadv.desktop" ]]; then
  install -m 644 "${TMP_DIR}/gamepadv.desktop" "${DESKTOP_DIR}/gamepadv.desktop"
fi

echo "==> Installed to ${INSTALL_DIR}/gamepadv"
if [[ ":${PATH}:" != *":${INSTALL_DIR}:"* ]]; then
  echo "Note: Add ${INSTALL_DIR} to your PATH (e.g. export PATH=\"\$HOME/.local/bin:\$PATH\")."
fi
echo "==> Done! Run 'gamepadv' to launch."
