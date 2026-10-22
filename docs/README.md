# Documentation

[English overview](../README.md) · [中文概览](../README_zh.md)

These guides describe the current repository. Check the release notes before assuming a published binary contains every feature. The Web UI uses English labels; the guides below include Chinese instructions using those same labels.

Release notes: [v0.3.4 / 更新说明](releases/v0.3.4.md). Previous releases: [v0.3.3](releases/v0.3.3.md), [v0.3.2](releases/v0.3.2.md), [v0.3.1](releases/v0.3.1.md). Check the [published releases](https://github.com/huangbogeng/cc-switch-ui/releases) for availability.

| Guide | What you will find / 内容 |
|-------|---------------------------|
| [User guide / 使用指南](user-guide.md) | Providers, Codex subscription routing, OAuth, MCP, Skills and usage |
| [Upgrading / 升级指南](upgrading.md) | First upgrade from an older release, `update`, automatic backups and rollback |
| [Provider editor / 服务商编辑](provider-editor.md) | Model discovery and mappings, OAuth Diagnostics, account preservation and concurrent saves |
| [Troubleshooting / 常见问题](troubleshooting.md) | Login, model discovery, OAuth checks, route recovery, deletion cleanup and partial sync failures |
| [Configuration protection and recovery](configuration-recovery.md) | Detailed ownership, conflict detection, journals, snapshots and restore semantics |
| [Development / 开发](development.md) | Toolchain, local servers, isolated workflow tests and upgrade regressions |
| [Frontend design](frontend-design.md) | Shared layouts, dropdown contrast and alignment, editor state ownership, dialogs and feedback |

Start with the user guide for normal operations. See [protocol compatibility](protocol-compatibility.md) for local-route capabilities and prompt preservation. Use the recovery reference when an operation reports a conflict, partial result or pending recovery.

Run one server per profile. The default profile is `~/.cc-switch`; Claude Code's live files are outside it. A database snapshot, a profile archive and a live configuration snapshot have different scopes. The [upgrade guide](upgrading.md#backup-locations-and-scope) compares them.
