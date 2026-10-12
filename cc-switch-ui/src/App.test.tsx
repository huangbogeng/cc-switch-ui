import { afterEach, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import App from './App';
import { queryClient } from '@/lib/queryClient';
import { cacheSet, cacheGet } from '@/lib/fetchCache';

vi.mock('@/components/ServerStatus', () => ({ ServerStatus: () => <span>Local Server</span> }));

vi.mock('./pages/ProvidersPage', async () => {
  const { useUnsavedChanges } = await import('./lib/useUnsavedChanges');
  function ProviderEditor() { useUnsavedChanges(true); return <div>Dirty provider editor</div>; }
  return { default: ProviderEditor };
});
vi.mock('./pages/DashboardPage', () => ({ default: () => <div>Dashboard content</div> }));
vi.mock('./pages/SettingsPage', () => ({ default: () => { throw new TypeError('Failed to fetch dynamically imported module: /ui/assets/missing-settings.js'); } }));

afterEach(() => { cleanup(); vi.restoreAllMocks(); localStorage.clear(); window.history.replaceState(null, '', '/'); });

it('guards sidebar, hash navigation and logout while preserving the editor', async () => {
  localStorage.setItem('ccswitch_token', 'test');
  queryClient.setQueryData(['session-test'], 'cached');
  cacheSet('session-test', 'cached');
  window.history.replaceState(null, '', '#providers');
  const confirm = vi.spyOn(window, 'confirm').mockReturnValue(false);
  render(<App />);
  await screen.findByText('Dirty provider editor');
  fireEvent.click(screen.getByRole('button', { name: /Dashboard/ }));
  expect(screen.getByText('Dirty provider editor')).toBeTruthy();
  expect(window.location.hash).toBe('#providers');
  window.history.replaceState(null, '', '#dashboard');
  fireEvent(window, new HashChangeEvent('hashchange'));
  expect(window.location.hash).toBe('#providers');
  fireEvent.click(screen.getAllByRole('button', { name: 'Logout' })[0]);
  expect(localStorage.getItem('ccswitch_token')).toBe('test');
  confirm.mockReturnValue(true);
  fireEvent.click(screen.getByRole('button', { name: /Dashboard/ }));
  expect(await screen.findByText('Dashboard content')).toBeTruthy();
  fireEvent.click(screen.getAllByRole('button', { name: 'Logout' })[0]);
  expect(queryClient.getQueryData(['session-test'])).toBeUndefined();
  expect(cacheGet('session-test')).toBeUndefined();
});

it('keeps navigation available after a missing page chunk and offers explicit reload', async () => {
  vi.spyOn(console, 'error').mockImplementation(() => {});
  localStorage.setItem('ccswitch_token', 'test');
  window.history.replaceState(null, '', '#settings');
  render(<App />);
  expect(await screen.findByRole('heading', { name: 'The page could not be loaded' })).toBeTruthy();
  expect(screen.getByRole('button', { name: 'Reload interface' })).toBeTruthy();
  expect(screen.queryByText(/missing-settings\.js/)).toBeNull();
  fireEvent.click(screen.getByRole('button', { name: /Dashboard/ }));
  expect(await screen.findByText('Dashboard content')).toBeTruthy();
  expect(screen.queryByRole('alert')).toBeNull();
  expect(localStorage.getItem('ccswitch_token')).toBe('test');
});
