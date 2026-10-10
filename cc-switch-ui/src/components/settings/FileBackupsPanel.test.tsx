import { beforeEach, expect, it, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { listFileBackups, type FileBackup } from '@/api';
import { FileBackupsPanel } from './FileBackupsPanel';
vi.mock('@/api', () => ({ listFileBackups: vi.fn() }));
const item: FileBackup = {
  id: 'claude-settings', label: 'Claude settings', sourcePath: '/home/test/.claude/settings.json',
  markerPath: '/backup/hash.source.json', originalPath: '/backup/hash.original',
  status: 'available', sizeBytes: 100, modifiedAt: 1, error: null,
};
beforeEach(() => { vi.resetAllMocks(); });

it('distinguishes original files, absent originals, and damaged backups with recovery scope', async () => {
  vi.mocked(listFileBackups).mockResolvedValue({ backups: [item,
    { ...item, id: 'absent', label: 'MCP config', status: 'originally-absent', originalPath: null },
    { ...item, id: 'bad', label: 'Legacy settings', status: 'invalid', error: 'Missing original snapshot' },
  ] });
  render(<FileBackupsPanel />);
  await userEvent.click(screen.getByRole('button', { name: 'Load file backups' }));
  await screen.findByText('Original file: /backup/hash.original');
  expect(screen.getByText(/The file did not originally exist/)).toBeTruthy();
  expect(screen.getByText('Backup unavailable: Missing original snapshot')).toBeTruthy();
  await userEvent.click(screen.getByText('Recover a complete configuration file manually'));
  expect(screen.getByText(/Database records stay unchanged/)).toBeTruthy();
  // In-page recovery navigation must not change the application's page hash.
  const target = document.createElement('div');
  target.id = 'claude-user-settings';
  target.scrollIntoView = vi.fn();
  document.body.appendChild(target);
  await userEvent.click(screen.getByRole('button', { name: 'Restore user settings' }));
  expect(target.scrollIntoView).toHaveBeenCalledOnce();
  target.remove();
});

it('marks a retained inventory as stale after refresh failure and allows retry', async () => {
  vi.mocked(listFileBackups).mockResolvedValueOnce({ backups: [item] }).mockRejectedValueOnce(new Error('Read failed')).mockResolvedValue({ backups: [] });
  render(<FileBackupsPanel />);
  await userEvent.click(screen.getByRole('button', { name: 'Load file backups' }));
  await screen.findByText('Claude settings');
  await userEvent.click(screen.getByRole('button', { name: 'Refresh file backups' }));
  await screen.findByText('Read failed');
  expect(screen.getByText(/previous successful refresh/)).toBeTruthy();
  await userEvent.click(screen.getByRole('button', { name: 'Refresh file backups' }));
  expect(screen.queryByText('Read failed')).toBeNull();
  expect(screen.queryByText('Claude settings')).toBeNull();
});
