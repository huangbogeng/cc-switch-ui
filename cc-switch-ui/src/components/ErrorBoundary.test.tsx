import { afterEach, expect, it, vi } from 'vitest';
import { cleanup, render, screen } from '@testing-library/react';
import { ErrorBoundary } from './ErrorBoundary';
import { lazy, Suspense } from 'react';

afterEach(() => { cleanup(); vi.restoreAllMocks(); });

function Failure({ message }: { message: string }): never {
  throw new TypeError(message);
}

it.each([
  'Failed to fetch dynamically imported module: /ui/assets/missing.js',
  'error loading dynamically imported module: /ui/assets/missing.js',
  'Importing a module script failed.',
  'Unable to preload CSS for /ui/assets/missing.css',
])('explains module loading failure without exposing its raw URL: %s', message => {
  vi.spyOn(console, 'error').mockImplementation(() => {});
  render(<ErrorBoundary page><Failure message={message} /></ErrorBoundary>);
  expect(screen.getByRole('heading', { name: 'The page could not be loaded' })).toBeTruthy();
  expect(screen.getByText(/Reload the interface to load the installed version/)).toBeTruthy();
  expect(screen.getByRole('button', { name: 'Reload interface' })).toBeTruthy();
  expect(screen.queryByText(message)).toBeNull();
});

it('retains the generic error fallback for unrelated render failures', () => {
  vi.spyOn(console, 'error').mockImplementation(() => {});
  render(<ErrorBoundary><Failure message="Unexpected render failure" /></ErrorBoundary>);
  expect(screen.getByRole('heading', { name: 'The interface encountered an unexpected error' })).toBeTruthy();
  expect(screen.getByText('Unexpected render failure')).toBeTruthy();
});

it('recovers a rejected lazy import without automatic navigation or token changes', async () => {
  vi.spyOn(console, 'error').mockImplementation(() => {});
  localStorage.setItem('ccswitch_token', 'synthetic-token');
  const page = window.location.href;
  const MissingPage = lazy(() => Promise.reject(new TypeError('Failed to fetch dynamically imported module: /ui/assets/missing.js')));
  render(<ErrorBoundary page><Suspense fallback={<span>Loading</span>}><MissingPage /></Suspense></ErrorBoundary>);
  expect(await screen.findByRole('heading', { name: 'The page could not be loaded' })).toBeTruthy();
  expect(window.location.href).toBe(page);
  expect(localStorage.getItem('ccswitch_token')).toBe('synthetic-token');
  localStorage.removeItem('ccswitch_token');
});
