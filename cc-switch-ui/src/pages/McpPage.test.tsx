import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { cleanup, render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { ApiError, listMcpServers, saveMcpServer, toggleMcpServer, importMcpServers, syncMcpServers, getManagedDeletion, deleteMcpServer } from '@/api';
import McpPage from './McpPage';

vi.mock('@/api', async (original) => ({
  ...await original<typeof import('@/api')>(),
  listMcpServers: vi.fn(), saveMcpServer: vi.fn(), toggleMcpServer: vi.fn(), importMcpServers: vi.fn(), syncMcpServers: vi.fn(), getManagedDeletion: vi.fn(), deleteMcpServer: vi.fn(),
}));
vi.mock('@/lib/fetchCache', () => ({ cacheGet: () => undefined, cacheSet: vi.fn() }));
const server = { id: 'test', name: 'Test', appType: 'claude_code', enabled: true, serverSpec: { command: 'npx' } };

beforeEach(() => { vi.mocked(getManagedDeletion).mockResolvedValue({ pending: null });
  vi.mocked(listMcpServers).mockResolvedValue({ servers: [server] });
});
afterEach(() => { cleanup(); vi.clearAllMocks(); vi.restoreAllMocks(); });

it('refreshes desired state and shows sync failure after a partially saved toggle', async () => {
  render(<McpPage />);
  await screen.findByRole('button', { name: 'Disable Test' });
  vi.mocked(listMcpServers).mockResolvedValue({ servers: [{ ...server, enabled: false }] });
  vi.mocked(toggleMcpServer).mockRejectedValue(new ApiError('Record saved, but sync failed. Retry Sync.', true));
  await userEvent.click(screen.getByRole('button', { name: 'Disable Test' }));
  expect(await screen.findByText('Record saved, but sync failed. Retry Sync.')).toBeTruthy();
  expect(screen.getByRole('button', { name: 'Enable Test' })).toBeTruthy();
  expect(listMcpServers).toHaveBeenCalledTimes(2);
});

it('keeps a saved draft open on sync failure and prevents accidental cancellation', async () => {
  render(<McpPage />);
  await userEvent.click(await screen.findByRole('button', { name: 'Edit Test' }));
  const input = screen.getByLabelText('Name');
  await userEvent.clear(input);
  await userEvent.type(input, 'Updated');
  const confirm = vi.spyOn(window, 'confirm').mockReturnValue(false);
  await userEvent.click(screen.getByRole('button', { name: 'Cancel' }));
  expect(confirm).toHaveBeenCalledOnce();
  expect((screen.getByLabelText('Name') as HTMLInputElement).value).toBe('Updated');
  vi.mocked(saveMcpServer).mockRejectedValue(new ApiError('Record saved, but sync failed. Retry Sync.', true));
  await userEvent.click(screen.getByRole('button', { name: 'Save' }));
  await screen.findByText('Record saved, but sync failed. Retry Sync.');
  expect((screen.getByLabelText('Name') as HTMLInputElement).value).toBe('Updated');
  await waitFor(() => expect(listMcpServers).toHaveBeenCalledTimes(2));
  expect(screen.getByRole('button', { name: 'Save' }).hasAttribute('disabled')).toBe(false);
});

const partial = { success: false, imported: 1, updated: 0, applied: 0, skipped: 0, failed: 1,
  items: [{ id: 'good', outcome: 'imported' as const, error: null }, { id: 'bad', outcome: 'failed' as const, error: 'Database write failed' }] };

it('shows committed imports and failed items together and refreshes records', async () => {
  vi.mocked(importMcpServers).mockResolvedValue(partial);
  render(<McpPage />);
  await userEvent.click(await screen.findByRole('button', { name: 'Import' }));
  await screen.findByText('Import partially completed: 1 imported, 0 updated, 0 skipped, 1 failed.');
  expect(screen.getByText(/Database write failed/)).toBeTruthy();
  await waitFor(() => expect(listMcpServers).toHaveBeenCalledTimes(2));
});

it('keeps import outcomes when the follow-up read fails', async () => {
  vi.mocked(importMcpServers).mockResolvedValue(partial);
  vi.mocked(listMcpServers).mockResolvedValueOnce({ servers: [server] }).mockRejectedValue(new Error('Read failed'));
  render(<McpPage />);
  await userEvent.click(await screen.findByRole('button', { name: 'Import' }));
  await screen.findByText('Import results are retained, but refreshing records failed: Read failed');
  expect(screen.getByText('Import partially completed: 1 imported, 0 updated, 0 skipped, 1 failed.')).toBeTruthy();
});

it('refreshes observed records after a lost import response and allows retry', async () => {
  vi.mocked(importMcpServers).mockRejectedValueOnce(new Error('Connection lost')).mockResolvedValue({ ...partial, success: true, imported: 0, skipped: 2, failed: 0, items: [] });
  render(<McpPage />);
  await userEvent.click(await screen.findByRole('button', { name: 'Import' }));
  await screen.findByText('Connection lost');
  await waitFor(() => expect(listMcpServers).toHaveBeenCalledTimes(2));
  await userEvent.click(screen.getByRole('button', { name: 'Import' }));
  await screen.findByText('Import completed: 0 imported, 0 updated, 2 skipped, 0 failed.');
  expect(screen.queryByText('Connection lost')).toBeNull();
});

it('does not claim a successful MCP sync after a failed atomic write', async () => {
  vi.mocked(syncMcpServers).mockResolvedValue({ ...partial, imported: 0, items: [partial.items[1]] });
  render(<McpPage />);
  await userEvent.click(await screen.findByRole('button', { name: 'Sync' }));
  await screen.findByText('Sync failed: 0 applied, 0 skipped, 1 failed.');
  expect(screen.queryByText('MCP configuration synced to Claude Code.')).toBeNull();
});

it('refreshes committed deletion and blocks mutations while exposing live cleanup recovery', async () => {
  vi.mocked(deleteMcpServer).mockImplementation(async () => {
    // Publish the committed outcome only after the user confirms deletion.
    vi.mocked(listMcpServers).mockResolvedValue({ servers: [] });
    vi.mocked(getManagedDeletion).mockResolvedValue({ pending: { operationId: 'nonce', id: 'test', name: 'Test', kind: 'mcp' } });
    throw new ApiError('Record deleted, live cleanup unfinished.', true);
  });
  render(<McpPage />);
  await screen.findByRole('button', { name: 'Edit Test' });
  await userEvent.click(screen.getByRole('button', { name: 'Delete Test' }));
  const confirmation = await screen.findByRole('alertdialog', { name: 'Delete managed item?' });
  expect(deleteMcpServer).not.toHaveBeenCalled();
  await userEvent.click(within(confirmation).getByRole('button', { name: 'Delete' }));
  await waitFor(() => expect(deleteMcpServer).toHaveBeenCalledExactlyOnceWith('test'));
  await screen.findByText(/record deleted, live cleanup unfinished/);
  expect(screen.queryByRole('button', { name: 'Edit Test' })).toBeNull();
  expect(screen.getByRole('button', { name: 'Sync' }).hasAttribute('disabled')).toBe(true);
  expect(screen.getByRole('button', { name: 'Retry live cleanup' }).hasAttribute('disabled')).toBe(false);
});

it('opens the MCP editor as a dialog and restores focus after Escape', async () => {
  render(<McpPage />);
  const edit = await screen.findByRole('button', { name: 'Edit Test' });
  await userEvent.click(edit);
  expect(screen.getByRole('dialog', { name: 'Edit MCP Server' })).toBeTruthy();
  await userEvent.keyboard('{Escape}');
  expect(screen.queryByRole('dialog')).toBeNull();
  await waitFor(() => expect(document.activeElement).toBe(edit));
});
