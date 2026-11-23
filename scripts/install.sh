#!/usr/bin/env sh
set -eu

REPO="${TOKSCALE_UI_REPO:-munim/tokscale-ui}"
BASE_URL="https://github.com/${REPO}/releases/latest/download"
MANIFEST_URL="${BASE_URL}/latest.json"
SUMS_URL="${BASE_URL}/SHA256SUMS.txt"

need_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "Error: required command '$1' is not installed." >&2
    exit 1
  fi
}

need_cmd curl
need_cmd uname

OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
  Darwin)
    case "$ARCH" in
      arm64|aarch64) PLATFORM_KEY="darwin-aarch64" ;;
      x86_64) PLATFORM_KEY="darwin-x86_64" ;;
      *)
        echo "Unsupported macOS architecture: $ARCH" >&2
        exit 1
        ;;
    esac
    ;;
  Linux)
    case "$ARCH" in
      x86_64|amd64) PLATFORM_KEY="linux-x86_64" ;;
      *)
        echo "Unsupported Linux architecture: $ARCH (x86_64 supported; build from source for $ARCH)" >&2
        exit 1
        ;;
    esac
    ;;
  *)
    echo "Unsupported OS: $OS. Use install.ps1 on Windows." >&2
    exit 1
    ;;
esac

TMP_DIR="$(mktemp -d)"
cleanup() {
  rm -rf "$TMP_DIR"
}
trap cleanup EXIT INT TERM

MANIFEST_FILE="${TMP_DIR}/latest.json"
echo "Fetching release manifest for ${PLATFORM_KEY}..."
curl -fsSL "$MANIFEST_URL" -o "$MANIFEST_FILE"

extract_url() {
  key="$1"
  file="$2"
  if command -v jq >/dev/null 2>&1; then
    jq -r --arg k "$key" '.platforms[$k].url // empty' "$file"
  elif command -v python3 >/dev/null 2>&1; then
    python3 -c 'import json,sys; d=json.load(open(sys.argv[1])); print(d.get("platforms",{}).get(sys.argv[2],{}).get("url",""))' "$file" "$key"
  else
    # Portable fallback parser for latest.json
    tr -d '\n' < "$file" | sed -n "s/.*\"${key}\"[[:space:]]*:[[:space:]]*{[^}]*\"url\"[[:space:]]*:[[:space:]]*\"\([^\"]*\)\".*/\1/p"
  fi
}

ASSET_URL="$(extract_url "$PLATFORM_KEY" "$MANIFEST_FILE")"
if [ -z "$ASSET_URL" ]; then
  echo "Error: could not resolve download URL for '${PLATFORM_KEY}' from ${MANIFEST_URL}." >&2
  exit 1
fi

ASSET_NAME="$(basename "$ASSET_URL")"
ASSET_FILE="${TMP_DIR}/${ASSET_NAME}"

echo "Downloading ${ASSET_NAME}..."
curl -fsSL "$ASSET_URL" -o "$ASSET_FILE"

SUMS_FILE="${TMP_DIR}/SHA256SUMS.txt"
if curl -fsSL "$SUMS_URL" -o "$SUMS_FILE" 2>/dev/null; then
  EXPECTED_SHA="$(awk -v name="$ASSET_NAME" '{ f=$2; sub(/^\.\//, "", f); if (f == name || $2 == name) { print $1; exit } }' "$SUMS_FILE")"
  if [ -n "$EXPECTED_SHA" ]; then
    if command -v sha256sum >/dev/null 2>&1; then
      ACTUAL_SHA="$(sha256sum "$ASSET_FILE" | awk '{print $1}')"
    elif command -v shasum >/dev/null 2>&1; then
      ACTUAL_SHA="$(shasum -a 256 "$ASSET_FILE" | awk '{print $1}')"
    else
      ACTUAL_SHA=""
    fi
    if [ -n "$ACTUAL_SHA" ] && [ "$ACTUAL_SHA" != "$EXPECTED_SHA" ]; then
      echo "Error: SHA-256 mismatch for ${ASSET_NAME} (expected ${EXPECTED_SHA}, got ${ACTUAL_SHA})." >&2
      exit 1
    fi
    echo "Verified SHA-256 checksum."
  fi
fi

if [ "$OS" = "Darwin" ]; then
  need_cmd tar
  EXTRACT_DIR="${TMP_DIR}/extract"
  mkdir -p "$EXTRACT_DIR"
  tar -xzf "$ASSET_FILE" -C "$EXTRACT_DIR"

  APP_SRC="$(find "$EXTRACT_DIR" -maxdepth 2 -name "*.app" -type d | head -n 1)"
  if [ -z "$APP_SRC" ]; then
    echo "Error: .app bundle not found inside ${ASSET_NAME}." >&2
    exit 1
  fi

  APP_NAME="$(basename "$APP_SRC")"
  if [ -w "/Applications" ]; then
    INSTALL_BASE="/Applications"
  else
    INSTALL_BASE="${HOME}/Applications"
    mkdir -p "$INSTALL_BASE"
  fi
  DEST_APP="${INSTALL_BASE}/${APP_NAME}"

  rm -rf "$DEST_APP"
  cp -R "$APP_SRC" "$DEST_APP"
  xattr -dr com.apple.quarantine "$DEST_APP" 2>/dev/null || true

  echo "Installed ${APP_NAME} to ${DEST_APP}."
  echo "Launch it from Applications or run: open \"${DEST_APP}\""
else
  BIN_DIR="${HOME}/.local/bin"
  APPS_DIR="${HOME}/.local/share/applications"
  mkdir -p "$BIN_DIR" "$APPS_DIR"

  DEST_BIN="${BIN_DIR}/tokscale-ui.AppImage"
  case "$ASSET_NAME" in
    *.tar.gz|*.tgz)
      EXTRACT_DIR="${TMP_DIR}/extract"
      mkdir -p "$EXTRACT_DIR"
      tar -xzf "$ASSET_FILE" -C "$EXTRACT_DIR"
      APPIMAGE_SRC="$(find "$EXTRACT_DIR" -maxdepth 2 -name "*.AppImage" -type f | head -n 1)"
      if [ -z "$APPIMAGE_SRC" ]; then
        echo "Error: .AppImage not found inside ${ASSET_NAME}." >&2
        exit 1
      fi
      cp "$APPIMAGE_SRC" "$DEST_BIN"
      ;;
    *)
      cp "$ASSET_FILE" "$DEST_BIN"
      ;;
  esac
  chmod +x "$DEST_BIN"

  cat > "${APPS_DIR}/tokscale-ui.desktop" <<EOF
[Desktop Entry]
Type=Application
Name=Tokscale UI
Comment=Desktop and menubar companion for tokscale
Exec=${DEST_BIN}
Terminal=false
Categories=Utility;Development;
StartupWMClass=tokscale-ui
EOF

  echo "Installed AppImage to ${DEST_BIN}."
  echo "Created desktop entry at ${APPS_DIR}/tokscale-ui.desktop."
  echo "Note: AppImages require FUSE (libfuse2 / libfuse2t64 on Ubuntu/Debian)."
fi
