# CC Switch Web

**Lightweight pure Web Claude Code provider manager**

[![Version](https://img.shields.io/badge/version-0.1.0-blue)](https://github.com/huangbogeng/cc-switch-ui)
[![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey.svg)](https://github.com/huangbogeng/cc-switch-ui)
[![License](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)

**[中文版](README_zh.md)**

---

## Overview

CC Switch Web is a browser-based Claude Code provider manager. It keeps the original cc-switch provider switching idea, but removes the Tauri desktop shell and runs as a Web admin service with a Rust backend and React frontend.

It is intended for local or self-hosted use:

- Manage Claude Code provider presets and custom providers.
- Switch the active provider and apply it to Claude Code live config.
- Authenticate GitHub Copilot / Codex OAuth flows.
- Start a local proxy endpoint for Codex OAuth-backed requests.
- Use a macOS-style Web UI without installing a desktop app.

---

## Acknowledgments

This project is based on the excellent open source project [cc-switch](https://github.com/farion1231/cc-switch) by [Jason Young (farion1231)](https://github.com/farion1231).

Special thanks to the original author for:

| Area | Contribution |
| --- | --- |
| Architecture | Original Tauri + React desktop application architecture |
| Provider Management | Multi-provider configuration management and switching mechanism |
| OAuth Authentication | Copilot/Codex OAuth authentication flow |
| Preset System | MiniMax, SiliconFlow, DeepSeek preset implementations |

This fork focuses on a pure Web deployment model and a browser-first admin experience.

---

## Features

- **Pure Web admin**: Access from a browser at `/ui`; no desktop client required.
- **Provider management**: Create, edit, delete, switch, and persist Claude providers.
- **Provider presets**: Built-in presets for MiniMax, SiliconFlow, DeepSeek, Codex, and custom providers.
- **Live config application**: Switching a provider writes to Claude Code live settings immediately.
- **OAuth support**: Codex/OpenAI and GitHub Copilot OAuth flows are exposed through the Web UI.
- **Proxy control**: Start/stop a local proxy endpoint from the Dashboard.
- **Modern frontend**: React 19, TypeScript, Vite, Tailwind CSS v3, Radix UI primitives, and lucide icons.

---

## Quick Start

### Prerequisites

- Rust 1.85+
- Node.js 18+
- npm

### Build Frontend

```bash
cd cc-switch-ui
npm install
npm run build
```

The Rust Web server serves the built frontend from `cc-switch-ui/dist` under `/ui`.

### Run Backend

```bash
# From the repository root
cargo run --bin cc-switch-web
```

Open:

```text
http://localhost:5007/ui
```

The admin token is printed in the server logs on startup unless `CC_SWITCH_ADMIN_TOKEN` is set.

---

## Configuration

| Variable | Default | Description |
| --- | --- | --- |
| `CC_SWITCH_ADMIN_TOKEN` | Generated on startup | Admin login token for the Web UI and API requests. |
| `CC_SWITCH_PROXY_PORT` | `15721` | Port used by the local proxy server started from the Dashboard. |
| `CC_SWITCH_TEST_HOME` | unset | Test-only home directory override used by the core library. |

The Web server listens on `0.0.0.0:5007`.

---

## Development

Run the backend and frontend separately during UI development:

```bash
# Terminal 1: backend API on port 5007
cargo run --bin cc-switch-web

# Terminal 2: Vite dev server
cd cc-switch-ui
npm install
npm run dev
```

The Vite dev server proxies `/api` requests to `http://localhost:5007`.

Recommended checks:

```bash
cd cc-switch-ui
npm run lint
npm run build

cd ..
cargo test
cargo build
```

---

## Project Structure

```text
.
├── cc-switch-lib/          # Rust core library
│   └── src/
│       ├── config.rs       # Claude config path and settings helpers
│       ├── database/       # SQLite provider persistence
│       ├── live.rs         # Live config application
│       └── oauth/          # Codex and Copilot OAuth managers
├── cc-switch-web/          # Axum Web server
│   └── src/
│       ├── handlers/       # REST API handlers
│       ├── proxy/          # Local proxy server
│       ├── state.rs        # Shared app state
│       └── main.rs         # Routes, auth middleware, static UI serving
└── cc-switch-ui/           # React frontend
    └── src/
        ├── api/            # Typed API client
        ├── components/     # UI, dashboard, provider components
        ├── config/         # Provider presets
        ├── lib/            # Frontend utilities
        └── pages/          # Top-level pages
```

---

## Frontend Notes

The frontend intentionally uses Tailwind CSS v3 through PostCSS:

- `tailwind.config.js`
- `postcss.config.cjs`
- `src/index.css` with `@tailwind base/components/utilities`

Avoid reintroducing Tailwind v4 or `@tailwindcss/vite` unless the layout and design system are migrated deliberately.

The UI code is split by responsibility:

- `pages/`: stateful page orchestration.
- `components/dashboard/`: dashboard panels and dashboard-specific display components.
- `components/providers/`: provider cards, preset selector, provider form dialog, form data conversion.
- `components/ui/`: shared low-level UI primitives.
- `lib/`: small reusable formatting and calculation helpers.

---

## API Surface

Main API groups:

| Area | Routes |
| --- | --- |
| Auth | `POST /api/auth/login` |
| Providers | `GET/POST /api/providers`, `GET/PUT/DELETE /api/providers/:id`, `POST /api/providers/:id/switch` |
| Codex OAuth | `/api/codex/oauth/*` |
| Copilot OAuth | `/api/copilot/oauth/*`, `/api/copilot/usage` |
| Proxy | `POST /api/proxy/start`, `POST /api/proxy/stop`, `GET /api/proxy/status` |

All API routes except login and health require `Authorization: Bearer <admin-token>`.

---

## License

MIT License. See [LICENSE](LICENSE) for details.
