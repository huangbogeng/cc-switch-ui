import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, expect, it, vi } from 'vitest';
import * as api from '@/api';
import DashboardPage from './DashboardPage';
vi.mock('@/api', async original => ({ ...await original<typeof import('@/api')>(), listProviders: vi.fn(), getCurrentProviderId: vi.fn(), getProxyStatus: vi.fn(), getProxyUsageSummary: vi.fn(), getCopilotUsage: vi.fn() }));
vi.mock('@/components/usage/UsageTrendChart', () => ({ default: () => <div>Activity chart</div> }));
const summary = { totals: { request_count: 0, input_tokens: 0, output_tokens: 0 }, providers: [], models: [], trend: [], sources: [] };
beforeEach(() => {
  vi.resetAllMocks();
  vi.mocked(api.listProviders).mockResolvedValue({ providers: {} });
  vi.mocked(api.getCurrentProviderId).mockResolvedValue({ current_provider_id: null });
  vi.mocked(api.getProxyStatus).mockResolvedValue({ running: false, listen_addr: null, active_target_provider_id: null, upstream_url: '', http_proxy_url: null, active_target_provider_name: null });
  vi.mocked(api.getProxyUsageSummary).mockResolvedValue(summary);
  vi.mocked(api.getCopilotUsage).mockRejectedValue(new Error('No linked Copilot account'));
});
it('keeps failure feedback outside page layout and retries unavailable activity', async () => {
  vi.mocked(api.getProxyUsageSummary).mockRejectedValueOnce(new Error('Activity read failed')).mockResolvedValue(summary);
  const { container } = render(<QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}><DashboardPage /></QueryClientProvider>);
  await screen.findByText('Recent activity is unavailable.');
  expect(screen.queryByText('Activity chart')).toBeNull();
  expect(container.contains(screen.getByRole('alert'))).toBe(false);
  fireEvent.click(screen.getByRole('button', { name: 'Dismiss notification' }));
  expect(screen.queryByRole('alert')).toBeNull();
  fireEvent.click(screen.getByRole('button', { name: 'Retry activity' }));
  await screen.findByText('Activity chart');
  expect(screen.queryByText('Recent activity is unavailable.')).toBeNull();
});
it('refreshes all core dashboard data without navigating away', async () => {
  render(<QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}><DashboardPage /></QueryClientProvider>);
  await waitFor(() => expect(screen.getByRole('button', { name: 'Refresh' }).hasAttribute('disabled')).toBe(false));
  fireEvent.click(screen.getByRole('button', { name: 'Refresh' }));
  await waitFor(() => expect(api.getProxyUsageSummary).toHaveBeenCalledTimes(2));
  expect(api.listProviders).toHaveBeenCalledTimes(2);
  expect(api.getCurrentProviderId).toHaveBeenCalledTimes(2);
  expect(api.getProxyStatus).toHaveBeenCalledTimes(2);
});
