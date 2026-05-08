# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

CC Switch Web is a lightweight pure Web architecture Claude Code provider manager. It is a fork of [cc-switch](https://github.com/farion1231/cc-switch) that removes the Tauri desktop framework, keeping only the web deployment mode.

## Development Commands

### Frontend (React + Vite)
```bash
cd cc-switch-ui
pnpm install
pnpm dev        # Dev server on http://localhost:5173 with API proxy to :5007
pnpm build      # Production build
pnpm lint       # ESLint check
```

### Backend (Rust + Axum)
```bash
cargo run --bin cc-switch-server  # Runs on http://localhost:5007
cargo build --release          # Build release binary
```

### Code Quality
```bash
cargo fmt && cargo clippy      # Rust formatting and linting
```

## Architecture

### Workspace Structure
- `cc-switch-lib/` — Core Rust library (shared between web and any future desktop apps)
  - `src/database/` — SQLite operations via rusqlite
  - `src/oauth/` — OAuth authentication (Codex, Copilot)
  - `src/config.rs` — Configuration management
  - `src/live.rs` — Live config sync mechanism
- `cc-switch-server/` — Axum HTTP server with REST API handlers
  - `src/handlers/` — API route handlers (auth, oauth, providers)
  - `src/proxy/` — Proxy server for provider requests
  - `src/main.rs` — Entry point, exposes binary `cc-switch-server`
- `cc-switch-ui/` — React 19 frontend
  - `src/api/index.ts` — API client layer
  - `src/pages/` — DashboardPage, LoginPage, ProvidersPage
  - `src/config/providerPresets.ts` — Provider preset configurations

### Frontend Proxy
Vite dev server proxies `/api/*` requests to `http://localhost:5007`. The built frontend serves static files from `cc-switch-ui/dist/`.

### Database
SQLite via rusqlite (bundled). Database file location is managed by the `dirs` crate.

## Tech Stack

| Layer | Technology |
|-------|------------|
| Frontend | React 19 + TypeScript + Vite |
| Backend | Rust + Axum 0.7 |
| Database | SQLite (rusqlite) |
| TLS | rustls + Hyper |

## References

- Original project: https://github.com/farion1231/cc-switch
- This is a pure Web architecture variant — the `src.bak/` and `src-tauri.bak/` directories contain the original Tauri-based reference code.
