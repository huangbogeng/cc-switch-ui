## 调研发现

## 主题
原始 cc-switch 项目的 Usage 追踪实现，以及 OAuth Provider 的 usage 处理方式。

## 当前发现
- 已确认原始仓库 `farion1231/cc-switch` 默认分支为 `main`。
- 原始项目存在两类 Usage 能力：
  1. Provider 额度/余额查询：用于单个 provider 卡片展示，不进入统一请求统计库。
  2. 请求级 Usage 追踪：从代理响应或会话日志中提取 usage，写入 SQLite，再做汇总、趋势、provider/model 统计。
- 对当前纯 Web 版最值得优先迁移的是”请求级 Usage 追踪”主线，而不是先做 provider 配额脚本。

## 原始项目关键路径
### 前端
- `src/components/settings/SettingsPage.tsx`：Usage 页签入口
- `src/components/usage/UsageDashboard.tsx`：Usage 主面板
- `src/components/usage/*`：summary、trend、request log、provider/model 统计、pricing 面板
- `src/lib/api/usage.ts`：前端调用层
- `src/lib/query/usage.ts`：React Query hooks
- `src/types/usage.ts`：Usage 类型定义

### 后端 / 数据层
- `src-tauri/src/commands/usage.rs`：Usage dashboard 命令入口
- `src-tauri/src/commands/provider.rs`：Provider usage / quota 查询入口
- `src-tauri/src/services/usage_stats.rs`：统计查询、去重、limit 检查、cost 回填
- `src-tauri/src/database/schema.rs`：`proxy_request_logs`、`usage_daily_rollups`、`model_pricing`、`session_log_sync`
- `src-tauri/src/database/dao/usage_rollup.rs`：明细聚合到按日 rollup
- `src-tauri/src/proxy/usage/parser.rs`：多格式 usage 解析
- `src-tauri/src/proxy/usage/calculator.rs`：成本计算
- `src-tauri/src/proxy/usage/logger.rs`：统一写库
- `src-tauri/src/proxy/response_handler.rs` / `response_processor.rs`：代理响应处理链路
- `src-tauri/src/services/session_usage*.rs`：Claude / Codex / Gemini 会话日志导入
- `src-tauri/src/services/provider/usage.rs`：通用 usage script 执行

## 数据流结论
### 请求级 Usage 追踪主线
上游响应 → usage parser → cost calculator → usage logger → `proxy_request_logs` → summary / trend / provider / model 查询

### 核心存储设计
- `proxy_request_logs`：请求级明细，记录 provider、app_type、model、request_model、tokens、cost、latency、status、session_id、is_streaming、data_source 等。
- `usage_daily_rollups`：日粒度汇总，用于历史数据压缩与快速查询。
- `model_pricing`：模型单价配置。
- `session_log_sync`：会话日志导入进度。

### OAuth 与 Usage 的关系
- OAuth 主要负责拿 token / 刷 token / 注入认证头。
- OAuth 本身不承担 usage 聚合逻辑。
- 真正的 usage 记录仍统一在代理响应后处理链路中完成。
- 因此对 Codex / Copilot 这类 OAuth provider，只要请求经过代理且响应带 usage 字段，就可以和普通 provider 走同一条统计链路。

### Provider 配额查询是另一条链路
- GitHub Copilot 有单独 quota 查询入口，用于 provider 卡片配额展示。
- 通用 provider 还支持 usage script 查询余额/套餐等。
- 这类配额查询不等同于请求级 usage 持久化，不应与主统计链路混在一起。

## 对当前项目最值得迁移的设计
1. 统一请求级 usage 日志模型（先建 `proxy_request_logs`）
2. parser / calculator / logger 三层拆分
3. 保留 `request_model`、`pricing_model_source`、`cost_multiplier`
4. 明细 + 日汇总混合查询模型
5. Session log 导入可后置，不作为第一阶段必需能力

## 对当前项目的直接建议
### 第一阶段优先级
1. 在代理层记录请求级 usage
2. 支持 OAuth provider 走统一 usage 记录链路
3. 提供基础 summary / trend / provider / model 查询接口
4. 前端先做 Usage 概览页，再逐步补充明细

### 先不要优先做
- provider usage script / quota 查询
- session log 导入
- 复杂图表和多数据源去重

## 原项目 Proxy 架构（vs 当前项目）

### 原项目 Proxy 核心文件
```
src-tauri/src/proxy/
├── forwarder.rs           # 核心请求转发器
├── handler_context.rs     # 请求上下文管理
├── providers/
│   ├── mod.rs            # Provider 类型枚举和工厂
│   ├── adapter.rs        # ProviderAdapter trait
│   ├── claude.rs         # Claude 官方 API 适配器
│   ├── codex.rs          # OpenAI Codex 适配器
│   ├── gemini.rs         # Google Gemini 适配器
│   └── ...
├── usage/
│   ├── logger.rs         # 使用量记录器
│   └── parser.rs         # 响应解析器
├── failover_switch.rs    # 故障转移管理器
└── circuit_breaker.rs   # 熔断器
```

### 原项目 ProviderType 枚举
- `Claude` - Anthropic 官方 API（x-api-key）
- `ClaudeAuth` - 中转服务（仅 Bearer）
- `Codex` - OpenAI Codex
- `Gemini` - Google Gemini（x-goog-api-key）
- `OpenRouter` - OpenRouter
- `GitHubCopilot` - GitHub Copilot OAuth
- `CodexOAuth` - ChatGPT Plus/Pro OAuth

### 原项目请求转发流程
```
Client Request
     ↓
RequestContext::new()  ── 选择 Provider（支持故障转移链）
     ↓
ProviderRouter::select_providers()  ── 获取 Provider 列表（熔断器感知）
     ↓
RequestForwarder::forward_with_retry()
     ↓
for provider in providers {
    ├── get_adapter(app_type)  ── 获取对应 ProviderAdapter
    ├── adapter.transform_request()  ── 格式转换
    ├── http_client.request()  ── 发送请求
    ├── adapter.extract_usage()  ── 从响应提取 usage
    └── on failure: 尝试下一个 Provider
}
     ↓
UsageLogger::log_request()  ── 记录到 proxy_request_logs 表
```

### 当前 cc-switch-web 代理架构
```
cc-switch-web/src/proxy/
├── forwarder.rs        # Forwarder（仅 Codex OAuth）
├── handlers.rs         # API 路由处理器
├── server.rs           # ProxyServer
├── transform_responses.rs  # Anthropic→Codex Responses 格式转换
└── types.rs
```

### 关键架构差异

| 维度 | 原项目 cc-switch | 当前 cc-switch-web |
|------|-----------------|-------------------|
| **Provider 支持** | 多种类型（Claude/Gemini/Codex/Copilot/OpenRouter） | 仅 Codex OAuth |
| **认证方式** | 每种 Provider 独立 AuthStrategy | 硬编码 CodexOAuthManager |
| **故障转移** | FailoverSwitchManager 支持多 Provider 链 | 未实现 |
| **熔断器** | ProviderRouter + CircuitBreaker | 未实现 |
| **请求转换** | ProviderAdapter trait 多态实现 | 硬编码 anthropic_to_codex_responses |
| **Usage 来源** | 多格式解析器（Claude/Codex/Gemini/OpenRouter） | 仅 from_openai_json |
| **数据库表** | proxy_request_logs（完整字段） | usage_records（简化字段） |

### 当前项目的设计约束

1. **handlers.rs:42-48** - 代理启动时强制检查必须是 Codex OAuth provider
2. **handlers.rs:139-145** - 设置代理目标时同样限制
3. **forwarder.rs:83-99** - Token 获取仅支持 Codex OAuth
4. **forwarder.rs:235-241** - Usage 记录仅针对非流式响应

## 外部来源记录
- GitHub 仓库：`farion1231/cc-switch`
- 调研结论来自只读代码路径分析，不包含对外部仓库的执行性指令
