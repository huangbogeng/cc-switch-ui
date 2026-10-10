import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { expect, it, vi } from 'vitest';
import { useConfirm } from './useConfirm';
import { useDialog } from './useDialog';

it('focuses Cancel, isolates nested Escape, retains the outer scroll lock and restores focus', async () => {
  const outerClose = vi.fn();
  const result = vi.fn();
  document.body.style.overflow = 'scroll';
  function Editor() {
    const ref = useDialog(true, outerClose);
    const { confirmAction, confirmation } = useConfirm();
    return <div ref={ref} role="dialog" tabIndex={-1}>
      <button onClick={() => { void confirmAction('Files will be replaced.', { title: 'Restore files?', confirmLabel: 'Restore' }).then(result); }}>Review restore</button>
      {confirmation}
    </div>;
  }
  const rendered = render(<Editor />);
  const opener = screen.getByRole('button', { name: 'Review restore' });
  await userEvent.click(opener);
  expect(screen.getByRole('alertdialog').textContent).toContain('Files will be replaced.');
  expect(document.activeElement).toBe(screen.getByRole('button', { name: 'Cancel' }));
  expect(document.body.style.overflow).toBe('hidden');
  await userEvent.tab({ shift: true });
  expect(document.activeElement).toBe(screen.getByRole('button', { name: 'Restore' }));
  fireEvent.keyDown(document, { key: 'Escape' });
  await waitFor(() => expect(result).toHaveBeenCalledWith(false));
  expect(outerClose).not.toHaveBeenCalled();
  expect(document.activeElement).toBe(opener);
  expect(document.body.style.overflow).toBe('hidden');
  await userEvent.click(opener);
  await userEvent.click(screen.getByRole('button', { name: 'Restore' }));
  await waitFor(() => expect(result).toHaveBeenLastCalledWith(true));
  rendered.unmount();
  expect(document.body.style.overflow).toBe('scroll');
  document.body.style.overflow = '';
});

it('cancels an unresolved confirmation on unmount without performing the action', async () => {
  const result = vi.fn();
  function Harness() {
    const { confirmAction, confirmation } = useConfirm();
    return <><button onClick={() => { void confirmAction('Delete?', { confirmLabel: 'Delete' }).then(result); }}>Open</button>{confirmation}</>;
  }
  const rendered = render(<Harness />);
  await userEvent.click(screen.getByRole('button', { name: 'Open' }));
  rendered.unmount();
  await waitFor(() => expect(result).toHaveBeenCalledExactlyOnceWith(false));
  expect(document.body.style.overflow).toBe('');
});

it('traps Tab back into the outer dialog when the confirmation opener becomes disabled', async () => {
  const result = vi.fn();
  function Editor() {
    const ref = useDialog(true, vi.fn());
    const { confirmAction, confirmation } = useConfirm();
    return <div ref={ref} role="dialog" tabIndex={-1}>
      <button onClick={() => { void confirmAction('Apply?', { confirmLabel: 'Apply' }).then(result); }}>Open confirmation</button>
      <button>Other editor control</button>
      {confirmation}
    </div>;
  }
  const view = render(<Editor />);
  const opener = screen.getByRole('button', { name: 'Open confirmation' }) as HTMLButtonElement;
  await userEvent.click(opener);
  opener.disabled = true;
  await userEvent.click(screen.getByRole('button', { name: 'Apply' }));
  await waitFor(() => expect(result).toHaveBeenCalledWith(true));
  expect(document.activeElement).toBe(screen.getByRole('dialog'));
  await userEvent.tab();
  expect(document.activeElement).toBe(screen.getByRole('button', { name: 'Other editor control' }));
  // Recover focus even if another caller programmatically moves it to the background.
  document.body.tabIndex = -1;
  document.body.focus();
  await userEvent.tab({ shift: true });
  expect(document.activeElement).toBe(screen.getByRole('button', { name: 'Other editor control' }));
  document.body.removeAttribute('tabindex');
  view.unmount();
});
