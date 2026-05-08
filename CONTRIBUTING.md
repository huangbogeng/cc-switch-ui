# Contributing to CC Switch Web

> [中文版本](#贡献指南)

Thank you for your interest in contributing to CC Switch Web! Please read our [Code of Conduct](./CODE_OF_CONDUCT.md) before participating.

## How to Contribute

- **Report bugs** — Found something broken? [Open a bug report](https://github.com/huangbogeng/cc-switch-ui/issues/new?template=bug_report.yml).
- **Suggest features** — Have an idea? [Submit a feature request](https://github.com/huangbogeng/cc-switch-ui/issues/new?template=feature_request.yml).
- **Improve docs** — Spot a typo or missing info? [Report a doc issue](https://github.com/huangbogeng/cc-switch-ui/issues/new?template=doc_issue.yml).
- **Contribute code** — Fix bugs or implement features via pull requests.

## Development Setup

### Prerequisites

- Node.js 18+ and pnpm 8+
- Rust 1.85+ and Cargo

### Quick Start

```bash
# Install frontend dependencies
cd cc-switch-ui && pnpm install

# Start development server (proxies API to :5007)
pnpm dev

# In another terminal, start the backend
cargo run --bin cc-switch-server
```

### Useful Commands

Frontend (in `cc-switch-ui/`):
```bash
pnpm dev          # Dev server with hot reload
pnpm build        # Production build
pnpm lint         # ESLint check
```

Backend (in project root):
```bash
cargo run --bin cc-switch-server    # Run backend on :5007
cargo build --release            # Release build
cargo fmt && cargo clippy        # Format and lint
```

## Code Style

- **Frontend**: ESLint for linting, TypeScript
- **Backend**: `cargo fmt` for formatting, `cargo clippy` for linting

## Pull Request Guidelines

1. **Open an issue first** for new features
2. **Keep PRs focused** — One feature or fix per PR
3. **Fill in the PR template** — Describe what and why

### Commit Convention

We use [Conventional Commits](https://www.conventionalcommits.org/):

```
feat(provider): add support for new provider
fix(proxy): resolve request forwarding issue
docs(readme): update installation instructions
chore(deps): update dependencies
```

## Questions?

- [Open a question](https://github.com/huangbogeng/cc-switch-ui/issues/new?template=question.yml)
- [GitHub Discussions](https://github.com/huangbogeng/cc-switch-ui/discussions)

---

# 贡献指南

> [English Version](#contributing-to-cc-switch-server)

感谢你对 CC Switch Web 的贡献兴趣！参与之前请阅读我们的[行为准则](./CODE_OF_CONDUCT.md)。

## 如何贡献

- **报告 Bug** — 发现问题？[提交 Bug 报告](https://github.com/huangbogeng/cc-switch-ui/issues/new?template=bug_report.yml)。
- **建议功能** — 有想法？[提交功能请求](https://github.com/huangbogeng/cc-switch-ui/issues/new?template=feature_request.yml)。
- **改进文档** — 发现错误或缺失？[报告文档问题](https://github.com/huangbogeng/cc-switch-ui/issues/new?template=doc_issue.yml)。
- **贡献代码** — 通过 Pull Request 修复 Bug 或实现新功能。

## 开发环境搭建

### 前提条件

- Node.js 18+ 和 pnpm 8+
- Rust 1.85+ 和 Cargo

### 快速开始

```bash
# 安装前端依赖
cd cc-switch-ui && pnpm install

# 启动开发服务器（API 代理到 :5007）
pnpm dev

# 另一个终端启动后端
cargo run --bin cc-switch-server
```

### 常用命令

前端（在 `cc-switch-ui/` 目录）：
```bash
pnpm dev          # 热重载开发服务器
pnpm build        # 生产构建
pnpm lint         # ESLint 检查
```

后端（项目根目录）：
```bash
cargo run --bin cc-switch-server    # 运行后端于 :5007
cargo build --release            # 发布版本构建
cargo fmt && cargo clippy         # 格式化和检查
```

## 代码规范

- **前端**：ESLint 检查，TypeScript
- **后端**：`cargo fmt` 格式化，`cargo clippy` 检查

## Pull Request 指南

1. **新功能先开 Issue 讨论**
2. **保持 PR 专注** — 每个 PR 只做一件事
3. **填写 PR 模板** — 描述改了什么和为什么

### 提交信息规范

我们使用 [Conventional Commits](https://www.conventionalcommits.org/)：

```
feat(provider): add support for new provider
fix(proxy): resolve request forwarding issue
docs(readme): update installation instructions
chore(deps): update dependencies
```

## 有疑问？

- [提问](https://github.com/huangbogeng/cc-switch-ui/issues/new?template=question.yml)
- [GitHub 讨论区](https://github.com/huangbogeng/cc-switch-ui/discussions)
