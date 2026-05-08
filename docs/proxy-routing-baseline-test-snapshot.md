# Task 0.2 - Proxy Routing Baseline Test Snapshot

## Scope
- Phase 0 baseline lock: captured **before any Phase 1 runtime implementation changes**.
- Workdir: `/home/huangbogeng/github.com/huangbogeng/cc-switch-ui`
- Capture time (UTC): `2026-05-08T03:13:08Z`

## Exact Commands Run
```bash
cargo test -p cc-switch-server 2>&1 | tee /tmp/task0_2_cargo_test_cc_switch_web.log
cargo test -p cc-switch-lib 2>&1 | tee /tmp/task0_2_cargo_test_cc_switch_lib.log
```

## Command Assumptions (Profile/Flags)
- Commands were run exactly as shown above, with no extra flags.
- `cargo test` default profile applied: `test` profile (`[unoptimized + debuginfo]` shown in output).
- No feature toggles, no explicit `--release`, and no explicit single-thread/serial test flags were set.

## Environment Notes
- OS: `Linux ub-server 5.15.0-107-generic #117-Ubuntu SMP Fri Apr 26 12:26:49 UTC 2024 x86_64 GNU/Linux`
- Rust: `rustc 1.95.0 (59807616e 2026-04-14)`
- Cargo: `cargo 1.95.0 (f2d3ce0bd 2026-03-21)`
- Worktree status during run: dirty (pre-existing changes detected, no revert performed).

## Pass/Fail Summary by Package
| Package | Result | Details |
|---|---|---|
| `cc-switch-server` | PASS | `31 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out` |
| `cc-switch-lib` | PASS | `42 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out` |
| `cc-switch-lib` doc-tests | PASS | `0 passed; 0 failed` |

## Failure Snapshot (Current Run)
- None in this baseline run.
- No runtime/environment constraint failure observed for the two required commands.

## MiniMax / Tool-call / Streaming Related Status
Observed in `cc-switch-server` test run:
- `proxy::adapters::minimax::response::tests::converts_openai_chat_response_to_anthropic_message` -> PASS
- `proxy::adapters::minimax::response::tests::converts_legacy_function_call_response` -> PASS
- `proxy::adapters::minimax::request::tests::normalizes_legacy_anthropic_url_to_chat_completions` -> PASS
- `proxy::adapters::minimax::request::tests::converts_tool_use_and_tool_result_roundtrip_shape` -> PASS
- `proxy::adapters::minimax::request::tests::converts_anthropic_messages_to_openai_chat` -> PASS
- `proxy::streaming_responses::tests::converts_openai_chat_stream_tool_calls` -> PASS
- `proxy::streaming_responses::tests::captures_openai_chat_stream_usage_from_usage_chunk` -> PASS
- `proxy::streaming_responses::tests::captures_openai_chat_stream_usage_with_input_output_aliases` -> PASS
- `proxy::streaming_responses::tests::closes_open_tool_blocks_on_done_without_duplicate_message_stop` -> PASS
- `proxy::streaming_responses::tests::keeps_tool_stop_reason_with_valid_closed_tool_block` -> PASS
- `proxy::streaming_responses::tests::downgrades_tool_stop_reason_without_valid_tool_block` -> PASS

Archive note:
- Requested "MiniMax/tool-call failure samples" are **not available** in this run because no matching failures occurred.
- Historical context reference (known prior issue pattern): MiniMax stream usage accounting previously showed zero-token symptom in logs, e.g. `Streaming usage recorded: provider=minimax ... input=0, output=0`; this baseline run did not reproduce it.
- Reproducible failure capture procedure for future regressions:
  1. Re-run package tests and keep full stdout/stderr:
     - `cargo test -p cc-switch-server 2>&1 | tee /tmp/task0_2_cargo_test_cc_switch_web.log`
     - `cargo test -p cc-switch-lib 2>&1 | tee /tmp/task0_2_cargo_test_cc_switch_lib.log`
  2. If a MiniMax/tool-call/streaming test fails, extract:
     - exact failing test name
     - first error line + assertion diff excerpt
     - 30-80 surrounding log lines for context
  3. Persist logs into repo for durable review:
     - `mkdir -p docs/test-logs`
     - `cp /tmp/task0_2_cargo_test_cc_switch_web.log docs/test-logs/`
     - `cp /tmp/task0_2_cargo_test_cc_switch_lib.log docs/test-logs/`
     - if `*.log` is gitignored, also store tracked text copies:
       - `cp /tmp/task0_2_cargo_test_cc_switch_web.log docs/test-logs/task0_2_cargo_test_cc_switch_web.txt`
       - `cp /tmp/task0_2_cargo_test_cc_switch_lib.log docs/test-logs/task0_2_cargo_test_cc_switch_lib.txt`
  4. Update this snapshot with the concrete failure excerpt and status.
- Raw logs archived at both temporary and durable paths:
  - Temporary:
    - `/tmp/task0_2_cargo_test_cc_switch_web.log`
    - `/tmp/task0_2_cargo_test_cc_switch_lib.log`
  - Durable (repo):
    - `docs/test-logs/task0_2_cargo_test_cc_switch_web.log`
    - `docs/test-logs/task0_2_cargo_test_cc_switch_lib.log`
    - `docs/test-logs/task0_2_cargo_test_cc_switch_web.txt`
    - `docs/test-logs/task0_2_cargo_test_cc_switch_lib.txt`

---

## Post-Implementation Verification (2026-05-08)

Follow-up verification after parity implementation:

```bash
cargo check -p cc-switch-server
cargo test -p cc-switch-server
cargo test -p cc-switch-lib
```

Observed result:
- `cc-switch-server`: PASS (`50 passed; 0 failed`)
- `cc-switch-lib`: PASS (`42 passed; 0 failed`)

This confirms the baseline-protected areas remained green after:
- router + retry + breaker integration
- failover switch synchronization
- request-log persistence/query API
- MiniMax streaming/tool-call hardening
- adapter contract test expansion
