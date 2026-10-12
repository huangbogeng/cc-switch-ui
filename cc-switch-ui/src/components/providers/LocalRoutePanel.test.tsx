import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import type { Provider } from '@/api';
import { LocalRoutePanel } from './LocalRoutePanel';

function provider(id: string, name: string): Provider {
  return {
    id,
    name,
    settingsConfig: { env: { ANTHROPIC_BASE_URL: `https://${id}.example.com` } },
    meta: { apiFormat: 'anthropic' },
    inFailoverQueue: false,
  };
}

describe('LocalRoutePanel', () => {
  it('changes the route target independently from the direct provider', () => {
    const onTargetChange = vi.fn();
    render(
      <LocalRoutePanel
        providers={[provider('direct', 'Direct'), provider('route', 'Route')]}
        currentProviderId="direct"
        proxyRunning={false}
        proxyTargetId="route"
        busy={false}
        onTargetChange={onTargetChange}
        onStartProxy={vi.fn()}
        onStopProxy={vi.fn()}
      />,
    );

    expect(screen.getByText('Direct Config').parentElement?.textContent).toContain('Direct');
    expect((screen.getByLabelText('Route Target') as HTMLSelectElement).value).toBe('route');
    fireEvent.change(screen.getByLabelText('Route Target'), { target: { value: 'direct' } });
    expect(onTargetChange).toHaveBeenCalledWith('direct');
  });

  it('starts the selected route target', () => {
    const onStartProxy = vi.fn();
    render(
      <LocalRoutePanel
        providers={[provider('direct', 'Direct'), provider('route', 'Route')]}
        currentProviderId="direct"
        proxyRunning={false}
        proxyTargetId="route"
        busy={false}
        onTargetChange={vi.fn()}
        onStartProxy={onStartProxy}
        onStopProxy={vi.fn()}
      />,
    );

    fireEvent.click(screen.getByRole('button', { name: 'Start Route' }));
    expect(onStartProxy).toHaveBeenCalledWith('route');
  });
});

it('offers restoration after an interrupted route instead of starting another takeover', () => {
  const onStopProxy = vi.fn();
  render(<LocalRoutePanel providers={[]} currentProviderId={null}
    proxyRunning={false} recoveryRequired proxyTargetId={null} busy={false}
    onTargetChange={vi.fn()} onStartProxy={vi.fn()} onStopProxy={onStopProxy} />);
  expect(screen.queryByRole('button', { name: 'Start Route' })).toBeNull();
  fireEvent.click(screen.getByRole('button', { name: 'Restore Direct Config' }));
  expect(onStopProxy).toHaveBeenCalledOnce();
});

it('disables route mutations when the current route status is unavailable', () => {
  const onStartProxy = vi.fn();
  render(<LocalRoutePanel providers={[provider('route', 'Route')]} currentProviderId={null}
    proxyRunning={false} proxyTargetId="route" statusUnavailable busy={false}
    onTargetChange={vi.fn()} onStartProxy={onStartProxy} onStopProxy={vi.fn()} />);
  expect(screen.queryByText('Stopped')).toBeNull();
  expect((screen.getByLabelText('Route Target') as HTMLSelectElement).disabled).toBe(true);
  const start = screen.getByRole('button', { name: 'Start Route' });
  expect((start as HTMLButtonElement).disabled).toBe(true);
  fireEvent.click(start);
  expect(onStartProxy).not.toHaveBeenCalled();
});

it('marks unsupported targets and blocks start without blocking route restoration', () => {
  const gemini = { ...provider('gemini', 'Gemini'), meta: { apiFormat: 'gemini_native' } };
  const onStartProxy = vi.fn();
  const props = { providers: [gemini], currentProviderId: null, proxyTargetId: 'gemini', busy: false, onTargetChange: vi.fn(), onStartProxy, onStopProxy: vi.fn() };
  const view = render(<LocalRoutePanel {...props} proxyRunning={false} />);
  expect(screen.getByRole('option', { name: 'Gemini (local route unavailable)' })).toHaveProperty('disabled', true);
  const start = screen.getByRole('button', { name: 'Start Route' });
  expect(start).toHaveProperty('disabled', true);
  fireEvent.click(start);
  expect(onStartProxy).not.toHaveBeenCalled();
  expect(screen.getByRole('status').textContent).toContain('Gemini Native is not supported');
  view.rerender(<LocalRoutePanel {...props} proxyRunning />);
  expect(screen.getByRole('button', { name: 'Stop Route' })).toHaveProperty('disabled', false);
});

it('ignores unsupported selection events while allowing a supported replacement', () => {
  const gemini = { ...provider('gemini', 'Gemini'), meta: { apiFormat: 'gemini_native' } };
  const onTargetChange = vi.fn();
  render(<LocalRoutePanel providers={[gemini, provider('native', 'Native')]} currentProviderId={null} proxyRunning={false} proxyTargetId="native" busy={false} onTargetChange={onTargetChange} onStartProxy={vi.fn()} onStopProxy={vi.fn()} />);
  fireEvent.change(screen.getByLabelText('Route Target'), { target: { value: 'gemini' } });
  expect(onTargetChange).not.toHaveBeenCalled();
  fireEvent.change(screen.getByLabelText('Route Target'), { target: { value: 'native' } });
  expect(onTargetChange).toHaveBeenCalledWith('native');
});
