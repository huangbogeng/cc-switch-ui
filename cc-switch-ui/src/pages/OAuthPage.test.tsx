import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, expect, it, vi } from 'vitest';
import * as api from '@/api';
import OAuthPage from './OAuthPage';
vi.mock('@/api', () => ({ getCodexOAuthStatus: vi.fn(), getCopilotOAuthStatus: vi.fn(), startCodexOAuth: vi.fn(), pollCodexOAuth: vi.fn(), startCopilotOAuth: vi.fn(), pollCopilotOAuth: vi.fn(), removeCodexAccount: vi.fn(), setDefaultCodexAccount: vi.fn(), removeCopilotAccount: vi.fn(), setDefaultCopilotAccount: vi.fn() }));
vi.mock('@/lib/fetchCache', () => ({ cacheGet: () => undefined, cacheSet: vi.fn() }));
const codex = { authenticated: true, accounts: [{ id: 'codex-account', login: 'ChatGPT user', is_default: false }] };
const copilot = { authenticated: true, accounts: [{ id: 'copilot-account', login: 'GitHub user', avatar_url: null, github_domain: 'github.com' }], default_account_id: null };
beforeEach(() => {
  vi.resetAllMocks();
  vi.mocked(api.getCodexOAuthStatus).mockResolvedValue(codex);
  vi.mocked(api.getCopilotOAuthStatus).mockResolvedValue(copilot);
});
it('distinguishes unavailable accounts from disconnected accounts and retries independently', async () => {
  vi.mocked(api.getCodexOAuthStatus).mockRejectedValueOnce(new Error('Account read failed')).mockResolvedValue(codex);
  render(<OAuthPage />);
  await screen.findByText('Account status unavailable');
  const retry = screen.getByRole('button', { name: 'Retry ChatGPT accounts' });
  expect(screen.getByRole('button', { name: 'Connect' }).hasAttribute('disabled')).toBe(true);
  fireEvent.click(retry);
  await screen.findAllByText('ChatGPT user');
  expect(screen.queryByText('Account status unavailable')).toBeNull();
  expect(api.getCodexOAuthStatus).toHaveBeenCalledTimes(2);
  expect(api.getCopilotOAuthStatus).toHaveBeenCalledTimes(1);
});
it('refreshes a default account change after losing its response without claiming success', async () => {
  vi.mocked(api.setDefaultCodexAccount).mockImplementation(async () => {
    vi.mocked(api.getCodexOAuthStatus).mockResolvedValue({ ...codex, accounts: [{ ...codex.accounts[0], is_default: true }] });
    throw new Error('Default response lost');
  });
  render(<OAuthPage />);
  await screen.findAllByText('ChatGPT user');
  fireEvent.click(screen.getAllByRole('button', { name: 'Set Default' })[0]);
  await screen.findByText('Default response lost');
  expect(screen.getByRole('button', { name: 'Default' }).hasAttribute('disabled')).toBe(true);
  expect(screen.queryByRole('status')).toBeNull();
});
it('refreshes removed Copilot accounts after losing the response', async () => {
  vi.mocked(api.removeCopilotAccount).mockImplementation(async () => {
    vi.mocked(api.getCopilotOAuthStatus).mockResolvedValue({ authenticated: false, accounts: [], default_account_id: null });
    throw new Error('Remove response lost');
  });
  render(<OAuthPage />);
  fireEvent.click(await screen.findByRole('button', { name: 'Remove GitHub user' }));
  fireEvent.click(await screen.findByRole('button', { name: 'Remove' }));
  await screen.findByText('Remove response lost');
  await waitFor(() => expect(screen.queryByRole('button', { name: 'Remove GitHub user' })).toBeNull());
  expect(screen.queryByRole('status')).toBeNull();
});
