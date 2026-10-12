import { expect, it } from 'vitest';
import type { Provider } from '@/api';
import { providerApiFormat, providerAuthMode, providerRouteUnsupportedReason } from './provider';

function provider(overrides: Partial<Provider> = {}): Provider {
  return { id: 'custom', name: 'Custom', settingsConfig: { env: {} }, meta: {}, inFailoverQueue: false, ...overrides };
}

it('resolves explicit metadata before legacy settings and vendor defaults', () => {
  expect(providerApiFormat(provider({ id: 'deepseek', settingsConfig: { api_format: 'openai_responses' }, meta: { apiFormat: 'anthropic' } }))).toBe('anthropic');
  expect(providerApiFormat(provider({ id: 'deepseek', settingsConfig: { api_format: 'openai_responses' } }))).toBe('openai_responses');
  for (const id of ['deepseek', 'minimax', 'openrouter', 'siliconflow', 'copilot']) expect(providerApiFormat(provider({ id }))).toBe('openai_chat');
  expect(providerApiFormat(provider({ id: 'custom-deepseek', meta: { providerType: 'claude' } }))).toBe('anthropic');
  expect(providerApiFormat(provider({ id: 'custom-deepseek' }))).toBe('openai_chat');
  expect(providerApiFormat(provider({ id: 'gemini' }))).toBe('gemini_native');
});

it('respects the dedicated Codex contract for every supported OAuth signal', () => {
  for (const meta of [{ authMode: 'oauth_proxy' }, { providerType: 'codex_oauth' }, { authBinding: { authProvider: 'codex_oauth' } }, { authBinding: { auth_provider: 'codex_oauth' } }]) {
    const p = provider({ meta: { ...meta, apiFormat: 'gemini_native' } });
    expect(providerAuthMode(p)).toBe('oauth_proxy');
    expect(providerApiFormat(p)).toBe('openai_responses');
    expect(providerRouteUnsupportedReason(p)).toBeNull();
  }
  expect(providerApiFormat(provider({ id: 'codex_oauth-import' }))).toBe('openai_responses');
});

it('keeps unrecognized protocols visible and unavailable for routing', () => {
  const p = provider({ meta: { apiFormat: 'future_protocol' } });
  expect(providerApiFormat(p)).toBe('future_protocol');
  expect(providerRouteUnsupportedReason(p)).not.toBeNull();
  expect(providerRouteUnsupportedReason(provider({ meta: { apiFormat: 'gemini_native' } }))).toContain('Gemini Native');
  expect(providerRouteUnsupportedReason(provider())).toBeNull();
});
