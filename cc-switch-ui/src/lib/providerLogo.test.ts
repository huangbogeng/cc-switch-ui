import { describe, expect, it } from 'vitest';
import type { Provider } from '@/api';
import { providerPresets } from '@/config/providerPresets';
import { buildProvider, formFromPreset } from '@/components/providers/providerForm';
import { providerLogos, resolveProviderLogo } from './providerLogo';

function provider(overrides: Partial<Provider> = {}): Provider {
  return { id: 'custom', name: 'Custom', settingsConfig: { env: {} }, meta: {}, inFailoverQueue: false, ...overrides };
}

describe('provider logos', () => {
  it('uses the same brand in the preset picker and the saved provider', () => {
    for (const preset of providerPresets) {
      expect(providerLogos[preset.icon]).toBeTruthy();
      const saved = buildProvider(formFromPreset(preset), preset);
      expect(saved.icon).toBe(preset.icon);
      expect(resolveProviderLogo(saved)).toBe(preset.icon);
    }
  });
  it('recognizes legacy presets and renamed providers by their endpoint', () => {
    expect(resolveProviderLogo(provider({ id: 'kimi-for-coding-global' }))).toBe('kimi');
    expect(resolveProviderLogo(provider({ name: 'Work', settingsConfig: { env: { ANTHROPIC_BASE_URL: 'https://api.deepseek.com/anthropic' } } }))).toBe('deepseek');
    expect(resolveProviderLogo(provider({ meta: { providerType: 'codex_oauth' } }))).toBe('openai');
  });
  it('preserves an explicitly selected icon during editing', () => {
    const existing = provider({ icon: 'minimax', iconColor: '#FF6B6B' });
    expect(resolveProviderLogo(existing)).toBe('minimax');
    const form = { ...formFromPreset(providerPresets[1]), id: existing.id };
    expect(buildProvider(form, null, existing).icon).toBe('minimax');
  });
  it('does not identify arbitrary URLs or matching names as brand assets', () => {
    for (const url of ['https://deepseek.com.evil.test', 'https://evil-deepseek.com', 'javascript:deepseek.com', 'invalid']) {
      expect(resolveProviderLogo(provider({ name: 'DeepSeek', icon: 'https://evil.test/logo.svg', websiteUrl: url }))).toBeUndefined();
    }
  });
});
