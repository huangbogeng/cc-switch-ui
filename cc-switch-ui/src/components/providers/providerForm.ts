import type { Provider } from '@/api';
import type { ProviderPreset } from '@/config/providerPresets';

export type ApiKeyField = 'ANTHROPIC_AUTH_TOKEN' | 'ANTHROPIC_API_KEY';
export type ApiFormat = 'anthropic' | 'openai_chat' | 'openai_responses' | 'gemini_native';

export interface ProviderFormData {
  id: string;
  name: string;
  authMode: 'api_key' | 'oauth_proxy';
  websiteUrl: string;
  baseUrl: string;
  notes: string;
  apiKey: string;
  apiKeyField: ApiKeyField;
  apiFormat: ApiFormat;
  isFullUrl: boolean;
  model: string;
  haikuModel: string;
  sonnetModel: string;
  opusModel: string;
  apiTimeoutMs: string;
  disableNonessentialTraffic: boolean;
  promptCacheKey: string;
}

export const emptyProviderForm: ProviderFormData = {
  id: '',
  name: '',
  authMode: 'api_key',
  websiteUrl: '',
  baseUrl: '',
  notes: '',
  apiKey: '',
  apiKeyField: 'ANTHROPIC_AUTH_TOKEN',
  apiFormat: 'anthropic',
  isFullUrl: false,
  model: '',
  haikuModel: '',
  sonnetModel: '',
  opusModel: '',
  apiTimeoutMs: '',
  disableNonessentialTraffic: false,
  promptCacheKey: '',
};

export function formFromPreset(preset: ProviderPreset): ProviderFormData {
  const env = preset.settingsConfig.env;
  const apiKeyField = preset.apiKeyField || findApiKeyField(env);
  const model = env.ANTHROPIC_MODEL || '';

  return {
    id: preset.id,
    name: preset.name,
    authMode: preset.authMode || 'api_key',
    websiteUrl: preset.websiteUrl,
    baseUrl: env.ANTHROPIC_BASE_URL || '',
    notes: '',
    apiKey: '',
    apiKeyField,
    apiFormat: preset.apiFormat || 'anthropic',
    isFullUrl: preset.isFullUrl || false,
    model,
    haikuModel: env.ANTHROPIC_DEFAULT_HAIKU_MODEL || model,
    sonnetModel: env.ANTHROPIC_DEFAULT_SONNET_MODEL || model,
    opusModel: env.ANTHROPIC_DEFAULT_OPUS_MODEL || model,
    apiTimeoutMs: env.API_TIMEOUT_MS || '',
    disableNonessentialTraffic: env.CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC === '1',
    promptCacheKey: '',
  };
}

export function formFromProvider(provider: Provider): ProviderFormData {
  const env = providerEnv(provider);
  const meta = providerMeta(provider);
  const authMode = meta.authMode === 'oauth_proxy' || meta.providerType === 'codex_oauth' ? 'oauth_proxy' : 'api_key';
  const apiKeyField = meta.apiKeyField || findApiKeyField(env);
  const model = env.ANTHROPIC_MODEL || '';

  return {
    id: provider.id,
    name: provider.name,
    authMode,
    websiteUrl: provider.websiteUrl || '',
    baseUrl: env.ANTHROPIC_BASE_URL || '',
    notes: provider.notes || '',
    apiKey: env[apiKeyField] || '',
    apiKeyField,
    apiFormat: meta.apiFormat || 'anthropic',
    isFullUrl: meta.isFullUrl || false,
    model,
    haikuModel: env.ANTHROPIC_DEFAULT_HAIKU_MODEL || model,
    sonnetModel: env.ANTHROPIC_DEFAULT_SONNET_MODEL || model,
    opusModel: env.ANTHROPIC_DEFAULT_OPUS_MODEL || model,
    apiTimeoutMs: env.API_TIMEOUT_MS || '',
    disableNonessentialTraffic: env.CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC === '1',
    promptCacheKey: meta.promptCacheKey || '',
  };
}

export function buildProvider(formData: ProviderFormData, selectedPreset: ProviderPreset | null): Provider {
  const authMode = selectedPreset?.authMode || formData.authMode;
  const id = formData.id.trim();
  const name = formData.name.trim();
  const websiteUrl = formData.websiteUrl.trim();
  const baseUrl = formData.baseUrl.trim();
  const apiKey = formData.apiKey.trim();
  const notes = formData.notes.trim();
  const apiTimeoutMs = formData.apiTimeoutMs.trim();
  const promptCacheKey = formData.promptCacheKey.trim();
  const env: Record<string, string> = selectedPreset ? { ...selectedPreset.settingsConfig.env } : {};

  if (baseUrl) env.ANTHROPIC_BASE_URL = baseUrl;
  if (authMode === 'api_key') {
    delete env.ANTHROPIC_AUTH_TOKEN;
    delete env.ANTHROPIC_API_KEY;
    env[formData.apiKeyField] = apiKey;
  }
  setOptionalEnv(env, 'ANTHROPIC_MODEL', formData.model);
  setOptionalEnv(env, 'ANTHROPIC_DEFAULT_HAIKU_MODEL', formData.haikuModel || formData.model);
  setOptionalEnv(env, 'ANTHROPIC_DEFAULT_SONNET_MODEL', formData.sonnetModel || formData.model);
  setOptionalEnv(env, 'ANTHROPIC_DEFAULT_OPUS_MODEL', formData.opusModel || formData.model);
  setOptionalEnv(env, 'API_TIMEOUT_MS', apiTimeoutMs);
  if (formData.disableNonessentialTraffic) {
    env.CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC = '1';
  } else {
    delete env.CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC;
  }

  const meta: Record<string, unknown> = {
    authMode,
    apiFormat: formData.apiFormat,
    apiKeyField: formData.apiKeyField,
    isFullUrl: formData.isFullUrl,
  };

  if (promptCacheKey) meta.promptCacheKey = promptCacheKey;
  if (authMode === 'oauth_proxy') {
    meta.providerType = selectedPreset?.providerType || 'codex_oauth';
    meta.authBinding = {
      source: 'managed_account',
      authProvider: 'codex_oauth',
    };
  }

  return {
    id,
    name,
    settingsConfig: { env },
    websiteUrl: selectedPreset?.websiteUrl || websiteUrl || undefined,
    notes: notes || undefined,
    meta,
    inFailoverQueue: false,
  };
}

function providerEnv(provider: Provider): Record<string, string> {
  const settings = provider.settingsConfig as { env?: Record<string, unknown> } | null;
  const env: Record<string, string> = {};
  for (const [key, value] of Object.entries(settings?.env || {})) {
    if (typeof value === 'string') env[key] = value;
  }
  return env;
}

function providerMeta(provider: Provider): {
  apiFormat?: ApiFormat;
  apiKeyField?: ApiKeyField;
  isFullUrl?: boolean;
  promptCacheKey?: string;
  authMode?: 'api_key' | 'oauth_proxy';
  providerType?: string;
} {
  return (provider.meta || {}) as {
    apiFormat?: ApiFormat;
    apiKeyField?: ApiKeyField;
    isFullUrl?: boolean;
    promptCacheKey?: string;
    authMode?: 'api_key' | 'oauth_proxy';
    providerType?: string;
  };
}

function findApiKeyField(env: Record<string, string>): ApiKeyField {
  return env.ANTHROPIC_API_KEY !== undefined ? 'ANTHROPIC_API_KEY' : 'ANTHROPIC_AUTH_TOKEN';
}

function setOptionalEnv(env: Record<string, string>, key: string, value: string) {
  const trimmed = value.trim();
  if (trimmed) {
    env[key] = trimmed;
  } else {
    delete env[key];
  }
}
