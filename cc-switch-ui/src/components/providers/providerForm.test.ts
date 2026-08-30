import { describe, expect, it } from 'vitest';

import type { Provider } from '@/api';
import { buildProvider, emptyProviderForm, formFromProvider } from './providerForm';

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
});
