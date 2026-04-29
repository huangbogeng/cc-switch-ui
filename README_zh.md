# CC Switch Web

**轻量化纯 Web 架构的 Claude Code Provider 管理器**

[![Version](https://img.shields.io/badge/version-0.1.0-blue)](https://github.com/huangbogeng/cc-switch-ui)
[![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey.svg)](https://github.com/huangbogeng/cc-switch-ui)
[![License](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)

**[English Version](README.md)**

---

## 项目简介

CC Switch Web 是一个基于浏览器的 Claude Code Provider 管理器。它保留 cc-switch 的 Provider 切换思路，但移除了 Tauri 桌面壳，改为 Rust 后端 + React 前端的纯 Web 管理服务。

适合本地或自托管使用：

- 管理 Claude Code Provider 预设和自定义 Provider。
- 切换当前 Provider，并立即写入 Claude Code live config。
- 处理 GitHub Copilot / Codex OAuth 流程。
- 从 Dashboard 启停本地代理端点。
- 使用 macOS 风格的 Web 管理界面，不需要安装桌面客户端。

---

## 致谢

本项目基于 [Jason Young (farion1231)](https://github.com/farion1231) 的优秀开源项目 [cc-switch](https://github.com/farion1231/cc-switch) 开发。

特别致谢原作者在以下方向上的工作：

| 领域 | 贡献 |
| --- | --- |
| 架构设计 | 原始 Tauri + React 桌面应用架构 |
| Provider 管理 | 多 Provider 配置管理与切换机制 |
| OAuth 认证 | Copilot/Codex OAuth 认证流程 |
| 预设系统 | MiniMax、SiliconFlow、DeepSeek 等预设实现 |

本 fork 专注于纯 Web 部署模式和浏览器优先的管理体验。

---

## 主要特性

- **纯 Web 管理界面**：通过 `/ui` 在浏览器访问，无需桌面客户端。
- **Provider 管理**：创建、编辑、删除、切换并持久化 Claude Provider。
- **Provider 预设**：内置 MiniMax、SiliconFlow、DeepSeek、Codex 和自定义 Provider 支持。
- **实时配置应用**：切换 Provider 后立即写入 Claude Code live settings。
- **OAuth 支持**：Web UI 暴露 Codex/OpenAI 与 GitHub Copilot OAuth 流程。
- **代理控制**：在 Dashboard 中启停本地代理端点。
- **现代前端**：React 19、TypeScript、Vite、Tailwind CSS v3、Radix UI primitives、lucide 图标。

---

## 快速开始

### 前置要求

- Rust 1.85+
- Node.js 18+
- npm

### 构建前端

```bash
cd cc-switch-ui
npm install
npm run build
```

Rust Web 服务会把 `cc-switch-ui/dist` 作为静态资源挂载到 `/ui`。

### 启动后端

```bash
# 在仓库根目录执行
cargo run --bin cc-switch-web
```

访问：

```text
http://localhost:5007/ui
```

如果没有设置 `CC_SWITCH_ADMIN_TOKEN`，管理 token 会在服务启动日志中打印。

---

## 配置项

| 环境变量 | 默认值 | 说明 |
| --- | --- | --- |
| `CC_SWITCH_ADMIN_TOKEN` | 启动时自动生成 | Web UI 登录和 API 请求使用的管理 token。 |
| `CC_SWITCH_PROXY_PORT` | `15721` | Dashboard 启动本地代理服务时使用的端口。 |
| `CC_SWITCH_TEST_HOME` | 未设置 | core library 测试用 home 目录覆盖。 |

Web 服务监听 `0.0.0.0:5007`。

---

## 开发

开发前端时建议前后端分开启动：

```bash
# Terminal 1: 后端 API，端口 5007
cargo run --bin cc-switch-web

# Terminal 2: Vite dev server
cd cc-switch-ui
npm install
npm run dev
```

Vite dev server 会把 `/api` 代理到 `http://localhost:5007`。

推荐检查命令：

```bash
cd cc-switch-ui
npm run lint
npm run build

cd ..
cargo test
cargo build
```

---

## 项目结构

```text
.
├── cc-switch-lib/          # Rust 核心库
│   └── src/
│       ├── config.rs       # Claude 配置路径和 settings 辅助逻辑
│       ├── database/       # SQLite Provider 持久化
│       ├── live.rs         # live config 应用逻辑
│       └── oauth/          # Codex 与 Copilot OAuth 管理器
├── cc-switch-web/          # Axum Web 服务
│   └── src/
│       ├── handlers/       # REST API handlers
│       ├── proxy/          # 本地代理服务
│       ├── state.rs        # 共享应用状态
│       └── main.rs         # 路由、认证中间件、静态 UI 挂载
└── cc-switch-ui/           # React 前端
    └── src/
        ├── api/            # 类型化 API client
        ├── components/     # UI、Dashboard、Provider 组件
        ├── config/         # Provider 预设
        ├── lib/            # 前端工具函数
        └── pages/          # 顶层页面
```

---

## 前端说明

当前前端明确使用 Tailwind CSS v3 + PostCSS：

- `tailwind.config.js`
- `postcss.config.cjs`
- `src/index.css` 中使用 `@tailwind base/components/utilities`

不要随意重新引入 Tailwind v4 或 `@tailwindcss/vite`；当前界面布局和对齐已经按 Tailwind v3 输出调过。

代码按职责拆分：

- `pages/`：有状态页面编排。
- `components/dashboard/`：Dashboard 面板和展示组件。
- `components/providers/`：Provider 卡片、预设选择器、Provider 表单弹窗、表单数据转换。
- `components/ui/`：共享底层 UI primitives。
- `lib/`：小型复用工具函数。

---

## API 概览

主要 API 分组：

| 领域 | 路由 |
| --- | --- |
| Auth | `POST /api/auth/login` |
| Providers | `GET/POST /api/providers`, `GET/PUT/DELETE /api/providers/:id`, `POST /api/providers/:id/switch` |
| Codex OAuth | `/api/codex/oauth/*` |
| Copilot OAuth | `/api/copilot/oauth/*`, `/api/copilot/usage` |
| Proxy | `POST /api/proxy/start`, `POST /api/proxy/stop`, `GET /api/proxy/status` |

除 login 和 health 外，API 请求需要携带 `Authorization: Bearer <admin-token>`。

---

## License

MIT License. 详见 [LICENSE](LICENSE)。

---

**[English Version](README.md)**
