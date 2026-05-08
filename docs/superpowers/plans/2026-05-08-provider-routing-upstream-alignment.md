# Provider Routing Upstream Alignment Implementation Plan

**Goal:** 对齐 upstream `cc-switch` 的 provider 代理路由语义，构建全 provider 的统一闭环（路由选择、重试、熔断、切换同步、usage/request-log），并修复 MiniMax 只是该闭环内的子问题。

**Scope:** `cc-switch-web` + `cc-switch-lib` 后端代理链路；不包含新增 UI 功能。

**Out of Scope:** 前端交互重设计、非代理模块重构。

---

## Phase 0 - Baseline & Contracts

### Task 0.1: 明确当前入口与契约断点
- Files:
  - `cc-switch-web/src/proxy/handlers.rs`
  - `cc-switch-web/src/proxy/forwarder.rs`
  - `cc-switch-web/src/proxy/types.rs`
  - `cc-switch-lib/src/providers/*.rs`
- Deliverables:
  - 当前请求路径图（handler -> forwarder -> adapter -> upstream）
  - 与 upstream 语义差异清单（必须落地项）

### Task 0.2: 锁定测试基线
- Commands:
  - `cargo test -p cc-switch-web`
  - `cargo test -p cc-switch-lib`
- Deliverables:
  - 当前通过/失败测试快照
  - MiniMax/tool-call 相关失败样例归档

---

## Phase 1 - Router + Breaker + Retry Skeleton

### Task 1.1: 引入 ProviderRouter（按 app_type 选择候选 provider）
- Create:
  - `cc-switch-web/src/proxy/provider_router.rs`
- Modify:
  - `cc-switch-web/src/proxy/mod.rs`
  - `cc-switch-web/src/proxy/server.rs`
- Requirements:
  - 支持 `auto_failover_enabled` 开关语义
  - failover 开启：按队列顺序返回候选 provider
  - failover 关闭：仅返回 current provider

### Task 1.2: 引入 CircuitBreaker（key=`app_type:provider_id`）
- Create:
  - `cc-switch-web/src/proxy/circuit_breaker.rs`
- Modify:
  - `cc-switch-web/src/proxy/provider_router.rs`
- Requirements:
  - 状态：Closed/Open/HalfOpen
  - `allow_request()` + `record_success()/record_failure()`
  - 与 provider health 更新点预留对接

### Task 1.3: 重构 Forwarder 为 `forward_with_retry`
- Modify:
  - `cc-switch-web/src/proxy/forwarder.rs`
- Requirements:
  - 以候选 provider 列表循环尝试
  - 每次失败记录 breaker 结果
  - 成功后立即返回，并记录实际 provider
  - 保留 adapter 驱动的 transform/auth/upstream 逻辑

### Task 1.4: Handler 统一走 Router + Retry
- Modify:
  - `cc-switch-web/src/proxy/handlers.rs`
- Requirements:
  - handler 不再硬编码 provider 判定
  - 所有代理请求统一走 `router.select_providers -> forward_with_retry`

### Verification (Phase 1)
- `cargo test -p cc-switch-web proxy`
- 新增单测覆盖：
  - failover 开关开/关的候选 provider 选择
  - breaker 打开时 provider 跳过
  - 单 provider 场景不被错误熔断阻塞

---

## Phase 2 - Failover Switch Sync + Request Log Semantics

### Task 2.1: 引入 FailoverSwitchManager
- Create:
  - `cc-switch-web/src/proxy/failover_switch.rs`
- Modify:
  - `cc-switch-web/src/proxy/forwarder.rs`
  - `cc-switch-web/src/proxy/server.rs`
- Requirements:
  - 去重切换（避免并发重复切换）
  - 成功切换后同步当前 provider 状态
  - 事件发射接口预留（与现有状态机制兼容）

### Task 2.2: 对齐 request-log/usage 记录点
- Modify:
  - `cc-switch-web/src/proxy/forwarder.rs`
  - `cc-switch-lib/src/database/mod.rs`（如需补字段或接口）
- Requirements:
  - 请求级记录与 usage 记录分层
  - 失败请求可观测（状态码/错误原因）
  - 成功请求记录 provider/model/token 基础信息

### Verification (Phase 2)
- 新增集成测试：
  - 第一 provider 失败、第二 provider 成功时的切换与日志
  - 切换去重（并发请求）

---

## Phase 3 - Streaming Contract Hardening (MiniMax Included)

### Task 3.1: 统一流式 message/tool stop 语义
- Modify:
  - `cc-switch-web/src/proxy/streaming_responses.rs`
  - `cc-switch-web/src/proxy/adapters/minimax/response.rs`
- Requirements:
  - tool block start/delta/stop 成对闭合
  - `finish_reason=tool_calls/function_call` 仅在有效 tool block 时映射 `tool_use`
  - 非法 tool 序列降级为 `end_turn`

### Task 3.2: 统一 [DONE] 收尾与 usage flush
- Modify:
  - `cc-switch-web/src/proxy/streaming_responses.rs`
  - `cc-switch-web/src/proxy/forwarder.rs`
- Requirements:
  - [DONE] 前后不会重复 `message_stop`
  - usage 可以晚到并在结束时 flush
  - 记录点与非流式路径一致

### Task 3.3: 补齐 adapter 契约测试（全 provider 最小覆盖）
- Modify:
  - `cc-switch-web/src/proxy/adapters/*/request.rs`
  - `cc-switch-web/src/proxy/adapters/*/response.rs`
- Requirements:
  - Anthropic/OpenAI/Gemini/Codex/Copilot/MiniMax 至少各 1 条请求转换 + 1 条响应转换断言

### Verification (Phase 3)
- `cargo test -p cc-switch-web minimax -- --nocapture`
- `cargo test -p cc-switch-web streaming_responses -- --nocapture`
- `cargo test -p cc-switch-web adapters -- --nocapture`

---

## Rollout Strategy

1. 先合入 Phase 1（主链建立）
2. 再合入 Phase 2（切换与日志语义）
3. 最后合入 Phase 3（流式契约强化）

每个 Phase 独立通过测试后再进入下一阶段，避免大批量不可回滚改动。

---

## Risk Control

- 风险 1：重试链与现有 handler 状态机冲突
  - 缓解：先在单 provider 场景回归，确保行为不变
- 风险 2：流式事件顺序回归
  - 缓解：事件序列断言测试（message_start/content_block_*/message_stop）
- 风险 3：usage 统计重复或漏记
  - 缓解：stream/non-stream 双路径幂等性测试

---

## Done Criteria

- 所有 provider 请求都经 `router.select_providers + forward_with_retry`
- failover 和 breaker 行为符合 upstream 语义
- MiniMax tool 调用在代理流式链路可稳定执行
- request-log/usage 具备请求级可观测性
- 关键测试套件通过且无新增高优先级回归

---

## Execution Status (2026-05-08)

- `Phase 1` completed:
  - `ProviderRouter` + `CircuitBreaker` introduced
  - request-time `forward_with_retry` wired
  - handlers/server path unified to runtime routing
- `Phase 2` completed:
  - `FailoverSwitchManager` introduced and wired
  - request-level log persistence added (`proxy_request_logs`)
  - request-log query API added (`/api/usage/request-logs`)
- `Phase 3` completed:
  - MiniMax streaming/tool-call guard semantics hardened
  - duplicate finish/[DONE] stability checks covered
  - adapter contract tests expanded (Claude/Gemini/Copilot/Codex)

Latest validation snapshot:
- `cargo check -p cc-switch-web` PASS
- `cargo test -p cc-switch-web` PASS (`50 passed`)
- `cargo test -p cc-switch-lib` PASS (`42 passed`)
