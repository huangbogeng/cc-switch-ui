# Troubleshooting / 常见问题

[Documentation](README.md) · [Recovery reference](configuration-recovery.md)

## Cannot open the UI or sign in

Run `cc-switch-ui status` and `cc-switch-ui doctor`. Use the admin URL shown at startup, including `/ui/`; the proxy port is not the Web UI port. For custom admin ports, use `status --port <port>` when needed.

启动日志中的 Admin Token 用于网页登录。进程重启后自动生成的 token 可能变化；升级时请使用新日志中显示的 token。固定 token 可通过启动环境变量或 `--admin-token` 设置。不要公开包含 token 的日志。

For headless machines, a tunnel keeps the default loopback binding:

```bash
ssh -L 5007:127.0.0.1:5007 user@your-server
```

Then open `http://127.0.0.1:5007/ui/` locally. Binding `--host 0.0.0.0` exposes the administration interface beyond localhost; use a trusted network and protect the token.

## Route status is unavailable or recovery is required

Unavailable status means the UI could not confirm the backend state. It does not mean the route is stopped. Check service health and reload before attempting route operations.

**Restore Direct Config** indicates an interrupted takeover retains a backup. Resolve that recovery before **Start Route**. Port bind failure does not write live config; a failed takeover write stops the newly bound listener. If a selected provider differs from **Route Target**, that is expected: one controls the configuration after stopping; the other controls current proxy forwarding.

## Codex authorization succeeds but requests fail

Check that the provider uses Codex OAuth, that **ChatGPT Account** points to a connected account or the intended default, and that **Route Target** is the Codex provider with the route running. Authorization alone does not activate routing. Review the request error in **Usage**; reconnect if authorization has expired, and verify the requested model is available to that account.

Codex 订阅路径不需要 API Key，但仍受账号权限和上游限额约束。不要在未启动本地路由时直接把 Claude Code 指向 ChatGPT 的 Codex API。

## Model choices or OAuth Diagnostics are unavailable

| Symptom | Check or action / 处理 |
|---------|-----------------------|
| OAuth **Fetch models** or **Check OAuth connection** is disabled | Connect on **OAuth → ChatGPT Codex**, then choose a connected account or a connected default in the editor. Wait for a current operation to finish. Neither action requires an API key or a running route. |
| A bound account is missing or disconnected | Reconnect or deliberately select another account. A missing explicit binding does not fall back to the default account. |
| Mapping arrows have no choices | Use **Fetch models** first. Closing its picker retains choices in all four menus. **Check OAuth connection** has a separate result and does not populate them. An empty catalog leaves manual inputs available. |
| Choices disappear after changing accounts or reopening the editor | This is expected: catalogs belong to the open editor's connection context. Fetch again for the current account; configured mappings remain unchanged. |
| The configured model is absent or **Not listed in catalog** | Keep or type the intended model ID. Catalog absence does not prove that generation is unavailable. Check an actual request through the local route. |
| Refresh fails but old choices remain | The last successful list is retained only for the same discovery context. Resolve the displayed error and retry **Refresh models**; old choices are not evidence of a successful refresh. |
| OAuth check succeeds but requests fail | Catalog reachability does not test generation, account limits or route activation. Check **Route Target**, route state and the request error in **Usage**. |

If an upgraded UI still shows unreadable native options or the previous editor
controls, reload the page to load the installed frontend. Check the installed
version against [release notes](README.md); these guides also describe repository
changes that may not be in your release. If the issue remains, report the browser,
version and affected control with a redacted screenshot.

详见[服务商编辑说明](provider-editor.md)；模型获取、连接检查和保存是独立操作，
不会自动替换四个映射或自动保存配置。

## Deletion recovery keeps appearing

A pending cleanup means the managed record was deleted but live cleanup did not finish. The panel intentionally survives navigation and restart until the operation is resolved. Merely dismissing a toast does not clear that operation.

- **Retry live cleanup** retries the captured cleanup after fixing its error. A changed target or configuration path stops cleanup to preserve current files.
- **Keep current files** confirms that current live contents should remain unmanaged and possibly active; it clears the pending intent without recreating the record.

如果旧的清理目标已被外部程序修改，先核对内容，再决定保留文件还是恢复原状态后重试。未解决的删除会阻止 MCP/Skills 写入和数据库备份/恢复。成功处理后，面板应消失；若仍显示，刷新确认服务返回的状态。

## Removed items appear again

Current removal records are remembered and skipped by Import. Use **Restore removed** to bring a chosen item back disabled. **Sync** does not discover new records. If an item was removed before this feature existed, the application has no retained removal decision; removing it once with the new version records that decision. A restored database may also contain older management/removal states. Plugin originals remain installed independently of managed copies.

## Save succeeded but Sync failed

The record may already be saved. Import/Sync item reports distinguish completed, skipped and failed items; HTTP success alone is not proof that every item succeeded. Review failed details, fix invalid JSON, a missing Skills source or permissions, then retry **Sync**. Earlier completed changes remain. Import itself does not sync.

## Configuration conflict or interrupted provider operation

For editor field conflicts, choose **Keep external changes** or **Use my changes** after reviewing the affected field names; cancelling retains the draft. Another external change invalidates that confirmation. **Reload from file** is available when you deliberately want to replace the draft with current values. **Recover operation** resumes a journaled provider action after its underlying filesystem/database error is fixed. Do not delete `provider-operation.json` to bypass recovery; it contains credentials and planned state. Invalid live JSON is rejected instead of overwritten.

## Database restore prevents startup

A staged restore waits for restart and keeps a safety snapshot. If validation failed before application began, `cc-switch-ui cancel-restore` can cancel it without a running server. Once application began, fix the reported error and restart to complete recovery. After restoration, select a provider explicitly and review/resync MCP/Skills; database restoration does not restore live files.

## Collect useful information for a report

Include the installed version, platform, failing action, whether the route is active, and a redacted error. Mention a recent upgrade or pending recovery. Do not upload the database, profile archives, OAuth stores, provider journals or unredacted logs: they may contain credentials.
