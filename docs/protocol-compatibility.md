# Claude Code route compatibility / 协议兼容范围

[Documentation](README.md) · [User guide](user-guide.md)

The local route receives Claude Code's Anthropic Messages requests. Provider
authentication and the selected upstream wire protocol are separate: `apiFormat`
selects the conversion while the provider adapter supplies credentials. Managed
Codex OAuth always uses its dedicated Responses contract.

This conservative implementation follows the protocol dispatch and content
preservation patterns in [cc-switch at 28dc5198](https://github.com/farion1231/cc-switch/tree/28dc5198115575ced2e3502f52181e7d211859c9/src-tauri/src/proxy).

| Route format | Behavior |
|---|---|
| `anthropic` | Preserve request fields, system arrays, reminders, cache markers and response bodies. Forward capability headers and query parameters. Configured model mapping and upstream credentials still apply. |
| `openai_chat` | Convert messages, function tools, tool results and images; convert JSON and SSE replies back to Messages. |
| `openai_responses` | Convert messages, function tools, tool results, images and supported PDF inputs; preserve the client's streaming choice and output budget for public Responses endpoints. |
| Codex OAuth | Use the dedicated Responses-only endpoint, force upstream SSE, omit unsupported sampling/budget fields and aggregate replies for non-streaming Messages clients. |
| `gemini_native` | Native conversion is not implemented. Reject route selection/start before taking over Claude Code configuration. |

## Choosing a route

Choose the provider's actual upstream format in the provider editor, then select
**Route Target → Start Route**. The incoming client protocol is always Claude
Code's Anthropic Messages format; selecting Chat or Responses describes the
upstream endpoint, not a change to the client protocol.

The resolver uses `meta.apiFormat`, then the legacy `settingsConfig.api_format`,
then the provider adapter's default. Credentials continue to come from the
provider's authentication adapter. Managed Codex OAuth always uses its dedicated
Responses contract, regardless of a conflicting format selector. Backend adapter
selection also recognizes the Codex-specific `authMode: oauth_proxy` declaration
and Codex `authBinding.authProvider` / legacy `auth_provider` signals before stale
provider or protocol hints. Missing managed authentication fails rather than
falling back to an API key. **Direct
Config → Select** writes direct connection settings; it does not run these
converters.

For a Codex subscription, authorize an account in **OAuth → ChatGPT Codex**,
choose the default or bind that account in the provider editor, and start the
route. See the [subscription setup steps](user-guide.md#codex-subscription-without-an-api-key--codex-订阅路由).

The provider card and editor use the same protocol fallback order. Editing a
legacy record retains its top-level settings and authentication adapter metadata;
choosing a different format writes the higher-priority `meta.apiFormat` value.
Codex OAuth displays a fixed Responses format and preserves the selected account
binding. The actual editing save path persists the canonical Codex provider type,
authentication mode and Responses format even on a name-only edit of an imported
OAuth record. API-key records keep the baseline-diff preservation behavior.
Saving an unchanged OAuth form also canonicalizes the record without mutating
the original baseline used for conflict detection.
Unsupported protocols remain available as saved configurations but are
marked unavailable in the route selector. An interrupted/running route can still
be stopped or restored even if its target is now unsupported.

Route-target failures are shown before any start request is made. A rejected
selection must not start the previously selected route. The backend remains the
final authority on supported protocols and validates again before takeover.

## Prompts and tool results

- Native Anthropic forwarding preserves prompt text and its structure.
- Chat/Responses conversion removes only an attribution line beginning with
  `x-anthropic-billing-header:` at the start of a system text block. Text following
  that line and occurrences inside ordinary prompt text are preserved.
- System/developer messages retain their roles on public converted routes.
  The Codex OAuth backend rejects `system` messages inside `input`: map those
  mid-turn messages to `developer`, preserving their text and position. The
  top-level system prompt still becomes `instructions`. This endpoint-specific
  exception differs from the pinned upstream converter, which preserves these
  roles. Responses content blocks retain their order around function calls and
  results.
- Converted routes remove the trailing `[1m]` client context-window selector
  from the upstream model ID. Native routes preserve it.
- Chat requires tool results immediately after assistant tool calls. Images from
  tool results are attached in a following user message; sibling reminder text
  remains user content and is never absorbed into tool output.
- Unknown tool-result parts, including `tool_reference`, survive as JSON text.
  Function tools load upfront on converted routes; this does **not** reproduce
  Anthropic's native deferred-loading semantics or its context savings.
- Tool failures retain an error marker, rather than looking like successful
  empty results. Unsupported hosted tools/content produce an explicit request
  error instead of silently disappearing.

The route does not replace Claude Code's identity prompt, remove all
`system-reminder` blocks, merge reminders into tool results, change probe models,
or run the upstream Copilot optimizers. It does not judge whether prompt text
from project files, Skills or MCP is malicious.

## Boundaries

Use an Anthropic-compatible route when native Claude capabilities are required.
Thinking settings map to supported OpenAI reasoning effort with the upstream
priority, disabled-thinking and model-tier rules. Before model mapping, the
converter preserves upstream's known client defaults for omitted adaptive
thinking. Native Anthropic requests keep their original thinking fields.
Responses replies transport encrypted reasoning through cc-switch's versioned
`ccswitch-openai-reasoning-v1:` envelope in `thinking.signature` or
`redacted_thinking.data`. Replay retains only `type`, `id`, `summary` and
`encrypted_content`, including when decoding older envelopes. Output-only
`status`/`content` fields and incomplete reasoning-only assistant turns are
removed from Responses input. Foreign vendor signatures are not replayed.
Codex OAuth requests include encrypted reasoning and preserve explicit parallel
call settings; the default allows parallel calls, matching upstream. Public
Responses endpoints retain their separate request constraints and do not force
Codex-only parameters. Opaque transport does not decrypt or validate a provider's
payload, or make it portable between models/accounts/providers.
Chat document inputs and hosted server tools are outside this version's scope.

Token counting is forwarded only on Anthropic-format routes. Converted routes
reject that optional endpoint before making an upstream generation request;
Claude Code can fall back to approximate counts.

Protocol conversion does not control Claude Code's local permissions, hooks,
history compaction or memory loading. Compaction can remove earlier instructions
before the route receives the next request.

## Alignment with cc-switch

| Area | Alignment / deliberate limit |
|---|---|
| Protocol selection | Separate authentication from explicit wire format; managed Codex retains its own contract. |
| Prompt/content handling | Preserve identity, reminders and roles; strip only leading billing attribution on converted routes. |
| Responses reasoning | Same versioned transport and replay whitelist; retain ordering and remove orphan reasoning. |
| Terminal events | HTTP 2xx failed/cancelled envelopes are errors; truncated streams fail; incomplete budget exhaustion maps to `max_tokens`. |
| Usage | Responses inclusive input is split into fresh, cache-read and cache-write buckets using upstream rules. |
| Media | URL/base64 images and Responses PDF/file inputs follow upstream source/title rules. Chat documents fail explicitly. |
| Codex request | Force stream/store=false, merge includes, omit unsupported sampling/budget fields, and respect parallel choice. |
| Intentional exclusions | Gemini native, hosted server tools, Copilot reminder/warmup optimizers, impersonation and full upstream stream recovery/keepalive machinery. |

The ported helpers retain upstream attribution and its [MIT license](licenses/cc-switch-MIT.txt).

Codex OAuth model discovery separately follows
[cc-switch's catalog service at d0b57827](https://github.com/farion1231/cc-switch/blob/d0b57827e453fcab5620a89a4c452648ae371d42/src-tauri/src/services/codex_oauth_models.rs).
It reads `chatgpt.com/backend-api/codex/models` using the selected managed account
and the upstream catalog client identity (`codex_cli_rs`, version `0.159.0`).
The generic API-key `/models` discovery path is unchanged. Catalog failures expose
an actionable message and HTTP status without returning upstream response bodies,
tokens or account headers to the browser. Discovery does not imply that every
listed model is available for generation, and does not change generation prompts.

This is a targeted compatibility port, not complete feature parity with the
upstream project. Converters are protocol adapters, not a prompt-injection
firewall. Project/Skill/MCP instructions still reach the model under Claude
Code's own loading and permission rules.

## Verification

`protocol_routes_preserve_prompts_and_convert_actual_http_responses` in
`cc-switch-cli/tests/workflows.rs` uses isolated homes, synthetic credentials and
local HTTP upstreams. It verifies native body/header/query preservation, rich
tool results and reminders, JSON/SSE round trips, token-count handling, and
rejection of unsupported protocols without changing live configuration.
Converter tests additionally cover image/error/reference content, mid-turn
system roles, attribution boundaries, optional tool schemas and public Responses
versus Codex request constraints. These checks do not establish compatibility
with every real provider or every Claude Code feature.

Run the committed checks from the repository root:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo test -p cc-switch-cli --test workflows
```

The HTTP workflow regression is committed and runs with temporary profiles,
synthetic credentials and loopback upstreams. It requires no provider account or
paid generation. Reasoning regressions verify both summarized and redacted
opaque carriers, replay whitelisting, orphan removal and rejection of foreign
signatures. Stream regressions cover failed/cancelled responses, unexpected EOF,
invalid tool arguments, single-byte UTF-8 chunks and CRLF framing.

Mock tests establish conversion and transport behavior. They do not establish a
real provider's acceptance of encrypted reasoning, account/model portability,
image interpretation, long-session behavior or native deferred-tool semantics.
Validate those capabilities separately against the intended provider and model.

## Troubleshooting

| Symptom | Meaning and action |
|---|---|
| Route selection/start rejects `gemini_native` | Choose an Anthropic-compatible endpoint or another implemented wire format. The rejection happens before configuration takeover. |
| Converted token-count request returns an error | No upstream generation is made for this optional request. Use an Anthropic route for native token counting. |
| Unsupported hosted tool or Chat document | Use an Anthropic route for native capabilities, or Responses for supported document inputs. |
| Codex OAuth rejects a `system` input message | The dedicated converter maps mid-turn system text to developer instructions without moving or removing it. Check that this dedicated route is selected. |
| Upstream authorization fails | Verify the selected account or API key. Reauthorize the account through the OAuth UI when its credentials are rejected. Protocol conversion does not grant model access. |
| Responses fails, is cancelled, or disconnects before a terminal event | Treat it as a failed request; the converter must not present an empty successful completion. |

## 中文说明

在 Provider 编辑器中选择上游实际使用的协议，再选择 **Route Target → Start Route**。
Claude Code 发给代理的请求仍使用 Anthropic Messages；**Direct Config → Select** 不执行
这些协议转换。Codex 订阅需先在 OAuth 页面授权账号，然后选择对应 Provider 并启动路由。

本版本优先保留提示正文、提醒和工具结果。Codex OAuth 对中途 `system` 消息有专门限制，
转换为 `developer` 时保留正文和消息位置。跨协议路由将工具引用保留为 JSON 文本，并
提前加载函数定义；这不等于原生工具延迟加载的语义或上下文节省。

Responses 加密 reasoning 使用版本化载体和字段白名单回传，保留不透明内容，不进行解密。
仓库回归测试验证传输和转换逻辑，不代表所有真实模型都会接受该状态，也不保证跨账号、
跨模型或跨服务商迁移。需要完整 Claude 原生能力时请选择 Anthropic 兼容端点。

Gemini 原生转换和托管服务端工具仍明确拒绝。路由不会清洗全部运行提醒，也不会接管
Claude Code 的权限、Hooks、记忆或历史压缩。认证失败应检查凭证或重新授权，协议适配
本身不会授予账号或模型权限。
