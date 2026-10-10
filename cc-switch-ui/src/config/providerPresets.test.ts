import { describe, expect, it } from 'vitest';

import { providerPresets } from './providerPresets';

describe('OrcaRouter preset', () => {
  it('uses the documented native Anthropic endpoint and model catalog', () => {
    const preset = providerPresets.find((item) => item.id === 'orcarouter');

    expect(preset).toBeDefined();
    expect(preset?.apiFormat).toBe('anthropic');
    expect(preset?.modelsUrl).toBe('https://api.orcarouter.ai/v1/models');
    expect(preset?.settingsConfig.env).toMatchObject({
      ANTHROPIC_BASE_URL: 'https://api.orcarouter.ai',
      ANTHROPIC_AUTH_TOKEN: '',
      ANTHROPIC_MODEL: 'anthropic/claude-sonnet-4.6',
      ANTHROPIC_DEFAULT_OPUS_MODEL: 'anthropic/claude-opus-4.7',
    });
  });

  it('keeps preset identifiers unique', () => {
    const ids = providerPresets.map((preset) => preset.id);
    expect(new Set(ids).size).toBe(ids.length);
  });
});

describe('ported presets', () => {
  it('disables the auto-mode server on every non-official gateway', () => {
    // The server-side auto-mode classifier only works on the official endpoint;
    // third-party gateways must pin it to 0 or sessions stall on a prompt.
    const native = new Set(['codex', 'gemini-native']);
    for (const preset of providerPresets) {
      if (preset.id === 'claude-official' || native.has(preset.id)) continue;
      expect(
        preset.settingsConfig.env.CLAUDE_CODE_AUTO_MODE_SERVER,
        `${preset.id} should pin CLAUDE_CODE_AUTO_MODE_SERVER=0`,
      ).toBe('0');
    }
  });

  it('pins the 256K window for Kimi For Coding', () => {
    const preset = providerPresets.find((item) => item.id === 'kimi-for-coding');
    expect(preset?.settingsConfig.env).toMatchObject({
      ANTHROPIC_BASE_URL: 'https://api.kimi.com/coding/',
      CLAUDE_CODE_MAX_CONTEXT_TOKENS: '262144',
      CLAUDE_CODE_AUTO_COMPACT_WINDOW: '262144',
    });
  });

  it('uses ANTHROPIC_API_KEY for AiHubMix', () => {
    const preset = providerPresets.find((item) => item.id === 'aihubmix');
    expect(preset?.apiKeyField).toBe('ANTHROPIC_API_KEY');
    expect(preset?.settingsConfig.env).toHaveProperty('ANTHROPIC_API_KEY');
  });

  it('leaves Claude Official env empty', () => {
    const preset = providerPresets.find((item) => item.id === 'claude-official');
    expect(preset?.settingsConfig.env).toEqual({});
  });
});
