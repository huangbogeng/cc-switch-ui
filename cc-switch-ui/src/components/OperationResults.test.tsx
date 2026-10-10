import { act, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, expect, it, vi } from 'vitest';
import type { OperationReport } from '@/api';
import { OperationResults } from './OperationResults';

const failed: OperationReport = { success: false, imported: 0, updated: 0, applied: 0, skipped: 0, failed: 1,
  items: [{ id: 'blocked', outcome: 'failed', error: 'Source missing' }] };
afterEach(() => vi.useRealTimers());

it('floats failed batch details and expires without changing page content', () => {
  vi.useFakeTimers();
  const { container } = render(<main><h1>Skills</h1><OperationResults report={failed} action="Import" /></main>);
  expect(container.contains(screen.getByRole('alert'))).toBe(false);
  expect(screen.getByText(/Source missing/)).toBeTruthy();
  act(() => vi.advanceTimersByTime(8000));
  expect(screen.queryByRole('alert')).toBeNull();
  expect(screen.getByRole('heading', { name: 'Skills' })).toBeTruthy();
});

it('shows a fresh report after dismissing an identical earlier result', () => {
  const { rerender } = render(<OperationResults report={failed} action="Import" />);
  fireEvent.click(screen.getByRole('button', { name: 'Dismiss notification' }));
  expect(screen.queryByRole('alert')).toBeNull();
  rerender(<OperationResults report={{ ...failed }} action="Import" />);
  expect(screen.getByRole('alert')).toBeTruthy();
});
