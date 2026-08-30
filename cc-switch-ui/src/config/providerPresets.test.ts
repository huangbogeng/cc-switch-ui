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
