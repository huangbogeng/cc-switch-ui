import { act, renderHook, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import type { ReactNode } from 'react';
import { expect, it, vi } from 'vitest';
import * as api from '@/api';
import { useRequestLogs } from './useUsage';

vi.mock('@/api', () => ({ getRequestLogs: vi.fn() }));
it('keeps previous rows during pagination but clears them when filters change', async () => {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  const initial = { data: [], total: 40, page: 0, page_size: 20 };
  vi.mocked(api.getRequestLogs).mockResolvedValueOnce(initial).mockImplementation(() => new Promise(() => {}));
  function wrapper({ children }: { children: ReactNode }) { return <QueryClientProvider client={client}>{children}</QueryClientProvider>; }
  const hook = renderHook(({ page, model }) => useRequestLogs({ page, page_size: 20, model }, 0), { wrapper, initialProps: { page: 0, model: 'first' } });
  await waitFor(() => expect(hook.result.current.data).toEqual(initial));
  hook.rerender({ page: 1, model: 'first' });
  expect(hook.result.current.data).toEqual(initial);
  expect(hook.result.current.isPlaceholderData).toBe(true);
  hook.rerender({ page: 0, model: 'second' });
  expect(hook.result.current.data).toBeUndefined();
  await act(async () => { hook.unmount(); client.clear(); });
});
