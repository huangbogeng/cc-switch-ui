import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { cleanup, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { listSkills, importSkills, syncSkills, getManagedDeletion } from '@/api';
import SkillsPage from './SkillsPage';
vi.mock('@/api', async (original) => ({ ...await original<typeof import('@/api')>(),
  listSkills: vi.fn(), importSkills: vi.fn(), syncSkills: vi.fn(), getManagedDeletion: vi.fn(),
}));
vi.mock('@/lib/fetchCache', () => ({ cacheGet: () => undefined, cacheSet: vi.fn() }));
beforeEach(() => { vi.mocked(getManagedDeletion).mockResolvedValue({ pending: null }); vi.mocked(listSkills).mockResolvedValue({ skills: [] }); });
afterEach(() => { cleanup(); vi.resetAllMocks(); });
const partial = { success: false, imported: 0, updated: 1, applied: 0, skipped: 1, failed: 1,
  items: [{ id: 'blocked', outcome: 'failed' as const, error: 'Collection save failed' },
    { id: 'good', outcome: 'updated' as const, error: null }, { id: 'existing', outcome: 'skipped' as const, error: null }] };
it('separates collection updates from imports and refreshes partial results', async () => {
  vi.mocked(importSkills).mockResolvedValue(partial);
  render(<SkillsPage />);
  await userEvent.click(await screen.findByRole('button', { name: 'Import' }));
  await screen.findByText('Import partially completed: 0 imported, 1 updated, 1 skipped, 1 failed.');
  await waitFor(() => expect(listSkills).toHaveBeenCalledTimes(2));
  expect(screen.getByText(/Collection save failed/)).toBeTruthy();
});
it('shows partial sync and replaces the report on a successful retry', async () => {
  vi.mocked(syncSkills).mockResolvedValueOnce({ ...partial, updated: 0, applied: 1 }).mockResolvedValue({ ...partial, success: true, updated: 0, applied: 1, skipped: 2, failed: 0, items: [] });
  render(<SkillsPage />);
  await userEvent.click(await screen.findByRole('button', { name: 'Sync' }));
  await screen.findByText('Sync partially completed: 1 applied, 1 skipped, 1 failed.');
  await userEvent.click(screen.getByRole('button', { name: 'Sync' }));
  await screen.findByText('Sync completed: 1 applied, 2 skipped, 0 failed.');
  expect(screen.queryByText(/Collection save failed/)).toBeNull();
});
