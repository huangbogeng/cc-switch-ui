import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render, screen } from '@testing-library/react';
import { beforeEach, expect, it, vi } from 'vitest';
import * as api from '@/api';
import UsagePage from './UsagePage';
vi.mock('@/api', async original => ({ ...await original<typeof import('@/api')>(), getProxyUsageSummary: vi.fn(), getDataSourceBreakdown: vi.fn(), getProviderStats: vi.fn(), getModelStats: vi.fn(), getRequestLogs: vi.fn(), syncSessionUsage: vi.fn() }));
vi.mock('@/components/usage/UsageTrendChart', () => ({ default: () => <div>Usage chart</div> }));
vi.mock('@/components/usage/UsageHeatmap', () => ({ default: () => <div>Usage heatmap</div> }));
const summary = { totals: { request_count: 0, input_tokens: 0, output_tokens: 0 }, providers: [], models: [], trend: [], sources: [] };
function preview() { return render(<QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}><UsagePage /></QueryClientProvider>); }
beforeEach(() => {
  vi.resetAllMocks();
  vi.mocked(api.getProxyUsageSummary).mockResolvedValue(summary);
  vi.mocked(api.getDataSourceBreakdown).mockResolvedValue([]);
  vi.mocked(api.getProviderStats).mockResolvedValue({ providers: [] });
  vi.mocked(api.getModelStats).mockResolvedValue({ models: [] });
  vi.mocked(api.getRequestLogs).mockResolvedValue({ data: [], total: 0, page: 0, page_size: 20 });
});
it('retries unavailable activity without claiming there is no activity', async () => {
  vi.mocked(api.getProxyUsageSummary).mockRejectedValueOnce(new Error('Summary unavailable')).mockResolvedValue(summary);
  const { container } = preview();
  await screen.findByText('Usage activity is unavailable.');
  expect(screen.queryByText('Usage chart')).toBeNull();
  expect(container.contains(screen.getByRole('alert'))).toBe(false);
  fireEvent.click(screen.getByRole('button', { name: 'Retry usage activity' }));
  await screen.findByText('Usage chart');
  expect(screen.queryByText('Usage activity is unavailable.')).toBeNull();
});
it('retries failed request logs on the current page', async () => {
  vi.mocked(api.getRequestLogs).mockRejectedValueOnce(new Error('Logs unavailable')).mockResolvedValue({ data: [], total: 0, page: 0, page_size: 20 });
  preview();
  fireEvent.click(await screen.findByRole('button', { name: 'Retry request logs' }));
  await screen.findByText('No request logs found');
  expect(api.getRequestLogs).toHaveBeenLastCalledWith(expect.objectContaining({ page: 0, page_size: 20 }), expect.any(AbortSignal));
});
it('reports partial session sync with imported counts and expandable errors', async () => {
  vi.mocked(api.syncSessionUsage).mockResolvedValue({ imported: 3, skipped: 2, filesScanned: 4, errors: ['Session file unreadable'] });
  preview();
  fireEvent.click(screen.getByRole('button', { name: 'Sync' }));
  await screen.findByText('Session sync partially completed: 3 imported, 2 skipped, 1 error.');
  fireEvent.click(screen.getByText('Sync errors'));
  expect(screen.getByText('Session file unreadable')).toBeTruthy();
});
