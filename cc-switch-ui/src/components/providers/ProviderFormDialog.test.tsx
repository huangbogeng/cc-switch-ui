import { fetchProviderModels, fetchCodexModels } from '@/api';
vi.mock('@/api', () => ({ fetchProviderModels: vi.fn(), fetchCodexModels: vi.fn(), detectProviderEndpointType: vi.fn() }));
import { act, fireEvent, render, screen, waitFor, within } from '@testing-library/react';
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

it('edits an optional context window without changing the model or saving automatically', async () => {
  const preset = providerPresets.find(item => item.id === 'claude-official')!;
  const initial = { ...formFromPreset(preset), noAuth: false, model: 'custom-large-model' };
  const submit = vi.fn();
  const preview = vi.fn().mockResolvedValue({ applied: false, recordPath: '/test/db', livePath: null, recordKeys: ['record/settingsConfig/env/CLAUDE_CODE_MAX_CONTEXT_TOKENS'], liveKeys: [], message: 'Context draft preview' });
  render(<ProviderFormDialog open editingId="gateway" selectedPreset={null} initialFormData={initial} saving={false} onPresetSelect={vi.fn()} onCancel={vi.fn()} onSubmit={submit} onPreview={preview} />);
  fireEvent.click(screen.getByText('Advanced'));
  const input = screen.getByLabelText('Context window (tokens)');
  expect(input).toHaveProperty('value', '');
  fireEvent.change(input, { target: { value: '1000000' } });
  expect(screen.getByLabelText('Main')).toHaveProperty('value', 'custom-large-model');
  expect(submit).not.toHaveBeenCalled();
  fireEvent.click(screen.getByRole('button', { name: 'Preview save' }));
  await screen.findByText('Context draft preview');
  expect(preview).toHaveBeenCalledWith(expect.objectContaining({ contextWindow: '1000000', model: 'custom-large-model' }));
  expect(submit).not.toHaveBeenCalled();
  fireEvent.submit(screen.getByRole('dialog').querySelector('form')!);
  expect(submit).toHaveBeenCalledWith(expect.objectContaining({ contextWindow: '1000000', model: 'custom-large-model' }));
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

it('locks Codex OAuth to Responses and explains model discovery without an account', () => {
  const preset = providerPresets.find(item => item.id === 'codex')!;
  const submit = vi.fn();
  render(<ProviderFormDialog open editingId={null} selectedPreset={preset} initialFormData={{ ...formFromPreset(preset), apiFormat: 'gemini_native' }} saving={false} onPresetSelect={vi.fn()} onCancel={vi.fn()} onSubmit={submit} />);
  const format = screen.getByLabelText('API Format');
  expect(format).toHaveProperty('value', 'openai_responses');
  expect(format).toHaveProperty('disabled', true);
  expect(screen.getByRole('button', { name: 'Fetch models' })).toHaveProperty('disabled', true);
  expect(screen.getByText(/Connect an account on the OAuth page and choose/)).toBeTruthy();
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

it('fetches the selected OAuth account catalog without an API key or endpoint', async () => {
  vi.mocked(fetchCodexModels).mockResolvedValue({ models: [{ id: 'oauth-selected-model' }] });
  vi.mocked(fetchProviderModels).mockClear();
  const preset = providerPresets.find(item => item.id === 'codex')!;
  const form = { ...formFromPreset(preset), baseUrl: '', apiKey: '', codexAccountId: 'fixture-b', model: 'configured-main' };
  const submit = vi.fn();
  render(<ProviderFormDialog open editingId="codex" selectedPreset={null} initialFormData={form} saving={false} codexAccounts={[{ id: 'fixture-a', login: 'Account A', is_default: true }, { id: 'fixture-b', login: 'Account B', is_default: false }]} onPresetSelect={vi.fn()} onCancel={vi.fn()} onSubmit={submit} />);
  fireEvent.click(screen.getByRole('button', { name: 'Fetch models' }));
  await screen.findByRole('radio', { name: /oauth-selected-model/ });
  expect(fetchCodexModels).toHaveBeenLastCalledWith('fixture-b');
  expect(fetchProviderModels).not.toHaveBeenCalled();
  expect(submit).not.toHaveBeenCalled();
  fireEvent.click(screen.getByRole('radio', { name: /oauth-selected-model/ }));
  fireEvent.click(screen.getByRole('button', { name: 'Apply model' }));
  expect(screen.getByLabelText('Main')).toHaveProperty('value', 'oauth-selected-model');
});

it('ignores the old OAuth response after account selection changes and fetches the default', async () => {
  let finish!: (value: { models: { id: string }[] }) => void;
  const pending = new Promise<{ models: { id: string }[] }>(resolve => { finish = resolve; });
  vi.mocked(fetchCodexModels).mockImplementationOnce(() => pending).mockResolvedValue({ models: [{ id: 'default-account-model' }] });
  const form = { ...formFromPreset(providerPresets.find(item => item.id === 'codex')!), codexAccountId: 'fixture-b' };
  render(<ProviderFormDialog open editingId="codex" selectedPreset={null} initialFormData={form} saving={false} codexAccounts={[{ id: 'fixture-a', login: 'Account A', is_default: true }, { id: 'fixture-b', login: 'Account B', is_default: false }]} onPresetSelect={vi.fn()} onCancel={vi.fn()} onSubmit={vi.fn()} />);
  fireEvent.click(screen.getByRole('button', { name: 'Fetch models' }));
  fireEvent.change(screen.getByLabelText('ChatGPT Account'), { target: { value: '' } });
  await act(async () => { finish({ models: [{ id: 'old-account-model' }] }); });
  expect(screen.queryByRole('dialog', { name: 'Choose model' })).toBeNull();
  fireEvent.click(screen.getByRole('button', { name: 'Fetch models' }));
  await screen.findByRole('radio', { name: /default-account-model/ });
  expect(fetchCodexModels).toHaveBeenLastCalledWith(undefined);
  expect(screen.queryByRole('radio', { name: /old-account-model/ })).toBeNull();
});

it('shows an OAuth discovery failure while retaining manually configured models', async () => {
  vi.mocked(fetchCodexModels).mockRejectedValue(new Error('Sign in again on the OAuth page.'));
  const form = { ...formFromPreset(providerPresets.find(item => item.id === 'codex')!), model: 'manual-model' };
  render(<ProviderFormDialog open editingId="codex" selectedPreset={null} initialFormData={form} saving={false} codexAccounts={[{ id: 'fixture-a', login: 'Account A', is_default: true }]} onPresetSelect={vi.fn()} onCancel={vi.fn()} onSubmit={vi.fn()} />);
  fireEvent.click(screen.getByRole('button', { name: 'Fetch models' }));
  expect(await screen.findByRole('alert')).toHaveProperty('textContent', 'Sign in again on the OAuth page.');
  expect(screen.getByLabelText('Main')).toHaveProperty('value', 'manual-model');
  expect(screen.getByRole('button', { name: 'Fetch models' })).toHaveProperty('disabled', false);
});

it('discards the catalog if the bound OAuth account is disconnected', async () => {
  vi.mocked(fetchCodexModels).mockResolvedValue({ models: [{ id: 'removed-account-model' }] });
  const props = { open: true, editingId: 'codex', selectedPreset: null, initialFormData: { ...formFromPreset(providerPresets.find(item => item.id === 'codex')!), codexAccountId: 'fixture-b' }, saving: false, onPresetSelect: vi.fn(), onCancel: vi.fn(), onSubmit: vi.fn() };
  const accountA = { id: 'fixture-a', login: 'Account A', is_default: true };
  const accountB = { id: 'fixture-b', login: 'Account B', is_default: false };
  const { rerender } = render(<ProviderFormDialog {...props} codexAccounts={[accountA, accountB]} />);
  fireEvent.click(screen.getByRole('button', { name: 'Fetch models' }));
  await screen.findByRole('radio', { name: /removed-account-model/ });
  rerender(<ProviderFormDialog {...props} codexAccounts={[accountA]} />);
  await waitFor(() => expect(screen.queryByRole('dialog', { name: 'Choose model' })).toBeNull());
  expect(screen.getByRole('button', { name: 'Fetch models' })).toHaveProperty('disabled', true);
});

it('updates all four mapping menus after fetching and selects each role without another request', async () => {
  vi.mocked(fetchCodexModels).mockClear();
  vi.mocked(fetchCodexModels).mockResolvedValue({ models: [{ id: 'catalog-a' }, { id: 'catalog-b' }] });
  const form = { ...formFromPreset(providerPresets.find(item => item.id === 'codex')!), model: 'unlisted-main', haikuModel: 'unlisted-haiku', sonnetModel: 'unlisted-sonnet', opusModel: 'unlisted-opus' };
  const submit = vi.fn();
  render(<ProviderFormDialog open editingId="codex" selectedPreset={null} initialFormData={form} saving={false} codexAccounts={[{ id: 'fixture-a', login: 'Account A', is_default: true }, { id: 'fixture-b', login: 'Account B', is_default: false }]} onPresetSelect={vi.fn()} onCancel={vi.fn()} onSubmit={submit} />);
  fireEvent.click(screen.getByRole('button', { name: 'Fetch models' }));
  await screen.findByRole('dialog', { name: 'Choose model' });
  fireEvent.click(within(screen.getByRole('dialog', { name: 'Choose model' })).getByRole('button', { name: 'Cancel' }));
  for (const role of ['Main', 'Haiku', 'Sonnet', 'Opus']) {
    fireEvent.click(screen.getByRole('button', { name: `Choose ${role} model` }));
    expect(screen.getByRole('option', { name: 'catalog-a' })).toBeTruthy();
    expect(screen.getByRole('option', { name: 'catalog-b' })).toBeTruthy();
    fireEvent.click(screen.getByRole('option', { name: 'catalog-b' }));
    expect(screen.getByLabelText(role)).toHaveProperty('value', 'catalog-b');
  }
  expect(fetchCodexModels).toHaveBeenCalledOnce();
  expect(submit).not.toHaveBeenCalled();
  fireEvent.change(screen.getByLabelText('ChatGPT Account'), { target: { value: 'fixture-b' } });
  for (const role of ['Main', 'Haiku', 'Sonnet', 'Opus']) {
    expect(screen.getByRole('button', { name: `Choose ${role} model` })).toHaveProperty('disabled', true);
    expect(screen.getByLabelText(role)).toHaveProperty('value', 'catalog-b');
  }
});

it('offers OAuth diagnostics instead of API-key detection and retains the draft', async () => {
  vi.mocked(fetchCodexModels).mockResolvedValue({ models: [{ id: 'gpt-6.1-sol' }] });
  const form = formFromPreset(providerPresets.find(item => item.id === 'codex')!);
  const submit = vi.fn();
  render(<ProviderFormDialog open editingId="codex" selectedPreset={null} initialFormData={form} saving={false} codexAccounts={[{ id: 'fixture-a', login: 'Account A', is_default: true }]} onPresetSelect={vi.fn()} onCancel={vi.fn()} onSubmit={submit} />);
  fireEvent.click(screen.getByText('Diagnostics'));
  expect(screen.queryByRole('button', { name: 'Detect endpoint type' })).toBeNull();
  fireEvent.click(screen.getByRole('button', { name: 'Check OAuth connection' }));
  expect(await screen.findByRole('status')).toHaveProperty('textContent', 'OAuth model catalog reached successfully. 1 models returned.');
  expect(submit).not.toHaveBeenCalled();
  expect(screen.getByLabelText('Main')).toHaveProperty('value', form.model);
  expect(screen.getByLabelText('API Format')).toHaveProperty('value', 'openai_responses');
});

it('retains endpoint type detection for API-key providers', () => {
  const form = { ...formFromPreset(providerPresets.find(item => item.id === 'claude-official')!), authMode: 'api_key' as const, noAuth: false, baseUrl: 'https://fixture.example', apiKey: 'fixture-key' };
  render(<ProviderFormDialog open editingId="fixture" selectedPreset={null} initialFormData={form} saving={false} onPresetSelect={vi.fn()} onCancel={vi.fn()} onSubmit={vi.fn()} />);
  fireEvent.click(screen.getByText('Diagnostics'));
  expect(screen.getByRole('button', { name: 'Detect endpoint type' })).toHaveProperty('disabled', false);
  expect(screen.queryByRole('button', { name: 'Check OAuth connection' })).toBeNull();
});
