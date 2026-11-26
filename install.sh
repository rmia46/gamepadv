#!/usr/bin/env bash
set -euo pipefail

REPO="rmia46/gamepadv"
TMP_DIR="$(mktemp -d)"
trap 'rm -rf "${TMP_DIR}"' EXIT

echo "========================================"
echo "  GamepadV Installer (${REPO})"
echo "========================================"

ARCH="$(uname -m)"
OS="$(uname -s)"
echo "[1/5] Detecting system: ${OS} (${ARCH})"
if [[ "${OS}" != "Linux" ]]; then
  echo "Error: GamepadV only supports Linux (evdev/uinput)." >&2
  exit 1
fi
if [[ "${ARCH}" != "x86_64" ]]; then
  echo "Error: Prebuilt releases currently target x86_64 (found ${ARCH}). Build from source with: cargo build --release" >&2
  exit 1
fi

echo "[2/5] Fetching latest release metadata from GitHub..."
RELEASE_JSON="$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest")"
TAG_NAME="$(printf '%s' "${RELEASE_JSON}" | grep -Eo '"tag_name":[[:space:]]*"[^"]+"' | head -n 1 | cut -d'"' -f4)"
TARBALL_URL="$(
  printf '%s' "${RELEASE_JSON}" \
    | grep -Eo 'https://[^"]+x86_64-linux-gnu\.tar\.gz' \
    | head -n 1
)"

if [[ -z "${TARBALL_URL}" ]]; then
  echo "Error: Could not find x86_64-linux-gnu.tar.gz in ${REPO} releases." >&2
  exit 1
fi

echo "      Found release: ${TAG_NAME:-latest}"
echo "      Asset URL:     ${TARBALL_URL}"

echo "[3/5] Downloading release archive..."
curl -fL --progress-bar "${TARBALL_URL}" -o "${TMP_DIR}/gamepadv.tar.gz"

echo "[4/5] Extracting archive..."
tar -xzvf "${TMP_DIR}/gamepadv.tar.gz" -C "${TMP_DIR}" | sed 's/^/      /'

echo "[5/5] Installing binary and desktop entry..."
if command -v sudo >/dev/null 2>&1; then
  echo "      Installing system-wide (sudo may prompt for password)..."
  sudo install -Dm755 "${TMP_DIR}/gamepadv" "/usr/local/bin/gamepadv"
  echo "      -> /usr/local/bin/gamepadv"
  if [[ -f "${TMP_DIR}/gamepadv.desktop" ]]; then
    sudo install -Dm644 "${TMP_DIR}/gamepadv.desktop" "/usr/share/applications/gamepadv.desktop"
    echo "      -> /usr/share/applications/gamepadv.desktop"
  fi
else
  mkdir -p "${HOME}/.local/bin" "${HOME}/.local/share/applications"
  install -m 755 "${TMP_DIR}/gamepadv" "${HOME}/.local/bin/gamepadv"
  echo "      -> ${HOME}/.local/bin/gamepadv"
  if [[ -f "${TMP_DIR}/gamepadv.desktop" ]]; then
    install -m 644 "${TMP_DIR}/gamepadv.desktop" "${HOME}/.local/share/applications/gamepadv.desktop"
    echo "      -> ${HOME}/.local/share/applications/gamepadv.desktop"
  fi
fi

echo "========================================"
echo "  GamepadV ${TAG_NAME:-} installed successfully!"
echo "  Run 'gamepadv' or open it from your app menu."
echo "========================================"
