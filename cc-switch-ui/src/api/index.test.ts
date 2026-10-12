import { afterEach, expect, it, vi } from 'vitest';
import { ApiError, saveMcpServer, restoreSkillBackup, fetchCodexModels } from './index';

afterEach(() => vi.unstubAllGlobals());

it('sends only the account selection to the dedicated managed Codex catalog route', async () => {
  const fetch = vi.fn().mockImplementation(async () => new Response(JSON.stringify({ models: [{ id: 'fixture-model' }] })));
  vi.stubGlobal('fetch', fetch);
  await fetchCodexModels('fixture-account');
  expect(fetch).toHaveBeenCalledWith('/api/providers/codex-models', expect.objectContaining({ method: 'POST', body: JSON.stringify({ accountId: 'fixture-account' }) }));
  await fetchCodexModels();
  expect(fetch).toHaveBeenLastCalledWith('/api/providers/codex-models', expect.objectContaining({ body: '{}' }));
});

it('preserves partial-save status on failed HTTP responses', async () => {
  vi.stubGlobal('fetch', vi.fn().mockResolvedValue(new Response(JSON.stringify({
    error: 'Record saved, but sync failed. Retry Sync.', saved: true,
  }), { status: 500 })));
  const error = await saveMcpServer({ id: 'test', name: 'Test', appType: 'claude_code', enabled: true, serverSpec: {} }).catch((e: unknown) => e);
  expect(error).toBeInstanceOf(ApiError);
  expect((error as ApiError).saved).toBe(true);
  expect((error as Error).message).toContain('Retry Sync');
});

it('sends backup IDs as JSON to a route that cannot collide with skill IDs', async () => {
  const fetch = vi.fn().mockResolvedValue(new Response(JSON.stringify({ success: true, directory: 'test', enabled: true })));
  vi.stubGlobal('fetch', fetch);
  const backupId = 'live:test-00000000-0000-0000-0000-000000000000';
  await restoreSkillBackup(backupId);
  expect(fetch).toHaveBeenCalledWith('/api/skill-backups/restore', expect.objectContaining({ method: 'POST', body: JSON.stringify({ backupId }) }));
});
