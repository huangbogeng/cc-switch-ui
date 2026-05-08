# Provider Integration Checklist

Use this checklist for every provider feature/fix PR.

## A. Architecture Boundaries
- [ ] Handler layer contains no provider-specific transform logic.
- [ ] Provider-specific behavior is confined to `proxy/adapters/<provider>/`.
- [ ] Shared behaviors reuse `cc-switch-lib` abstractions.

## B. Database Contract
- [ ] Main forwarding path does not rely on DB success for request success.
- [ ] Usage writes are best-effort and do not break response flow.
- [ ] Model field has fallback strategy when upstream omits it.
- [ ] Any schema change includes safe, idempotent migration.

## C. Live Config Consistency
- [ ] `switch_provider` updates runtime target state and live settings.
- [ ] `proxy_start` enforces proxied live settings and verifies effective `base_url`.
- [ ] `proxy_stop` restores direct live settings.
- [ ] Provider auth keys are mutually exclusive in `env` (`AUTH_TOKEN` vs `API_KEY`).
- [ ] Stale `ANTHROPIC_*` keys are cleaned to prevent cross-provider residue.

## D. Provider Adapter Contract
- [ ] Auth extraction priority is explicit and tested.
- [ ] Base URL normalization avoids path duplication.
- [ ] Request transform supports expected Anthropic input shapes.
- [ ] Streaming and non-streaming response paths are both supported.
- [ ] Usage extraction supports provider response format.

## E. Observability
- [ ] `info` logs exist for meaningful transitions and request outcomes.
- [ ] High-frequency polling routes log at `debug` level.
- [ ] Error logs include provider id, path/context, and reason.

## F. Tests
- [ ] Unit tests for request transform and URL normalization.
- [ ] Usage parser tests for field variants (`prompt/completion`, `input/output`).
- [ ] Provider switch consistency test (`direct -> proxy -> direct`).
- [ ] Non-regression checks for MiniMax and DeepSeek paths.
