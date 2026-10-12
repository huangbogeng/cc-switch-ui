# Provider editor

Editing an existing provider merges only fields changed from the opened record. Changes made elsewhere to other fields are preserved. Conflicting record or live fields require an explicit **Keep external changes** / **Use my changes** choice. Cancel keeps the draft; another concurrent change invalidates the resolution and requires confirmation again. Conflict responses show field names, never credential values.

Saving the current direct provider applies edited provider-owned fields to Claude Code. User settings and unrelated live values are preserved. An inactive provider or a provider edited while local routing is active is saved to the database only. Legacy save APIs keep their existing strict checks. Unfinished provider operations must be recovered first.

**Preview save** lists the target database/live paths and field names that saving would change, without returning credential values. Existing-provider preview and save share the same merge planner and routing context. New-provider preview is record-only. Preview creates no backups, journals, or database changes; editing the draft clears it. Save rechecks the latest state. This previews saving the editor, not selecting a different provider.

## Model discovery and mappings

1. Click **Fetch models** to load a catalog and open the searchable picker. Fetching leaves all four mappings unchanged.
2. Choose Main, Haiku, Sonnet or Opus, then **Apply model** to update only that role in the draft. Cancel closes the picker without applying a model; its fetched catalog remains available in the editor.
3. Use the arrow beside any mapping to browse the same catalog without fetching again. Each menu opens with all candidates regardless of the current input text. Its separate search covers the entire catalog; **Show more** reveals additional batches of 100 results. Choosing a model changes only that mapping in the draft.
4. **Choose model** reopens the large picker using the cached list. Use **Refresh models** inside it to request a new catalog. **Save** explicitly persists the draft.

Each mapping remains a manually editable input. A configured ID missing from the
catalog stays available in its menu. Menus align to their inputs, open above or
below as space permits, and scroll within the viewport. Fetching does not prove
generation access and never replaces all mappings automatically.

The catalog is held only while the editor is open. Changing the endpoint,
credentials, authentication mode or OAuth account invalidates it; late responses
from the previous context are ignored. Changing a model mapping does not clear
the list. A failed refresh retains the previous list for the same context.

For Codex OAuth, the picker reads the selected connected account's Codex catalog;
**Default account** follows the current default. No API key, custom endpoint or
running local route is needed. The backend resolves and refreshes the managed
token and uses the fixed Codex catalog endpoint. Switching accounts discards the
previous list and ignores its pending responses. A missing or disconnected bound
account does not fall back to another account. Reconnect on the OAuth page if the
token is unavailable; manual model IDs remain editable after a failed fetch.

## Diagnostics

For Codex OAuth, **Diagnostics → Check OAuth connection** reads the model catalog
using the selected managed account or the current default. No API key or running
local route is needed. The result shows whether the catalog was reached and
whether each configured mapping appears in it. Catalog presence is separate from
generation access; manually configured models can remain valid when unlisted.
Changes to the selected/default account clear previous results and ignore pending
responses. Editing mappings reuses the checked catalog for that account. Connection
errors provide the server's sanitized guidance. This check leaves the draft and
saved provider unchanged.

The diagnostics catalog is a separate snapshot from the **Fetch models** picker.
Checking the connection does not populate the mapping menus; fetching models does
not mark Diagnostics as checked. For catalog comparisons, a trailing `[1m]`
suffix is ignored, matching the local route's model normalization.

| Provider mode | Action | What the result means |
|---------------|--------|-----------------------|
| Codex OAuth | **Check OAuth connection** | The managed account can reach its catalog; mappings are shown as **Listed in catalog**, **Not listed in catalog** or **Unset**. |
| API key / custom endpoint | **Detect endpoint type** | Identifies the endpoint's wire protocol. It does not certify generation access. |

OAuth Diagnostics does not send a generation request, change the draft or save
the provider. Verify generation separately through the running local route.

## Protocol and account preservation

The editor resolves protocol metadata, legacy `api_format`, and provider defaults
in the same order as the local route. A name-only edit keeps the effective
protocol, top-level settings, authentication adapter and account binding.
An explicit protocol edit updates `meta.apiFormat`, which takes precedence over
legacy fields. Codex OAuth is always saved as OpenAI Responses and its protocol
selector is read-only. An imported record identified by its OAuth mode or Codex
binding is canonicalized by the actual save path, including a name-only edit;
the chosen account and unrelated metadata remain intact. Saving an unchanged
OAuth form also canonicalizes it while retaining the original conflict baseline.
Selecting the default account intentionally clears the explicit binding;
other edits keep it.

Gemini Native can be retained for configuration/model discovery. Its local-route
limitation is shown in the editor and route selector. See
[protocol compatibility](protocol-compatibility.md) for conversion boundaries.

See [frontend design conventions](frontend-design.md#provider-editor) for component
ownership, menu geometry and interaction requirements.
