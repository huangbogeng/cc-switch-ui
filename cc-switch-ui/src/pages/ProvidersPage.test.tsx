import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, expect, it, vi } from 'vitest';
import * as api from '../api';
import ProvidersPage from './ProvidersPage';

vi.mock('../api', () => ({
  listProviders: vi.fn(), getCurrentProviderId: vi.fn(), getProxyStatus: vi.fn(),
  getCodexOAuthStatus: vi.fn(), recoverProviderOperation: vi.fn(), saveProvider: vi.fn(), switchProvider: vi.fn(),
  deleteProvider: vi.fn(), setProxyTarget: vi.fn(), startProxy: vi.fn(), stopProxy: vi.fn(),
}));
vi.mock('@/lib/fetchCache', () => ({ cacheGet: () => null, cacheSet: vi.fn() }));
vi.mock('@/components/providers/LocalRoutePanel', () => ({ LocalRoutePanel: ({ onStopProxy, onStartProxy, onTargetChange, busy }: { onStopProxy: () => void; onStartProxy: (id: string) => void; onTargetChange: (id: string) => void; busy: boolean }) => <><button disabled={busy} onClick={onStopProxy}>Stop test route</button><button disabled={busy} onClick={() => onStartProxy('test')}>Start test route</button><button disabled={busy} onClick={() => onTargetChange('test')}>Change test route</button></> }));
vi.mock('@/components/providers/ProviderCard', () => ({
  ProviderCard: ({ provider, onEdit, onSwitch, onDelete, busy }: { provider: api.Provider; onEdit: (provider: api.Provider) => void; onSwitch: (id: string) => void; onDelete: (id: string) => void; busy: boolean }) => (
    <><button onClick={() => onEdit(provider)}>Edit {provider.name}</button><button disabled={busy} onClick={() => onSwitch(provider.id)}>Select {provider.name}</button><button disabled={busy} onClick={() => onDelete(provider.id)}>Delete {provider.name}</button></>
  ),
}));
vi.mock('@/components/providers/ProviderFormDialog', () => ({
  ProviderFormDialog: ({ open, initialFormData, error, onSubmit }: { open: boolean; initialFormData: object; error?: string; onSubmit: (form: object) => void }) => open ? (
    <div role="dialog"><button onClick={() => onSubmit({ ...initialFormData, apiKey: 'new-key' })}>Submit draft</button>{error}</div>
  ) : null,
}));
const provider: api.Provider = {
  id: 'test', name: 'Test', inFailoverQueue: false, settingsConfig: { env: { ANTHROPIC_AUTH_TOKEN: 'old-key', ANTHROPIC_BASE_URL: 'https://test.example' } }, meta: { apiFormat: 'anthropic' },
};
beforeEach(() => {
  vi.resetAllMocks();
  vi.mocked(api.listProviders).mockResolvedValue({ providers: { test: provider } });
  vi.mocked(api.getCurrentProviderId).mockResolvedValue({ current_provider_id: 'test' });
  vi.mocked(api.getProxyStatus).mockResolvedValue({ running: false, listen_addr: null, active_target_provider_id: 'test', upstream_url: '', http_proxy_url: null, active_target_provider_name: 'Test' });
  vi.mocked(api.getCodexOAuthStatus).mockResolvedValue({ accounts: [], authenticated: false });
});
it('sends the editor baseline and explains when a saved record was applied', async () => {
  vi.mocked(api.saveProvider).mockResolvedValue({ success: true, saved: true, applied: true, message: 'Saved and applied to direct config.', warnings: ['Recovered earlier save.'] });
  render(<QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}><ProvidersPage /></QueryClientProvider>);
  fireEvent.click(await screen.findByText('Edit Test'));
  fireEvent.click(screen.getByText('Submit draft'));
  await waitFor(() => expect(api.saveProvider).toHaveBeenCalledWith(expect.objectContaining({ settingsConfig: expect.objectContaining({ env: expect.objectContaining({ ANTHROPIC_AUTH_TOKEN: 'new-key' }) }) }), provider, 'refuse', undefined));
  expect(await screen.findByRole('status')).toHaveProperty('textContent', 'Saved and applied to direct config. Recovered earlier save.');
  expect(screen.queryByRole('dialog')).toBeNull();
});
it('keeps the draft open when save reports a conflict', async () => {
  vi.mocked(api.saveProvider).mockRejectedValue(new Error('Configuration changed externally'));
  render(<QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}><ProvidersPage /></QueryClientProvider>);
  fireEvent.click(await screen.findByText('Edit Test'));
  fireEvent.click(screen.getByText('Submit draft'));
  expect(await screen.findByText('Configuration changed externally')).toBeTruthy();
  expect(screen.getByRole('dialog')).toBeTruthy();
});
it('shows switch warnings', async () => {
  vi.mocked(api.switchProvider).mockResolvedValue({ success: true, warnings: ['Backfill failed; review previous provider.'] });
  render(<QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}><ProvidersPage /></QueryClientProvider>);
  fireEvent.click(await screen.findByText('Select Test'));
  expect(await screen.findByRole('status')).toHaveProperty('textContent', 'Backfill failed; review previous provider.');
});
it('offers recovery for an interrupted save and refreshes the resulting state', async () => {
  vi.mocked(api.getCurrentProviderId)
    .mockResolvedValueOnce({ current_provider_id: 'test', recovery_required: true })
    .mockResolvedValue({ current_provider_id: 'test', recovery_required: false });
  vi.mocked(api.recoverProviderOperation).mockResolvedValue({ success: true, message: 'Unfinished save recovered.', warnings: ['External config preserved.'] });
  render(<QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}><ProvidersPage /></QueryClientProvider>);
  fireEvent.click(await screen.findByText('Recover operation'));
  expect(await screen.findByRole('status')).toHaveProperty('textContent', 'Unfinished save recovered. External config preserved.');
  await waitFor(() => expect(screen.queryByText('Recover operation')).toBeNull());
});

it('refreshes pending recovery after a failed switch without claiming success', async () => {
  vi.mocked(api.getCurrentProviderId).mockResolvedValueOnce({ current_provider_id: 'test' }).mockResolvedValue({ current_provider_id: 'test', recovery_required: true });
  vi.mocked(api.switchProvider).mockRejectedValue(new Error('Device selection could not be saved; recovery required'));
  render(<QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}><ProvidersPage /></QueryClientProvider>);
  fireEvent.click(await screen.findByText('Select Test'));
  expect(await screen.findByText('Recover operation')).toBeTruthy();
  expect(await screen.findByText('Device selection could not be saved; recovery required')).toBeTruthy();
  expect(screen.queryByRole('status')).toBeNull();
});
it('explains that takeover-time selection applies after stopping', async () => {
  vi.mocked(api.switchProvider).mockResolvedValue({ success: true, applied: false, message: 'Direct provider selected. It will be applied when the local route stops.' });
  render(<QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}><ProvidersPage /></QueryClientProvider>);
  fireEvent.click(await screen.findByText('Select Test'));
  expect(await screen.findByRole('status')).toHaveProperty('textContent', 'Direct provider selected. It will be applied when the local route stops.');
});

it('shows a retry state instead of an empty list when initial loading fails', async () => {
  vi.mocked(api.listProviders).mockRejectedValueOnce(new Error('Server unavailable')).mockResolvedValue({ providers: { test: provider } });
  render(<QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}><ProvidersPage /></QueryClientProvider>);
  expect(await screen.findByText('Unable to load provider configuration')).toBeTruthy();
  expect(screen.queryByText('No providers yet')).toBeNull();
  fireEvent.click(screen.getByRole('button', { name: 'Retry' }));
  expect(await screen.findByText('Edit Test')).toBeTruthy();
  expect(screen.queryByText('Unable to load provider configuration')).toBeNull();
});
it('allows dismissing query errors without losing the retry control', async () => {
  vi.mocked(api.listProviders).mockRejectedValue(new Error('Server unavailable'));
  render(<QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}><ProvidersPage /></QueryClientProvider>);
  await screen.findByText('Server unavailable');
  fireEvent.click(screen.getByRole('button', { name: 'Dismiss notification' }));
  expect(screen.queryByText('Server unavailable')).toBeNull();
  expect(screen.getByRole('button', { name: 'Retry' })).toBeTruthy();
});
it('refreshes recovery and route state after a route stop loses its response', async () => {
  vi.mocked(api.stopProxy).mockRejectedValue(new Error('Response lost'));
  vi.mocked(api.getCurrentProviderId).mockResolvedValueOnce({ current_provider_id: 'test' }).mockResolvedValue({ current_provider_id: 'test', recovery_required: true });
  render(<QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}><ProvidersPage /></QueryClientProvider>);
  fireEvent.click(await screen.findByRole('button', { name: 'Stop test route' }));
  await screen.findByText('Response lost');
  expect(screen.getByText('Recover operation')).toBeTruthy();
  expect(api.getProxyStatus).toHaveBeenCalledTimes(2);
  expect(screen.queryByRole('status')).toBeNull();
});
it('refreshes records after a deletion loses its response', async () => {
  vi.mocked(api.getCurrentProviderId).mockResolvedValue({ current_provider_id: null });
  vi.mocked(api.getProxyStatus).mockResolvedValue({ running: false, listen_addr: null, active_target_provider_id: null, upstream_url: '', http_proxy_url: null, active_target_provider_name: null });
  vi.mocked(api.deleteProvider).mockImplementation(async () => {
    vi.mocked(api.listProviders).mockResolvedValue({ providers: {} });
    throw new Error('Deletion response lost');
  });
  render(<QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}><ProvidersPage /></QueryClientProvider>);
  fireEvent.click(await screen.findByText('Delete Test'));
  fireEvent.click(await screen.findByRole('button', { name: 'Delete' }));
  await screen.findByText('Deletion response lost');
  expect(screen.getByText('No providers yet')).toBeTruthy();
  expect(screen.queryByText('Edit Test')).toBeNull();
});

it('keeps the provider draft and retries with an explicit observed conflict decision', async () => {
  vi.mocked(api.saveProvider).mockRejectedValueOnce(Object.assign(new Error('External change'), { conflictKeys: ['live/env/ANTHROPIC_AUTH_TOKEN'], conflictRevision: 'observed' })).mockResolvedValue({ success: true, applied: true });
  render(<QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}><ProvidersPage /></QueryClientProvider>);
  fireEvent.click(await screen.findByText('Edit Test'));
  fireEvent.click(screen.getByText('Submit draft'));
  await screen.findByRole('dialog', { name: 'Provider settings changed externally' });
  expect(screen.getByText('live/env/ANTHROPIC_AUTH_TOKEN')).toBeTruthy();
  fireEvent.click(screen.getByRole('button', { name: 'Keep external changes' }));
  await waitFor(() => expect(api.saveProvider).toHaveBeenLastCalledWith(expect.anything(), expect.anything(), 'keepTheirs', 'observed'));
  await waitFor(() => expect(screen.queryByRole('dialog', { name: 'Provider settings changed externally' })).toBeNull());
});

it('does not start an old route when target selection returns an application error', async () => {
  vi.mocked(api.setProxyTarget).mockResolvedValue({ success: false, error: 'Unsupported route format' });
  vi.mocked(api.startProxy).mockResolvedValue({ success: true });
  render(<QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}><ProvidersPage /></QueryClientProvider>);
  fireEvent.click(await screen.findByText('Start test route'));
  expect(await screen.findByText('Unsupported route format')).toBeTruthy();
  expect(api.startProxy).not.toHaveBeenCalled();
  expect(api.getProxyStatus).toHaveBeenCalledTimes(2);
});

it('shows application-level target errors when switching a running route', async () => {
  vi.mocked(api.getProxyStatus).mockResolvedValue({ running: true, listen_addr: null, active_target_provider_id: 'test', upstream_url: '', http_proxy_url: null, active_target_provider_name: 'Test' });
  vi.mocked(api.setProxyTarget).mockResolvedValue({ success: false, error: 'Target not available' });
  render(<QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}><ProvidersPage /></QueryClientProvider>);
  fireEvent.click(await screen.findByText('Change test route'));
  expect(await screen.findByText('Target not available')).toBeTruthy();
  expect(api.startProxy).not.toHaveBeenCalled();
  expect(api.getProxyStatus).toHaveBeenCalledTimes(2);
});

it('starts only after successful target selection', async () => {
  vi.mocked(api.setProxyTarget).mockResolvedValue({ success: true });
  vi.mocked(api.startProxy).mockResolvedValue({ success: true });
  render(<QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}><ProvidersPage /></QueryClientProvider>);
  fireEvent.click(await screen.findByText('Start test route'));
  await waitFor(() => expect(api.startProxy).toHaveBeenCalledOnce());
  expect(vi.mocked(api.setProxyTarget).mock.invocationCallOrder[0]).toBeLessThan(vi.mocked(api.startProxy).mock.invocationCallOrder[0]);
});

it('saves inferred OAuth metadata through the actual editor submit path', async () => {
  const imported: api.Provider = { id: 'legacy-binding', name: 'Imported OAuth', inFailoverQueue: false, settingsConfig: { env: { ANTHROPIC_MODEL: 'configured-model' } }, meta: { apiFormat: 'gemini_native', authBinding: { authProvider: 'codex_oauth', accountId: 'synthetic-account' } } };
  vi.mocked(api.listProviders).mockResolvedValue({ providers: { [imported.id]: imported } });
  vi.mocked(api.saveProvider).mockResolvedValue({ success: true, saved: true, applied: false });
  render(<QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}><ProvidersPage /></QueryClientProvider>);
  fireEvent.click(await screen.findByText('Edit Imported OAuth'));
  fireEvent.click(screen.getByText('Submit draft'));
  await waitFor(() => expect(api.saveProvider).toHaveBeenCalledWith(expect.objectContaining({ meta: expect.objectContaining({ providerType: 'codex_oauth', authMode: 'oauth_proxy', apiFormat: 'openai_responses', authBinding: imported.meta && (imported.meta as { authBinding: unknown }).authBinding }) }), imported, 'refuse', undefined));
});
