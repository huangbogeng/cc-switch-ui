import { fireEvent, render } from '@testing-library/react';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { ProviderLogo } from './ProviderLogo';
beforeEach(() => vi.stubEnv('BASE_URL', '/ui/'));
afterEach(() => vi.unstubAllEnvs());
it('loads a bundled logo and falls back if it fails', () => {
  const { container, rerender } = render(<ProviderLogo icon="deepseek" />);
  const image = container.querySelector('img')!;
  expect(image.getAttribute('src')).toBe('/ui/logos/deepseek.svg');
  fireEvent.error(image);
  expect(container.querySelector('img')).toBeNull();
  expect(container.querySelector('svg')).toBeTruthy();
  rerender(<ProviderLogo icon="minimax" />);
  expect(container.querySelector('img')?.getAttribute('src')).toBe('/ui/logos/minimax.svg');
});
it('uses a neutral icon for an unknown provider without fetching external URLs', () => {
  const { container } = render(<ProviderLogo icon="https://example.com/logo.svg" />);
  expect(container.querySelector('img')).toBeNull();
});
