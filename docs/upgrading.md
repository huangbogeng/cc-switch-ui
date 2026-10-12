# Upgrading / 升级指南

[Documentation](README.md) · [Configuration recovery](configuration-recovery.md)

This guide describes the current upgrade implementation. A merged PR does not itself publish a release. The installer requires a stable release archive, matching binary version and `.sha256` asset; older releases without checksums are rejected before replacing installed files.

## Upgrading to v0.3.3 / 升级到 v0.3.3

v0.3.3 retains database **schema 5**, as used by v0.3.1 and v0.3.2. Existing
providers, model mappings and OAuth bindings remain intact. Once the release is
published, Linux/macOS release installations can use `cc-switch-ui update`;
Windows installations replace the release package while the service is stopped.

Reload the UI after upgrading. Open a Codex provider, review **ChatGPT Account**,
then use **Fetch models** to populate all four mapping menus. **Diagnostics →
Check OAuth connection** checks catalog reachability separately; neither action
requires an API key or a running local route. Fetching and checking do not change
saved mappings. Select any intended models, then explicitly **Save**. Verify
generation through **Route Target → Start Route** and a request from Claude Code.
See the [editor guide](provider-editor.md) for cache and account behavior.

The Linux CI upgrade matrix includes published v0.3.2 alongside v0.1.9, v0.2.0
and v0.3.1. The v0.3.1/v0.3.2 cases prepare profiles with the published program
and exercise its own updater with idle/running installations, synthetic account
bindings and remembered removals. Tests use isolated profiles and mock downloads.

本次不新增数据库迁移，不会因获取模型而改写已有映射或账号绑定。升级前保留完整
profile 与重要 live 配置的备份，升级后刷新页面并检查账号、模型和路由。
获取模型和连接诊断成功只证明目录可访问，生成权限仍需实际请求确认。

## Upgrading to v0.3.2 / 升级到 v0.3.2

v0.3.2 retains database **schema 5**, the same as v0.3.1; upgrading that profile does not require another schema migration. The installer still retains the previous program and complete profile. Older schemas receive a private pre-migration database snapshot as before.

After upgrading, review the effective API Format and Codex account binding, then choose **Route Target → Start Route**. Target-selection errors now stop startup instead of starting a previous target. Gemini Native and unknown protocols remain unavailable for local routing; stopping/restoring a route remains available. See [protocol compatibility](protocol-compatibility.md) before using converted tools or reasoning history.

The OrcaRouter built-in preset, initial database seed and bundled brand asset are removed in v0.3.2. Existing provider records and their credentials are retained; manage any retained record as a custom provider. Existing Codex providers keep their model IDs; the `gpt-6.1-sol` default applies to newly created frontend presets, and access depends on the account.

Linux installer process checks now recognize PATH, relative and alias launches using the running executable's identity, with exact argument boundaries. An already-installed v0.3.1 updater embeds its older installer logic until upgraded. If it refuses a valid PATH-launched service, use the versioned installer instead:

```bash
curl -fsSL https://raw.githubusercontent.com/huangbogeng/cc-switch-ui/v0.3.2/install.sh | bash
```

The Linux upgrade suite also starts the published v0.3.1 program to prepare a schema-5 profile, verifies account bindings and remembered removals survive replacement/rollback, and exercises that published program's own `update` command with idle and running installations. This uses isolated profiles, synthetic credentials and mock downloads.

升级前备份完整 `~/.cc-switch`，另存重要 live 配置。v0.3.1 → v0.3.2 不增加数据库 schema；保留旧程序和数据副本，验证路由、账号和待恢复操作后再清理备份。旧版 updater 的 PATH 识别问题可通过上方固定版本安装脚本绕过。已有服务商记录和模型设置不会因为预设调整而被删除或改写。

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

On Linux, the v0.3.2 installer verifies `/proc/PID/exe` against the installed executable and reads NUL-separated arguments. macOS retains the conservative command-path check, so ambiguous launches or arguments may require stopping the service before updating.

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
