# Local Routing Logic Parity Design (cc-switch-ui vs upstream cc-switch)

## 1. Goal and Scope

This spec targets **logic parity only** with upstream `farion1231/cc-switch` proxy routing architecture.

In scope:
- request-time provider selection
- ordered candidate retry/failover
- provider-scoped circuit breaker state transitions
- failover success side-effect switch
- unified provider adapter contract for `minimax`, `deepseek`, `chatgpt` (codex path)

Out of scope:
- UI parity
- config page parity
- visual behavior parity

## 2. Parity Definition

Routing logic is considered aligned only when all conditions are true:

1. Provider selection is done at request time, not fixed at proxy startup.
2. Forwarder executes ordered single-attempt retries over provider candidates.
3. Circuit breaker gates requests before attempt and records result after attempt.
4. `auto_failover=true` uses failover queue; `auto_failover=false` uses current provider only.
5. Adapters handle protocol/auth transformation only; they do not decide routing.

## 3. Architecture Boundaries

### 3.1 ProviderRouter
Responsibilities:
- Build ordered provider candidates from runtime state.
- Apply breaker-based filtering.
- Maintain request allow/record result integration points.

Contract:
- `select_providers(app_type, current_provider, all_providers, auto_failover) -> Vec<Provider>`
- `allow_provider_request(app_type, provider_id) -> AllowResult`
- `record_success(app_type, provider_id)`
- `record_failure(app_type, provider_id)`

### 3.2 Forwarder
Responsibilities:
- Execute retry/failover loop over candidates.
- Resolve adapter/auth/upstream dynamically per candidate.
- Normalize timeout/error classification for retry decision.
- Trigger switch side effect after fallback success.

Contract:
- `forward_with_retry(req, candidates, context) -> Response`
- `forward_once(req, provider, adapter, context) -> AttemptResult`

### 3.3 CircuitBreaker
Responsibilities:
- Provider-scoped state machine: `Closed/Open/HalfOpen`.
- Half-open permit gate and release semantics.

Contract:
- `allow_request() -> AllowResult`
- `record_success()`
- `record_failure()`
- `release_half_open_permit()`

### 3.4 FailoverSwitch
Responsibilities:
- Persist/sync effective routing target when fallback provider succeeds.
- Isolated side-effect module; no request payload logic.

### 3.5 Provider Adapters
Responsibilities (all providers follow same contract):
- `get_auth_info`
- `transform_request`
- `transform_response / parse_usage`
- `extract_upstream_url`

No adapter may bypass common forwarder retry/failover path.

## 4. Request Data Flow

1. Request enters proxy endpoint.
2. Build request context (`app_type`, timeouts, route target, auto-failover flag).
3. Router selects ordered provider candidates and applies breaker gate.
4. Forwarder loops candidates:
   - allow request
   - resolve adapter/auth/url
   - single upstream attempt
   - success: record success, optional failover switch, return
   - failure: record failure/release permit, decide continue or return
5. Response handling:
   - non-streaming: parse usage and persist
   - streaming: parse chunks, flush usage at stream end, persist

## 5. Provider Expansion Feasibility

With the above boundaries, adding providers scales linearly:
- main routing pipeline remains unchanged
- each new provider requires only adapter-specific auth/transform/usage parsing
- complexity stays localized to provider adapter modules

This supports incremental parity for `minimax`, `deepseek`, and `chatgpt` without branching core routing logic.

## 6. Failure and Recovery Semantics

- Empty candidate set after breaker filtering returns deterministic "no available provider" failure.
- Non-retryable failures terminate current request immediately.
- Retryable failures continue to next candidate until success or exhaustion.
- Breaker state is isolated by `(app_type, provider_id)`.
- Failover switch updates runtime target only after successful fallback.

## 7. Verification Criteria

### 7.1 Core routing parity
- Request-time selection is observable in proxy request path.
- Candidate retry loop executes in forwarder.
- Breaker transitions validated by unit tests.

### 7.2 Provider-specific parity
- `minimax`: stream/non-stream usage extraction and retry behavior validated.
- `deepseek`: auth + OpenAI-compatible response/usage path validated.
- `chatgpt` (codex): Responses conversion path and retry behavior validated.

### 7.3 Non-goals check
- No UI parity work required for acceptance.

## 8. Risks and Controls

Risk:
- Existing local in-progress changes may conflict with parity edits.

Control:
- Keep edits concentrated in routing modules (`provider_router`, `forwarder`, `circuit_breaker`, `failover_switch`, adapters).
- Avoid broad refactors outside routing path.
- Verify with focused tests before broad integration tests.

## 9. Implementation Decomposition

Phase 1 (routing core parity):
- finalize request-time selection + retry/failover loop + breaker integration

Phase 2 (provider parity completion):
- `minimax`, `deepseek`, `chatgpt` behavior checks and adapter-level gap fixes

Phase 3 (stability hardening):
- failure classification consistency, timeout behavior, regression tests

