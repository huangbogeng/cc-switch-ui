import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, expect, it, vi } from 'vitest';
import { ClaudeUserSettingsPanel } from './ClaudeUserSettingsPanel';
import { getClaudeUserSettings, saveClaudeUserSettings, restoreClaudeUserSettings, previewClaudeUserSettings } from '@/api';

vi.mock('@/api', () => ({
  getClaudeUserSettings: vi.fn(), saveClaudeUserSettings: vi.fn(), restoreClaudeUserSettings: vi.fn(), previewClaudeUserSettings: vi.fn(),
}));
const initial = {
  path: '/test/.claude/settings.json', revision: 'version-1',
  values: { timeoutMs: '2000', nonessentialTraffic: null },
  initialValues: { timeoutMs: null, nonessentialTraffic: '1' },
};
beforeEach(() => {
  vi.resetAllMocks();
  vi.mocked(getClaudeUserSettings).mockResolvedValue(initial);
  vi.mocked(saveClaudeUserSettings).mockResolvedValue({ success: true });
  vi.mocked(restoreClaudeUserSettings).mockResolvedValue({ success: true });
});

it('writes a validated user override with the loaded revision and preserves default traffic', async () => {
  render(<ClaudeUserSettingsPanel />);
  const timeout = await screen.findByLabelText('API Timeout MS');
  fireEvent.change(timeout, { target: { value: '0' } });
  fireEvent.click(screen.getByRole('button', { name: 'Save User Settings' }));
  expect(await screen.findByText(/API timeout must be a positive integer/)).toBeTruthy();
  expect(saveClaudeUserSettings).not.toHaveBeenCalled();
  fireEvent.change(timeout, { target: { value: '4000' } });
  fireEvent.click(screen.getByRole('button', { name: 'Save User Settings' }));
  await waitFor(() => expect(saveClaudeUserSettings).toHaveBeenCalledWith('version-1', { timeoutMs: '4000', nonessentialTraffic: null }, initial.values, 'refuse', undefined));
  expect(await screen.findByRole('status')).toHaveProperty('textContent', 'User settings saved to the live file.');
});

it('keeps a conflicting draft and requires confirmation before reloading it', async () => {
  vi.mocked(saveClaudeUserSettings).mockRejectedValue(new Error('Configuration changed externally'));
  const confirm = vi.spyOn(window, 'confirm').mockReturnValue(false);
  render(<ClaudeUserSettingsPanel />);
  const timeout = await screen.findByLabelText('API Timeout MS');
  fireEvent.change(timeout, { target: { value: '4000' } });
  fireEvent.click(screen.getByRole('button', { name: 'Save User Settings' }));
  expect(await screen.findByText(/Configuration changed externally/)).toBeTruthy();
  expect(timeout).toHaveProperty('value', '4000');
  fireEvent.click(screen.getByRole('button', { name: 'Reload from file' }));
  expect(confirm).toHaveBeenCalled();
  expect(getClaudeUserSettings).toHaveBeenCalledOnce();
  confirm.mockReturnValue(true);
  fireEvent.click(screen.getByRole('button', { name: 'Reload from file' }));
  await waitFor(() => expect(timeout).toHaveProperty('value', '2000'));
  confirm.mockRestore();
});

it('previews backup values and only restores after confirmation', async () => {
  render(<ClaudeUserSettingsPanel />);
  await screen.findByLabelText('API Timeout MS');
  fireEvent.click(screen.getByRole('button', { name: 'Restore user settings' }));
  expect(screen.getByRole('alertdialog').textContent).toContain('API Timeout MS: Claude Code default');
  expect(restoreClaudeUserSettings).not.toHaveBeenCalled();
  fireEvent.click(screen.getByRole('button', { name: 'Cancel' }));
  fireEvent.click(screen.getByRole('button', { name: 'Restore user settings' }));
  fireEvent.click(screen.getByRole('button', { name: 'Restore' }));
  await waitFor(() => expect(restoreClaudeUserSettings).toHaveBeenCalledWith('version-1'));
  expect(await screen.findByRole('status')).toHaveProperty('textContent', 'User settings restored to the live file.');
});

it('reports a completed write when refresh fails and prevents another write until reload', async () => {
  vi.mocked(getClaudeUserSettings).mockResolvedValueOnce(initial).mockRejectedValueOnce(new Error('offline')).mockResolvedValue(initial);
  render(<ClaudeUserSettingsPanel />);
  fireEvent.change(await screen.findByLabelText('API Timeout MS'), { target: { value: '' } });
  fireEvent.click(screen.getByRole('button', { name: 'Save User Settings' }));
  expect(await screen.findByText(/The write completed, but refreshing failed/)).toBeTruthy();
  expect(screen.getByRole('status')).toHaveProperty('textContent', 'User settings saved to the live file.');
  expect(screen.getByRole('button', { name: 'Save User Settings' })).toHaveProperty('disabled', true);
  fireEvent.click(screen.getByRole('button', { name: 'Reload from file' }));
  await waitFor(() => expect(screen.getByLabelText('API Timeout MS')).toHaveProperty('value', '2000'));
  expect(saveClaudeUserSettings).toHaveBeenCalledOnce();
});

it('retains the draft and requires an explicit choice for field conflicts', async () => {
  vi.mocked(saveClaudeUserSettings).mockRejectedValueOnce(Object.assign(new Error('Settings changed externally'), { conflictKeys: ['API_TIMEOUT_MS'], conflictRevision: 'latest' })).mockResolvedValue({ success: true });
  render(<ClaudeUserSettingsPanel />);
  const timeout = await screen.findByLabelText('API Timeout MS');
  fireEvent.change(timeout, { target: { value: '4000' } });
  fireEvent.click(screen.getByRole('button', { name: 'Save User Settings' }));
  await screen.findByRole('dialog', { name: 'Settings changed externally' });
  expect(timeout).toHaveProperty('value', '4000');
  fireEvent.click(screen.getByRole('button', { name: 'Keep external changes' }));
  await waitFor(() => expect(saveClaudeUserSettings).toHaveBeenLastCalledWith('version-1', { timeoutMs: '4000', nonessentialTraffic: null }, initial.values, 'keepTheirs', 'latest'));
  await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
});

it('previews without saving and clears the result when the draft changes', async () => {
  vi.mocked(previewClaudeUserSettings).mockResolvedValue({ path: initial.path, revision: 'version-1', changes: [{ field: 'API_TIMEOUT_MS', before: '2000', after: '4000' }] });
  render(<ClaudeUserSettingsPanel />);
  const timeout = await screen.findByLabelText('API Timeout MS');
  fireEvent.change(timeout, { target: { value: '4000' } });
  fireEvent.click(screen.getByRole('button', { name: 'Preview changes' }));
  await screen.findByText('2000 → 4000');
  expect(saveClaudeUserSettings).not.toHaveBeenCalled();
  fireEvent.change(timeout, { target: { value: '5000' } });
  expect(screen.queryByText('2000 → 4000')).toBeNull();
});
