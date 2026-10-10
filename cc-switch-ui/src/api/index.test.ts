import { afterEach, expect, it, vi } from 'vitest';
import { ApiError, saveMcpServer } from './index';

afterEach(() => vi.unstubAllGlobals());

it('preserves partial-save status on failed HTTP responses', async () => {
  vi.stubGlobal('fetch', vi.fn().mockResolvedValue(new Response(JSON.stringify({
    error: 'Record saved, but sync failed. Retry Sync.', saved: true,
  }), { status: 500 })));
  const error = await saveMcpServer({ id: 'test', name: 'Test', appType: 'claude_code', enabled: true, serverSpec: {} }).catch((e: unknown) => e);
  expect(error).toBeInstanceOf(ApiError);
  expect((error as ApiError).saved).toBe(true);
  expect((error as Error).message).toContain('Retry Sync');
});
