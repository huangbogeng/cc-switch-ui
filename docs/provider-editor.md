# Provider editor

Editing an existing provider merges only fields changed from the opened record. Changes made elsewhere to other fields are preserved. Conflicting record or live fields require an explicit **Keep external changes** / **Use my changes** choice. Cancel keeps the draft; another concurrent change invalidates the resolution and requires confirmation again. Conflict responses show field names, never credential values.

Saving the current direct provider applies edited provider-owned fields to Claude Code. User settings and unrelated live values are preserved. An inactive provider or a provider edited while local routing is active is saved to the database only. Legacy save APIs keep their existing strict checks. Unfinished provider operations must be recovered first.

**Preview save** lists the target database/live paths and field names that saving would change, without returning credential values. Existing-provider preview and save share the same merge planner and routing context. New-provider preview is record-only. Preview creates no backups, journals, or database changes; editing the draft clears it. Save rechecks the latest state. This previews saving the editor, not selecting a different provider.
