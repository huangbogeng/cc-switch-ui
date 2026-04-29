import type { Provider } from '@/api';
import type { ProviderPreset } from '@/config/providerPresets';

export interface ProviderFormData {
  id: string;
  name: string;
  websiteUrl: string;
  notes: string;
  apiKey: string;
}

export const emptyProviderForm: ProviderFormData = {
  id: '',
  name: '',
  websiteUrl: '',
  notes: '',
  apiKey: '',
};

export function formFromPreset(preset: ProviderPreset): ProviderFormData {
  return {
    id: preset.id,
    name: preset.name,
    websiteUrl: preset.websiteUrl,
    notes: '',
    apiKey: '',
  };
}

export function formFromProvider(provider: Provider): ProviderFormData {
  return {
    id: provider.id,
    name: provider.name,
    websiteUrl: provider.websiteUrl || '',
    notes: provider.notes || '',
    apiKey: '',
  };
}

export function buildProvider(formData: ProviderFormData, selectedPreset: ProviderPreset | null): Provider {
  let settingsConfig: Record<string, unknown>;

  if (selectedPreset) {
    const env: Record<string, string> = {};
    for (const [key, value] of Object.entries(selectedPreset.settingsConfig.env)) {
      env[key] = key === 'ANTHROPIC_AUTH_TOKEN' ? formData.apiKey : value;
    }
    settingsConfig = { env };
  } else if (formData.websiteUrl) {
    settingsConfig = {
      env: {
        ANTHROPIC_BASE_URL: formData.websiteUrl,
        ANTHROPIC_AUTH_TOKEN: formData.apiKey,
      },
    };
  } else {
    settingsConfig = {
      env: {
        ANTHROPIC_AUTH_TOKEN: formData.apiKey,
      },
    };
  }

  return {
    id: formData.id,
    name: formData.name,
    settingsConfig,
    websiteUrl: selectedPreset?.websiteUrl || formData.websiteUrl || undefined,
    notes: formData.notes || undefined,
    meta: {},
    inFailoverQueue: false,
  };
}
