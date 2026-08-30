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
    expect(screen.getByText('Local Route Takeover').parentElement?.textContent).toContain('Route');
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
