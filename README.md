# CC Switch Web

**Lightweight Pure Web Claude Code Provider Manager**

[![Version](https://img.shields.io/badge/version-0.1.0-blue)](https://github.com/huangbogeng/cc-switch-ui)
[![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey.svg)](https://github.com/huangbogeng/cc-switch-ui)
[![License](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)

**[中文版](README_zh.md)**

---

## Acknowledgments

This project is based on the excellent open source project [cc-switch](https://github.com/farion1231) by [Jason Young (farion1231)](https://github.com/farion1231).

**Special thanks to Jason Young for his innovative work:**

| Area | Contribution |
|------|--------------|
| Architecture | Original Tauri + React desktop application architecture |
| Provider Management | Multi-provider configuration management and switching mechanism |
| OAuth Authentication | Complete Copilot/Codex OAuth authentication flow |
| Preset System | MiniMax, SiliconFlow, DeepSeek preset implementations |

This fork builds on the excellent work of the original author by removing the Tauri desktop framework and adopting a pure Web deployment model, focusing on Claude Code provider management.

---

## Key Features

- **Pure Web Architecture**: No desktop client installation needed, manage through browser
- **Provider Presets**: Built-in presets for MiniMax, SiliconFlow, DeepSeek, and more
- **Live Config Sync**: Changes take effect immediately (Live Config)
- **Easy Deployment**: Frontend and backend separation, deployable on any web server

---

## Quick Start

### Prerequisites

- Rust 1.85+
- Node.js 18+
- pnpm 8+

### Build

```bash
# Build Rust backend
cargo build --release

# Build React frontend
cd cc-switch-ui && pnpm install && pnpm build
```

### Run

```bash
# Start backend (default port 5007)
cargo run --bin cc-switch-web

# Or run the compiled binary directly
./target/release/cc-switch-web
```

Visit `http://localhost:5007` to use.

---

## Project Structure

```
cc-switch-web/       # Axum Web backend
│   └── src/
│       ├── handlers/  # REST API handlers
│       └── proxy/     # Proxy service
├── cc-switch-ui/     # React frontend
│   └── src/
│       ├── api/       # API client layer
│       ├── pages/     # Page components
│       └── config/    # Provider presets
└── cc-switch-lib/    # Rust core library
    └── src/
        ├── database/  # SQLite operations
        ├── oauth/     # OAuth authentication
        └── live.rs    # Live config sync
```

---

## Tech Stack

| Layer | Technology |
|-------|------------|
| Frontend | React 19 + TypeScript + Vite |
| Backend | Rust + Axum 0.7 |
| Database | SQLite (rusqlite) |
| Proxy | Hyper + Rustls |

---

## License

MIT License - See [LICENSE](LICENSE) file for details.
