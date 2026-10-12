import { act, fireEvent, render, screen } from '@testing-library/react';
import { expect, it, vi } from 'vitest';
import { fetchCodexModels } from '@/api';
import { CodexDiagnostics } from './CodexDiagnostics';

vi.mock('@/api', () => ({ fetchCodexModels: vi.fn() }));
const configured = { model: 'catalog-main[1m]', haikuModel: 'manual-model', sonnetModel: 'catalog-main', opusModel: '' };

it('checks the selected OAuth account and reports catalog presence without changing mappings', async () => {
  vi.mocked(fetchCodexModels).mockResolvedValue({ models: [{ id: 'catalog-main' }, { id: 'catalog-main' }, { id: 'other' }] });
  const snapshot = structuredClone(configured);
  render(<CodexDiagnostics accountId="fixture-b" resolvedAccountId="fixture-b" canCheck configured={configured} />);
  fireEvent.click(screen.getByRole('button', { name: 'Check OAuth connection' }));
  expect(await screen.findByRole('status')).toHaveProperty('textContent', 'OAuth model catalog reached successfully. 2 models returned.');
  expect(fetchCodexModels).toHaveBeenLastCalledWith('fixture-b');
  expect(screen.getAllByText('Listed in catalog')).toHaveLength(2);
  expect(screen.getByText('Not listed in catalog')).toBeTruthy();
  expect(configured).toEqual(snapshot);
});

it('uses the default account and shows failures without keeping a previous success', async () => {
  vi.mocked(fetchCodexModels).mockResolvedValueOnce({ models: [{ id: 'catalog-main' }] }).mockRejectedValueOnce(new Error('Sign in again on the OAuth page.'));
  render(<CodexDiagnostics resolvedAccountId="fixture-a" canCheck configured={configured} />);
  fireEvent.click(screen.getByRole('button', { name: 'Check OAuth connection' }));
  await screen.findByRole('status');
  expect(fetchCodexModels).toHaveBeenLastCalledWith(undefined);
  fireEvent.click(screen.getByRole('button', { name: 'Check OAuth connection' }));
  expect(await screen.findByRole('alert')).toHaveProperty('textContent', 'Sign in again on the OAuth page.');
  expect(screen.queryByRole('status')).toBeNull();
});

it('ignores the response from a previous account and clears diagnostics after disconnection', async () => {
  let finish!: (value: { models: { id: string }[] }) => void;
  vi.mocked(fetchCodexModels).mockImplementationOnce(() => new Promise(resolve => { finish = resolve; }));
  const { rerender } = render(<CodexDiagnostics resolvedAccountId="fixture-a" canCheck configured={configured} />);
  fireEvent.click(screen.getByRole('button', { name: 'Check OAuth connection' }));
  rerender(<CodexDiagnostics resolvedAccountId="fixture-b" canCheck configured={configured} />);
  await act(async () => { finish({ models: [{ id: 'old-account-model' }] }); });
  expect(screen.queryByRole('status')).toBeNull();
  rerender(<CodexDiagnostics resolvedAccountId="fixture-b" canCheck={false} configured={configured} />);
  expect(screen.getByRole('button', { name: 'Check OAuth connection' })).toHaveProperty('disabled', true);
  expect(screen.getByText(/Connect an account on the OAuth page/)).toBeTruthy();
});

it('reuses the checked catalog when mappings change within the same account', async () => {
  vi.mocked(fetchCodexModels).mockClear();
  vi.mocked(fetchCodexModels).mockResolvedValue({ models: [{ id: 'catalog-main' }] });
  const { rerender } = render(<CodexDiagnostics resolvedAccountId="fixture-a" canCheck configured={configured} />);
  fireEvent.click(screen.getByRole('button', { name: 'Check OAuth connection' }));
  await screen.findByRole('status');
  rerender(<CodexDiagnostics resolvedAccountId="fixture-a" canCheck configured={{ ...configured, haikuModel: 'catalog-main' }} />);
  expect(screen.getAllByText('Listed in catalog')).toHaveLength(3);
  expect(fetchCodexModels).toHaveBeenCalledOnce();
});
