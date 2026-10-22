import { describe, expect, it } from 'vitest';

import type { Provider } from '@/api';
import { providerPresets } from '@/config/providerPresets';
import { buildEditedProvider, buildProvider, emptyProviderForm, formFromPreset, formFromProvider, validateProviderDraft } from './providerForm';

function provider(overrides: Partial<Provider> = {}): Provider {
  return {
    id: 'local-provider',
    name: 'Local Provider',
    settingsConfig: { env: {} },
    meta: {},
    inFailoverQueue: false,
    ...overrides,
  };
}

describe('explicit custom-model context windows', () => {
  it('leaves custom providers at the client default until a window is explicitly entered', () => {
    const form = { ...emptyProviderForm, id: 'gateway', name: 'Gateway', model: 'custom-large-model' };
    const result = buildProvider(form, null);
    expect(result.settingsConfig).toEqual({ env: {
      ANTHROPIC_MODEL: 'custom-large-model',
      ANTHROPIC_DEFAULT_HAIKU_MODEL: 'custom-large-model',
      ANTHROPIC_DEFAULT_SONNET_MODEL: 'custom-large-model',
      ANTHROPIC_DEFAULT_OPUS_MODEL: 'custom-large-model',
    } });
  });

  it('inherits an explicitly configured preset window', () => {
    const kimi = providerPresets.find(item => item.id === 'kimi-for-coding')!;
    expect(formFromPreset(kimi).contextWindow).toBe('262144');
  });

  it('updates only the declared window and retains model identity, picker profiles and other settings', () => {
    const existing = provider({ settingsConfig: {
      env: { ANTHROPIC_MODEL: 'custom-large-model', CLAUDE_CODE_MAX_CONTEXT_TOKENS: '262144', CLAUDE_CODE_AUTO_COMPACT_WINDOW: '180000' },
      modelPicker: { options: [{ model: 'other-model', behavesAs: 'claude-sonnet-4-6' }] },
      modelOverrides: { 'claude-sonnet-4-6': 'known-gateway-model' },
    } });
    const baseline = structuredClone(existing);
    const form = formFromProvider(existing);
    expect(form.contextWindow).toBe('262144');
    const result = buildEditedProvider({ ...form, contextWindow: ' 1000000 ' }, null, existing);
    const expected = structuredClone(existing);
    (expected.settingsConfig as { env: Record<string, string> }).env.CLAUDE_CODE_MAX_CONTEXT_TOKENS = '1000000';
    expect(result.settingsConfig).toEqual(expected.settingsConfig);
    expect(existing).toEqual(baseline);
  });

  it('clears only the provider window override when the field is emptied', () => {
    const existing = provider({ settingsConfig: { env: { CLAUDE_CODE_MAX_CONTEXT_TOKENS: '1000000', CLAUDE_CODE_AUTO_COMPACT_WINDOW: '180000' } } });
    const result = buildEditedProvider({ ...formFromProvider(existing), contextWindow: '' }, null, existing);
    expect(result.settingsConfig).toEqual({ env: { CLAUDE_CODE_AUTO_COMPACT_WINDOW: '180000' } });
  });

  it.each(['0', '-1', '1M', '1e6', '200000.5', '9007199254740992'])('rejects an invalid window: %s', contextWindow => {
    expect(() => validateProviderDraft({ ...emptyProviderForm, id: 'gateway', name: 'Gateway', contextWindow })).toThrow('Context window must be a positive whole number of tokens.');
  });

  it.each(['', '200000', ' 1000000 '])('accepts an unset window or an explicit token count: %s', contextWindow => {
    expect(() => validateProviderDraft({ ...emptyProviderForm, id: 'gateway', name: 'Gateway', contextWindow })).not.toThrow();
  });
});

describe('provider form serialization', () => {
  it('preserves the legacy API key field while editing', () => {
    const form = formFromProvider(
      provider({
        settingsConfig: {
          env: {
            ANTHROPIC_BASE_URL: 'http://127.0.0.1:11434/v1',
            ANTHROPIC_API_KEY: 'legacy-key',
          },
        },
      }),
    );

    expect(form.apiKeyField).toBe('ANTHROPIC_API_KEY');
    expect(form.apiKey).toBe('legacy-key');
  });

  it('writes exactly the selected API key field', () => {
    const result = buildProvider(
      {
        ...emptyProviderForm,
        id: ' local-provider ',
        name: ' Local Provider ',
        baseUrl: ' http://127.0.0.1:11434/v1 ',
        apiKey: ' secret ',
        apiKeyField: 'ANTHROPIC_API_KEY',
        apiFormat: 'openai_chat',
      },
      null,
    );
    const env = (result.settingsConfig as { env: Record<string, string> }).env;

    expect(result.id).toBe('local-provider');
    expect(result.name).toBe('Local Provider');
    expect(env.ANTHROPIC_API_KEY).toBe('secret');
    expect(env).not.toHaveProperty('ANTHROPIC_AUTH_TOKEN');
    expect(result.meta).toMatchObject({ apiFormat: 'openai_chat' });
  });

  it('serializes a managed Codex account binding without an API key', () => {
    const result = buildProvider(
      {
        ...emptyProviderForm,
        id: 'codex-account',
        name: 'Codex Account',
        authMode: 'oauth_proxy',
        codexAccountId: ' account-1 ',
      },
      null,
    );
    const env = (result.settingsConfig as { env: Record<string, string> }).env;

    expect(env).not.toHaveProperty('ANTHROPIC_AUTH_TOKEN');
    expect(result.meta).toMatchObject({
      authMode: 'oauth_proxy',
      providerType: 'codex_oauth',
      authBinding: {
        source: 'managed_account',
        authProvider: 'codex_oauth',
        accountId: 'account-1',
      },
    });
  });

  it('preserves unknown settings and provider metadata while editing', () => {
    const existing = provider({
      settingsConfig: {
        env: {
          ANTHROPIC_BASE_URL: 'https://example.com',
          ANTHROPIC_AUTH_TOKEN: 'old-key',
          CUSTOM_SETTING: 'keep-me',
        },
      },
      meta: { apiFormat: 'anthropic', customMetadata: 'keep-me' },
      sortIndex: 7,
      createdAt: 123,
      inFailoverQueue: true,
    });
    const form = formFromProvider(existing);
    const result = buildProvider({ ...form, name: 'Updated' }, null, existing);
    const env = (result.settingsConfig as { env: Record<string, string> }).env;

    expect(result.name).toBe('Updated');
    expect(result.sortIndex).toBe(7);
    expect(result.createdAt).toBe(123);
    expect(result.inFailoverQueue).toBe(true);
    expect(env.CUSTOM_SETTING).toBe('keep-me');
    expect(result.meta).toMatchObject({ customMetadata: 'keep-me' });
  });

  it('saves the official subscription preset with a clean, keyless env', () => {
    const official = providerPresets.find((preset) => preset.id === 'claude-official');
    expect(official?.noAuth).toBe(true);

    const result = buildProvider(
      { ...emptyProviderForm, id: 'claude-official', name: 'Claude Official' },
      official ?? null,
    );
    const env = (result.settingsConfig as { env: Record<string, string> }).env;

    // No blank credential field is persisted, so switching to it clears any
    // third-party takeover instead of writing an empty token.
    expect(env).not.toHaveProperty('ANTHROPIC_AUTH_TOKEN');
    expect(env).not.toHaveProperty('ANTHROPIC_API_KEY');
    expect(env).not.toHaveProperty('ANTHROPIC_BASE_URL');
  });
  it('retains subscription authentication through an edit without the preset', () => {
    const preset = providerPresets.find((item) => item.id === 'claude-official')!;
    const created = buildProvider(formFromPreset(preset), preset);
    const edit = formFromProvider(created);
    expect(edit.noAuth).toBe(true);
    const saved = buildProvider(edit, null, created);
    expect(saved.meta).toMatchObject({ noAuth: true });
    expect(saved.settingsConfig).toEqual({ env: {} });
  });

  it('preserves legacy user settings without adding them to new providers', () => {
    const existing = provider({ settingsConfig: { env: {
      API_TIMEOUT_MS: '1234', CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC: '1',
    } } });
    const saved = buildProvider(formFromProvider(existing), null, existing);
    expect(saved.settingsConfig).toEqual(existing.settingsConfig);
    const created = buildProvider({ ...emptyProviderForm, id: 'new', name: 'New' }, null);
    expect(created.settingsConfig).toEqual({ env: {} });
  });

  it('recognizes legacy keyless official records without reclassifying API providers', () => {
    expect(formFromProvider(provider({ id: 'claude-official' })).noAuth).toBe(true);
    expect(formFromProvider(provider({ id: 'claude-official',
      settingsConfig: { env: { ANTHROPIC_API_KEY: 'secret' } },
    })).noAuth).toBe(false);
  });

});

it('preserves stored values hidden by form defaults when editing a different field', async () => {
  const { buildEditedProvider } = await import('./providerForm');
  const base: Provider = { id: 'legacy', name: 'Legacy', settingsConfig: { env: { ANTHROPIC_AUTH_TOKEN: 'key', UNKNOWN_USER_KEY: 'keep' } }, meta: {}, inFailoverQueue: false };
  const draft = formFromProvider(base); draft.name = 'Renamed';
  const edited = buildEditedProvider(draft, null, base);
  expect(edited.name).toBe('Renamed'); expect(edited.meta).toEqual({}); expect(edited.settingsConfig).toEqual(base.settingsConfig);
});

it('preserves legacy Chat settings and unrelated top-level fields when only renaming', () => {
  const existing: Provider = { id: 'legacy', name: 'Legacy', inFailoverQueue: false, meta: {}, settingsConfig: { api_format: 'openai_chat', customSetting: { keep: true }, env: { ANTHROPIC_AUTH_TOKEN: 'synthetic-key' } } };
  const edited = buildProvider({ ...formFromProvider(existing), name: 'Renamed' }, null, existing);
  expect(edited.meta).toMatchObject({ apiFormat: 'openai_chat' });
  expect(edited.settingsConfig).toMatchObject({ api_format: 'openai_chat', customSetting: { keep: true } });
});

it('retains the inferred vendor protocol and auth adapter when editing an imported record', () => {
  const existing: Provider = { id: 'imported', name: 'Imported', inFailoverQueue: false, settingsConfig: { env: {} }, meta: { providerType: 'deepseek' } };
  const edited = buildProvider(formFromProvider(existing), null, existing);
  expect(edited.meta).toMatchObject({ apiFormat: 'openai_chat', providerType: 'deepseek' });
});

it('preserves binding-only OAuth accounts, including unknown binding fields', () => {
  const existing: Provider = { id: 'imported', name: 'Imported', inFailoverQueue: false, settingsConfig: { env: {} }, meta: { authBinding: { source: 'managed_account', authProvider: 'codex_oauth', accountId: 'synthetic-account', extra: 'keep' } } };
  const form = formFromProvider(existing);
  expect(form.authMode).toBe('oauth_proxy');
  expect(form.apiFormat).toBe('openai_responses');
  const edited = buildProvider(form, null, existing);
  expect(edited.meta).toMatchObject({ providerType: 'codex_oauth', authBinding: { accountId: 'synthetic-account', extra: 'keep' } });
  const defaultAccount = buildProvider({ ...form, codexAccountId: '' }, null, existing);
  expect((defaultAccount.meta as { authBinding: object }).authBinding).not.toHaveProperty('accountId');
});

it('normalizes conflicting Codex protocol metadata while keeping configured models', () => {
  const existing: Provider = { id: 'codex', name: 'Codex', inFailoverQueue: false, settingsConfig: { env: { ANTHROPIC_MODEL: 'existing-model' } }, meta: { providerType: 'codex_oauth', apiFormat: 'gemini_native' } };
  const form = formFromProvider(existing);
  expect(form.apiFormat).toBe('openai_responses');
  expect(buildProvider({ ...form, apiFormat: 'openai_chat' }, null, existing).meta).toMatchObject({ apiFormat: 'openai_responses' });
  expect(form.model).toBe('existing-model');
  const preset = formFromPreset(providerPresets.find(item => item.id === 'codex')!);
  expect([preset.model, preset.haikuModel, preset.sonnetModel, preset.opusModel]).toEqual(Array(4).fill('gpt-6.1-sol'));
});

it.each([
  { authMode: 'oauth_proxy' },
  { authBinding: { authProvider: 'codex_oauth', accountId: 'synthetic-account', extra: 'keep' } },
  { authBinding: { auth_provider: 'codex_oauth', accountId: 'synthetic-account' } },
])('persists the locked Codex contract through a name-only edit of an imported record: %j', (oauthMeta) => {
  const existing: Provider = { id: 'legacy-binding', name: 'Imported', inFailoverQueue: false, settingsConfig: { env: { ANTHROPIC_MODEL: 'configured-model' } }, meta: { ...oauthMeta, apiFormat: 'gemini_native', otherMetadata: 'keep' } };
  const form = formFromProvider(existing);
  const edited = buildEditedProvider({ ...form, name: 'Renamed' }, null, existing);
  expect(edited.meta).toMatchObject({ authMode: 'oauth_proxy', providerType: 'codex_oauth', apiFormat: 'openai_responses', otherMetadata: 'keep' });
  expect((edited.meta as Record<string, unknown>).authBinding).toEqual((existing.meta as Record<string, unknown>).authBinding);
  expect(edited.settingsConfig).toEqual(existing.settingsConfig);
  expect((existing.meta as Record<string, unknown>).apiFormat).toBe('gemini_native');
  const unchangedDraft = buildEditedProvider(form, null, existing);
  expect(unchangedDraft).not.toBe(existing);
  expect(unchangedDraft.meta).toMatchObject({ authMode: 'oauth_proxy', providerType: 'codex_oauth', apiFormat: 'openai_responses' });
  expect((existing.meta as Record<string, unknown>).apiFormat).toBe('gemini_native');
});

it('does not canonicalize an API-key record during an unrelated edit', () => {
  const existing: Provider = { id: 'legacy-chat', name: 'Chat', inFailoverQueue: false, settingsConfig: { api_format: 'openai_chat', extra: 'keep', env: {} }, meta: { otherMetadata: 'keep' } };
  const edited = buildEditedProvider({ ...formFromProvider(existing), name: 'Renamed' }, null, existing);
  expect(edited.meta).toEqual(existing.meta);
  expect(edited.settingsConfig).toEqual(existing.settingsConfig);
});
