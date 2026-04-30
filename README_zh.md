# CC Switch Web

**管理 Claude Code 的 Provider 配置从未如此简单**

[![Version](https://img.shields.io/badge/version-0.1.0-blue)](https://github.com/huangbogeng/cc-switch-ui)
[![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey.svg)](https://github.com/huangbogeng/cc-switch-ui)
[![License](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)

**[English Version](README.md)**

---

## 它解决了什么问题

配置 Claude Code 的 API Provider 需要手动编辑 JSON 文件、记住各种 API 端点、处理 OAuth 授权。

**CC Switch Web 让这一切在浏览器中完成：**

- 一键切换 Provider，无需手动编辑配置
- 内置 6 种常用 Provider 预设（MiniMax、SiliconFlow、DeepSeek、OpenRouter、Gemini Native、Codex）
- 支持 API Key 和 OAuth 两种认证方式
- 实时写入 Claude Code 配置，切换后立即生效

---

## 支持的 Provider

| Provider | 类型 | 认证方式 | 说明 |
|----------|------|---------|------|
| MiniMax | 国内官方 | API Key | MiniMax M2.7 模型 |
| SiliconFlow | 聚合平台 | API Key | 支持多种模型 |
| DeepSeek | 国内官方 | API Key | DeepSeek V4 模型 |
| OpenRouter | 聚合平台 | API Key | 100+ 模型可选 |
| Gemini Native | Google | API Key | Gemini 原生 API |
| Codex | OpenAI | OAuth | 通过本地代理转发 |

---

## 快速开始

### 1. 启动服务

```bash
# 编译
cargo build --release

# 运行
cargo run --bin cc-switch-web
```

### 2. 访问 Web UI

打开浏览器访问：**http://localhost:5007/ui**

首次登录需要 admin token，可在启动日志中找到。

### 3. 添加 Provider

1. 进入 **Providers** 页面
2. 选择预设或自定义配置
3. 填入 API Key
4. 点击保存并切换

切换后 Claude Code 会立即使用新的 Provider。

---

## 功能特性

### Provider 管理
- 内置 6 种 Provider 预设
- 支持自定义 Provider
- 一键切换，实时生效

### OAuth 认证
- **Codex**：通过本地代理转发 OAuth 请求
- **GitHub Copilot**：OAuth 认证（开发中）

### 本地代理
- 启动本地代理服务处理 Codex 请求
- 支持 HTTP/SOCKS5 代理配置
- 流式响应格式转换

### Live Config
- 切换 Provider 时自动写入 Claude Code 配置
- 无需手动编辑配置文件

---

## 技术架构

```
┌─────────────────────────────────────────────────┐
│               Browser (React UI)                 │
│         http://localhost:5007/ui                 │
└─────────────────────┬───────────────────────────┘
                      │ HTTP /api/*
┌─────────────────────▼───────────────────────────┐
│           cc-switch-web (Rust + Axum)            │
│              localhost:5007                      │
│  ┌─────────────┐  ┌─────────────┐  ┌────────┐ │
│  │ REST API    │  │ OAuth       │  │ Proxy  │ │
│  │ /api/*      │  │ codex/copilot│  │ :15721 │ │
│  └─────────────┘  └─────────────┘  └────────┘ │
└─────────────────────┬───────────────────────────┘
                      │
┌─────────────────────▼───────────────────────────┐
│              cc-switch-lib (Rust)                │
│  ┌─────────────┐  ┌─────────────┐  ┌────────┐ │
│  │ SQLite DB   │  │ Live Config │  │ OAuth  │ │
│  └─────────────┘  └─────────────┘  └────────┘ │
└─────────────────────────────────────────────────┘
```

---

## 配置说明

| 环境变量 | 默认值 | 说明 |
|---------|--------|------|
| `CC_SWITCH_ADMIN_TOKEN` | 自动生成 | Web UI 管理员密码 |
| `CC_SWITCH_PROXY_PORT` | `15721` | 本地代理端口 |
| `CC_SWITCH_TEST_HOME` | - | 测试用 home 目录 |

---

## 开发

### 前端开发

```bash
cd cc-switch-ui
pnpm install
pnpm dev        # 开发服务器 http://localhost:5173
pnpm build      # 生产构建
pnpm lint       # 代码检查
```

### 后端开发

```bash
cargo run --bin cc-switch-web  # API 服务器
cargo fmt && cargo clippy      # 格式和检查
```

### 测试

```bash
cargo test
```

---

## 项目结构

```
cc-switch-ui/          # React 前端 (TypeScript + Vite)
cc-switch-web/         # Axum HTTP 服务器 (Rust)
cc-switch-lib/         # 共享核心库 (Rust)
  └── src/
      ├── database/    # SQLite 数据持久化
      ├── oauth/       # OAuth 认证 (Codex + Copilot)
      ├── config.rs    # 配置管理
      └── live.rs      # Live Config 同步
```

---

## 与原版 cc-switch 的区别

| 特性 | cc-switch (原版) | CC Switch Web (本项目) |
|------|-----------------|----------------------|
| 部署方式 | Tauri 桌面应用 | 纯 Web 服务 |
| 系统托盘 | 支持 | 不支持 |
| MCP 管理 | 支持 | 规划中 |
| 云同步 | 支持 | 不支持 |
| 核心功能 | 完整功能集 | 聚焦 Provider 管理 |

---

## 致谢

基于优秀开源项目 [cc-switch](https://github.com/farion1231/cc-switch) 开发。

---

## License

MIT License

---

**[English Version](README.md)**