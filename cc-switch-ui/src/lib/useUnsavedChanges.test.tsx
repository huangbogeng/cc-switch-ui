import { afterEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { confirmDiscardChanges, useUnsavedChanges } from './useUnsavedChanges';

function Editor({ dirty, busy = false }: { dirty: boolean; busy?: boolean }) {
  const discard = useUnsavedChanges(dirty, busy);
  return <button onClick={() => { if (discard()) window.dispatchEvent(new Event('discard')); }}>Cancel</button>;
}

afterEach(() => { cleanup(); vi.restoreAllMocks(); });

describe('unsaved changes', () => {
  it('protects navigation and dialog cancellation, and cleans up on unmount', () => {
    const confirm = vi.spyOn(window, 'confirm').mockReturnValue(false);
    const discard = vi.fn();
    window.addEventListener('discard', discard);
    const view = render(<Editor dirty />);
    expect(confirmDiscardChanges()).toBe(false);
    fireEvent.click(screen.getByRole('button', { name: 'Cancel' }));
    expect(discard).not.toHaveBeenCalled();
    confirm.mockReturnValue(true);
    expect(confirmDiscardChanges()).toBe(true);
    fireEvent.click(screen.getByRole('button', { name: 'Cancel' }));
    expect(discard).toHaveBeenCalledOnce();
    view.unmount();
    confirm.mockClear();
    expect(confirmDiscardChanges()).toBe(true);
    expect(confirm).not.toHaveBeenCalled();
    window.removeEventListener('discard', discard);
  });

  it('blocks navigation during save and protects browser unload', () => {
    const confirm = vi.spyOn(window, 'confirm').mockReturnValue(true);
    const view = render(<Editor dirty={false} busy />);
    expect(confirmDiscardChanges()).toBe(false);
    expect(confirm).not.toHaveBeenCalled();
    const event = new Event('beforeunload', { cancelable: true });
    window.dispatchEvent(event);
    expect(event.defaultPrevented).toBe(true);
    view.rerender(<Editor dirty={false} />);
    expect(confirmDiscardChanges()).toBe(true);
    const cleanEvent = new Event('beforeunload', { cancelable: true });
    window.dispatchEvent(cleanEvent);
    expect(cleanEvent.defaultPrevented).toBe(false);
  });
});
