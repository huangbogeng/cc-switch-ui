# Proxy Routing Gap Analysis (Task 0.1)

## Scope

- Current repo: `/home/huangbogeng/github.com/huangbogeng/cc-switch-ui`
- Current code inspected:
  - `cc-switch-web/src/proxy/handlers.rs`
  - `cc-switch-web/src/proxy/forwarder.rs`
  - `cc-switch-web/src/proxy/types.rs`
  - `cc-switch-web/src/proxy/server.rs`
  - `cc-switch-lib/src/providers/*.rs`
- Upstream reference (reproducible):
  - Repository: `https://github.com/farion1231/cc-switch`
  - Pinned baseline SHA used in this analysis: `b05be92aa1928787f9d5a904c51d7b32867cd65c`
  - Path root: `src-tauri/src/proxy/`
  - Focus files:
    - `handlers.rs`
    - `handler_context.rs`
    - `forwarder.rs`
    - `provider_router.rs`
    - `circuit_breaker.rs`
    - `failover_switch.rs`
    - `types.rs`

### Upstream Pinning Guidance

Use these commands to reproduce the same upstream baseline before implementation:

```bash
git clone https://github.com/farion1231/cc-switch /tmp/cc-switch-upstream
git -C /tmp/cc-switch-upstream checkout b05be92aa1928787f9d5a904c51d7b32867cd65c
```

Or browse exact files at commit URL form:

```text
https://github.com/farion1231/cc-switch/blob/b05be92aa1928787f9d5a904c51d7b32867cd65c/src-tauri/src/proxy/forwarder.rs
```

---

## 1) Current Flow in This Repo (as-is)

### 1.1 Control-plane (start/select target)

```text
HTTP API
  └─ proxy_start (handlers.rs)
      ├─ get_active_target_provider() from DB
      ├─ create_registry() and find_for_provider(provider)
      ├─ build ProxyConfig
      │   ├─ fixed upstream_url = https://chatgpt.com/backend-api/codex/responses
      │   ├─ optional global http_proxy_url
      │   ├─ prompt_cache_key / fallback / fast_mode / model_mapping
      └─ ProxyServer::start(..., provider_id, app_type="claude")
```

### 1.2 Data-plane (request path)

```text
Incoming request to local proxy
  └─ ProxyServer router (/v1/*axum)
      └─ handle_proxy()
          └─ Forwarder::forward(state, req)
              ├─ adapter = pre-selected once at server start (single provider)
              ├─ adapter.get_auth_info()
              ├─ read+parse JSON body
              ├─ apply_model_mapping()
              ├─ build prompt_cache_key
              ├─ adapter.transform_request()
              ├─ send one upstream HTTP request (reqwest)
              ├─ if stream:
              │   ├─ choose stream format by adapter.streaming_response_format()
              │   └─ optionally extract/save usage from stream
              └─ else:
                  ├─ adapter.transform_response()
                  └─ save usage record (if present)
```

### 1.3 Contract breakpoint summary (current)

- Routing decision is made once at proxy startup, not per request.
- No provider chain in request execution path (single selected provider only).
- No retry/failover loop in `Forwarder::forward`.
- No circuit breaker gating before request.
- No failover switch callback to synchronize effective provider when fallback succeeds.

---

## 2) Upstream Target Flow (strict alignment target)

Based on `farion1231/cc-switch@b05be92` under `src-tauri/src/proxy`.

```text
Request enters handler (messages/chat/responses/gemini...)
  └─ RequestContext::new(...)
      ├─ load AppProxyConfig (per app)
      ├─ provider_router.select_providers(app_type)
      │   ├─ auto_failover_enabled=false: current provider only
      │   └─ auto_failover_enabled=true: failover queue ordered providers
      │       filtered by circuit breaker availability
      └─ create_forwarder(shared ProviderRouter + FailoverSwitchManager + timeouts)

  └─ RequestForwarder::forward_with_retry(..., providers)
      └─ for each provider in chain:
          ├─ router.allow_provider_request(provider, app_type)
          │   (HalfOpen permit aware)
          ├─ forward once to this provider
          ├─ on success:
          │   ├─ router.record_result(..., success=true)
          │   ├─ update runtime status/failover_count
          │   └─ failover_manager.try_switch(...) when provider changed
          └─ on failure:
              ├─ classify retryability / special rectifier path
              ├─ router.record_result(..., success=false) or neutral permit release
              └─ continue next provider in chain

Result:
  handler returns provider-specific transformed response
  with routing semantics = ProviderRouter + retry/failover + circuit-breaker state
```

Key semantics to preserve:
- Shared `ProviderRouter` state across requests.
- Circuit breaker lifecycle (`Closed/Open/HalfOpen`) with permit handling.
- Failover queue ordering from DB.
- Retry/failover performed in forwarder layer, not startup layer.

---

## 3) Gap List vs Upstream (must-land items)

### 3.1 P0 (Phase 1 hard requirements)

1. Missing per-request ProviderRouter selection
- Current contract point:
  - `cc-switch-web/src/proxy/server.rs` binds one `Provider` into `ProxyState::new(...)` at start.
  - `cc-switch-web/src/proxy/forwarder.rs` reads `state.provider` as a single immutable route target.
- Target contract (module/interface):
  - Add `cc-switch-web/src/proxy/provider_router.rs` with:
    - `ProviderRouter::select_providers(app_type: &str) -> Result<Vec<Provider>, ...>`
    - `ProviderRouter::allow_provider_request(...)`
    - `ProviderRouter::record_result(...)`
- Impacted files:
  - `cc-switch-web/src/proxy/server.rs`
  - `cc-switch-web/src/proxy/forwarder.rs`
  - `cc-switch-web/src/proxy/handlers.rs`
  - `cc-switch-web/src/proxy/provider_router.rs` (new)

2. Missing retry/failover execution contract
- Current contract point:
  - `Forwarder::forward(...) -> Result<Response, StatusCode>` performs one upstream attempt.
- Target contract (module/interface):
  - Introduce retry/failover entry:
    - `Forwarder::forward_with_retry(..., providers: Vec<Provider>) -> Result<..., ...>`
  - Keep per-provider send logic in a single-attempt internal helper.
- Impacted files:
  - `cc-switch-web/src/proxy/forwarder.rs`
  - `cc-switch-web/src/proxy/types.rs`
  - `cc-switch-web/src/proxy/server.rs`

3. Missing circuit breaker gate + state recording
- Current contract point:
  - No breaker state object and no pre-request gating.
- Target contract (module/interface):
  - Add `cc-switch-web/src/proxy/circuit_breaker.rs` with:
    - `CircuitBreaker`, `CircuitState`, `AllowResult`
    - `allow_request`, `record_success`, `record_failure`, `release_half_open_permit`
  - Wire via `ProviderRouter` in `forwarder` request loop.
- Impacted files:
  - `cc-switch-web/src/proxy/forwarder.rs`
  - `cc-switch-web/src/proxy/circuit_breaker.rs` (new)
  - `cc-switch-web/src/proxy/provider_router.rs` (new)
  - `cc-switch-web/src/proxy/mod.rs`

4. Missing failover switch side effect path
- Current contract point:
  - No interface to apply logical provider switch after fallback success.
- Target contract (module/interface):
  - Add `cc-switch-web/src/proxy/failover_switch.rs` with manager API equivalent to:
    - `FailoverSwitchManager::try_switch(app_type, provider_id, provider_name)`
- Impacted files:
  - `cc-switch-web/src/proxy/forwarder.rs`
  - `cc-switch-web/src/proxy/failover_switch.rs` (new)
  - app state boundary module where provider target is persisted/emitted

### 3.2 P1 (important for behavioral parity)

1. Route surface mismatch
- Current: `cc-switch-web/src/proxy/server.rs` router only defines `/v1/*axum` and `/health`.
- Target: add explicit handlers/routes compatible with upstream endpoint set.
- Concrete interfaces:
  - `handle_messages`, `handle_chat_completions`, `handle_responses`, `handle_gemini`-style split.
- Impacted files:
  - `cc-switch-web/src/proxy/server.rs`
  - `cc-switch-web/src/proxy/handlers.rs`

2. App-scoped routing config contract missing in local proxy types
- Current: `cc-switch-web/src/proxy/types.rs::ProxyConfig` is transport-only.
- Target: app-scope routing/failover contract fields equivalent to upstream `AppProxyConfig`:
  - `auto_failover_enabled`
  - `max_retries`
  - `streaming_first_byte_timeout`
  - `streaming_idle_timeout`
  - `non_streaming_timeout`
  - circuit thresholds
- Impacted files:
  - `cc-switch-web/src/proxy/types.rs`
  - `cc-switch-web/src/proxy/handlers.rs`
  - DB/config access wiring used by proxy path

3. Adapter selection timing differs
- Current: adapter resolved once at proxy start via `create_registry().find_for_provider(...)`.
- Target: adapter/provider binding resolved at request execution time per candidate provider.
- Concrete interfaces:
  - keep `ProviderRegistry::find_for_provider(&Provider)` as lookup primitive
  - call it inside retry path, not only startup path
- Impacted files:
  - `cc-switch-web/src/proxy/server.rs`
  - `cc-switch-web/src/proxy/forwarder.rs`
  - `cc-switch-lib/src/providers/registry.rs`

### 3.3 P2 (defer-able, but tracked)

1. Advanced rectifier/optimizer parity not in local baseline
- Upstream includes dedicated modules for thinking rectifier and optimizer branches.
- Impacted files:
  - likely new modules under `cc-switch-web/src/proxy/` when enabled

2. Extended status/observability fields differ
- Upstream `types.rs::ProxyStatus` includes failover and richer runtime metrics.
- Local `ProxyStatus` in `cc-switch-web/src/proxy/types.rs` is minimal.
- Impacted files:
  - `cc-switch-web/src/proxy/types.rs`
  - `cc-switch-web/src/proxy/forwarder.rs`

---

## 4) Phase 1 Acceptance Criteria (strict, command-bound)

Phase 1 is accepted only when every AC below passes with command + assertion.

1. AC1: request-time routing is implemented
- Command:
```bash
rg -n "select_providers\(|forward_with_retry\(" cc-switch-web/src/proxy
```
- Expected assertion:
  - output contains at least one `select_providers(` call from request path and one `forward_with_retry(` entrypoint.

2. AC2: retry+failover loop works (provider A fail -> provider B success)
- Command:
```bash
cargo test -p cc-switch-web proxy::forwarder::tests::failover_to_next_provider_on_first_failure -- --exact
```
- Expected assertion:
  - test result contains `... ok` and summary contains `1 passed; 0 failed`.

3. AC3: circuit breaker state transitions are enforced
- Command:
```bash
cargo test -p cc-switch-web proxy::circuit_breaker::tests::half_open_permit_is_consumed_and_released -- --exact
```
- Expected assertion:
  - test result contains `... ok`; failure indicates permit leak or invalid transition.

4. AC4: provider result recording is wired per attempt
- Command:
```bash
cargo test -p cc-switch-web proxy::provider_router::tests::record_result_updates_provider_health -- --exact
```
- Expected assertion:
  - test result contains `... ok`; test verifies success/failure updates are persisted or observable via router state.

5. AC5: failover switch callback executes when fallback wins
- Command:
```bash
cargo test -p cc-switch-web proxy::forwarder::tests::invokes_failover_switch_on_fallback_success -- --exact
```
- Expected assertion:
  - test result contains `... ok`; test asserts switch manager mock/spying was called once with fallback provider id.

6. AC6: adapter transform contracts remain intact (stream + non-stream)
- Command:
```bash
cargo test -p cc-switch-web proxy::adapters:: -- --nocapture
```
- Expected assertion:
  - all adapter tests pass; no regression in request/response transform behavior.

7. AC7: single-provider mode has no regression
- Command:
```bash
cargo test -p cc-switch-web proxy::forwarder::tests::single_provider_no_failover_path -- --exact
```
- Expected assertion:
  - test result contains `... ok`; test verifies exactly one upstream attempt and no failover switch invocation.

Note:
- If the named tests do not exist yet, Phase 1 is not complete; creating these executable tests is part of the acceptance contract.

---

## 5) Suggested Implementation Order (for next task)

1. Introduce `provider_router` + `circuit_breaker` scaffolding (minimal API parity first).
2. Refactor `Forwarder` to support provider chain and `forward_with_retry`.
3. Move provider resolution from startup path to request path.
4. Add failover switch manager integration.
5. Expand route surface only after P0 routing semantics are stable.

This ordering enforces the prior decision: **strictly align with upstream routing semantics first (ProviderRouter + retry + circuit breaker + failover switch)**.
