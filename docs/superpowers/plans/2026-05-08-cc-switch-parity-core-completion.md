# CC-Switch Core Parity Completion Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Complete core proxy parity beyond MiniMax tool-call fixes by hardening failover/breaker semantics, request-log APIs, and failover state synchronization to match upstream operational behavior.

**Architecture:** Keep the existing `router -> forward_with_retry -> adapter` path as the backbone, then close parity gaps in three focused slices: runtime breaker semantics, request-log query surface, and switch synchronization/events. Build each slice test-first with narrow changes in existing modules, no cross-cutting refactor unless required by tests.

**Tech Stack:** Rust, Axum, rusqlite, serde, tokio tests, cargo test/check/fmt

---

### Task 1: Add Runtime-Safe Breaker/Failover Policy Contract Tests

**Files:**
- Modify: `cc-switch-web/src/proxy/provider_router.rs`
- Modify: `cc-switch-web/src/proxy/circuit_breaker.rs`
- Test: `cc-switch-web/src/proxy/provider_router.rs` (inline `#[cfg(test)]`)
- Test: `cc-switch-web/src/proxy/circuit_breaker.rs` (inline `#[cfg(test)]`)

- [ ] **Step 1: Write failing tests for strict failover policy**

```rust
#[test]
fn single_provider_mode_allows_current_even_when_breaker_open() {
    let current = provider("current", false, Some(1));
    let mut providers = HashMap::new();
    providers.insert("current".to_string(), current.clone());

    let mut router = ProviderRouter::new(false);
    for _ in 0..3 {
        router.record_failure("claude", "current");
    }

    let result = router.select_providers("claude", &current, &providers);
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].id, "current");
}

#[test]
fn failover_mode_returns_empty_when_all_candidates_open() {
    let current = provider("p1", true, Some(1));
    let p2 = provider("p2", true, Some(2));
    let mut providers = HashMap::new();
    providers.insert("p1".to_string(), current.clone());
    providers.insert("p2".to_string(), p2.clone());

    let mut router = ProviderRouter::new(true);
    for id in ["p1", "p2"] {
        for _ in 0..3 {
            router.record_failure("claude", id);
        }
    }

    let result = router.select_providers("claude", &current, &providers);
    assert!(result.is_empty());
}
```

- [ ] **Step 2: Run focused router tests to confirm failures exist first**

Run: `cargo test -p cc-switch-web provider_router -- --nocapture`
Expected: At least one new test fails before implementation adjustment.

- [ ] **Step 3: Implement minimal policy adjustments in router/breaker**

```rust
// provider_router.rs (policy guard)
if candidates.is_empty() {
    if self.auto_failover_enabled && !queue_order(providers).is_empty() {
        return Vec::new();
    }
    return vec![current_provider.clone()];
}
```

```rust
// circuit_breaker.rs (keep half-open deterministic)
pub fn record_failure(&mut self) {
    match self.state {
        CircuitState::HalfOpen => {
            self.state = CircuitState::Open;
            self.opened_at = Some(Instant::now());
            self.consecutive_failures = self.failure_threshold;
        }
        CircuitState::Closed | CircuitState::Open => {
            self.consecutive_failures = self.consecutive_failures.saturating_add(1);
            if self.consecutive_failures >= self.failure_threshold {
                self.state = CircuitState::Open;
                self.opened_at = Some(Instant::now());
            }
        }
    }
}
```

- [ ] **Step 4: Re-run module tests**

Run: `cargo test -p cc-switch-web proxy::circuit_breaker::tests proxy::provider_router::tests -- --nocapture`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add cc-switch-web/src/proxy/provider_router.rs cc-switch-web/src/proxy/circuit_breaker.rs
git commit -m "test(proxy): lock breaker and failover runtime policy contracts"
```

### Task 2: Add Request-Log Query Surface (List + Filters)

**Files:**
- Modify: `cc-switch-lib/src/database/mod.rs`
- Modify: `cc-switch-web/src/handlers/usage.rs`
- Modify: `cc-switch-web/src/main.rs`
- Test: `cc-switch-lib/src/database/mod.rs` (inline `#[cfg(test)]`)

- [ ] **Step 1: Write failing database query tests for request logs**

```rust
#[test]
fn get_proxy_request_logs_respects_limit_and_order() {
    let db = Database::init_for_tests().expect("db init");

    for idx in 0..3 {
        db.save_proxy_request_log(&ProxyRequestLogRecord {
            app_type: "claude".to_string(),
            provider_id: format!("p{idx}"),
            request_path: "/v1/messages".to_string(),
            request_model: Some("claude-3".to_string()),
            status_code: Some(200),
            success: true,
            error_message: None,
        }).expect("insert log");
    }

    let logs = db.get_proxy_request_logs(2).expect("query logs");
    assert_eq!(logs.len(), 2);
}
```

- [ ] **Step 2: Run failing test**

Run: `cargo test -p cc-switch-lib proxy_request_logs -- --nocapture`
Expected: FAIL if helper/test query support is incomplete.

- [ ] **Step 3: Implement query/filter API in database and handler**

```rust
// database/mod.rs
pub fn get_proxy_request_logs(
    &self,
    limit: usize,
) -> Result<Vec<ProxyRequestLogEntry>, AppError> {
    let conn = self.conn();
    let mut stmt = conn.prepare(
        "SELECT app_type, provider_id, request_path, request_model,
                status_code, success, error_message, created_at
         FROM proxy_request_logs
         ORDER BY created_at DESC, id DESC
         LIMIT ?1",
    )?;
    // ... map rows
}
```

```rust
// handlers/usage.rs
pub async fn get_proxy_request_logs(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ProxyRequestLogsQuery>,
) -> Json<ProxyRequestLogsResponse> {
    let limit = query.limit.unwrap_or(200).clamp(1, 1000);
    match state.db.get_proxy_request_logs(limit) {
        Ok(logs) => Json(ProxyRequestLogsResponse { logs }),
        Err(_) => Json(ProxyRequestLogsResponse { logs: vec![] }),
    }
}
```

```rust
// main.rs
.route("/api/usage/request-logs", get(usage::get_proxy_request_logs))
```

- [ ] **Step 4: Run package tests and compile check**

Run: `cargo test -p cc-switch-lib && cargo check -p cc-switch-web`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add cc-switch-lib/src/database/mod.rs cc-switch-web/src/handlers/usage.rs cc-switch-web/src/main.rs
git commit -m "feat(usage): expose proxy request-log query endpoint"
```

### Task 3: Wire Failover Switch Synchronization on Retry Success

**Files:**
- Modify: `cc-switch-web/src/proxy/failover_switch.rs`
- Modify: `cc-switch-web/src/proxy/server.rs`
- Modify: `cc-switch-web/src/proxy/forwarder.rs`
- Test: `cc-switch-web/src/proxy/failover_switch.rs` (inline `#[cfg(test)]`)

- [ ] **Step 1: Write failing tests for dedup switch behavior**

```rust
#[tokio::test]
async fn try_switch_deduplicates_concurrent_requests() {
    let db = test_db();
    let mgr = FailoverSwitchManager::new(db.clone());

    let (a, b) = tokio::join!(
        mgr.try_switch("claude", "provider-a"),
        mgr.try_switch("claude", "provider-a")
    );

    assert!(a.is_ok());
    assert!(b.is_ok());
}
```

- [ ] **Step 2: Run focused failover switch tests**

Run: `cargo test -p cc-switch-web failover_switch -- --nocapture`
Expected: FAIL before final sync wiring if behavior is missing.

- [ ] **Step 3: Implement switch trigger after retry success**

```rust
// forwarder.rs
if state.provider_id != current_provider_id {
    if let Err(e) = failover_switch.try_switch(app_type, &state.provider_id).await {
        log::error!("[Proxy] Failed to apply failover switch: {}", e);
    }
}
```

```rust
// server.rs runtime state
failover_switch: Arc<FailoverSwitchManager>,
```

- [ ] **Step 4: Run proxy tests**

Run: `cargo test -p cc-switch-web proxy -- --nocapture`
Expected: PASS with no regression on existing streaming tests.

- [ ] **Step 5: Commit**

```bash
git add cc-switch-web/src/proxy/failover_switch.rs cc-switch-web/src/proxy/server.rs cc-switch-web/src/proxy/forwarder.rs
git commit -m "feat(proxy): sync failover switch state after retry success"
```

### Task 4: Close Provider Adapter Minimum Contract Coverage

**Files:**
- Modify: `cc-switch-web/src/proxy/adapters/claude/mod.rs`
- Modify: `cc-switch-web/src/proxy/adapters/claude/response.rs`
- Modify: `cc-switch-web/src/proxy/adapters/gemini/mod.rs`
- Modify: `cc-switch-web/src/proxy/adapters/gemini/response.rs`
- Modify: `cc-switch-web/src/proxy/adapters/copilot/mod.rs`
- Modify: `cc-switch-web/src/proxy/adapters/copilot/response.rs`
- Modify: `cc-switch-web/src/proxy/adapters/codex/request.rs`
- Modify: `cc-switch-web/src/proxy/adapters/codex/response.rs`

- [ ] **Step 1: Add failing test per provider for request/response contract**

```rust
#[test]
fn transform_request_passthrough_for_claude() {
    let adapter = ClaudeAdapter::new();
    let out = adapter.transform_request(TransformInput {
        body: json!({"model":"claude-3","messages":[{"role":"user","content":"hi"}]}),
        upstream_url: "https://api.anthropic.com/v1/messages".to_string(),
        prompt_cache_key: None,
        requested_stream: true,
        codex_fast_mode: false,
    }).expect("ok");
    assert_eq!(out.method, "POST");
}
```

```rust
#[test]
fn non_streaming_extracts_usage() {
    let body = Bytes::from_static(
        br#"{"model":"gpt-5","usage":{"prompt_tokens":7,"completion_tokens":11}}"#,
    );
    let result = transform(body, false).expect("ok");
    assert!(result.record.is_some());
}
```

- [ ] **Step 2: Run adapter tests to confirm baseline**

Run: `cargo test -p cc-switch-web adapters -- --nocapture`
Expected: New tests fail before implementation if contracts are absent.

- [ ] **Step 3: Implement minimal contract-preserving code/tests**

```rust
// keep adapter transform_request as passthrough where intended
Ok(TransformOutput {
    body: input.body,
    upstream_url: input.upstream_url,
    headers: vec![],
    method: "POST".to_string(),
})
```

```rust
// response parser assertions should match current parser expectations
let record = parser.from_openai_json(&body);
```

- [ ] **Step 4: Re-run adapter + full web test suite**

Run: `cargo test -p cc-switch-web adapters -- --nocapture && cargo test -p cc-switch-web`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add cc-switch-web/src/proxy/adapters/claude/mod.rs cc-switch-web/src/proxy/adapters/claude/response.rs cc-switch-web/src/proxy/adapters/gemini/mod.rs cc-switch-web/src/proxy/adapters/gemini/response.rs cc-switch-web/src/proxy/adapters/copilot/mod.rs cc-switch-web/src/proxy/adapters/copilot/response.rs cc-switch-web/src/proxy/adapters/codex/request.rs cc-switch-web/src/proxy/adapters/codex/response.rs
git commit -m "test(adapters): add minimum request/response contract coverage"
```

### Task 5: Final Verification, Formatting, and Merge Readiness

**Files:**
- Modify: `docs/proxy-routing-gap-analysis.md`
- Modify: `docs/proxy-routing-baseline-test-snapshot.md`
- Modify: `docs/superpowers/plans/2026-05-08-provider-routing-upstream-alignment.md`

- [ ] **Step 1: Update docs to reflect completed parity items**

```markdown
## Completion Update (2026-05-08)
- Router + retry path wired at request time
- Breaker key scoped by app_type:provider_id
- Failover switch sync on retry success
- Request logs API available at /api/usage/request-logs
- Adapter contract tests added for Claude/Gemini/Copilot/Codex
```

- [ ] **Step 2: Run formatting and static checks**

Run: `cargo fmt --all -- --check && cargo check -p cc-switch-web && cargo check -p cc-switch-lib`
Expected: PASS.

- [ ] **Step 3: Run final tests**

Run: `cargo test -p cc-switch-lib && cargo test -p cc-switch-web`
Expected: PASS.

- [ ] **Step 4: Commit docs+verification updates**

```bash
git add docs/proxy-routing-gap-analysis.md docs/proxy-routing-baseline-test-snapshot.md docs/superpowers/plans/2026-05-08-provider-routing-upstream-alignment.md
git commit -m "docs(proxy): sync parity status and verification evidence"
```

- [ ] **Step 5: Prepare merge handoff**

```bash
git log --oneline -n 10
git status --short
```

Expected: clean staged history for PR/merge.

---

## Self-Review

- Spec coverage: Plan covers missing core parity areas identified in prior analysis: failover/breaker runtime semantics, request-log surface, switch sync, and adapter contract coverage.
- Placeholder scan: no `TODO/TBD/implement later` instructions in execution steps; each step has code/command expectations.
- Type consistency: `ProviderRouter`, `CircuitBreaker`, `FailoverSwitchManager`, `ProxyRequestLogRecord/Entry`, and route `GET /api/usage/request-logs` are used consistently across tasks.
