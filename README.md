<div align="center">

# CC Switch Web

### 轻量化纯 Web 架构的 Claude Code 提供商管理器 / Lightweight Pure Web Claude Code Provider Manager

[![Version](https://img.shields.io/badge/version-0.1.0-blue)](https://github.com/huangbogeng/cc-switch-ui)
[![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey.svg)](https://github.com/huangbogeng/cc-switch-ui)
[![License](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)

**[English](#english) / [中文](#中文)**

---

<!--ts-->
* [English](#english)
  * [Acknowledgments](#acknowledgments)
  * [Key Features](#key-features)
  * [Quick Start](#quick-start)
  * [Project Structure](#project-structure)
  * [Tech Stack](#tech-stack)
* [中文](#中文)
  * [致谢](#致谢)
  * [主要特性](#主要特性)
  * [快速开始](#快速开始)
  * [项目结构](#项目结构)
  * [技术栈](#技术栈)
<!--te-->

---

# English

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

---

***

---

# 中文

## 致谢

本项目基于 [Jason Young (farion1231)](https://github.com/farion1231) 的优秀开源项目 [cc-switch](https://github.com/farion1231/cc-switch) 开发。

**特别致谢原作者 Jason Young 的创新工作：**

| 领域 | 贡献 |
|------|------|
| 架构设计 | 原始 Tauri + React 桌面应用架构 |
| Provider 管理 | 多提供商配置管理与切换机制 |
| OAuth 认证 | 完整的 Copilot/Codex OAuth 认证流程 |
| 预设系统 | MiniMax、SiliconFlow、DeepSeek 等预设实现 |

本 fork 在原作者卓越工作的基础上，移除了 Tauri 桌面框架，采用纯 Web 部署模式，专注于 Claude Code 的提供商管理功能。

---

## 主要特性

- **纯 Web 架构**：无需安装桌面客户端，通过浏览器即可管理
- **Provider 预设**：内置 MiniMax、SiliconFlow、DeepSeek 等常用预设
- **实时配置同步**：支持修改后立即生效（Live Config）
- **简易部署**：前后端分离，可部署在任何 Web 服务器上

---

## 快速开始

### 前置要求

- Rust 1.85+
- Node.js 18+
- pnpm 8+

### 构建

```bash
# 构建 Rust 后端
cargo build --release

# 构建 React 前端
cd cc-switch-ui && pnpm install && pnpm build
```

### 运行

```bash
# 启动后端服务（默认端口 5007）
cargo run --bin cc-switch-web

# 或直接运行编译产物
./target/release/cc-switch-web
```

访问 `http://localhost:5007` 即可使用。

---

## 项目结构

```
cc-switch-web/       # Axum Web 后端
│   └── src/
│       ├── handlers/  # REST API handlers
│       └── proxy/     # 代理服务
├── cc-switch-ui/     # React 前端
│   └── src/
│       ├── api/       # API 调用层
│       ├── pages/     # 页面组件
│       └── config/    # 提供商预设
└── cc-switch-lib/    # Rust 核心库
    └── src/
        ├── database/  # SQLite 数据库操作
        ├── oauth/     # OAuth 认证
        └── live.rs    # 实时配置同步
```

---

## 技术栈

| 层级 | 技术 |
|------|------|
| 前端 | React 19 + TypeScript + Vite |
| 后端 | Rust + Axum 0.7 |
| 数据库 | SQLite (rusqlite) |
| 代理 | Hyper + Rustls |

---

## License

MIT License - 详见 [LICENSE](LICENSE) 文件
