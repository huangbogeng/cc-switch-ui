import { afterEach, expect, it, vi } from 'vitest';
import { cleanup, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { ApiError, saveSkill } from '@/api';
import { SkillFormDialog } from './SkillFormDialog';

vi.mock('@/api', async (original) => ({
  ...await original<typeof import('@/api')>(), saveSkill: vi.fn(),
}));
afterEach(() => { cleanup(); vi.clearAllMocks(); vi.restoreAllMocks(); });

it('refreshes saved records but retains the editor when applying a skill fails', async () => {
  const saved = vi.fn();
  const close = vi.fn();
  vi.mocked(saveSkill).mockRejectedValue(new ApiError('Record saved, but Skills sync failed. Retry Sync.', true));
  render(<SkillFormDialog open onClose={close} onSaved={saved} initialSkill={{
    id: 'test', name: 'Test', directory: 'test', appType: 'claude_code', enabled: true, installedAt: 0,
  }} />);
  await userEvent.clear(screen.getByLabelText('Name'));
  await userEvent.type(screen.getByLabelText('Name'), 'Edited');
  await userEvent.click(screen.getByRole('button', { name: 'Save' }));
  await screen.findByText('Record saved, but Skills sync failed. Retry Sync.');
  await waitFor(() => expect(saved).toHaveBeenCalledOnce());
  expect(close).not.toHaveBeenCalled();
  expect((screen.getByLabelText('Name') as HTMLInputElement).value).toBe('Edited');
});
