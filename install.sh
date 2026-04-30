#!/bin/bash
set -e

REPO="huangbogeng/cc-switch-ui"

# Get latest release version
VERSION=$(curl -s https://api.github.com/repos/$REPO/releases/latest | grep tag_name | cut -d'"' -f4)
if [ -z "$VERSION" ]; then
  echo "Failed to get latest release version"
  exit 1
fi

echo "Installing cc-switch-web $VERSION..."

# Detect platform
PLATFORM=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

case "$PLATFORM-$ARCH" in
  darwin-arm64)
    DEST="darwin-arm64"
    ;;
  darwin-x86_64)
    DEST="darwin-x64"
    ;;
  linux-x86_64)
    DEST="linux-x64"
    ;;
  linux-aarch64|linux-arm64)
    DEST="linux-arm64"
    ;;
  cygwin*|mingw*|msys*|win*)
    DEST="windows-x64"
    ;;
  *)
    echo "Unsupported platform: $PLATFORM-$ARCH"
    exit 1
    ;;
esac

# Download
FILENAME="cc-switch-web-$DEST.tar.gz"
URL="https://github.com/$REPO/releases/download/$VERSION/$FILENAME"

INSTALL_DIR="${CC_SWITCH_INSTALL_DIR:-$HOME/.cc-switch}"
mkdir -p "$INSTALL_DIR"

echo "Downloading $URL..."
if ! curl -fSL "$URL" -o /tmp/$FILENAME; then
  echo "Failed to download. This release might not support $DEST."
  exit 1
fi

echo "Extracting to $INSTALL_DIR..."
tar -xzf /tmp/$FILENAME -C "$INSTALL_DIR"
rm -f /tmp/$FILENAME

# Add to PATH
SHELL_RC=""
if [ -f "$HOME/.bashrc" ]; then
  SHELL_RC="$HOME/.bashrc"
elif [ -f "$HOME/.zshrc" ]; then
  SHELL_RC="$HOME/.zshrc"
else
  SHELL_RC="$HOME/.profile"
fi

# Check if already in PATH
if ! grep -q "$INSTALL_DIR" "$SHELL_RC" 2>/dev/null; then
  echo "" >> "$SHELL_RC"
  echo "# Added by cc-switch-web installer" >> "$SHELL_RC"
  echo "export PATH=\"$INSTALL_DIR:\$PATH\"" >> "$SHELL_RC"
  echo "Added $INSTALL_DIR to PATH in $SHELL_RC"
  echo "Please run 'source $SHELL_RC' or restart your terminal to use 'cc-switch-web'"
else
  echo "$INSTALL_DIR already in PATH"
fi

echo ""
echo "========================================"
echo "  cc-switch-web installed successfully!"
echo ""
echo "  Installation dir: $INSTALL_DIR"
echo "  Version: $VERSION"
echo ""
echo "  Run 'cc-switch-web' to start the server"
echo "  Then open: http://localhost:5007/ui"
echo "========================================"