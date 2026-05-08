# Provider Development Protocol

This document defines the shared development protocol for provider integration in `cc-switch-ui`.

It is the source of truth for:
- architecture boundaries,
- reusable components,
- database contracts,
- runtime consistency rules,
- and provider implementation requirements.

## 1. Architecture Layers

### 1.1 `cc-switch-web/src/handlers/*`
- Responsibility: API orchestration only.
- Must not contain provider-specific request/response mapping logic.
- Allowed actions:
  - read/write state via `cc-switch-lib::database`,
  - trigger proxy lifecycle (`start/stop/status`),
  - apply live config sync.

### 1.2 `cc-switch-web/src/proxy/*`
- Responsibility: request forwarding runtime.
- Contains:
  - provider selection and failover,
  - forwarding and retries,
  - streaming transformation,
  - usage recording trigger points.
- Must remain provider-agnostic except adapter dispatch.

### 1.3 `cc-switch-web/src/proxy/adapters/*`
- Responsibility: provider-specific differences only.
- Required per provider:
  - auth extraction,
  - request transform,
  - response/usage transform,
  - upstream URL extraction/normalization.
- Must not duplicate database or routing logic.

### 1.4 `cc-switch-lib/*`
- Responsibility: shared primitives.
- Includes:
  - database schema + persistence API,
  - live config sync,
  - usage parsing,
  - common types and errors.

## 2. Reuse Rules

## 2.1 Must Reuse
- `Forwarder` main flow for HTTP lifecycle.
- `UsageParser` for OpenAI/Anthropic usage extraction.
- `settings_for_live` + `apply_provider_to_live` for live config synchronization.
- `ProviderRouter` / failover components for routing behavior.

## 2.2 May Extend
- Provider-specific `request.rs` and `response.rs`.
- Provider-specific request rectification (for known upstream incompatibility).
- Provider-specific model mapping inputs (via provider env/meta).

## 2.3 Must Not Duplicate
- Provider-local database writes for common usage/request logs.
- Provider-local proxy lifecycle logic.
- Provider-local ad hoc live settings writes.

## 3. Database Protocol

Database is part of the public architecture contract. Any schema change must preserve compatibility and migration safety.

## 3.1 Table Responsibilities
- `providers`: provider metadata and configuration (`settings_config`, `meta`, routing flags).
- `proxy_config`: global outbound proxy transport config.
- `proxy_target_config`: active route target provider id.
- `proxy_port_config`: local proxy listen port.
- `usage_records`: provider usage accounting source of truth.
- `proxy_request_logs`: optional request-level diagnostics (non-critical path).

## 3.2 Write Path Policy
- Main request success/failure must not depend on request-log persistence.
- Usage persistence failures are observability errors, not serving errors.
- Any DB write in forwarding path must degrade gracefully and never break response delivery.

## 3.3 Schema Evolution Policy
- Additive migrations first; avoid destructive changes.
- New NOT NULL columns must provide safe defaults or data backfill.
- Migration code must be idempotent and safe for old local databases.

## 3.4 Field Contract Policy
- Provider model identifiers may be absent in upstream responses.
- DB writes requiring model must apply fallback (`request_model`, mapped model, or `"unknown"`).
- Usage field parsing must support both:
  - `prompt_tokens` / `completion_tokens`
  - `input_tokens` / `output_tokens`

## 4. Runtime Consistency Contract

Three states must stay aligned:
- database provider state (`current_provider_id`, `proxy_target_provider_id`),
- proxy runtime state (`running`, active target),
- live Claude settings file (`base_url`, auth keys, model env).

## 4.1 Required Operations
- `switch_provider`: updates DB state and syncs live config.
- `proxy_start`: applies proxied live config, verifies `base_url`, then starts runtime.
- `proxy_stop`: applies direct live config before stopping runtime.

## 4.2 Drift Detection
- `proxy_status` should expose effective live settings summary:
  - `live_base_url`,
  - optional `live_model`,
  - optional `live_auth_mode`.
- Drift means runtime mode and live file mode are inconsistent.
- Drift must be logged as error-level signal.

## 5. Provider Contract

Each provider implementation must satisfy:

1. Auth Contract
- Define deterministic auth source priority.
- Ensure token/key conflict cleanup in live env (`ANTHROPIC_AUTH_TOKEN` vs `ANTHROPIC_API_KEY`).

2. URL Contract
- Normalize base URL to correct upstream endpoint without double suffix.

3. Request Contract
- Accept Anthropic input shape and transform to provider-compatible shape.
- Ensure streaming config consistency (`stream_options.include_usage` when required).

4. Response Contract
- Preserve client-compatible response shape expected by frontend/Claude.
- Extract usage for both streaming and non-streaming paths where available.

5. Error Contract
- Return actionable errors in logs with provider id, status, and path.

## 6. Logging Protocol

Logs are in-memory/terminal observability and must not be required for correctness.

- `info`: state transitions and meaningful request lifecycle events.
- `debug`: high-frequency polling/status endpoints.
- `error`: hard failures, drift, and integrity violations.

Every critical operation should include:
- action name,
- provider id,
- mode/context,
- result and reason on failure.

## 7. Test Contract

A new or modified provider is not complete without passing:

1. Request transform tests (normal + edge cases).
2. URL normalization tests.
3. Usage parsing tests for expected upstream format variants.
4. Direct/proxy switch consistency test:
   - direct -> proxy -> direct.
5. Existing provider non-regression (at least MiniMax + DeepSeek).

## 8. Completion Criteria for Provider Work

A provider change is complete only if:
- contract rules above are satisfied,
- tests are added/updated and passing,
- no state drift introduced in switch/start/stop flows,
- no provider-specific duplication of shared architecture.
