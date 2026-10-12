import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { providerPresets } from '@/config/providerPresets';
import { ProviderFormDialog } from './ProviderFormDialog';
import { buildProvider, formFromPreset, formFromProvider } from './providerForm';

describe('subscription provider editing', () => {
  it('keeps credentials hidden and submits the keyless mode after reopening', () => {
    const preset = providerPresets.find((item) => item.id === 'claude-official')!;
    const provider = buildProvider(formFromPreset(preset), preset);
    const onSubmit = vi.fn();
    render(<ProviderFormDialog
      open editingId={provider.id} selectedPreset={null}
      initialFormData={formFromProvider(provider)} saving={false}
      onPresetSelect={vi.fn()} onCancel={vi.fn()} onSubmit={onSubmit}
    />);
    expect(screen.getByText(/Uses your logged-in Claude subscription/)).toBeTruthy();
    expect(screen.queryByLabelText('API Key')).toBeNull();
    expect(screen.queryByLabelText('Base URL')).toBeNull();
    expect(screen.queryByLabelText('API Timeout MS')).toBeNull();
    expect(screen.queryByLabelText('Disable Claude Code nonessential traffic')).toBeNull();
    fireEvent.submit(screen.getByRole('dialog').querySelector('form')!);
    expect(onSubmit).toHaveBeenCalledWith(expect.objectContaining({ noAuth: true }));
  });
});

it('previews the current draft without saving and invalidates the result after editing', async () => {
  const preset = providerPresets.find(item => item.id === 'claude-official')!;
  const provider = buildProvider(formFromPreset(preset), preset);
  const onSubmit = vi.fn();
  const onPreview = vi.fn().mockResolvedValue({ applied: false, recordPath: '/test/db', livePath: null, recordKeys: ['record/name'], liveKeys: [], message: 'Record only' });
  render(<ProviderFormDialog open editingId={provider.id} selectedPreset={null} initialFormData={formFromProvider(provider)} saving={false} onPresetSelect={vi.fn()} onCancel={vi.fn()} onSubmit={onSubmit} onPreview={onPreview} />);
  fireEvent.click(screen.getByRole('button', { name: 'Preview save' }));
  await screen.findByText('Record only');
  expect(onSubmit).not.toHaveBeenCalled();
  fireEvent.change(screen.getByLabelText('Name'), { target: { value: 'Changed' } });
  expect(screen.queryByText('Record only')).toBeNull();
});
