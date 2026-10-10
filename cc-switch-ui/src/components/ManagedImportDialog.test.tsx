import { beforeEach, expect, it, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { listImportCandidates, listRemovedItems } from '@/api';
import { ManagedImportDialog } from './ManagedImportDialog';
vi.mock('@/api', () => ({ listImportCandidates: vi.fn(), listRemovedItems: vi.fn() }));
const items = [{ id: 'one', name: 'One', source: 'Plugin' }, { id: 'two', name: 'Two', source: 'Claude Code' }];
beforeEach(() => {
  vi.clearAllMocks();
  vi.mocked(listImportCandidates).mockResolvedValue({ items, report: { success: true, imported: 0, updated: 0, applied: 0, skipped: 0, failed: 0, items: [] } });
  vi.mocked(listRemovedItems).mockResolvedValue(items);
});
it('scans without importing and submits only explicitly selected candidates', async () => {
  const apply = vi.fn().mockResolvedValue(undefined); const close = vi.fn();
  render(<ManagedImportDialog kind="skill" mode="Import" onApply={apply} onClose={close} />);
  await screen.findByRole('checkbox', { name: /One/ });
  expect(apply).not.toHaveBeenCalled();
  expect((screen.getByRole('button', { name: 'Import selected' }) as HTMLButtonElement).disabled).toBe(true);
  await userEvent.click(screen.getByRole('checkbox', { name: /One/ }));
  await userEvent.click(screen.getByRole('button', { name: 'Import selected' }));
  expect(apply).toHaveBeenCalledExactlyOnceWith(['one']); expect(close).toHaveBeenCalledOnce();
});
it('cancels discovery without changing records', async () => {
  const apply = vi.fn(); const close = vi.fn();
  render(<ManagedImportDialog kind="mcp" mode="Import" onApply={apply} onClose={close} />);
  await screen.findByRole('checkbox', { name: /One/ });
  await userEvent.click(screen.getByRole('button', { name: 'Cancel' }));
  expect(apply).not.toHaveBeenCalled(); expect(close).toHaveBeenCalledOnce();
});
it('restores only selected removed items and explains disabled restoration', async () => {
  const apply = vi.fn().mockResolvedValue(undefined);
  render(<ManagedImportDialog kind="skill" mode="Restore" onApply={apply} onClose={vi.fn()} />);
  await screen.findByRole('checkbox', { name: /Two/ });
  expect(screen.getByText(/Restore selected records as disabled/)).toBeTruthy();
  await userEvent.click(screen.getByRole('checkbox', { name: /Two/ }));
  await userEvent.click(screen.getByRole('button', { name: 'Restore selected' }));
  expect(apply).toHaveBeenCalledExactlyOnceWith(['two']); expect(listImportCandidates).not.toHaveBeenCalled();
});
