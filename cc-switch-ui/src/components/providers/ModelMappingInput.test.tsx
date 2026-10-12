import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { expect, it, vi } from 'vitest';
import { ModelMappingInput } from './ModelMappingInput';

it('shows fetched models even when the configured value does not match them and permits manual input', () => {
  const change = vi.fn();
  render(<ModelMappingInput id="main" label="Main" value="unlisted-current" options={[' alpha ', 'beta', 'alpha']} onChange={change} />);
  fireEvent.click(screen.getByRole('button', { name: 'Choose Main model' }));
  expect(screen.getAllByRole('option')).toHaveLength(3);
  expect(screen.getByRole('option', { name: 'unlisted-current' }).getAttribute('aria-selected')).toBe('true');
  expect(screen.getByRole('option', { name: 'alpha' })).toBeTruthy();
  fireEvent.click(screen.getByRole('option', { name: 'beta' }));
  expect(change).toHaveBeenLastCalledWith('beta');
  fireEvent.change(screen.getByLabelText('Main'), { target: { value: 'manual-model' } });
  expect(change).toHaveBeenLastCalledWith('manual-model');
});

it('searches the full catalog beyond the first rendered page and closes on escape', async () => {
  const user = userEvent.setup();
  render(<ModelMappingInput id="sonnet" label="Sonnet" value="model-0" options={Array.from({ length: 250 }, (_, i) => `model-${i}`)} onChange={vi.fn()} />);
  await user.click(screen.getByRole('button', { name: 'Choose Sonnet model' }));
  expect(screen.getAllByRole('option')).toHaveLength(100);
  await user.type(screen.getByLabelText('Search Sonnet models'), 'model-249');
  expect(screen.getAllByRole('option')).toHaveLength(1);
  expect(screen.getByRole('option', { name: 'model-249' })).toBeTruthy();
  await user.keyboard('{Escape}');
  expect(screen.queryByRole('listbox')).toBeNull();
  expect(document.activeElement).toBe(screen.getByRole('button', { name: 'Choose Sonnet model' }));
});

it('supports keyboard selection and anchors the popup to the input width', async () => {
  const user = userEvent.setup(); const change = vi.fn();
  const { container } = render(<ModelMappingInput id="opus" label="Opus" value="manual" options={['alpha', 'beta']} onChange={change} />);
  const anchor = container.querySelector('input')!.parentElement!;
  vi.spyOn(anchor, 'getBoundingClientRect').mockReturnValue({ left: 40, right: 300, top: 80, bottom: 120, width: 260, height: 40, x: 40, y: 80, toJSON: () => ({}) });
  await user.click(screen.getByRole('button', { name: 'Choose Opus model' }));
  expect(screen.getByRole('dialog', { name: 'Opus model choices' }).style.width).toBe('260px');
  await user.type(screen.getByLabelText('Search Opus models'), 'beta');
  await user.keyboard('{ArrowDown}{Enter}');
  expect(change).toHaveBeenCalledWith('beta');
  expect(screen.queryByRole('listbox')).toBeNull();
});

it('dismisses stale choices when the catalog clears and closes on an outside click', async () => {
  const props = { id: 'haiku', label: 'Haiku', value: 'manual', onChange: vi.fn() };
  const { rerender } = render(<ModelMappingInput {...props} options={['alpha']} />);
  fireEvent.click(screen.getByRole('button', { name: 'Choose Haiku model' }));
  fireEvent.pointerDown(document.body);
  expect(screen.queryByRole('listbox')).toBeNull();
  fireEvent.click(screen.getByRole('button', { name: 'Choose Haiku model' }));
  rerender(<ModelMappingInput {...props} options={[]} />);
  await waitFor(() => expect(screen.queryByRole('listbox')).toBeNull());
  expect(screen.getByRole('button', { name: 'Choose Haiku model' })).toHaveProperty('disabled', true);
  expect(screen.getByLabelText('Haiku')).toHaveProperty('value', 'manual');
});

it.each([
  { viewport: 940, top: 600, bottom: 640, direction: 'down' },
  { viewport: 400, top: 240, bottom: 280, direction: 'up' },
])('keeps the menu and pagination inside the viewport when opening $direction', async ({ viewport, top, bottom, direction }) => {
  const original = window.innerHeight;
  Object.defineProperty(window, 'innerHeight', { value: viewport, configurable: true });
  try {
    const { container } = render(<ModelMappingInput id="edge" label="Main" value="" options={Array.from({ length: 150 }, (_, i) => `model-${i}`)} onChange={vi.fn()} />);
    const anchor = container.querySelector('input')!.parentElement!;
    vi.spyOn(anchor, 'getBoundingClientRect').mockReturnValue({ left: 40, right: 300, top, bottom, width: 260, height: 40, x: 40, y: top, toJSON: () => ({}) });
    fireEvent.click(screen.getByRole('button', { name: 'Choose Main model' }));
    const style = screen.getByRole('dialog', { name: 'Main model choices' }).style;
    const height = parseFloat(style.maxHeight);
    if (direction === 'down') {
      expect(parseFloat(style.top)).toBe(bottom + 4);
      expect(parseFloat(style.top) + height).toBeLessThanOrEqual(viewport - 8);
    } else {
      expect(parseFloat(style.bottom)).toBe(viewport - top + 4);
      expect(viewport - parseFloat(style.bottom) - height).toBeGreaterThanOrEqual(8);
    }
    expect(screen.getByRole('button', { name: 'Show more' })).toBeTruthy();
  } finally {
    Object.defineProperty(window, 'innerHeight', { value: original, configurable: true });
  }
});
