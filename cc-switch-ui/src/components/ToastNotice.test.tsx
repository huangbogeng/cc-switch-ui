import { act, fireEvent, render, screen, within } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { NotificationViewport, ToastNotice } from './ToastNotice';

afterEach(() => vi.useRealTimers());

describe('ToastNotice', () => {
  it('renders outside page content and expires without removing page content', () => {
    vi.useFakeTimers();
    const onDismiss = vi.fn();
    const { container } = render(<main><h1>Providers</h1><ToastNotice message="Saved" onDismiss={onDismiss} /></main>);
    expect(container.contains(screen.getByRole('status'))).toBe(false);
    act(() => vi.advanceTimersByTime(5000));
    expect(onDismiss).toHaveBeenCalledOnce();
    expect(screen.queryByText('Saved')).toBeNull();
    expect(screen.getByRole('heading', { name: 'Providers' })).toBeTruthy();
  });

  it('pauses dismissal on hover and keyboard focus independently', () => {
    vi.useFakeTimers();
    const onDismiss = vi.fn();
    render(<ToastNotice message="Saved" onDismiss={onDismiss} />);
    const notice = screen.getByRole('status');
    const close = screen.getByRole('button', { name: 'Dismiss notification' });
    fireEvent.mouseEnter(notice);
    fireEvent.focus(close);
    fireEvent.mouseLeave(notice);
    act(() => vi.advanceTimersByTime(10000));
    expect(onDismiss).not.toHaveBeenCalled();
    fireEvent.blur(close, { relatedTarget: document.body });
    act(() => vi.advanceTimersByTime(5000));
    expect(onDismiss).toHaveBeenCalledOnce();
  });

  it('supports manual dismissal and cancels the timeout', () => {
    vi.useFakeTimers();
    const onDismiss = vi.fn();
    render(<ToastNotice tone="error" message="Failed" onDismiss={onDismiss} />);
    expect(screen.getByRole('alert')).toBeTruthy();
    fireEvent.click(screen.getByRole('button', { name: 'Dismiss notification' }));
    expect(screen.queryByRole('alert')).toBeNull();
    act(() => vi.advanceTimersByTime(10000));
    expect(onDismiss).toHaveBeenCalledOnce();
  });

  it('cancels timers on unmount', () => {
    vi.useFakeTimers();
    const onDismiss = vi.fn();
    const { unmount } = render(<ToastNotice message="Saved" onDismiss={onDismiss} />);
    unmount();
    act(() => vi.advanceTimersByTime(5000));
    expect(onDismiss).not.toHaveBeenCalled();
  });

  it('puts multiple notices in the shared viewport', () => {
    const viewport = render(<NotificationViewport />);
    render(<><ToastNotice message="First" onDismiss={vi.fn()} /><ToastNotice message="Second" onDismiss={vi.fn()} /></>);
    const region = screen.getByRole('region', { name: 'Notifications' });
    expect(within(region).getAllByRole('status')).toHaveLength(2);
    expect(viewport.container.contains(region)).toBe(true);
  });
});
