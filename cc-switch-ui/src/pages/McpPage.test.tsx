import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { cleanup, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { ApiError, listMcpServers, saveMcpServer, toggleMcpServer } from '@/api';
import McpPage from './McpPage';

vi.mock('@/api', async (original) => ({
  ...await original<typeof import('@/api')>(),
  listMcpServers: vi.fn(), saveMcpServer: vi.fn(), toggleMcpServer: vi.fn(),
}));
vi.mock('@/lib/fetchCache', () => ({ cacheGet: () => undefined, cacheSet: vi.fn() }));
const server = { id: 'test', name: 'Test', appType: 'claude_code', enabled: true, serverSpec: { command: 'npx' } };

beforeEach(() => {
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
