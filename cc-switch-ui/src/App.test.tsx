import { afterEach, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import App from './App';

vi.mock('./pages/ProvidersPage', async () => {
  const { useUnsavedChanges } = await import('./lib/useUnsavedChanges');
  function ProviderEditor() { useUnsavedChanges(true); return <div>Dirty provider editor</div>; }
  return { default: ProviderEditor };
});
vi.mock('./pages/DashboardPage', () => ({ default: () => <div>Dashboard content</div> }));

afterEach(() => { cleanup(); vi.restoreAllMocks(); localStorage.clear(); window.history.replaceState(null, '', '/'); });

it('guards sidebar, hash navigation and logout while preserving the editor', () => {
  localStorage.setItem('ccswitch_token', 'test');
  window.history.replaceState(null, '', '#providers');
  const confirm = vi.spyOn(window, 'confirm').mockReturnValue(false);
  render(<App />);
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
  expect(screen.getByText('Dashboard content')).toBeTruthy();
});
