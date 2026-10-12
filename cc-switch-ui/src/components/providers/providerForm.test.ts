import { describe, expect, it } from 'vitest';

import type { Provider } from '@/api';
import { providerPresets } from '@/config/providerPresets';
import { buildProvider, emptyProviderForm, formFromPreset, formFromProvider } from './providerForm';

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
