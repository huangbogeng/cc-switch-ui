# Unified Tool Pipeline Design (MiniMax + DeepSeek + ChatGPT)

## Goal
Build a provider-agnostic tool-call normalization pipeline that runs before provider adapters, with Compat behavior by default, covering MiniMax, DeepSeek, and ChatGPT (both Codex OAuth and OpenAI API key paths), across Anthropic Messages, OpenAI Chat Completions, and OpenAI Responses request protocols.

## Scope
In scope:
- Unified request-side normalization pipeline in `cc-switch-server` proxy layer
- Protocol coverage: `/v1/messages`, `/chat/completions` + `/v1/chat/completions`, `/responses` + `/v1/responses`
- Provider coverage in first release: `minimax`, `deepseek`, `codex_oauth`, openai-compatible ChatGPT provider
- Compat strategy: repair when safe, block only when non-repairable
- Minimal observability and request-log integration for pipeline decisions

Out of scope:
- Pricing, billing, and cost analytics redesign
- Subscription/coding-plan product features
- Broad UI redesign
- Rewriting upstream adapter auth flows

## Architecture
### Request flow
1. Ingress handler receives protocol-specific payload.
2. Protocol payload is normalized into an internal provider-agnostic representation.
3. Tool pipeline runs in Compat mode:
   - preserve classification from original payload
   - sanitize orphan tool results
   - merge adjacent tool result + text when safe
   - validate minimum contract
4. Normalized payload is materialized back to protocol-specific shape expected by adapter transform.
5. Existing provider adapter `transform_request` runs with reduced provider-specific cleanup responsibility.
6. Forward to upstream.

### Core design principles
- One normalization pipeline for all target providers and all three protocols.
- Adapter code keeps provider-specific mapping logic, not generic tool consistency repair logic.
- Repair-first behavior under Compat; block only for non-repairable semantic violations.
- Deterministic and observable outcomes (warnings and blocked reasons logged).

## Components and Boundaries
### New module: `cc-switch-server/src/proxy/tool_pipeline/mod.rs`
- Public entry:
  - `run_tool_pipeline(request_ctx, body) -> PipelineOutput`
- Orchestrates stages only.

### New module: `cc-switch-server/src/proxy/tool_pipeline/model.rs`
- Internal normalized model:
  - `NormalizedTurn`
  - `ToolCallRef`
  - `ToolResultRef`
  - `PipelineIssue` (`warning` or `error`)
- Holds protocol-independent semantic representation.

### New module: `cc-switch-server/src/proxy/tool_pipeline/normalize.rs`
- Converts protocol payloads (`messages` / `chat` / `responses`) into normalized model.
- Preserves sufficient metadata for reversible materialization.

### New module: `cc-switch-server/src/proxy/tool_pipeline/sanitize.rs`
- Removes orphan `tool_result` that cannot be associated with prior tool call.
- Drops empty `tool_call_id` results.
- Emits warning issues with stable reason codes.

### New module: `cc-switch-server/src/proxy/tool_pipeline/merge.rs`
- Merges `[tool_result, adjacent_text]` safely into one tool-result semantic unit.
- Never rewrites ambiguous sequences.

### New module: `cc-switch-server/src/proxy/tool_pipeline/validate.rs`
- Validates minimal non-repairable constraints after sanitize/merge.
- Returns structured blocking errors for local 400 responses.

### Existing module changes
- `cc-switch-server/src/proxy/forwarder.rs`
  - Invoke pipeline before adapter transform.
  - Propagate pipeline warnings/blocked outcomes to logs.
- Provider adapters (`minimax`, `deepseek`, `codex`, `openrouter`)
  - Remove duplicate generic sanitation logic where present.
  - Retain provider-specific mapping.
- `cc-switch-lib/src/database/mod.rs`
  - Request log schema support for pipeline diagnostics (new optional field or compact warning payload strategy).
  - Migration must remain backward compatible.

## Protocol and Provider Behavior Matrix
### Protocols
1. Anthropic Messages (`/v1/messages`)
- Treat `tool_use`/`tool_result` as first-class semantics in normalized model.

2. OpenAI Chat Completions (`/chat/completions`, `/v1/chat/completions`)
- Normalize `assistant.tool_calls` and `role=tool` responses into unified tool entities.

3. OpenAI Responses (`/responses`, `/v1/responses`)
- Normalize function call / function_call_output to unified tool entities.
- Materialize back preserving Responses semantics.

### Providers (release 1)
1. MiniMax
- Full Compat pipeline enabled.

2. DeepSeek
- Full Compat pipeline enabled on openai-compatible route.

3. ChatGPT (Codex OAuth)
- Full Compat pipeline enabled on responses/chat entry routes.
- Keep existing auth/account binding behavior unchanged.

4. ChatGPT (OpenAI API)
- Full Compat pipeline enabled on openai-compatible path.

## Error Handling and Status Semantics
### Repairable issues (warning, continue)
- Orphan `tool_result` removable without ambiguity
- Adjacent text that can be safely merged into a tool-result output
- Empty or whitespace-only tool-result content cleanup

### Non-repairable issues (error, block)
- `tool_result` references missing tool call with no safe inference path
- Structural type violations in tool-calling critical fields

### Status codes
- Local `400`: non-repairable request semantic errors
- Existing upstream/route status behavior retained for other failures (`5xx` etc.)

## Observability
### Structured runtime fields
Per request (at info/debug level as configured):
- `provider_type`
- `protocol_in`
- `pipeline_repairs_count`
- `pipeline_blocked` (bool)
- `blocked_reason` (optional)

### Request-log integration
- Persist repair/blocked summary for troubleshooting.
- Migration must auto-upgrade legacy schemas.

## Testing Strategy
### Unit tests
- `normalize.rs`: protocol to normalized model mapping correctness
- `sanitize.rs`: orphan/empty tool-result handling
- `merge.rs`: safe merge patterns and non-merge ambiguous patterns
- `validate.rs`: correct block decisions and reason codes

### Adapter regression tests
- MiniMax, DeepSeek, Codex/OpenAI request transforms still produce expected upstream payload shape after pipeline integration

### End-to-end proxy tests
- One scenario per protocol with valid tool roundtrip
- One orphan tool-result scenario repaired under Compat
- One non-repairable scenario blocked locally with deterministic `400`

## Rollout Plan
1. Introduce internal pipeline modules + tests (no behavior switch yet).
2. Integrate pipeline into forwarder behind default-on Compat behavior for target providers.
3. Enable request-log diagnostics and schema migration.
4. Run protocol/provider matrix tests.
5. Keep extension points for future providers without changing adapter contracts.

## Non-Goals and Guardrails
- Do not introduce provider-specific hacks in the common pipeline.
- Do not alter auth flows for Codex OAuth/OpenAI providers.
- Do not expand into pricing/usage product scope during this phase.
- Keep migration additive and safe for existing local databases.

## Acceptance Criteria
- MiniMax + DeepSeek + ChatGPT (Codex OAuth and OpenAI API) all pass shared tool-call normalization path.
- Known `tool_result` sequencing failure class is either repaired or blocked locally, never sent upstream as malformed.
- All three request protocols are covered by tests.
- Legacy DB instances auto-migrate without manual table reset.
