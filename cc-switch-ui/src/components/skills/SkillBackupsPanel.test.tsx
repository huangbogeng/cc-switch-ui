import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { cleanup, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { ApiError, listSkillBackups, restoreSkillBackup } from '@/api';
import { SkillBackupsPanel } from './SkillBackupsPanel';

vi.mock('@/api', async (original) => ({
  ...await original<typeof import('@/api')>(), listSkillBackups: vi.fn(), restoreSkillBackup: vi.fn(),
}));
const backup = { id: 'live:test-00000000-0000-0000-0000-000000000000', directory: 'test', origin: 'live' as const, modifiedAt: 0 };
beforeEach(() => {
  vi.mocked(listSkillBackups).mockResolvedValue({ backups: [backup] });
  vi.spyOn(window, 'confirm').mockReturnValue(true);
});
afterEach(() => { cleanup(); vi.restoreAllMocks(); vi.resetAllMocks(); });

it('loads backup history on demand and requires confirmation to restore', async () => {
  const onRestored = vi.fn();
  const onBusyChange = vi.fn();
  render(<SkillBackupsPanel disabled={false} onRestored={onRestored} onBusyChange={onBusyChange} />);
  expect(listSkillBackups).not.toHaveBeenCalled();
  await userEvent.click(screen.getByRole('button', { name: 'Load backups' }));
  expect(await screen.findByLabelText('Backup version')).toBeTruthy();
  vi.mocked(window.confirm).mockReturnValue(false);
  await userEvent.click(screen.getByRole('button', { name: 'Restore files' }));
  expect(restoreSkillBackup).not.toHaveBeenCalled();
  vi.mocked(window.confirm).mockReturnValue(true);
  vi.mocked(restoreSkillBackup).mockResolvedValue({ success: true, directory: 'test', enabled: false });
  await userEvent.click(screen.getByRole('button', { name: 'Restore files' }));
  await screen.findByText('Source files restored. This skill remains disabled.');
  expect(onRestored).toHaveBeenCalledOnce();
  expect(restoreSkillBackup).toHaveBeenCalledWith(backup.id);
  await waitFor(() => expect(onBusyChange).toHaveBeenLastCalledWith(false));
});

it('refreshes records but reports pending sync when recovery is only partially applied', async () => {
  const onRestored = vi.fn();
  vi.mocked(restoreSkillBackup).mockRejectedValue(new ApiError('Source restored, but sync failed. Retry Sync.', true));
  render(<SkillBackupsPanel disabled={false} onRestored={onRestored} onBusyChange={vi.fn()} />);
  await userEvent.click(screen.getByRole('button', { name: 'Load backups' }));
  await screen.findByLabelText('Backup version');
  await userEvent.click(screen.getByRole('button', { name: 'Restore files' }));
  await screen.findByText('Source restored, but sync failed. Retry Sync.');
  expect(onRestored).toHaveBeenCalledOnce();
  expect(screen.queryByText('Skill restored and synced to Claude Code.')).toBeNull();
  expect(screen.getByRole('button', { name: 'Restore files' }).hasAttribute('disabled')).toBe(false);
});

it('retains a successful restore notice if refreshing history fails', async () => {
  vi.mocked(restoreSkillBackup).mockResolvedValue({ success: true, directory: 'test', enabled: true });
  render(<SkillBackupsPanel disabled={false} onRestored={vi.fn()} onBusyChange={vi.fn()} />);
  await userEvent.click(screen.getByRole('button', { name: 'Load backups' }));
  await screen.findByLabelText('Backup version');
  vi.mocked(listSkillBackups).mockRejectedValue(new Error('Read failed'));
  await userEvent.click(screen.getByRole('button', { name: 'Restore files' }));
  await screen.findByText('Restore completed, but backup refresh failed: Read failed');
  expect(screen.getByText('Skill restored and synced to Claude Code.')).toBeTruthy();
});

it('allows retry after loading history fails', async () => {
  vi.mocked(listSkillBackups).mockRejectedValueOnce(new Error('Read failed'));
  render(<SkillBackupsPanel disabled={false} onRestored={vi.fn()} onBusyChange={vi.fn()} />);
  await userEvent.click(screen.getByRole('button', { name: 'Load backups' }));
  await screen.findByText('Read failed');
  await userEvent.click(screen.getByRole('button', { name: 'Load backups' }));
  await screen.findByLabelText('Backup version');
  expect(screen.queryByText('Read failed')).toBeNull();
});
