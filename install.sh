#!/bin/bash
set -e

REPO="huangbogeng/cc-switch-ui"
INSTALL_DIR="${CC_SWITCH_INSTALL_DIR:-$HOME/.local/share/cc-switch-server}"
DATA_DIR="$HOME/.cc-switch"

echo "=========================================="
echo "    Installing CC Switch Server"
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
FILENAME="cc-switch-server-$DEST.tar.gz"
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
# The tarball contains a 'cc-switch-server' folder, so we strip 1 component.
tar -xzf /tmp/$FILENAME -C "$INSTALL_DIR" --strip-components=1
rm -f /tmp/$FILENAME

# Ensure it's executable
chmod +x "$INSTALL_DIR/cc-switch-server"
# Backward-compatible command alias
ln -sf "$INSTALL_DIR/cc-switch-server" "$INSTALL_DIR/cc-switch-web"

# 5. Add to PATH
SHELL_FILES=()

# 检测常用的 Shell 配置文件
if [ -f "$HOME/.zshrc" ] || [ -n "$ZSH_VERSION" ]; then
  SHELL_FILES+=("$HOME/.zshrc")
fi
if [ -f "$HOME/.bashrc" ] || [ -n "$BASH_VERSION" ]; then
  SHELL_FILES+=("$HOME/.bashrc")
fi
if [ ${#SHELL_FILES[@]} -eq 0 ]; then
  SHELL_FILES+=("$HOME/.profile")
fi

echo "-> Configuring PATH for your shells..."
for RC_FILE in "${SHELL_FILES[@]}"; do
  # 如果文件不存在，自动创建一个空文件
  touch "$RC_FILE" 2>/dev/null || true
  
  if ! grep -q "export PATH=.*$INSTALL_DIR" "$RC_FILE" 2>/dev/null; then
    echo "" >> "$RC_FILE"
    echo "# Added by CC Switch Server Installer" >> "$RC_FILE"
    echo "export PATH=\"\$PATH:$INSTALL_DIR\"" >> "$RC_FILE"
    echo "  ✅ Added to $RC_FILE"
  else
    echo "  ℹ️  Already configured in $RC_FILE"
  fi
done

echo ""
echo "🎉 CC Switch Server has been successfully installed!"
echo "Install directory: $INSTALL_DIR"
echo "Data directory: $DATA_DIR"
echo ""
echo "To get started, simply run:"
if [[ " ${SHELL_FILES[*]} " =~ ".zshrc" ]]; then
  echo "    source ~/.zshrc"
elif [[ " ${SHELL_FILES[*]} " =~ ".bashrc" ]]; then
  echo "    source ~/.bashrc"
else
  echo "    source ${SHELL_FILES[0]}"
fi
echo "    cc-switch-server"
echo "    # legacy alias: cc-switch-web"
echo ""
echo "The admin UI will be available at: http://localhost:5007/ui"
