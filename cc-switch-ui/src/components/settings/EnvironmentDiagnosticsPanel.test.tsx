import { render, screen, fireEvent } from '@testing-library/react';
import { expect, it, vi } from 'vitest';
import { getEnvironmentDiagnostics } from '@/api';
import { EnvironmentDiagnosticsPanel } from './EnvironmentDiagnosticsPanel';
vi.mock('@/api', () => ({ getEnvironmentDiagnostics: vi.fn() }));
it('checks on demand, reports partial results and allows retry after failure', async () => {
  vi.mocked(getEnvironmentDiagnostics).mockRejectedValueOnce(new Error('Offline')).mockResolvedValue({ findings: [{ variable: 'ANTHROPIC_BASE_URL', application: 'Claude Code', source: '/test/.bashrc', kind: 'shell' }], warnings: ['.profile: could not read'] });
  render(<EnvironmentDiagnosticsPanel />);
  expect(getEnvironmentDiagnostics).not.toHaveBeenCalled();
  fireEvent.click(screen.getByRole('button', { name: 'Check overrides' }));
  await screen.findByText('Offline');
  fireEvent.click(screen.getByRole('button', { name: 'Check overrides' }));
  await screen.findByText('ANTHROPIC_BASE_URL');
  expect(screen.getByText(/Potential shell assignment/)).toBeTruthy();
  expect(screen.getByText('.profile: could not read')).toBeTruthy();
});
