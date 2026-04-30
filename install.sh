#!/bin/bash
set -e

REPO="huangbogeng/cc-switch-ui"
INSTALL_DIR="${CC_SWITCH_INSTALL_DIR:-$HOME/.cc-switch}"

echo "=========================================="
echo "    Installing CC Switch Web Admin"
echo "=========================================="

# 1. Fetch the latest release version
echo "-> Fetching latest release info from GitHub..."
VERSION=$(curl -s https://api.github.com/repos/$REPO/releases/latest | grep tag_name | cut -d'"' -f4)
if [ -z "$VERSION" ]; then
  echo "❌ Failed to get latest release version. Are you rate-limited by GitHub API?"
  exit 1
fi
echo "-> Found latest version: $VERSION"

# 2. Detect platform and architecture
PLATFORM=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

case "$PLATFORM-$ARCH" in
  darwin-arm64)
    DEST="macos-aarch64"
    ;;
  darwin-x86_64)
    DEST="macos-x86_64"
    ;;
  linux-x86_64)
    DEST="linux-x86_64"
    ;;
  linux-aarch64|linux-arm64)
    DEST="linux-aarch64"
    ;;
  *)
    echo "❌ Unsupported platform: $PLATFORM-$ARCH"
    echo "Please build from source: cargo build --release"
    exit 1
    ;;
esac

# 3. Download the tarball
FILENAME="cc-switch-web-$DEST.tar.gz"
URL="https://github.com/$REPO/releases/download/$VERSION/$FILENAME"

echo "-> Downloading $URL..."
if ! curl -fSL "$URL" -o /tmp/$FILENAME; then
  echo "❌ Failed to download the release asset."
  exit 1
fi

# 4. Extract and Install
echo "-> Extracting to $INSTALL_DIR..."
# Ensure the directory exists
mkdir -p "$INSTALL_DIR"

# Extract directly into INSTALL_DIR. 
# The tarball contains a 'cc-switch-web' folder, so we strip 1 component.
tar -xzf /tmp/$FILENAME -C "$INSTALL_DIR" --strip-components=1
rm -f /tmp/$FILENAME

# Ensure it's executable
chmod +x "$INSTALL_DIR/cc-switch-web"

# 5. Add to PATH
SHELL_RC=""
if [ -n "$ZSH_VERSION" ] || [ -f "$HOME/.zshrc" ]; then
  SHELL_RC="$HOME/.zshrc"
elif [ -n "$BASH_VERSION" ] || [ -f "$HOME/.bashrc" ]; then
  SHELL_RC="$HOME/.bashrc"
else
  SHELL_RC="$HOME/.profile"
fi

if ! grep -q "export PATH=.*$INSTALL_DIR" "$SHELL_RC" 2>/dev/null; then
  echo "" >> "$SHELL_RC"
  echo "# Added by CC Switch Web Installer" >> "$SHELL_RC"
  echo "export PATH=\"\$PATH:$INSTALL_DIR\"" >> "$SHELL_RC"
  echo "-> ✅ Added $INSTALL_DIR to PATH in $SHELL_RC"
else
  echo "-> ℹ️  $INSTALL_DIR is already in your PATH."
fi

echo ""
echo "🎉 CC Switch Web has been successfully installed!"
echo ""
echo "To get started, simply run:"
echo "    source $SHELL_RC"
echo "    cc-switch-web"
echo ""
echo "The admin UI will be available at: http://localhost:5007/ui"