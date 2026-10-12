import { fetchProviderModels } from '@/api';
vi.mock('@/api', () => ({ fetchProviderModels: vi.fn(), detectProviderEndpointType: vi.fn() }));
import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
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

it('fetches without editing, then applies a model only to the selected role', async () => {
  vi.mocked(fetchProviderModels).mockResolvedValue({ models:[{id:'picked-model'}] });
  const preset = providerPresets.find(item => item.id === 'claude-official')!;
  const form = formFromPreset(preset); form.baseUrl = 'https://provider.example'; form.apiKey = 'test-key'; form.noAuth = false; form.authMode = 'api_key'; form.model = 'original-main';
  const submit = vi.fn();
  render(<ProviderFormDialog open editingId="test" selectedPreset={null} initialFormData={form} saving={false} onPresetSelect={vi.fn()} onCancel={vi.fn()} onSubmit={submit} />);
  fireEvent.click(screen.getByRole('button', {name:'Fetch models'}));
  await screen.findByRole('dialog', {name:'Choose model'});
  expect(submit).not.toHaveBeenCalled();
  fireEvent.change(screen.getByLabelText('Use for'), {target:{value:'sonnetModel'}});
  fireEvent.click(screen.getByRole('radio', {name:/picked-model/})); fireEvent.click(screen.getByRole('button', {name:'Apply model'}));
  expect(screen.getByLabelText('Main')).toHaveProperty('value','original-main'); expect(screen.getByLabelText('Sonnet')).toHaveProperty('value','picked-model');
  fireEvent.change(screen.getByLabelText('Base URL'), {target:{value:'https://changed.example'}});
  await waitFor(() => expect(screen.getByRole('button', {name:'Fetch models'})).toBeTruthy());
  expect(screen.queryByRole('button', {name:'Choose model'})).toBeNull();
});

it('ignores a response from the previous endpoint while a new model list is fetched', async () => {
  let finish!: (value: { models: { id: string }[] }) => void;
  const pending = new Promise<{ models: { id: string }[] }>(resolve => { finish = resolve; });
  vi.mocked(fetchProviderModels).mockImplementationOnce(() => pending).mockResolvedValue({ models:[{id:'new-endpoint-model'}] });
  const form = formFromPreset(providerPresets.find(item => item.id === 'claude-official')!);
  Object.assign(form, {baseUrl:'https://old.example',apiKey:'test-key',noAuth:false,authMode:'api_key'});
  render(<ProviderFormDialog open editingId="test" selectedPreset={null} initialFormData={form} saving={false} onPresetSelect={vi.fn()} onCancel={vi.fn()} onSubmit={vi.fn()} />);
  fireEvent.click(screen.getByRole('button', {name:'Fetch models'}));
  fireEvent.change(screen.getByLabelText('Base URL'), {target:{value:'https://new.example'}});
  await waitFor(() => expect(screen.getByRole('button', {name:'Fetch models'})).toHaveProperty('disabled',false));
  await act(async () => { finish({models:[{id:'old-endpoint-model'}]}); });
  expect(screen.queryByRole('dialog', {name:'Choose model'})).toBeNull();
  fireEvent.click(screen.getByRole('button', {name:'Fetch models'}));
  await screen.findByRole('radio', {name:/new-endpoint-model/});
  expect(screen.queryByRole('radio', {name:/old-endpoint-model/})).toBeNull();
});

it('locks Codex OAuth to Responses and keeps model discovery disabled', () => {
  const preset = providerPresets.find(item => item.id === 'codex')!;
  const submit = vi.fn();
  render(<ProviderFormDialog open editingId={null} selectedPreset={preset} initialFormData={{ ...formFromPreset(preset), apiFormat: 'gemini_native' }} saving={false} onPresetSelect={vi.fn()} onCancel={vi.fn()} onSubmit={submit} />);
  const format = screen.getByLabelText('API Format');
  expect(format).toHaveProperty('value', 'openai_responses');
  expect(format).toHaveProperty('disabled', true);
  expect(screen.getByRole('button', { name: 'Fetch models' })).toHaveProperty('disabled', true);
  fireEvent.submit(screen.getByRole('dialog').querySelector('form')!);
  expect(submit).toHaveBeenCalledWith(expect.objectContaining({ apiFormat: 'openai_responses' }));
});

it('explains Gemini routing limits while allowing its configuration to be saved', () => {
  const preset = providerPresets.find(item => item.apiFormat === 'gemini_native')!;
  const submit = vi.fn();
  render(<ProviderFormDialog open editingId={null} selectedPreset={preset} initialFormData={formFromPreset(preset)} saving={false} onPresetSelect={vi.fn()} onCancel={vi.fn()} onSubmit={submit} />);
  expect(screen.getByText('Gemini Native can be saved for configuration and model discovery. Local routing is not supported.')).toBeTruthy();
  fireEvent.submit(screen.getByRole('dialog').querySelector('form')!);
  expect(submit).toHaveBeenCalledWith(expect.objectContaining({ apiFormat: 'gemini_native' }));
});
