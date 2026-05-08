# Backend Modularization Plan (Rust Best Practices)

Date: 2026-05-08
Repo: `cc-switch-ui`
Scope: `cc-switch-server` + `cc-switch-lib`

## Current Status (Updated: 2026-05-08)

1. Phase 0 completed (baseline tests captured under `docs/test-logs/`).
2. Phase 1 completed (proxy streaming split delivered with compatibility bridge).
3. Active PR: `#22` (`feat/backend-modularization-phase1`).

## 1. Objectives

1. Reduce oversized backend files by introducing directory-level modules.
2. Clarify functional boundaries across proxy pipeline, handlers, and lib services.
3. Enforce Rust best practices (typed errors, minimal visibility, no panic paths in runtime).
4. Keep behavior parity during refactor (no functional regressions).

## 2. Non-Goals

1. No provider feature expansion in this refactor.
2. No UI behavior changes.
3. No data schema redesign beyond splitting file layout.

## 3. Quality Gates (Required Per Phase)

1. `cargo fmt --all`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. Targeted tests for touched modules
4. At least one proxy E2E-style regression path: DeepSeek thinking + stream usage + failover

Runtime constraints:

1. Avoid new `unwrap/expect` in non-test code.
2. Prefer `pub(crate)` over `pub` unless external crate API requires exposure.
3. Keep logs structured with stable keys (`app_type`, `provider`, `path`, `model`, `status`, `elapsed_ms`).

## 4. Target Module Boundaries

### 4.1 `cc-switch-server/src/proxy/`

Target directories:

1. `orchestrator/`
- Owns request attempt lifecycle and retry/failover flow.
- Files: `mod.rs`, `flow.rs`, `retry.rs`, `usage_recording.rs`

2. `routing/`
- Owns provider selection, breaker state, failover switch.
- Files: `mod.rs`, `provider_router.rs`, `circuit_breaker.rs`, `failover_switch.rs`

3. `transport/`
- Owns upstream HTTP call and header translation.
- Files: `mod.rs`, `upstream_client.rs`, `headers.rs`

4. `streaming/`
- Owns SSE conversion and stream state machines.
- Files: `mod.rs`, `openai_chat.rs`, `responses.rs`, `common.rs`

5. `adapters/`
- Keeps provider-specific protocol mapping only.

### 4.2 `cc-switch-server/src/handlers/`

Directory modules per domain:

1. `providers/`: `routes.rs`, `dto.rs`, `service.rs`
2. `settings/`: `routes.rs`, `dto.rs`, `service.rs`
3. `usage/`: `routes.rs`, `dto.rs`, `service.rs`
4. `auth/`: `routes.rs`, `service.rs`

### 4.3 `cc-switch-lib/src/`

1. `database/`
- Split into `models.rs`, `schema.rs`, `migrations.rs`, `repo_providers.rs`, `repo_usage.rs`, `repo_settings.rs`.

2. `oauth/codex` and `oauth/copilot`
- Split into `client.rs`, `token_store.rs`, `refresh.rs`, `device_flow.rs`, `accounts.rs`.

## 5. Execution Plan (Commit-Scoped)

## Phase 0: Baseline Stabilization

- [x] Record current test baseline for proxy + adapters.
- [x] Add/confirm regression tests for DeepSeek reasoning_content roundtrip.
- [x] Snapshot file-size baseline for top 10 largest files.

Verification:

```bash
cargo test -p cc-switch-server proxy::streaming_responses -- --nocapture
cargo test -p cc-switch-server deepseek -- --nocapture
```

## Phase 1: Proxy Streaming Split (Lowest Risk First)

- [x] Create `proxy/streaming/` directory and move helpers to `common.rs`.
- [x] Move OpenAI Chat SSE conversion into `openai_chat.rs`.
- [x] Move Responses SSE conversion into `responses.rs`.
- [x] Keep old module path via `pub use` bridge to avoid wide touching.
- [x] Extract tool-block/finalization helpers without over-fragmenting files.

Verification:

```bash
cargo test -p cc-switch-server streaming_responses -- --nocapture
cargo clippy --workspace --all-targets -- -D warnings
```

## Phase 2: Forwarder Decomposition

- [ ] Split `forwarder.rs` into orchestrator flow + retry policy + usage recording helpers.
- [ ] Extract upstream request building/sending into `transport/upstream_client.rs`.
- [ ] Keep behavior-equivalent logs and status handling.

Verification:

```bash
cargo test -p cc-switch-server proxy -- --nocapture
cargo clippy --workspace --all-targets -- -D warnings
```

## Phase 3: Handler Domain Decomposition

- [ ] Convert `providers.rs`, `settings.rs`, `usage.rs` into directory modules.
- [ ] Pull request/response structs into `dto.rs`.
- [ ] Keep routing table stable from `main.rs` call sites.

Verification:

```bash
cargo test -p cc-switch-server handlers -- --nocapture
cargo clippy --workspace --all-targets -- -D warnings
```

## Phase 4: Lib Database Modularization

- [ ] Split `database/mod.rs` into model/schema/migration/repo files.
- [ ] Keep public Database API stable.
- [ ] Add focused tests per repo file.

Verification:

```bash
cargo test -p cc-switch-lib database -- --nocapture
cargo clippy --workspace --all-targets -- -D warnings
```

## Phase 5: OAuth Modularization

- [ ] Decompose `oauth/codex/mod.rs` and `oauth/copilot/mod.rs` into lifecycle submodules.
- [ ] Isolate persistence and refresh logic to typed service structs.
- [ ] Keep external manager methods stable.

Verification:

```bash
cargo test -p cc-switch-lib oauth -- --nocapture
cargo clippy --workspace --all-targets -- -D warnings
```

## 6. Risk Controls

1. Mechanical move first, logic edits second.
2. Use bridge re-exports during transition to minimize cascading edits.
3. One phase per PR recommended.
4. Preserve error semantics and HTTP statuses as compatibility contract.

## 7. Deliverables

1. Modularized backend directory structure.
2. Updated architecture notes under `docs/architecture/`.
3. Reduced oversized files (target: no runtime file > 500 LOC in proxy and handlers).
4. Green fmt/clippy/tests across touched crates.

## 8. Start-Now Tasklist

1. Start Phase 2 (`forwarder.rs`) decomposition with no behavior changes.
2. Keep streaming module count stable (avoid over-splitting).
3. Continue using small, reviewable commits per boundary change.
