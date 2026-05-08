# MiniMax Routing Parity Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Align `cc-switch-ui` / `cc-switch-web` MiniMax routing behavior with upstream `farion1231/cc-switch` for route entry coverage, provider selection/failover policy, and request-path compatibility.

**Architecture:** Expand proxy entry routes first so MiniMax-compatible clients can hit canonical endpoints (`/chat/completions`, `/v1/chat/completions`, `/responses`). Then upgrade provider routing from static in-memory toggles to DB-driven app-scoped policy. Finally harden MiniMax URL/path construction and add integration tests across routing + failover + usage recording boundaries.

**Tech Stack:** Rust (`axum`, `tokio`), existing `cc-switch-web` proxy modules, SQLite-backed provider config via `cc_switch_lib::database`, cargo test.

---

## File Structure

- Modify: `cc-switch-web/src/proxy/server.rs`
  - Responsibility: expose proxy HTTP route matrix and wire request handlers to runtime state.
- Modify: `cc-switch-web/src/proxy/handlers.rs`
  - Responsibility: normalize incoming paths and dispatch by client format.
- Modify: `cc-switch-web/src/proxy/session.rs`
  - Responsibility: classify request format from incoming path for downstream adapters/usage.
- Modify: `cc-switch-web/src/proxy/provider_router.rs`
  - Responsibility: provider candidate selection, app-scoped failover policy, circuit-breaker gating.
- Modify: `cc-switch-web/src/proxy/forwarder.rs`
  - Responsibility: consume ordered provider candidates and report success/failure back to router.
- Modify: `cc-switch-web/src/proxy/adapters/minimax/mod.rs`
  - Responsibility: MiniMax base URL extraction and final upstream URL building.
- Modify: `cc-switch-web/src/proxy/adapters/minimax/request.rs`
  - Responsibility: request normalization required by MiniMax OpenAI-compatible endpoints.
- Modify: `cc-switch-web/src/handlers/settings.rs`
  - Responsibility: expose/read app-scoped failover and circuit-breaker config used by router.
- Modify: `cc-switch-web/src/handlers/providers.rs`
  - Responsibility: keep proxy target + current provider semantics coherent with routing selection.
- Test: `cc-switch-web/src/proxy/server.rs` (unit tests)
- Test: `cc-switch-web/src/proxy/provider_router.rs` (unit tests)
- Test: `cc-switch-web/src/proxy/adapters/minimax/mod.rs` (unit tests)
- Create: `cc-switch-web/tests/minimax_routing_parity.rs`
  - Responsibility: black-box integration tests for MiniMax routing parity scenarios.
- Modify: `docs/superpowers/plans/2026-05-08-minimax-routing-parity-plan.md` (checklist status only during execution)

### Task 1: Expand Proxy Route Entry Matrix

**Files:**
- Modify: `cc-switch-web/src/proxy/server.rs`
- Modify: `cc-switch-web/src/proxy/handlers.rs`
- Modify: `cc-switch-web/src/proxy/session.rs`
- Test: `cc-switch-web/src/proxy/server.rs`

- [ ] **Step 1: Write the failing tests for route coverage**

```rust
#[tokio::test]
async fn proxy_accepts_openai_chat_paths() {
    // assert routes exist for /chat/completions and /v1/chat/completions
}

#[tokio::test]
async fn proxy_accepts_responses_paths() {
    // assert routes exist for /responses and /v1/responses
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p cc-switch-web proxy_accepts_openai_chat_paths proxy_accepts_responses_paths -- --nocapture`
Expected: FAIL with 404/route-not-found assertions.

- [ ] **Step 3: Write minimal implementation in server router**

```rust
Router::new()
    .route("/v1/*axum", get(handle_proxy).post(handle_proxy))
    .route("/chat/completions", post(handle_proxy))
    .route("/v1/chat/completions", post(handle_proxy))
    .route("/responses", post(handle_proxy))
    .route("/v1/responses", post(handle_proxy))
```

- [ ] **Step 4: Update request format classification**

```rust
if path.contains("/responses") {
    ClientFormat::Responses
} else if path.contains("/chat/completions") {
    ClientFormat::OpenAIChat
} else {
    ClientFormat::Anthropic
}
```

- [ ] **Step 5: Run tests to verify pass**

Run: `cargo test -p cc-switch-web proxy_accepts_openai_chat_paths proxy_accepts_responses_paths -- --nocapture`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add cc-switch-web/src/proxy/server.rs cc-switch-web/src/proxy/handlers.rs cc-switch-web/src/proxy/session.rs
git commit -m "feat(proxy): add openai/responses route entry coverage"
```

### Task 2: Upgrade ProviderRouter to App-Scoped DB-Driven Policy

**Files:**
- Modify: `cc-switch-web/src/proxy/provider_router.rs`
- Modify: `cc-switch-web/src/proxy/server.rs`
- Modify: `cc-switch-web/src/proxy/forwarder.rs`
- Test: `cc-switch-web/src/proxy/provider_router.rs`

- [ ] **Step 1: Write failing tests for app-scoped failover behavior**

```rust
#[tokio::test]
async fn router_uses_db_failover_switch_per_app_type() {
    // when app_type=claude failover on, returns queue order
    // when app_type=codex failover off, returns current only
}

#[tokio::test]
async fn router_returns_error_when_all_candidates_circuit_open() {
    // all in queue but all breakers open => explicit error
}
```

- [ ] **Step 2: Run tests to verify failure**

Run: `cargo test -p cc-switch-web router_uses_db_failover_switch_per_app_type router_returns_error_when_all_candidates_circuit_open -- --nocapture`
Expected: FAIL due to missing DB-driven selection/error branch.

- [ ] **Step 3: Implement router interface and selection contract**

```rust
pub async fn select_providers(
    &self,
    app_type: &str,
    current_provider: &Provider,
    providers: &HashMap<String, Provider>,
) -> Result<Vec<Provider>, RouterSelectionError>
```

```rust
match auto_failover_enabled_from_db(app_type).await {
    true => select_queue_candidates(app_type, providers).await,
    false => Ok(vec![current_provider.clone()]),
}
```

- [ ] **Step 4: Wire server/forwarder to new `Result` contract**

```rust
let provider_candidates = router.select_providers(...).await?;
```

```rust
Err(RouterSelectionError::AllCircuitOpen) => StatusCode::SERVICE_UNAVAILABLE
```

- [ ] **Step 5: Run target tests**

Run: `cargo test -p cc-switch-web provider_router -- --nocapture`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add cc-switch-web/src/proxy/provider_router.rs cc-switch-web/src/proxy/server.rs cc-switch-web/src/proxy/forwarder.rs
git commit -m "feat(proxy): make provider routing app-scoped and db-driven"
```

### Task 3: MiniMax URL/Endpoint Compatibility Hardening

**Files:**
- Modify: `cc-switch-web/src/proxy/adapters/minimax/mod.rs`
- Modify: `cc-switch-web/src/proxy/adapters/minimax/request.rs`
- Test: `cc-switch-web/src/proxy/adapters/minimax/mod.rs`

- [ ] **Step 1: Write failing URL-build tests for MiniMax**

```rust
#[test]
fn minimax_build_url_avoids_double_v1() {
    // base_url with /v1 + endpoint /v1/chat/completions => single /v1
}

#[test]
fn minimax_build_url_supports_prefix_and_full_endpoint_modes() {
    // base_url as prefix and full endpoint both succeed
}
```

- [ ] **Step 2: Run tests to verify failure**

Run: `cargo test -p cc-switch-web minimax_build_url_avoids_double_v1 minimax_build_url_supports_prefix_and_full_endpoint_modes -- --nocapture`
Expected: FAIL on duplicated path or incorrect endpoint mode.

- [ ] **Step 3: Implement deterministic URL builder**

```rust
fn build_url(base_url: &str, endpoint: &str) -> String {
    let base = base_url.trim_end_matches('/');
    let ep = endpoint.trim_start_matches('/');
    normalize_v1_segments(format!("{base}/{ep}"))
}
```

- [ ] **Step 4: Add request normalization guardrails**

```rust
if request_body.get("stream_options").is_none() {
    // preserve compatibility for providers returning usage on final chunk
}
```

- [ ] **Step 5: Run adapter tests**

Run: `cargo test -p cc-switch-web minimax -- --nocapture`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add cc-switch-web/src/proxy/adapters/minimax/mod.rs cc-switch-web/src/proxy/adapters/minimax/request.rs
git commit -m "fix(minimax): harden endpoint building and request compatibility"
```

### Task 4: End-to-End Parity Tests for Routing + Failover

**Files:**
- Create: `cc-switch-web/tests/minimax_routing_parity.rs`
- Modify: `cc-switch-web/src/handlers/providers.rs`
- Modify: `cc-switch-web/src/handlers/settings.rs`

- [ ] **Step 1: Write failing integration tests**

```rust
#[tokio::test]
async fn minimax_route_uses_proxy_target_then_current_then_hint() {
    // assert fallback chain for provider resolution
}

#[tokio::test]
async fn minimax_failover_tries_queue_order_and_stops_on_success() {
    // P1 fail -> P2 success, verify no P3 request
}

#[tokio::test]
async fn minimax_all_open_circuits_returns_503() {
    // explicit 503 when all candidates unavailable
}
```

- [ ] **Step 2: Run tests to verify failure**

Run: `cargo test -p cc-switch-web --test minimax_routing_parity -- --nocapture`
Expected: FAIL due to current routing behavior mismatch.

- [ ] **Step 3: Implement missing handler glue**

```rust
// providers.rs: ensure switch updates both current provider and proxy target coherently
state.db.set_current_provider(&id, APP_TYPE)?;
state.db.set_proxy_target_provider_id(&id)?;
```

```rust
// settings.rs: expose failover/circuit config fields consumed by router
```

- [ ] **Step 4: Run integration tests**

Run: `cargo test -p cc-switch-web --test minimax_routing_parity -- --nocapture`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add cc-switch-web/tests/minimax_routing_parity.rs cc-switch-web/src/handlers/providers.rs cc-switch-web/src/handlers/settings.rs
git commit -m "test(proxy): add minimax routing parity integration coverage"
```

### Task 5: Full Verification and Delivery Checklist

**Files:**
- Modify: `docs/superpowers/plans/2026-05-08-minimax-routing-parity-plan.md` (mark checkboxes only)

- [ ] **Step 1: Run format + lint + tests**

Run: `cargo fmt --all -- --check`
Expected: PASS.

Run: `cargo clippy -p cc-switch-web --all-targets -- -D warnings`
Expected: PASS.

Run: `cargo test -p cc-switch-web -- --nocapture`
Expected: PASS.

- [ ] **Step 2: Smoke run proxy server**

Run: `cargo run -p cc-switch-web`
Expected: server starts, health endpoint reachable, no startup panic.

- [ ] **Step 3: Final commit (if verification-only changes exist)**

```bash
git add -A
git commit -m "chore: finalize minimax routing parity verification"
```

- [ ] **Step 4: Summarize delivery**

```text
- Route entry matrix parity status
- Router policy parity status
- MiniMax compatibility parity status
- Remaining known gaps (if any)
```

## Self-Review

- Spec coverage check: plan covers route entry expansion, router policy parity, MiniMax URL/path compatibility, and end-to-end failover behavior.
- Placeholder scan: no `TODO/TBD/implement later` placeholders remain; every task has concrete files and commands.
- Type consistency: `select_providers` contract and route path names are used consistently across server/router/forwarder tasks.
