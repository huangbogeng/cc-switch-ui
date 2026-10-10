# CC Switch UI

**面向 Claude Code 的浏览器 Provider 管理器与本地路由服务。**

[![Version](https://img.shields.io/badge/version-0.2.0-blue)](https://github.com/huangbogeng/cc-switch-ui)
[![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey.svg)](https://github.com/huangbogeng/cc-switch-ui)
[![License](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)

[English](README.md)

CC Switch UI 在一个 Web 界面中管理 Claude Code 的 Provider、OAuth 账号、MCP Server、Skills、用量数据与本地协议转换代理。项目基于 [cc-switch](https://github.com/farion1231/cc-switch) 演进，采用 Web + Rust 架构，适合无头服务器和浏览器管理场景。

> 管理接口默认只监听 `127.0.0.1`。仅应在可信网络中使用 `--host 0.0.0.0`，并妥善保管 admin token。

## 目录

- [工作方式](#工作方式)
- [功能](#功能)
- [支持的 Provider](#支持的-provider)
- [快速开始](#快速开始)
- [配置与数据](#配置与数据)
- [架构](#架构)
- [开发](#开发)
- [项目定位](#项目定位)

## 工作方式

CC Switch UI 将“直接 Provider”和“本地路由”作为两个彼此独立的控制项：

| 模式 | Claude Code 实际使用的配置 | 适用场景 |
|------|----------------------------|----------|
| **直接配置** | 将所选 Provider 的端点与凭证写入 `~/.claude/settings.json` | 不经过本地代理，快速切换 Provider |
| **本地路由** | Claude Code 连接本地代理，再由代理转发到路由目标 | 协议转换、请求日志、熔断或故障转移 |

修改路由目标不会覆盖当前选择的直接 Provider；停止本地路由后，系统会恢复一致的直接配置。

## 功能

- **Provider 管理：** 内置预设、自定义兼容端点、端点检测、模型发现和一键切换。
- **本地路由：** Anthropic、OpenAI Chat、OpenAI Responses 与 Gemini 适配器，支持流式转换、熔断器和故障转移队列。
- **OAuth 账号：** Codex 与 GitHub Copilot 设备码授权及多账号管理，并支持 GHES。
- **MCP Server：** CRUD、导入、独立启停，以及合并式同步到 `~/.claude.json`。
- **Claude Code Skills：** CRUD、集合管理、导入，并从 `~/.cc-switch/skills/` 同步到 `~/.claude/skills/`。
- **用量监控：** 代理请求日志及可选的 Claude Code 本地会话 JSONL 导入，提供来源拆分和趋势图。
- **实时配置：** 采用合并式写入，保留 Claude Code 配置文件中的无关字段。

## 支持的 Provider

UI 内置 7 个持续维护的预设，同时支持自定义兼容端点。

| Provider | 类型 | 认证方式 | 说明 |
|----------|------|----------|------|
| DeepSeek | 官方 | API Key | DeepSeek 对话与推理模型 |
| Codex | OpenAI | OAuth | ChatGPT Plus/Pro 订阅 |
| MiniMax | 官方 | API Key | MiniMax 模型 |
| SiliconFlow | 聚合平台 | API Key | 多模型目录 |
| OpenRouter | 聚合平台 | API Key | 多供应商模型目录 |
| [OrcaRouter](https://www.orcarouter.ai/ref/ref_c975e760c319b5162c21) | 聚合平台 | API Key | Anthropic 原生接口与多供应商模型目录 |
| Gemini Native | Google | API Key | Gemini 原生 API 格式 |

使用 OrcaRouter 时，可通过[项目专属链接注册或获取 API Key](https://www.orcarouter.ai/ref/ref_c975e760c319b5162c21)，然后选择内置的 **OrcaRouter** 预设。该预设可以从服务端获取当前模型列表。

自定义 Provider 可选择 `anthropic`、`openai_chat`、`openai_responses` 或 Gemini 兼容协议。面对未知端点时，可在 Provider 编辑器中使用**检测端点类型**与**获取模型**。

## 快速开始

### 1. 在 Linux 或 macOS 上安装

```bash
curl -fsSL https://raw.githubusercontent.com/huangbogeng/cc-switch-ui/main/install.sh | bash
```

安装脚本默认将程序放在 `~/.local/share/cc-switch-ui`，用户数据保存在 `~/.cc-switch`。

其他平台可从 [GitHub Releases](https://github.com/huangbogeng/cc-switch-ui/releases) 下载构建产物，或按照后文从源码构建。

### 2. 启动服务

```bash
# 默认：管理界面监听 127.0.0.1:5007，本地代理监听 15721
cc-switch-ui start
```

打开 [http://localhost:5007/ui](http://localhost:5007/ui)，使用启动日志中打印的 admin token 登录。

常用 CLI 命令：

```bash
cc-switch-ui status   # 检查服务状态
cc-switch-ui doctor   # 诊断安装、PATH 与权限问题
cc-switch-ui version  # 查看安装版本
cc-switch-ui stop     # 停止托管服务
```

如果确实需要从其他设备访问，请显式开放监听：

```bash
cc-switch-ui start --host 0.0.0.0 --port 5007 --proxy-port 15721
```

### 3. 添加并切换 Provider

1. 打开 **Providers** 页面。
2. 选择预设，或新建自定义 Provider。
3. 填写 API Key，或完成 OAuth 授权。
4. 保存后点击**切换**。

Claude Code 会立即使用所选的直接 Provider。

### 4. 可选：启用本地路由

1. 在 **Providers** 页面的**本地路由**区域选择**路由目标**。
2. 启动本地路由。

此后 Claude Code 的请求会先进入本地代理，再经过适配后转发到路由目标。停止路由即可回到直接配置。

## 配置与数据

### 环境变量

| 变量 | 默认值 | 说明 |
|------|--------|------|
| `CC_SWITCH_ADMIN_TOKEN` | 启动时生成 | Web UI 与 API 的管理 token |
| `CC_SWITCH_PROXY_PORT` | `15721` | 本地代理监听端口 |
| `CC_SWITCH_UI_DIR` | 自动检测 | 前端构建产物所在目录 |

管理接口的 host 与 port 可通过 `cc-switch-ui start --host ... --port ...` 修改。CLI 配置会持久化到 `~/.cc-switch/cli.json`。

### 管理的路径

| 路径 | 用途 |
|------|------|
| `~/.cc-switch/cc-switch.db` | Provider、账号、路由、MCP、Skills 元数据与用量数据 |
| `~/.cc-switch/cli.json` | CLI host 与 port 配置 |
| `~/.cc-switch/skills/` | CC Switch UI 管理的 Skills 单一事实源 |
| `~/.claude/settings.json` | Claude Code 当前 Provider 配置 |
| `~/.claude.json` | Claude Code MCP Server 配置 |
| `~/.claude/skills/` | 已启用的 Claude Code Skills |

迁移或移除安装前，请备份 `~/.cc-switch`。API Key 与 OAuth 凭证属于敏感数据。

## 架构

```mermaid
flowchart LR
    Browser[React Web UI] -->|REST /api| Server[Axum Server]
    Server --> Core[cc-switch-lib]
    Core --> DB[(SQLite)]
    Core --> Config[Claude 配置文件]
    Claude[Claude Code] -->|直接模式| Provider[Provider API]
    Claude -->|本地路由模式| Proxy[本地代理]
    Proxy -->|适配、流式转换、故障转移| Provider
```

Rust workspace 包含三个 crate：

- `cc-switch-cli`：面向安装用户的命令行入口与服务生命周期管理。
- `cc-switch-server`：REST API、OAuth 回调、前端静态资源托管和本地代理。
- `cc-switch-lib`：持久化、实时配置、MCP/Skills 同步与 OAuth 核心逻辑。

前端位于 `cc-switch-ui/`，使用 React、TypeScript 与 Vite。

## 开发

环境要求：Node.js 24.15+、npm 与 Rust 1.85+（项目工具链由 `rust-toolchain.toml` 固定）。

### 从源码运行

```bash
# 终端 1：后端 API
cargo run -p cc-switch-server

# 终端 2：前端开发服务器
cd cc-switch-ui
npm ci
npm run dev
```

如需由 CLI 托管前端构建产物：

```bash
cd cc-switch-ui
npm ci
npm run build
cd ..
cargo run -p cc-switch-cli -- start
```

### 质量检查

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace

cd cc-switch-ui
npm test
npm run lint
npm run build
```

### 仓库结构

```text
cc-switch-cli/       CLI 与服务生命周期
cc-switch-lib/       共享领域逻辑与 SQLite 持久化
cc-switch-server/    REST API、OAuth 处理与本地代理
cc-switch-ui/        React 前端
install.sh           Linux/macOS 安装脚本
```

修改 Provider 或代理行为时，应将协议特定逻辑限制在适配器边界内，保证切换/启动/停止过程中的实时配置一致性，并根据影响范围覆盖请求、响应、流式传输、用量与失败行为测试。

提交修改前请阅读 [CONTRIBUTING.md](CONTRIBUTING.md) 与 [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md)。

## 项目定位

| 能力 | cc-switch | CC Switch UI |
|------|-----------|--------------|
| 部署方式 | Tauri 桌面应用 | 浏览器 UI 与无头 Web 服务 |
| 系统托盘 | 支持 | 不支持 |
| 工具定位 | 多种 AI 编程工具 | Claude Code CLI |
| MCP 与 Skills | 支持 | 支持，聚焦 Claude Code |
| 云同步 | 支持 | 不支持 |
| OAuth 账号 | 多 Provider | Codex 与 GitHub Copilot 多账号 |

## 致谢

CC Switch UI 基于开源项目 [cc-switch](https://github.com/farion1231/cc-switch) 开发，感谢原项目维护者与贡献者。

## License

本项目基于 [MIT License](LICENSE) 开源。

### MCP 与 Skills 同步保护

保存或启停托管的 MCP/Skill 会同步应用到 Claude Code。记录已保存但同步失败时，API 返回 `saved: true`；修复错误原因后点击 **Sync** 重试。一次同步可能在处理后续 Skill 失败前已经应用前面的条目，重试会继续应用期望状态。

同步保留尚未导入或添加到 CC Switch 的 MCP 条目和 Skill 目录。禁用、删除托管 Skill 或替换已有安装时，旧目录会移入 `~/.claude/skills/.cc-switch-backups/`，备份需要手动清理。恢复时先禁用对应的托管记录，再把备份复制回原 Skill 目录。删除记录会保留 `~/.cc-switch/skills/` 中的源文件。Skill 目录只能使用单个目录名；托管目录或源内容包含符号链接时会拒绝操作，不会跟随链接。

Skills 页面支持加载和恢复备份，包括已经删除的 Skill。已有记录保持原来的启停状态；已删除的记录会重建并启用。恢复会保留所选备份，并备份当前源文件。内容相同的同步不会重复生成备份，检测到外部修改时会停止替换。首写快照和冲突检测的边界参见[配置保护与恢复说明](docs/configuration-recovery.md)。

### 数据库升级保护

新版首次打开旧数据库（包括没有 schema 版本号的旧库）时，会先使用 SQLite Backup API 创建包含已提交 WAL 数据的快照，保存到 `~/.cc-switch/backups/upgrades/schema-<旧版本>-to-<新版本>-<UUID>.db`。Unix 上快照权限为 `0600`。备份失败则停止启动，原数据库不进行迁移；所有建表、兼容迁移与版本号更新在同一个事务中提交，失败会回滚。再次正常启动不会重复生成升级快照。新程序拒绝打开高于其支持版本的数据库。

这些快照保留旧 schema，独立于 Settings 的手动数据库备份，不会自动删除。需要降级时先停止所有 CC Switch 服务，备份当前数据，再恢复与旧程序匹配的数据库快照；不要在运行中的数据库上复制文件。OAuth、Skills 源文件与 CLI 设置不包含在数据库快照中，应一并备份整个 `~/.cc-switch`。升级保护针对 CC Switch UI 的旧发布版，并不意味着可以直接共享上游桌面 CC Switch 的数据库。

### 发布安装包升级与回滚（Linux/macOS）

重新运行安装脚本即可升级。脚本先下载新包及 `.sha256` 校验文件，校验版本和包内容，通过后才停止来自当前安装目录的服务。数据库、OAuth、CLI 配置与 Skills 源文件一起备份，旧程序和前端一起保留在安装目录同级的 `cc-switch-upgrade-backups/<版本>.<随机后缀>/`。这些副本含凭证，默认私有且不会自动删除。

新版使用原启动参数（包括显式 host、port、proxy-port、admin-token 和 ui-dir），或已有 CLI 默认配置与当前 shell 的环境变量启动。健康检查必须确认新进程已成功绑定端口，然后检查 `/health`。失败会停止新进程，保留失败现场，恢复旧程序及升级前数据，并重新启动原来运行的服务。原服务没有运行时，验证完成后仍保持停止。已有账号和 Skills 不需要重新导入。

如果旧 token 仅由旧进程随机生成，或者仅来自启动旧服务时的另一个 shell 环境，安装脚本无法恢复该 token；重启后需要使用私有 `startup.log` 中显示的新 token 登录。显式 `--admin-token` 或当前 shell 导出的 `CC_SWITCH_ADMIN_TOKEN` 可保持登录 token。无法可靠识别的启动参数、其他安装目录的服务、符号链接目录和已有升级锁会在替换前拒绝升级。自动流程不能代替所有外部服务的停服协调。

新安装脚本需要下一次发布提供的校验文件；历史发布包缺少校验文件时会安全退出，不会覆盖旧安装。Windows 仍使用手动安装包，此自动升级脚本不适用。升级备份、数据库迁移快照均需用户自行保管和清理。

### 命令行更新

安装包含本功能的新版本后，后续更新只需运行：

```bash
cc-switch-ui update
```

命令使用二进制内置的升级流程，更新当前程序所在的发布安装目录，无需再复制安装命令。已经是最新稳定版本时，不会停服、备份或替换文件；需要修复当前安装时可运行 `cc-switch-ui update --force` 强制重装，仍执行完整校验、备份和失败回滚。源码构建目录不能原地更新。Linux/macOS 支持此命令，Windows 会提示手动更新。旧版本没有 `update` 命令，需要先通过安装脚本升级一次。
