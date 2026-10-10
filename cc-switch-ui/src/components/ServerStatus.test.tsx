import { act, render, screen } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { expect, it, vi } from 'vitest';
import { getProxyStatus } from '@/api';
import { providerKeys } from '@/lib/useProviders';
import { ServerStatus } from './ServerStatus';
vi.mock('@/api', () => ({ getProxyStatus: vi.fn() }));
it('shows actual connectivity and marks a failed refresh unavailable even with cached data', async () => {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  vi.mocked(getProxyStatus).mockResolvedValue({ running: false, listen_addr: null, upstream_url: '', http_proxy_url: null, active_target_provider_id: null, active_target_provider_name: null });
  const view = render(<QueryClientProvider client={client}><ServerStatus /></QueryClientProvider>);
  expect(screen.getByText('Connecting…')).toBeTruthy();
  await screen.findByText('Local Server');
  vi.mocked(getProxyStatus).mockRejectedValue(new Error('Offline'));
  await act(async () => { await client.invalidateQueries({ queryKey: providerKeys.route }); });
  expect(await screen.findByText('Server unavailable')).toBeTruthy();
  view.unmount(); client.clear();
});
