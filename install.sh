#!/bin/sh
# agent-git installer — https://github.com/exisz/agent-git
set -e

REPO="exisz/agent-git"
BINARY="agent-git"

echo "Installing $BINARY..."

# Detect OS and arch
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

case "$OS" in
  linux)  ARTIFACT="agent-git-linux-x86_64" ;;
  darwin)
    case "$ARCH" in
      arm64|aarch64) ARTIFACT="agent-git-macos-arm64" ;;
      *)             ARTIFACT="agent-git-macos-x86_64" ;;
    esac
    ;;
  *) echo "Unsupported OS: $OS. Install via cargo: cargo install agent-git"; exit 1 ;;
esac

# Get latest release tag
TAG=$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" | grep '"tag_name"' | sed 's/.*"tag_name": *"\([^"]*\)".*/\1/')
if [ -z "$TAG" ]; then
  echo "Failed to fetch latest release. Install via cargo: cargo install agent-git"
  exit 1
fi

URL="https://github.com/$REPO/releases/download/$TAG/$ARTIFACT"
echo "Downloading $TAG ($ARTIFACT)..."

# Download to temp
TMP=$(mktemp)
curl -fsSL "$URL" -o "$TMP"
chmod +x "$TMP"

# Install
INSTALL_DIR="/usr/local/bin"
if [ ! -w "$INSTALL_DIR" ]; then
  INSTALL_DIR="$HOME/.local/bin"
  mkdir -p "$INSTALL_DIR"
fi

mv "$TMP" "$INSTALL_DIR/$BINARY"
echo "Installed $BINARY to $INSTALL_DIR/$BINARY"

# Setup alias
echo ""
echo "To activate (wraps git command):"
echo "  $BINARY alias install"
echo "  source ~/.zshrc  # or ~/.bashrc"
