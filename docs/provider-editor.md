# Provider editor

Editing an existing provider merges only fields changed from the opened record. Changes made elsewhere to other fields are preserved. Conflicting record or live fields require an explicit **Keep external changes** / **Use my changes** choice. Cancel keeps the draft; another concurrent change invalidates the resolution and requires confirmation again. Conflict responses show field names, never credential values.

Saving the current direct provider applies edited provider-owned fields to Claude Code. User settings and unrelated live values are preserved. An inactive provider or a provider edited while local routing is active is saved to the database only. Legacy save APIs keep their existing strict checks. Unfinished provider operations must be recovered first.
