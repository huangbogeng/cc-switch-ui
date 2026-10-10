import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, expect, it, vi } from 'vitest';
import * as api from '@/api';
import { DatabaseBackupsPanel } from './DatabaseBackupsPanel';
vi.mock('@/api', () => ({ getDatabaseBackups: vi.fn(), createDatabaseBackup: vi.fn(), deleteDatabaseBackup: vi.fn(), scheduleDatabaseRestore: vi.fn(), cancelDatabaseRestore: vi.fn() }));
const snapshot: api.DatabaseBackupStatus = {
  entries: [{ id: 'manual-test.db', kind: 'manual', sizeBytes: 1024, createdAt: '2026-10-10T00:00:00Z', protected: false }],
  pending: null, directory: '/test/backups/database', manualLimit: 20,
};
beforeEach(() => {
  vi.resetAllMocks();vi.mocked(api.getDatabaseBackups).mockResolvedValue(snapshot);
  vi.mocked(api.createDatabaseBackup).mockResolvedValue({ success: true, id: 'manual-new.db' });
  vi.mocked(api.scheduleDatabaseRestore).mockResolvedValue({ success: true, restartRequired: true });
  vi.mocked(api.cancelDatabaseRestore).mockResolvedValue({ success: true });
});
it('confirms restart-time replacement and explains what remains separate', async () => {
  const confirm=vi.spyOn(window,'confirm').mockReturnValue(false);render(<DatabaseBackupsPanel />);
  fireEvent.click(await screen.findByRole('button',{name:'Restore on restart'}));
  expect(api.scheduleDatabaseRestore).not.toHaveBeenCalled();expect(confirm).toHaveBeenCalledWith(expect.stringContaining('Live Claude files, installed Skills, OAuth logins and device preferences are not restored'));
  confirm.mockReturnValue(true);fireEvent.click(screen.getByRole('button',{name:'Restore on restart'}));
  await waitFor(()=>expect(api.scheduleDatabaseRestore).toHaveBeenCalledWith('manual-test.db'));
  expect(await screen.findByRole('status')).toHaveProperty('textContent',expect.stringContaining('current database is unchanged until restart'));confirm.mockRestore();
});
it('shows pending restore and keeps protected backups from deletion', async () => {
  vi.mocked(api.getDatabaseBackups).mockResolvedValue({ ...snapshot, entries: [{...snapshot.entries[0],protected:true}],pending:{sourceId:'manual-test.db',applying:false} });
  render(<DatabaseBackupsPanel />);
  expect(await screen.findByRole('button',{name:'Cancel planned restore'})).toBeTruthy();
  expect(screen.getByRole('button',{name:'Delete backup'})).toHaveProperty('disabled',true);
  expect(screen.getByRole('button',{name:'Restore on restart'})).toHaveProperty('disabled',true);
  fireEvent.click(screen.getByRole('button',{name:'Cancel planned restore'}));
  expect(await screen.findByRole('status')).toHaveProperty('textContent','Planned restore cancelled. The current database was kept.');
});
it('reports completed creation when refresh fails and requires reload', async () => {
  vi.mocked(api.getDatabaseBackups).mockResolvedValueOnce(snapshot).mockRejectedValueOnce(new Error('offline')).mockResolvedValue(snapshot);
  render(<DatabaseBackupsPanel />);await screen.findByText(/Manual backup/);
  fireEvent.click(screen.getByRole('button',{name:'Create backup'}));
  expect(await screen.findByText(/The action completed, but refreshing failed/)).toBeTruthy();
  expect(screen.getByRole('status')).toHaveProperty('textContent','Database backup created.');
  expect(screen.getByRole('button',{name:'Create backup'})).toHaveProperty('disabled',true);
  fireEvent.click(screen.getByRole('button',{name:'Reload backups'}));
  await waitFor(()=>expect(screen.getByRole('button',{name:'Create backup'})).toHaveProperty('disabled',false));
  expect(api.createDatabaseBackup).toHaveBeenCalledOnce();
});
it('does not stage or claim success when backend validation fails', async () => {
  const confirm=vi.spyOn(window,'confirm').mockReturnValue(true);vi.mocked(api.scheduleDatabaseRestore).mockRejectedValue(new Error('Backup was created by a newer schema version'));
  render(<DatabaseBackupsPanel />);fireEvent.click(await screen.findByRole('button',{name:'Restore on restart'}));
  expect(await screen.findByText('Backup was created by a newer schema version')).toBeTruthy();expect(screen.queryByRole('status')).toBeNull();confirm.mockRestore();
});
