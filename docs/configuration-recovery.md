# Configuration write protection and recovery

Claude settings and MCP edits use a JSON patch against the current file. Invalid JSON or a non-object root stops the operation. Before publishing, the writer compares the current bytes with the bytes it read. If another process edited the file, it reapplies the patch to the new contents, up to three attempts. Persistent conflicts leave the external contents intact and report an error. Retry the operation after the external writer has finished.

In-process JSON patches share a lock. External programs do not participate in that lock: there remains a small race between the final comparison and the rename. This is optimistic conflict detection, not a filesystem transaction across programs or multiple files.

The first guarded write retains the original bytes under `~/.cc-switch/backups/live-first-write/`. Each target is identified by a SHA-256 of its absolute path. `<hash>.source.json` records the original path and whether the file existed; `<hash>.original` holds the original bytes when it did. Both backup files are private on Unix. Repeated edits preserve that first snapshot. A no-op or invalid JSON creates no backup. If the first attempt conflicts, the backup still reflects the original bytes observed before that attempt.

To manually recover a configuration, stop CC Switch and other programs that write that file, locate its `.source.json`, and copy the associated `.original` to the recorded path. Keep a copy of the current file first. The first-write backup is a full-file snapshot, so restoring it also restores unrelated fields to their original state. A marker with `existed: false` has no original file to recover.

Skills use per-directory staging and content digests. Unchanged contents skip replacement and backup creation. Changed source or target contents detected before publication abort the replacement. Old installations remain under `~/.claude/skills/.cc-switch-backups/`; restoring a backup also preserves the previous source under `~/.cc-switch/skills/.cc-switch-backups/`.

The Skills page can load these backups and restore source files. The selected backup is retained. Existing records keep their enabled state; a deleted record is recreated and enabled. Enabled records are then synced to Claude Code. If the source was restored but syncing failed, the UI reports the partial outcome and the user can retry Sync. Backups require manual cleanup. Dates shown in the UI are content modification times, not the time the backup was created.

Provider selection and Route Target are independent. While the local route runs, selecting a provider chooses the Direct Config to apply after stopping; it does not change the active Route Target. Stopping restores that Direct Config and preserves user settings edited during takeover. With no direct selection, it restores the owned fields from the pre-route backup. Providers referenced by either selection cannot be deleted until another provider is selected.

An interrupted route retains its backup and takeover key snapshot in SQLite. The Providers page offers **Restore Direct Config** before another start. Old backups without a takeover snapshot still restore connection fields, but cannot identify all old provider-exclusive compatibility fields for cleanup. A failed port bind never writes live config. A failed live write stops the newly bound listener.
