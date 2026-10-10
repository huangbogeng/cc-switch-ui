# Development / 开发

[Documentation](README.md) · [Frontend design](frontend-design.md) · [Contributing](../CONTRIBUTING.md)

## Toolchain

Use Node.js **24.15+**, npm and the Rust **1.95** toolchain pinned in `rust-toolchain.toml`, including rustfmt and clippy. CI uses the same toolchain. Use `npm ci` with the committed lockfile for reproducible dependency installation.

## Run locally

From the repository root, start the API:

```bash
cargo run -p cc-switch-server
```

In another terminal:

```bash
cd cc-switch-ui
npm ci
npm run dev
```

Vite serves the frontend under `/ui/` and proxies `/api` to `http://localhost:5007`. Sign in using the backend startup token. Development servers use the normal profile by default; point test/manual fixtures at an isolated home if you need to avoid changing your real Claude configuration.

For a production-style build:

```bash
cd cc-switch-ui
npm ci
npm run build
cd ..
cargo run -p cc-switch-cli -- start
```

The frontend output is `cc-switch-ui/dist/`. Release packages place `dist/` beside the CLI executable. `--ui-dir` or `CC_SWITCH_UI_DIR` can override the served frontend directory.

## Checks

From the repository root:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

From `cc-switch-ui/`:

```bash
npm run lint
npm test
npm run build
```

Run real-service workflow tests separately with:

```bash
cargo test -p cc-switch-cli --test workflows
```

These tests use temporary homes, loopback ports and local mock upstreams. They exercise CLI/HTTP operations, SQLite and live file outcomes without your accounts or configuration. Frontend tests separately cover component behavior; they do not substitute for visual browser inspection.

## Older-release upgrade regressions

Build the new CLI, then provide an extracted old release binary:

```bash
cargo build -p cc-switch-cli
bash -n install.sh
python3 tests/test_upgrade.py \
  --new-binary target/debug/cc-switch-ui \
  --old-binary /absolute/path/to/old-release/cc-switch-ui -v
```

The test runner isolates HOME and uses real processes for upgrade, rollback and updater checks. CI runs the matrix with old releases `v0.1.9` and `v0.2.0`. Do not substitute your running installation or real profile for the fixtures.

## Structure and contracts

| Location | Responsibility |
|----------|----------------|
| `cc-switch-cli/` | CLI defaults, start/stop/status, installer-backed updates |
| `cc-switch-server/` | Axum REST API, OAuth handlers, static assets and proxy |
| `cc-switch-lib/` | SQLite, guarded live writes, provider journals, MCP/Skills and recovery |
| `cc-switch-ui/` | React pages, shared components, API client and local brand assets |
| `tests/test_upgrade.py` | Isolated release-upgrade and rollback regressions |

Keep protocol conversions in adapters. Provider/proxy changes should cover request/response/stream behavior, usage and failure cases as affected. Preserve independent Direct Config / Route Target selections and switch/start/stop consistency. File-operation changes must respect ownership, external edits and pending recovery; read the [recovery reference](configuration-recovery.md).

Frontend changes should follow the shared [design conventions](frontend-design.md), preserve accessible focus/dialog behavior and reduced motion, and retain lazy page loading. Provider logo provenance is recorded in [the asset README](../cc-switch-ui/public/logos/README.md); bundled fonts have their own [provenance and license](../cc-switch-ui/public/fonts/README.md).
