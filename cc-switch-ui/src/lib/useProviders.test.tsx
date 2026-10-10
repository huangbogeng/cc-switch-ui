import { act, renderHook } from '@testing-library/react';
import { QueryClient, QueryClientProvider, focusManager } from '@tanstack/react-query';
import type { ReactNode } from 'react';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import * as api from '@/api';
import { useProviders, useCurrentProvider, useRouteStatus, useRefreshProviders } from './useProviders';

vi.mock('@/api', () => ({ listProviders: vi.fn(), getCurrentProviderId: vi.fn(), getProxyStatus: vi.fn() }));
let client: QueryClient;
beforeEach(() => {
  vi.useFakeTimers();
  vi.resetAllMocks();
  focusManager.setFocused(true);
  client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  vi.mocked(api.listProviders).mockResolvedValue({ providers: {} });
  vi.mocked(api.getCurrentProviderId).mockResolvedValue({ current_provider_id: 'direct' });
  vi.mocked(api.getProxyStatus).mockResolvedValue({ running: false, listen_addr: null, upstream_url: '', http_proxy_url: null, active_target_provider_id: null, active_target_provider_name: null });
});
afterEach(() => { client.clear(); focusManager.setFocused(undefined); vi.useRealTimers(); });
function wrapper({ children }: { children: ReactNode }) {
  return <QueryClientProvider client={client}>{children}</QueryClientProvider>;
}
async function tick(ms: number) { await act(async () => { await vi.advanceTimersByTimeAsync(ms); }); }

it('shares cached requests, polls runtime more often than configuration and pauses in the background', async () => {
  const useOverview = () => { useProviders(); useCurrentProvider(); useRouteStatus(); };
  const first = renderHook(useOverview, { wrapper });
  await tick(1);
  const second = renderHook(useOverview, { wrapper });
  await tick(1);
  expect(api.listProviders).toHaveBeenCalledTimes(1);
  expect(api.getProxyStatus).toHaveBeenCalledTimes(1);
  await tick(5_000);
  expect(api.listProviders).toHaveBeenCalledTimes(1);
  expect(api.getCurrentProviderId).toHaveBeenCalledTimes(2);
  expect(api.getProxyStatus).toHaveBeenCalledTimes(2);
  act(() => focusManager.setFocused(false));
  await tick(60_000);
  expect(api.listProviders).toHaveBeenCalledTimes(1);
  expect(api.getProxyStatus).toHaveBeenCalledTimes(2);
  act(() => focusManager.setFocused(true));
  await tick(1);
  expect(api.listProviders).toHaveBeenCalledTimes(2);
  expect(api.getProxyStatus).toHaveBeenCalledTimes(3);
  first.unmount(); second.unmount();
});

it('refreshes both provider views after mutation and aborts unused requests', async () => {
  const hook = renderHook(() => ({ list: useProviders(), current: useCurrentProvider(), refresh: useRefreshProviders() }), { wrapper });
  await tick(1);
  vi.mocked(api.getCurrentProviderId).mockResolvedValue({ current_provider_id: 'next', recovery_required: true });
  await act(async () => { await hook.result.current.refresh(); });
  await tick(1);
  expect(hook.result.current.current.data?.current_provider_id).toBe('next');
  expect(hook.result.current.current.data?.recovery_required).toBe(true);
  expect(api.listProviders).toHaveBeenCalledTimes(2);
  hook.unmount();
  client.clear();
  let signal: AbortSignal | undefined;
  vi.mocked(api.listProviders).mockImplementation((options) => {
    signal = options?.signal ?? undefined;
    return new Promise(() => {});
  });
  const pending = renderHook(useProviders, { wrapper });
  await tick(1);
  pending.unmount();
  expect(signal?.aborted).toBe(true);
});
