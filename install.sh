#!/bin/bash
# Download and verify before stopping services. Keep program and data rollback copies.
set -euo pipefail
umask 077
REPO="huangbogeng/cc-switch-ui"
INSTALL_DIR="${CC_SWITCH_INSTALL_DIR:-$HOME/.local/share/cc-switch-ui}"
DATA_DIR="$HOME/.cc-switch"
fail() { echo "Error: $*" >&2; exit 1; }
for tool in curl tar ps; do command -v "$tool" >/dev/null || fail "Required command missing: $tool"; done
case "$INSTALL_DIR" in /*) ;; *) fail 'CC_SWITCH_INSTALL_DIR must be absolute';; esac
mkdir -p "$(dirname "$INSTALL_DIR")"
PARENT=$(cd "$(dirname "$INSTALL_DIR")" && pwd -P)
INSTALL_DIR="$PARENT/$(basename "$INSTALL_DIR")"
case "$INSTALL_DIR" in /|//|*/.|*/..|"$HOME"|"$DATA_DIR"|"$DATA_DIR"/*) fail 'Unsafe installation directory';; esac
[ ! -L "$INSTALL_DIR" ] && [ ! -L "$DATA_DIR" ] || fail 'Linked installation/data directories are unsupported'
if [ -e "$INSTALL_DIR" ] && [ ! -x "$INSTALL_DIR/cc-switch-ui" ]; then
    fail 'Existing destination is not a CC Switch UI installation'
fi
LOCK="$INSTALL_DIR.upgrade-lock"
mkdir "$LOCK" 2>/dev/null || fail "Another upgrade may be running; inspect $LOCK before removing a stale lock"
DATA_LOCK="$HOME/.cc-switch.upgrade-lock"
if ! mkdir "$DATA_LOCK" 2>/dev/null; then
    rmdir "$LOCK"
    fail "Another upgrade may be using this profile; inspect $DATA_LOCK before removing a stale lock"
fi
STAGING=''; BACKUP=''; WAS_RUNNING=0; OLD_MOVED=0; NEW_INSTALLED=0; NEW_PID=''; DATA_EXISTED=0; DATA_BACKED_UP=0; DATA_TOUCHED=0; COMPLETE=0
RESTART_ARGS=(); HEALTH_ARGS=()
alive() { kill -0 "$1" 2>/dev/null && ! ps -p "$1" -o stat= 2>/dev/null | grep -q '^Z'; }
stop_process() {
    local process_id="$1" attempt
    alive "$process_id" || return 0
    kill -TERM "$process_id" || return 1
    for attempt in {1..100}; do
        alive "$process_id" || { wait "$process_id" 2>/dev/null || true; return 0; }
        sleep 0.1
    done
    echo "Service $process_id did not stop; refusing to replace its data" >&2
    return 1
}
start_service() {
    local binary="$1" logfile="$2"
    RUST_LOG=info nohup "$binary" start ${RESTART_ARGS[@]+"${RESTART_ARGS[@]}"} >"$logfile" 2>&1 < /dev/null &
    NEW_PID=$!
}
healthy() {
    local binary="$1" logfile="$2" attempt
    for attempt in {1..100}; do
        alive "$NEW_PID" || return 1
        if grep -q "Server listening on http://" "$logfile" && "$binary" status ${HEALTH_ARGS[@]+"${HEALTH_ARGS[@]}"} >/dev/null 2>&1; then
            sleep 0.2
            alive "$NEW_PID" && return 0
        fi
        sleep 0.1
    done
    return 1
}
capture_restart_options() {
    local option value index=1
    local arguments=("$@")
    [ "${arguments[$index]:-}" = start ] || fail 'Cannot preserve service arguments; stop it before upgrading'
    index=$((index + 1))
    while [ "$index" -lt "${#arguments[@]}" ]; do
        option="${arguments[$index]}"
        case "$option" in
            --host|--port|--proxy-port|--admin-token|--ui-dir)
                index=$((index + 1))
                [ "$index" -lt "${#arguments[@]}" ] || fail 'Cannot preserve service arguments; stop it before upgrading'
                value="${arguments[$index]}";;
            --host=*|--port=*|--proxy-port=*|--admin-token=*|--ui-dir=*)
                value="${option#*=}"; option="${option%%=*}";;
            *) fail 'Cannot preserve service arguments; stop it before upgrading';;
        esac
        RESTART_ARGS+=("$option" "$value")
        case "$option" in
            --port) HEALTH_ARGS+=(--port "$value");;
            --host)
                case "$value" in 0.0.0.0) value=127.0.0.1;; ::) value=::1;; esac
                HEALTH_ARGS+=(--host "$value");;
        esac
        index=$((index + 1))
    done
}
cleanup() {
    local result=$?
    trap - EXIT INT TERM
    if [ "$COMPLETE" -eq 0 ]; then
        echo 'Installation did not complete.' >&2
        if [ -n "$NEW_PID" ] && ! stop_process "$NEW_PID"; then
            echo "Rollback paused: stop $NEW_PID before restoring data. Copies: $BACKUP" >&2
            rm -rf "$LOCK" "$DATA_LOCK"
            exit 1
        fi
        if [ "$NEW_INSTALLED" -eq 1 ] && [ -e "$INSTALL_DIR" ]; then
            mv "$INSTALL_DIR" "$BACKUP/failed-program" || { echo "Cannot move failed program; recover from $BACKUP" >&2; exit 1; }
        fi
        if [ "$OLD_MOVED" -eq 1 ] && [ -e "$BACKUP/previous" ]; then
            mv "$BACKUP/previous" "$INSTALL_DIR" || { echo "Cannot restore old program; recover from $BACKUP" >&2; exit 1; }
        fi
        if [ "$DATA_TOUCHED" -eq 1 ]; then
            if [ -e "$DATA_DIR" ]; then mv "$DATA_DIR" "$BACKUP/failed-data" || exit 1; fi
            if [ "$DATA_EXISTED" -eq 1 ] && [ "$DATA_BACKED_UP" -eq 1 ]; then
                tar -xzf "$BACKUP/data.tar.gz" -C "$HOME" || { echo "Data restore failed: $BACKUP/data.tar.gz" >&2; exit 1; }
            fi
        fi
        if [ "$WAS_RUNNING" -eq 1 ] && [ -x "$INSTALL_DIR/cc-switch-ui" ]; then
            start_service "$INSTALL_DIR/cc-switch-ui" "$BACKUP/rollback.log"
            if healthy "$INSTALL_DIR/cc-switch-ui" "$BACKUP/rollback.log"; then echo 'Previous service and data restored.' >&2;
            else echo "Old service restart failed; inspect $BACKUP/rollback.log" >&2; fi
        fi
        [ -z "$BACKUP" ] || echo "Recovery copies: $BACKUP" >&2
    fi
    [ -z "$STAGING" ] || rm -rf "$STAGING"
    rm -rf "$LOCK" "$DATA_LOCK"
    exit "$result"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM
STAGING=$(mktemp -d "$PARENT/.cc-switch-stage.XXXXXX")
PLATFORM=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)
case "$PLATFORM-$ARCH" in
    darwin-arm64) DEST=macos-aarch64;;
    darwin-x86_64) DEST=macos-x86_64;;
    linux-x86_64) DEST=linux-x86_64;;
    *) fail "No release asset for $PLATFORM-$ARCH; build from source";;
esac
VERSION=$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" | sed -n 's/.*"tag_name"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p')
[[ "$VERSION" =~ ^v[0-9]+\.[0-9]+\.[0-9]+$ ]] || fail 'Cannot determine stable release version'
# A stable-channel update must never silently downgrade a newer installed version.
CURRENT_CORE="${CC_SWITCH_CURRENT_VERSION:-}"
CURRENT_CORE="${CURRENT_CORE%%-*}"
CURRENT_CORE="${CURRENT_CORE%%+*}"
if [[ "$CURRENT_CORE" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    IFS=. read -r -a CURRENT_PARTS <<< "$CURRENT_CORE"
    IFS=. read -r -a RELEASE_PARTS <<< "${VERSION#v}"
    for index in 0 1 2; do
        if (( 10#${RELEASE_PARTS[$index]} < 10#${CURRENT_PARTS[$index]} )); then
            COMPLETE=1
            echo "Installed version ${CC_SWITCH_CURRENT_VERSION} is newer than stable $VERSION; no downgrade performed."
            exit 0
        fi
        if (( 10#${RELEASE_PARTS[$index]} > 10#${CURRENT_PARTS[$index]} )); then break; fi
    done
fi
if [ "${CC_SWITCH_CURRENT_VERSION:-}" = "${VERSION#v}" ] && [ "${CC_SWITCH_UPDATE_FORCE:-0}" != 1 ]; then
    COMPLETE=1
    echo "Already up to date ($VERSION)."
    exit 0
fi
FILENAME="cc-switch-ui-$DEST.tar.gz"
URL="https://github.com/$REPO/releases/download/$VERSION/$FILENAME"
echo "Downloading $VERSION ($DEST)..."
curl -fSL "$URL" -o "$STAGING/$FILENAME"
curl -fSL "$URL.sha256" -o "$STAGING/checksum" || fail 'Release lacks a checksum; no installed files were replaced'
EXPECTED=$(awk 'NR==1 {print $1}' "$STAGING/checksum")
[[ "$EXPECTED" =~ ^[0-9a-fA-F]{64}$ ]] || fail 'Invalid checksum file'
if command -v sha256sum >/dev/null; then ACTUAL=$(sha256sum "$STAGING/$FILENAME" | awk '{print $1}');
else ACTUAL=$(shasum -a 256 "$STAGING/$FILENAME" | awk '{print $1}'); fi
[ "$ACTUAL" = "$EXPECTED" ] || fail 'Release checksum mismatch; installed files unchanged'
tar -tzf "$STAGING/$FILENAME" > "$STAGING/entries"
while IFS= read -r entry; do
    case "$entry" in cc-switch-ui|cc-switch-ui/|cc-switch-ui/*) ;; *) fail 'Unexpected release archive path';; esac
    case "/$entry/" in */../*) fail 'Unsafe release archive path';; esac
done < "$STAGING/entries"
tar -xzf "$STAGING/$FILENAME" -C "$STAGING"
[ -f "$STAGING/cc-switch-ui/cc-switch-ui" ] && [ -f "$STAGING/cc-switch-ui/dist/index.html" ] || fail 'Release is missing program or frontend'
chmod +x "$STAGING/cc-switch-ui/cc-switch-ui"
REPORTED_VERSION=$("$STAGING/cc-switch-ui/cc-switch-ui" --version)
[ "$REPORTED_VERSION" = "cc-switch-ui ${VERSION#v}" ] || fail 'Binary version does not match release'
ln -sf cc-switch-ui "$STAGING/cc-switch-ui/cc-switch-server"
ln -sf cc-switch-ui "$STAGING/cc-switch-ui/cc-switch-web"
# A private backup outside the data directory survives failed migration and restore.
[ ! -L "$PARENT/cc-switch-upgrade-backups" ] || fail 'Linked backup directory is unsupported'
mkdir -p "$PARENT/cc-switch-upgrade-backups"
BACKUP=$(mktemp -d "$PARENT/cc-switch-upgrade-backups/$VERSION.XXXXXX")
if [ -f "$DATA_DIR/cc-switch-server.pid" ]; then
    OLD_PID=$(cat "$DATA_DIR/cc-switch-server.pid")
    [[ "$OLD_PID" =~ ^[0-9]+$ ]] && [ "$OLD_PID" -gt 1 ] || fail 'Invalid service PID; stop existing services before upgrading'
    if alive "$OLD_PID"; then
        SERVICE_ARGS=()
        if [ "$(uname -s)" = Linux ]; then
            # argv[0] may be a PATH name, relative path or an alias. Trust the kernel's
            # executable identity, never a command-line prefix supplied by a process.
            [ "$INSTALL_DIR/cc-switch-ui" -ef "/proc/$OLD_PID/exe" ] || fail "Service $OLD_PID is not from this installation; stop it before upgrading"
            [ -r "/proc/$OLD_PID/cmdline" ] || fail 'Cannot read service arguments; stop it before upgrading'
            while IFS= read -r -d '' argument; do SERVICE_ARGS+=("$argument"); done < "/proc/$OLD_PID/cmdline"
        else
            # Keep the conservative macOS fallback until executable identity is available.
            COMMAND=$(ps -p "$OLD_PID" -o args=)
            case "$COMMAND" in "$INSTALL_DIR/cc-switch-ui "*|"$INSTALL_DIR/cc-switch-server "*|"$INSTALL_DIR/cc-switch-web "*) ;;
                *) fail "Service $OLD_PID is not from this installation; stop it before upgrading";; esac
            read -r -a SERVICE_ARGS <<< "$COMMAND"
        fi
        capture_restart_options "${SERVICE_ARGS[@]}"
        stop_process "$OLD_PID" || fail 'Unable to stop existing service'
        WAS_RUNNING=1
    fi
fi
# Reject a live endpoint that cannot be tied to the stopped PID, including stale PID files.
if "$STAGING/cc-switch-ui/cc-switch-ui" status ${HEALTH_ARGS[@]+"${HEALTH_ARGS[@]}"} >/dev/null 2>&1; then
    fail 'A service is still responding; stop all services using this profile before upgrading'
fi
if [ -d "$DATA_DIR" ]; then
    DATA_EXISTED=1
    tar -czf "$BACKUP/data.tar.gz" -C "$HOME" .cc-switch
    tar -tzf "$BACKUP/data.tar.gz" >/dev/null
    DATA_BACKED_UP=1
fi
if [ -e "$INSTALL_DIR" ]; then OLD_MOVED=1; mv "$INSTALL_DIR" "$BACKUP/previous"; fi
NEW_INSTALLED=1
mv "$STAGING/cc-switch-ui" "$INSTALL_DIR"
DATA_TOUCHED=1
start_service "$INSTALL_DIR/cc-switch-ui" "$BACKUP/startup.log"
healthy "$INSTALL_DIR/cc-switch-ui" "$BACKUP/startup.log" || fail "New service failed health check; see $BACKUP/startup.log"
if [ "$WAS_RUNNING" -eq 0 ]; then
    stop_process "$NEW_PID" || fail 'Cannot stop validation service'
    NEW_PID=''
fi
COMPLETE=1
# PATH changes happen only after successful validation. Existing shell aliases are preserved.
SHELL_FILES=()
[ ! -f "$HOME/.zshrc" ] || SHELL_FILES+=("$HOME/.zshrc")
[ ! -f "$HOME/.bashrc" ] || SHELL_FILES+=("$HOME/.bashrc")
[ "${#SHELL_FILES[@]}" -gt 0 ] || SHELL_FILES+=("$HOME/.profile")
for RC_FILE in "${SHELL_FILES[@]}"; do
    if ! grep -Fq "$INSTALL_DIR" "$RC_FILE" 2>/dev/null; then
        printf '\n# Added by CC Switch UI Installer\nexport PATH="$PATH":%q\n' "$INSTALL_DIR" >> "$RC_FILE" || echo "Add $INSTALL_DIR to PATH manually" >&2
    fi
done
echo "Installed $VERSION. Recovery copies: $BACKUP"
if [ "$WAS_RUNNING" -eq 1 ]; then
    echo 'Service restarted. Startup details (including login token):'
    sed -n '/  Web UI:/p; /  Admin Token:/p' "$BACKUP/startup.log"
else
    echo 'Validation passed. Run cc-switch-ui start to start the service.'
fi
