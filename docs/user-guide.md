# User guide / 使用指南

[Documentation](README.md) · [Upgrade guide](upgrading.md) · [Troubleshooting](troubleshooting.md)

## Start and sign in / 启动与登录

```bash
cc-switch-ui start
```

Open `http://127.0.0.1:5007/ui/` and enter the **Admin Token** from the startup log. The admin port defaults to `5007`; the local route port defaults to `15721`. The proxy listener is started through the UI separately from the admin service.

启动后在浏览器中打开上述地址，输入日志里的 Admin Token。需要固定 token 时，可以在启动服务的终端设置 `CC_SWITCH_ADMIN_TOKEN`。更改管理端口后，应使用对应 URL；改变 host/port 的启动参数会写入 CLI 配置。

All UI labels are English. **Dashboard** summarizes Direct Config, Local Route and recent usage; **Providers** manages configurations and route targets. Operation notices appear at the side and disappear automatically. Hover or focus a notice to pause its timer; recovery actions remain visible until resolved.

## Direct Config and Route Target / 两种选择

| Control | Effect / 作用 |
|---------|---------------|
| Provider row → **Select** | Chooses Direct Config. With the route stopped, applies provider-owned fields to Claude Code. While routing, records the provider to apply after stopping. |
| **Local Route → Route Target** | Chooses which provider receives requests through the local proxy. Independent of Direct Config. |
| **Start Route** | Binds the proxy and takes over Claude Code's connection settings. |
| **Stop Route** | Stops routing and restores Direct Config while preserving user-owned settings. |

切换服务商与切换路由目标是两个独立操作。路由运行时点击 **Select** 不会改变当前转发目标；请使用 **Route Target**。停止路由后，再检查 Dashboard / Providers 显示的直接配置。

Local routing supports Anthropic, OpenAI Chat and OpenAI Responses endpoints.
Gemini native conversion is not implemented and cannot be selected as a route.
For native Claude tools and thinking-history behavior, choose an
Anthropic-compatible endpoint. See [protocol compatibility](protocol-compatibility.md).

Unsupported targets are labelled **local route unavailable** and cannot be
selected or started. If an existing target becomes unsupported, choose a
supported replacement; **Stop Route** and **Restore Direct Config** remain
available. Failed target changes display the server error and do not start a
previous target.

## Add a provider / 添加服务商

1. Open **Providers → Add Provider** and choose a preset or custom configuration.
2. Review the endpoint and model fields; enter an API key when required.
3. For compatible custom endpoints, use **Detect endpoint type** and **Fetch models** when available.
4. Save, then choose **Select** for direct configuration or use **Route Target → Start Route** for protocol conversion.

列表搜索匹配名称、ID、端点域名、认证方式和备注。编辑当前直接配置会同时尝试更新 live 文件；发生冲突时先重新读取并核对其他程序的修改。未选中的配置只更新记录。正在运行的路由会保持接管状态。

The **Claude Official** preset clears third-party connection settings and relies on your existing Claude Code login. It does not create an Anthropic subscription or authenticate Claude Code for you.

See the [provider editor guide](provider-editor.md) for concurrent edits and save behavior.

## Codex subscription without an API key / Codex 订阅路由

1. Open **OAuth → ChatGPT Codex → Connect**. Follow the displayed verification URL and device code.
2. Return to the UI and wait for authorization to finish. For multiple accounts, choose the default or bind a specific account in the provider editor.
3. Open **Providers → Add Provider**, choose **Codex**, and review **ChatGPT Account** and the model settings. Leave **Default account** to follow the default OAuth account.
4. Save the provider. Choose it in **Local Route → Route Target**, then click **Start Route**.
5. Confirm the route is running, then send a request from Claude Code. Review **Usage** for the request result.

这条路径使用已授权的 ChatGPT OAuth 账号，由本地服务完成协议转换和凭证注入，Provider 表单不需要填写 API Key。它依赖本地路由服务运行；账号权限、模型可用性和限额仍由上游决定。不要把 ChatGPT 端点当作 Claude Code 原生 Anthropic 端点直接配置使用。

The Codex preset uses `gpt-6.1-sol` for new configurations. Saved providers keep
their configured models when reopened; available models and access depend on the
upstream account. **API Format** is fixed to OpenAI Responses for Codex OAuth.
**Fetch models** remains disabled for this authentication mode; enter an available
model ID manually. Editing a bound account preserves that binding, while selecting
**Default account** deliberately removes the explicit account selection.

For this route, mid-conversation `system` messages become `developer` inputs with
their text and position preserved. The service requests encrypted reasoning for
stateless tool loops; support still depends on the upstream model. If credentials
are rejected, verify the provider's account binding and reconnect through the
OAuth page. See [protocol compatibility](protocol-compatibility.md) for content
handling, failure behavior and unsupported capabilities.

**OAuth** also manages GitHub Copilot accounts and supports a GitHub Enterprise host where available. Connecting an account alone does not change the active route. Check the provider authentication mode and selected target before sending traffic.

## MCP and Skills / 管理与同步

**Import** opens a read-only discovery dialog. Select the items to add, then click **Import selected**; cancelling changes no records or files. Existing enabled states are preserved. **Import and enable** is checked by default and applies only newly imported items. Uncheck it to save records only, then use **Sync** when ready. Applying failures retain saved records and report how to retry. Saving or toggling managed records applies the desired state to Claude Code. Unmanaged MCP entries and Skills directories are preserved.

导入和同步会报告每一项的结果。部分失败不代表全部回滚：已完成的项目会保留，修复失败原因后重试 **Sync**。Skills 内容不变时跳过替换和重复备份。

The row switch enables or disables a managed item. **Remove** deletes its managed record, cleans the captured live entry/directory, and remembers that decision so ordinary Import will skip it, including after restart. **Recovery → Restore removed** lists retained removal records and restores chosen items as disabled; enable them explicitly when ready. Skill source files and plugin originals remain available. Plugin controls affect the managed copy, not the original plugin installation.

Removing a record may leave pending live cleanup. **Deletion recovery → Retry live cleanup** completes it. **Keep current files** clears the intent after confirmation and leaves the files unmanaged, potentially still active. See [deletion recovery](troubleshooting.md#deletion-recovery-keeps-appearing).

## Settings and backups / 用户配置与备份

**Settings → Claude Code User Settings** edits **API Timeout MS** and **Disable nonessential traffic** directly in the live user configuration. These fields are independent of provider selection. Blank timeout and **Use Claude Code default** remove overrides. **Reload from file** refreshes external changes; **Restore user settings** restores only these fields from the first-write snapshot.

**Settings → Backup Center** groups manual database snapshots, first-write configuration snapshots and Skills backups. **Restore on restart** stages a database restore; after restarting, select a provider explicitly and review MCP/Skills before syncing. A pending restore blocks starting a local route. Read the [recovery reference](configuration-recovery.md) before replacing a complete live configuration file.

**Backup Center → Inspect backup storage** inventories known database, migration, original-file, Skills and release-upgrade locations. It shows paths, logical bytes, top-level entry counts and the latest modification time without reading backup contents. Missing, partial and unavailable locations are distinct. Recovery remains in the existing panels or the [upgrade recovery guide](configuration-recovery.md); inventory does not certify restorability.

## Usage / 用量

The **Usage** page distinguishes local proxy logs from imported Claude Code session data. Choose the data source and date range before comparing totals. Session import is optional; requests made outside the local route will not appear as proxy logs. Request details and model/provider breakdowns help locate upstream failures.

用量页面显示的是所选数据来源下的记录，不应当作订阅剩余额度或服务商账单。模型价格配置可能影响成本估算；请同时核对服务商实际计费。

### Skill collection controls

Use the switch beside a collection title to enable or disable every managed skill in that collection. A centered switch and the enabled count indicate a partially enabled group; clicking it enables the whole group. Group controls include members hidden by search or pagination. Individual skill switches remain independent. The group state is saved together and synced once; if file sync fails, the saved state remains visible and Sync can retry it.

For the MCP/Skills action contract and upstream comparison, see [interaction decisions](mcp-skills-design.md).

On Linux, `update` recognizes running release binaries started through PATH, relative paths, or the bundled command aliases, and preserves argument boundaries when restarting. A PID referring to a different executable is rejected. macOS retains conservative command-path checks.

Claude Code User Settings compares only fields you edited. Unedited fields retain external changes. A conflicting edit offers **Keep external changes** or **Use my changes**; cancelling keeps the draft. If the file changes again during confirmation, review is required again.

In **Claude Code User Settings**, **Preview changes** reads the current live file and shows only the fields that saving would change. It uses the same merge and validation rules as Save. Preview creates no backups and changes no files; Save rechecks concurrent edits. Editing the draft clears the preview. This preview covers the two user settings in this panel.

Use **Settings → Configuration overrides → Check overrides** when a provider change appears ineffective. The read-only check reports supported Claude Code/Codex variable names and sources, never their values. It inspects the server environment and common shell files without executing them. Shell assignments are potential overrides; terminal/client environments can differ. Partial scan failures are listed. Nothing is automatically removed.
