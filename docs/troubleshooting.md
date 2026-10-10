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

## Deletion recovery keeps appearing

A pending cleanup means the managed record was deleted but live cleanup did not finish. The panel intentionally survives navigation and restart until the operation is resolved. Merely dismissing a toast does not clear that operation.

- **Retry live cleanup** retries the captured cleanup after fixing its error. A changed target or configuration path stops cleanup to preserve current files.
- **Keep current files** confirms that current live contents should remain unmanaged and possibly active; it clears the pending intent without recreating the record.

如果旧的清理目标已被外部程序修改，先核对内容，再决定保留文件还是恢复原状态后重试。未解决的删除会阻止 MCP/Skills 写入和数据库备份/恢复。成功处理后，面板应消失；若仍显示，刷新确认服务返回的状态。

## Save succeeded but Sync failed

The record may already be saved. Import/Sync item reports distinguish completed, skipped and failed items; HTTP success alone is not proof that every item succeeded. Review failed details, fix invalid JSON, a missing Skills source or permissions, then retry **Sync**. Earlier completed changes remain. Import itself does not sync.

## Configuration conflict or interrupted provider operation

Reload the editor and compare external changes before saving again. **Recover operation** resumes a journaled provider action after its underlying filesystem/database error is fixed. Do not delete `provider-operation.json` to bypass recovery; it contains credentials and planned state. Invalid live JSON is rejected instead of overwritten.

## Database restore prevents startup

A staged restore waits for restart and keeps a safety snapshot. If validation failed before application began, `cc-switch-ui cancel-restore` can cancel it without a running server. Once application began, fix the reported error and restart to complete recovery. After restoration, select a provider explicitly and review/resync MCP/Skills; database restoration does not restore live files.

## Collect useful information for a report

Include the installed version, platform, failing action, whether the route is active, and a redacted error. Mention a recent upgrade or pending recovery. Do not upload the database, profile archives, OAuth stores, provider journals or unredacted logs: they may contain credentials.
