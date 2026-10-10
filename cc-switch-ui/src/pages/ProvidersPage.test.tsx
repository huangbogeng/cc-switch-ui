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
vi.mock('@/components/providers/LocalRoutePanel', () => ({ LocalRoutePanel: () => null }));
vi.mock('@/components/providers/ProviderCard', () => ({
  ProviderCard: ({ provider, onEdit, onSwitch }: { provider: api.Provider; onEdit: (provider: api.Provider) => void; onSwitch: (id: string) => void }) => (
    <><button onClick={() => onEdit(provider)}>Edit {provider.name}</button><button onClick={() => onSwitch(provider.id)}>Select {provider.name}</button></>
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
  render(<ProvidersPage />);
  fireEvent.click(await screen.findByText('Edit Test'));
  fireEvent.click(screen.getByText('Submit draft'));
  await waitFor(() => expect(api.saveProvider).toHaveBeenCalledWith(expect.objectContaining({ settingsConfig: expect.objectContaining({ env: expect.objectContaining({ ANTHROPIC_AUTH_TOKEN: 'new-key' }) }) }), provider));
  expect(await screen.findByRole('status')).toHaveProperty('textContent', 'Saved and applied to direct config. Recovered earlier save.');
  expect(screen.queryByRole('dialog')).toBeNull();
});
it('keeps the draft open when save reports a conflict', async () => {
  vi.mocked(api.saveProvider).mockRejectedValue(new Error('Configuration changed externally'));
  render(<ProvidersPage />);
  fireEvent.click(await screen.findByText('Edit Test'));
  fireEvent.click(screen.getByText('Submit draft'));
  expect(await screen.findByText('Configuration changed externally')).toBeTruthy();
  expect(screen.getByRole('dialog')).toBeTruthy();
});
it('shows switch warnings', async () => {
  vi.mocked(api.switchProvider).mockResolvedValue({ success: true, warnings: ['Backfill failed; review previous provider.'] });
  render(<ProvidersPage />);
  fireEvent.click(await screen.findByText('Select Test'));
  expect(await screen.findByRole('status')).toHaveProperty('textContent', 'Backfill failed; review previous provider.');
});
it('offers recovery for an interrupted save and refreshes the resulting state', async () => {
  vi.mocked(api.getCurrentProviderId)
    .mockResolvedValueOnce({ current_provider_id: 'test', recovery_required: true })
    .mockResolvedValue({ current_provider_id: 'test', recovery_required: false });
  vi.mocked(api.recoverProviderOperation).mockResolvedValue({ success: true, message: 'Unfinished save recovered.', warnings: ['External config preserved.'] });
  render(<ProvidersPage />);
  fireEvent.click(await screen.findByText('Recover operation'));
  expect(await screen.findByRole('status')).toHaveProperty('textContent', 'Unfinished save recovered. External config preserved.');
  await waitFor(() => expect(screen.queryByText('Recover operation')).toBeNull());
});

it('refreshes pending recovery after a failed switch without claiming success', async () => {
  vi.mocked(api.getCurrentProviderId).mockResolvedValueOnce({ current_provider_id: 'test' }).mockResolvedValue({ current_provider_id: 'test', recovery_required: true });
  vi.mocked(api.switchProvider).mockRejectedValue(new Error('Device selection could not be saved; recovery required'));
  render(<ProvidersPage />);
  fireEvent.click(await screen.findByText('Select Test'));
  expect(await screen.findByText('Recover operation')).toBeTruthy();
  expect(await screen.findByText('Device selection could not be saved; recovery required')).toBeTruthy();
  expect(screen.queryByRole('status')).toBeNull();
});
it('explains that takeover-time selection applies after stopping', async () => {
  vi.mocked(api.switchProvider).mockResolvedValue({ success: true, applied: false, message: 'Direct provider selected. It will be applied when the local route stops.' });
  render(<ProvidersPage />);
  fireEvent.click(await screen.findByText('Select Test'));
  expect(await screen.findByRole('status')).toHaveProperty('textContent', 'Direct provider selected. It will be applied when the local route stops.');
});
