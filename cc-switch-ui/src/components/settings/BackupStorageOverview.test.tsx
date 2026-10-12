import { render, screen, fireEvent } from '@testing-library/react';
import { expect, it, vi } from 'vitest';
import { getBackupStorage } from '@/api';
import { BackupStorageOverview } from './BackupStorageOverview';
vi.mock('@/api', () => ({ getBackupStorage: vi.fn() }));
it('loads on demand, distinguishes missing and partial inventory, and retries failures', async () => {
  vi.mocked(getBackupStorage).mockRejectedValueOnce(new Error('Unavailable')).mockResolvedValue({ locations: [
    { id:'db',label:'Database',path:'/test/db',scope:'Database only',recovery:'Restart to restore',entries:1,sizeBytes:2048,modifiedAt:1,status:'partial',warnings:['Unreadable entry'] },
    { id:'upgrade',label:'Upgrade',path:'/test/upgrade',scope:'Program and data',recovery:'Manual rollback',entries:0,sizeBytes:0,modifiedAt:null,status:'missing',warnings:[] },
  ] });
  render(<BackupStorageOverview />); expect(getBackupStorage).not.toHaveBeenCalled();
  fireEvent.click(screen.getByRole('button', { name:'Inspect backup storage' })); await screen.findByText('Unavailable');
  fireEvent.click(screen.getByRole('button', { name:'Inspect backup storage' })); await screen.findByText('Unreadable entry');
  expect(screen.getByText(/Incomplete totals/)).toBeTruthy(); expect(screen.getByText('No backup directory yet.')).toBeTruthy(); expect(screen.getByText('/test/db')).toBeTruthy();
});
