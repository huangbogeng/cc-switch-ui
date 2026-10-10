import { fireEvent, render, screen } from '@testing-library/react';
import { expect, it, vi } from 'vitest';
import { LocalRoutePanel } from './LocalRoutePanel';

it('offers restoration after an interrupted route instead of starting another takeover', () => {
  const onStopProxy = vi.fn();
  render(<LocalRoutePanel providers={[]} currentProviderId={null}
    proxyRunning={false} recoveryRequired proxyTargetId={null} busy={false}
    onTargetChange={vi.fn()} onStartProxy={vi.fn()} onStopProxy={onStopProxy} />);
  expect(screen.queryByRole('button', { name: 'Start Route' })).toBeNull();
  fireEvent.click(screen.getByRole('button', { name: 'Restore Direct Config' }));
  expect(onStopProxy).toHaveBeenCalledOnce();
});
