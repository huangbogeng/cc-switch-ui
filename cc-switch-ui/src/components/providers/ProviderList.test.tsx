import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import type { Provider } from '@/api';
import { ProviderList } from './ProviderList';
const providers: Provider[] = [
  { id: 'alpha', name: 'Alpha', settingsConfig: { env: { ANTHROPIC_BASE_URL: 'https://alpha.example.com' } }, notes: 'Work account', meta: {}, inFailoverQueue: false },
  { id: 'beta', name: 'Beta', settingsConfig: { env: { ANTHROPIC_BASE_URL: 'https://beta.example.com' } }, meta: {}, inFailoverQueue: false },
];
const props = { providers, currentProviderId: 'alpha', busy: false, onSwitch: vi.fn(), onEdit: vi.fn(), onDelete: vi.fn() };
describe('ProviderList', () => {
  it('searches endpoint and notes, and clears an empty result', () => {
    render(<ProviderList {...props} />);
    const search = screen.getByRole('searchbox');
    fireEvent.change(search, { target: { value: 'BETA.EXAMPLE.COM' } });
    expect(screen.getByRole('heading', { name: 'Beta' })).toBeTruthy();
    expect(screen.queryByRole('heading', { name: 'Alpha' })).toBeNull();
    fireEvent.change(search, { target: { value: ' work account ' } });
    expect(screen.getByRole('heading', { name: 'Alpha' })).toBeTruthy();
    expect(screen.queryByRole('heading', { name: 'Beta' })).toBeNull();
    fireEvent.change(search, { target: { value: 'missing' } });
    expect(screen.getByRole('status').textContent).toContain('No providers match');
    fireEvent.click(screen.getByRole('button', { name: 'Clear search' }));
    expect(screen.getByRole('heading', { name: 'Alpha' })).toBeTruthy();
    expect(screen.getByRole('heading', { name: 'Beta' })).toBeTruthy();
  });
  it('dispatches actions for the matching provider after filtering', () => {
    const onSwitch = vi.fn(); const onEdit = vi.fn(); const onDelete = vi.fn();
    render(<ProviderList {...props} onSwitch={onSwitch} onEdit={onEdit} onDelete={onDelete} />);
    fireEvent.change(screen.getByRole('searchbox'), { target: { value: 'beta' } });
    fireEvent.click(screen.getByRole('button', { name: 'Select' }));
    fireEvent.click(screen.getByRole('button', { name: 'Edit Beta' }));
    fireEvent.click(screen.getByRole('button', { name: 'Delete Beta' }));
    expect(onSwitch).toHaveBeenCalledWith('beta');
    expect(onEdit).toHaveBeenCalledWith(providers[1]);
    expect(onDelete).toHaveBeenCalledWith('beta');
  });
  it('blocks mutations while busy without blocking search', () => {
    render(<ProviderList {...props} busy />);
    for (const button of screen.getAllByRole('button')) expect((button as HTMLButtonElement).disabled).toBe(true);
    expect((screen.getByRole('searchbox') as HTMLInputElement).disabled).toBe(false);
  });
});
