#!/usr/bin/env bash
set -euo pipefail

REPO="BitrixStudio/duplicate"
BIN="duplicate"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"

need() { command -v "$1" >/dev/null 2>&1 || { echo "Missing dependency: $1" >&2; exit 1; }; }
need curl
need tar
need uname

OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
  Linux)  OS_TAG="unknown-linux-gnu" ;;
  Darwin) OS_TAG="apple-darwin" ;;
  *) echo "Unsupported OS: $OS" >&2; exit 1 ;;
esac

case "$ARCH" in
  x86_64|amd64) ARCH_TAG="x86_64" ;;
  arm64|aarch64) ARCH_TAG="aarch64" ;;
  *) echo "Unsupported arch: $ARCH" >&2; exit 1 ;;
esac

TARGET="${ARCH_TAG}-${OS_TAG}"

# Get latest release tag
TAG="$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" \
  | grep -Eo '"tag_name":[[:space:]]*"[^"]+"' \
  | head -n1 \
  | cut -d'"' -f4)"

ASSET="${BIN}-${TAG}-${TARGET}.tar.gz"
URL="https://github.com/${REPO}/releases/download/${TAG}/${ASSET}"

echo "Installing ${BIN} ${TAG} for ${TARGET}..."
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

curl -fsSL "$URL" -o "$tmp/$ASSET"
tar -xzf "$tmp/$ASSET" -C "$tmp"

mkdir -p "$INSTALL_DIR"
install -m 755 "$tmp/$BIN" "$INSTALL_DIR/$BIN"

echo "Installed to: $INSTALL_DIR/$BIN"

if ! echo "$PATH" | tr ':' '\n' | grep -qx "$INSTALL_DIR"; then
  echo ""
  echo "Note: $INSTALL_DIR is not on your PATH."
  echo "Add this to your shell profile (~/.bashrc, ~/.zshrc, etc.):"
  echo "  export PATH=\"\$PATH:$INSTALL_DIR\""
fi

echo "Run: $BIN --help"