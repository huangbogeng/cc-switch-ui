# MCP and Skills interaction decisions

Upstream comparison: `farion1231/cc-switch` main at `790ed800` (2026-10-10). References: [MCP panel](https://github.com/farion1231/cc-switch/blob/790ed800/src/components/mcp/UnifiedMcpPanel.tsx), [Skills panel](https://github.com/farion1231/cc-switch/blob/790ed800/src/components/skills/UnifiedSkillsPanel.tsx), [Skill import](https://github.com/farion1231/cc-switch/blob/790ed800/src/components/skills/SkillImportDialog.tsx), [MCP service](https://github.com/farion1231/cc-switch/blob/790ed800/src-tauri/src/services/mcp.rs), [Skill service](https://github.com/farion1231/cc-switch/blob/790ed800/src-tauri/src/services/skill.rs).

| Area | Upstream behavior | CC Switch UI decision |
| --- | --- | --- |
| Ownership | One managed store projects to several applications; per-app matrix switches | Explicitly manage Claude Code only; one switch per record and a collection switch for Skills |
| Discovery | Skill import lists discovered directories and their origins before application selection | Read-only scan lists metadata and source; nothing is selected by default; explicit selection saves records without syncing |
| Disable | Per-app enable flags control distribution while retaining managed records | Retain records and sources, update enabled state and sync the managed live entries |
| Remove | MCP removes managed records and live entries; Skill uninstall creates recoverable backups | Atomically save cleanup intent and a remembered removal, preserve Skill sources and back up live directories; normal Import skips removed identities |
| Restore | Skill backups are listed for explicit restoration | Restore removed records as disabled; live-directory backups remain a separate recovery operation |
| Bulk scope | Matrix bulk actions explicitly resolve current search/filter scope | A collection switch changes the entire named collection, including hidden search results; mixed state shows the full enabled count |
| Failure | Write locks, per-item/app failure feedback, and explicit resync/retry | Serialize backend mutations; refresh observed records after lost responses; retain failed import dialogs; distinguish saved state from failed live sync |
| Search | MCP uses an explicit field allow-list and excludes env/headers | Search name, ID, transport, command and URL origin; exclude args, env, headers, URL credentials/query/path |
| Layout | Consistent app columns, bounded dialogs and compact progress/actions | Shared right-aligned switches and action order, bounded import/MCP lists, floating operation reports and explicit load/retry states |

## User contract

- **Import** reads existing Claude Code/plugin files, then saves only explicitly selected records. Cancel is read-only. Existing states and removal decisions are preserved.
- **Enable / Disable** saves the desired state and applies it. If the record was saved but file sync failed, use **Sync** to retry applying that state; toggling again changes the desired state.
- **Sync** applies the current managed list. It does not discover or restore removed items.
- **Remove** cleans the managed live entry and remembers the decision. Plugin originals stay installed, so plugin-supplied instructions may still be available through the plugin itself.
- **Restore removed** validates retained sources/configuration, recreates disabled records and clears the removal decision atomically. It does not enable anything or write live configuration.
- **Restore a Skill backup** repairs a backed-up live/source directory. It is a different action from restoring a removed record. Review the [recovery reference](configuration-recovery.md).

A failed batch retains completed work and reports individual failures. Candidate rescans clear the previous selection; reselect the remaining candidates before retrying. At most 2,000 records can be submitted per request. Discovery lists render in batches of 80; MCP records render in batches of 60.

## Persistence limits

Schema 5 stores removal decisions inside database snapshots. Startup takes the existing pre-upgrade backup before migrating. Removals made before schema 5 cannot be reconstructed automatically. Restoring an older database snapshot restores its older removal decisions as well. Retained MCP recovery configuration stays private; discovery/removed-item endpoints return metadata only.
