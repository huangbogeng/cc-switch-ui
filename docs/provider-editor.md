# Provider editor

Editing an existing provider merges only fields changed from the opened record. Changes made elsewhere to other fields are preserved. Conflicting record or live fields require an explicit **Keep external changes** / **Use my changes** choice. Cancel keeps the draft; another concurrent change invalidates the resolution and requires confirmation again. Conflict responses show field names, never credential values.

Saving the current direct provider applies edited provider-owned fields to Claude Code. User settings and unrelated live values are preserved. An inactive provider or a provider edited while local routing is active is saved to the database only. Legacy save APIs keep their existing strict checks. Unfinished provider operations must be recovered first.

**Preview save** lists the target database/live paths and field names that saving would change, without returning credential values. Existing-provider preview and save share the same merge planner and routing context. New-provider preview is record-only. Preview creates no backups, journals, or database changes; editing the draft clears it. Save rechecks the latest state. This previews saving the editor, not selecting a different provider.

**Fetch models** opens a searchable picker without changing the draft. Choose Main, Haiku, Sonnet or Opus and explicitly **Apply model** to update only that role. Cancel leaves the draft unchanged. Unlisted configured models and manual IDs remain available; fetching does not prove model access. Editing the endpoint or credentials invalidates its cached list, and stale requests are ignored. Failed refreshes retain the previous list for the same endpoint. Choices render in batches of 100; inline suggestions are limited to 100. Save remains explicit.
