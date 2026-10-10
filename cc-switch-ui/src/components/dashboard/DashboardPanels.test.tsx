import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { DashboardHeroCard } from './DashboardPanels';
const status = { running: false, recovery_required: true, listen_addr: null, upstream_url: '', http_proxy_url: null, active_target_provider_id: 'route', active_target_provider_name: 'Route account' };
describe('DashboardHeroCard', () => {
  it('does not report a stopped route when status is unavailable', () => {
    render(<DashboardHeroCard currentProvider={null} routeTarget={null} status={null} />);
    expect(screen.queryByText('Stopped')).toBeNull();
    expect(screen.getByText('Route status is not available.')).toBeTruthy();
  });
  it('surfaces recovery and falls back to the reported target name', () => {
    render(<DashboardHeroCard currentProvider={null} routeTarget={null} status={status} />);
    expect(screen.getByText('Recovery needed')).toBeTruthy();
    expect(screen.getByText('Route account')).toBeTruthy();
    expect(screen.getByText('Restore Direct Config in Providers.')).toBeTruthy();
    expect(screen.queryByText('Stopped')).toBeNull();
  });
});
