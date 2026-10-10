# Upgrading / 升级指南

[Documentation](README.md) · [Configuration recovery](configuration-recovery.md)

This guide describes the current upgrade implementation. A merged PR does not itself publish a release. The installer requires a stable release archive, matching binary version and `.sha256` asset; older releases without checksums are rejected before replacing installed files.

## First upgrade from an older release / 旧版首次升级

On Linux x86_64 or macOS Intel / Apple Silicon, an older binary without `update` needs the installer once:

```bash
curl -fsSL https://raw.githubusercontent.com/huangbogeng/cc-switch-ui/main/install.sh | bash
```

之后安装的版本包含 `update` 时，直接使用：

```bash
cc-switch-ui update
cc-switch-ui version
cc-switch-ui status
```

`update` targets its own release installation, skips the same stable version, and does not downgrade a newer installed version. `cc-switch-ui update --force` reinstalls the same stable release; it still does not bypass checksum/version validation or downgrade protection. Source build directories cannot be updated in place this way.

The default application directory is `~/.local/share/cc-switch-ui`. A custom installer location uses an absolute `CC_SWITCH_INSTALL_DIR`; future `update` calls use the executable's directory. User data stays in `~/.cc-switch`.

Windows upgrades remain manual: retain the complete profile, download the new release and replace the executable while the service is stopped. Other architectures need a source build. Do not run an older binary against a database migrated by a newer release.

## What an upgrade does / 自动流程

1. Download into staging and verify the archive checksum, contents and binary version before stopping the service.
2. Identify and stop a managed process from this installation; reject a foreign or still-responding service.
3. Retain the old program/frontend and archive the complete `~/.cc-switch` profile privately.
4. Start the new executable. For an existing older database, create a consistent pre-migration SQLite snapshot, then migrate in a transaction.
5. Verify that the new process binds successfully and responds to the health check. Restart a previously running service; leave an idle installation stopped after validation.

迁移无需手动导出 Provider。数据库备份失败会在迁移前停止启动，迁移失败会回滚 schema 事务，较新且不支持的 schema 会被拒绝。成功迁移后的重复启动不会再产生该次 schema 升级快照。兼容范围是旧版 **CC Switch UI**，不包括直接共享上游桌面版的数据库。

The installer preserves supported explicit launch options (`--host`, `--port`, `--proxy-port`, `--admin-token`, `--ui-dir`) or uses saved CLI defaults and the updating shell's environment. Ambiguous arguments and linked installation/data directories are rejected. Use one server per profile.

A randomly generated token from the old process cannot be recovered. The successful update prints startup details; its private `startup.log` contains the new token. A token explicitly passed as an option or exported in the updating shell can be retained. Never attach that log publicly without removing credentials.

## Backup locations and scope

| Backup | Default location | Scope |
|--------|------------------|-------|
| Installer recovery set | `~/.local/share/cc-switch-upgrade-backups/<version>.<suffix>/` | `previous/` program/frontend, `data.tar.gz` complete profile, private startup/rollback logs |
| Pre-migration database | `~/.cc-switch/backups/upgrades/schema-<old>-to-<new>-<UUID>.db` | Original SQLite schema and committed WAL data |
| Backup Center database | `~/.cc-switch/backups/database/` | Database records; separate from migration snapshots |
| First-write live configuration | `~/.cc-switch/backups/live-first-write/` | Original guarded configuration files and source markers |
| Skills installation history | `~/.claude/skills/.cc-switch-backups/` | Previous managed installed directories; source history has its own backup directory |

自定义安装目录时，安装恢复集位于安装目录的父目录下 `cc-switch-upgrade-backups/`。这些备份包含凭证，保留后需要自行管理。数据库快照不包含 OAuth 存储、Skills 源文件、CLI/device 设置或 live 文件。完整 profile 归档也不等于当前 `~/.claude` 文件的完整备份。

## Failed validation and rollback / 失败回滚

If startup or health validation fails, the installer attempts to stop the new process, retain its failed program/data, restore the previous program/profile and restart a previously running service. It reports the recovery directory. Inspect `startup.log` and, when present, `rollback.log` locally.

回滚也可能因进程无法停止、权限或磁盘问题失败；脚本会报告需要处理的步骤和恢复副本。先解决进程或文件系统问题，再在所有写入程序停止后恢复对应程序与完整 profile。不要只替换 `.db` 并遗留旧 WAL 文件，也不要删除备份来绕过错误。live 配置文件的独立恢复见[恢复参考](configuration-recovery.md)。

Installer rollback concerns program/profile replacement. It does not promise a transaction over external Claude files or other processes. `clean` deletes primary records; it is not an upgrade or rollback command.

## Verify after upgrading / 升级后确认

Open the UI with the current token, review Direct Config and Local Route, and resolve any pending recovery before restarting routing. Check providers, OAuth accounts, MCP and Skills, then make a small request through the intended route. Keep the recovery set until satisfied with the upgrade.
