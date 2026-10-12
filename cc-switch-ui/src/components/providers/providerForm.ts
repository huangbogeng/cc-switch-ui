import type { Provider } from '@/api';
import { providerPresets, type ProviderPreset } from '@/config/providerPresets';

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
  noAuth: boolean;
  promptCacheKey: string;
  codexAccountId: string;
  codexFastMode: boolean;
  subagentModel: string;
  effortLevel: string;
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
  noAuth: false,
  promptCacheKey: '',
  codexAccountId: '',
  codexFastMode: false,
  subagentModel: '',
  effortLevel: '',
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
    noAuth: !!preset.noAuth,
    promptCacheKey: '',
    codexAccountId: '',
    codexFastMode: preset.codexFastMode || false,
    subagentModel: env.CLAUDE_CODE_SUBAGENT_MODEL || '',
    effortLevel: env.CLAUDE_CODE_EFFORT_LEVEL || '',
  };
}

export function formFromProvider(provider: Provider): ProviderFormData {
  const env = providerEnv(provider);
  const meta = providerMeta(provider);
  const authMode = meta.authMode === 'oauth_proxy' || meta.providerType === 'codex_oauth' ? 'oauth_proxy' : 'api_key';
  const apiKeyField = meta.apiKeyField || findApiKeyField(env);
  const fallbackApiKeyField: ApiKeyField =
    apiKeyField === 'ANTHROPIC_AUTH_TOKEN' ? 'ANTHROPIC_API_KEY' : 'ANTHROPIC_AUTH_TOKEN';
  const model = env.ANTHROPIC_MODEL || '';

  return {
    id: provider.id,
    name: provider.name,
    authMode,
    websiteUrl: provider.websiteUrl || '',
    baseUrl: env.ANTHROPIC_BASE_URL || '',
    notes: provider.notes || '',
    apiKey: env[apiKeyField] || env[fallbackApiKeyField] || '',
    apiKeyField,
    apiFormat: meta.apiFormat || 'anthropic',
    isFullUrl: meta.isFullUrl || false,
    model,
    haikuModel: env.ANTHROPIC_DEFAULT_HAIKU_MODEL || model,
    sonnetModel: env.ANTHROPIC_DEFAULT_SONNET_MODEL || model,
    opusModel: env.ANTHROPIC_DEFAULT_OPUS_MODEL || model,
    noAuth: meta.noAuth ?? (providerPresets.some((preset) => preset.id === provider.id && preset.noAuth)
      && !env.ANTHROPIC_BASE_URL && !env.ANTHROPIC_AUTH_TOKEN && !env.ANTHROPIC_API_KEY),
    promptCacheKey: meta.promptCacheKey || '',
    codexAccountId: meta.authBinding?.accountId || '',
    codexFastMode: meta.codexFastMode || false,
    subagentModel: env.CLAUDE_CODE_SUBAGENT_MODEL || '',
    effortLevel: env.CLAUDE_CODE_EFFORT_LEVEL || '',
  };
}

export function buildProvider(
  formData: ProviderFormData,
  selectedPreset: ProviderPreset | null,
  existingProvider?: Provider,
): Provider {
  const authMode = selectedPreset?.authMode || formData.authMode;
  const id = formData.id.trim();
  const name = formData.name.trim();
  const websiteUrl = formData.websiteUrl.trim();
  const baseUrl = formData.baseUrl.trim();
  const apiKey = formData.apiKey.trim();
  const notes = formData.notes.trim();
  const noAuth = selectedPreset?.noAuth ?? formData.noAuth;
  const promptCacheKey = formData.promptCacheKey.trim();
  const env: Record<string, string> = existingProvider
    ? providerEnv(existingProvider)
    : selectedPreset
      ? { ...selectedPreset.settingsConfig.env }
      : {};

  if (noAuth) {
    delete env.ANTHROPIC_BASE_URL;
    delete env.ANTHROPIC_AUTH_TOKEN;
    delete env.ANTHROPIC_API_KEY;
  } else if (baseUrl) env.ANTHROPIC_BASE_URL = baseUrl;
  if (authMode === 'api_key') {
    delete env.ANTHROPIC_AUTH_TOKEN;
    delete env.ANTHROPIC_API_KEY;
    // Never persist an empty credential field: keyless providers (e.g. the
    // official Claude subscription) must save a clean env, not a blank token.
    if (apiKey && !noAuth) env[formData.apiKeyField] = apiKey;
  }
  setOptionalEnv(env, 'ANTHROPIC_MODEL', formData.model);
  setOptionalEnv(env, 'ANTHROPIC_DEFAULT_HAIKU_MODEL', formData.haikuModel || formData.model);
  setOptionalEnv(env, 'ANTHROPIC_DEFAULT_SONNET_MODEL', formData.sonnetModel || formData.model);
  setOptionalEnv(env, 'ANTHROPIC_DEFAULT_OPUS_MODEL', formData.opusModel || formData.model);
  setOptionalEnv(env, 'CLAUDE_CODE_SUBAGENT_MODEL', formData.subagentModel);
  setOptionalEnv(env, 'CLAUDE_CODE_EFFORT_LEVEL', formData.effortLevel);

  const existingMeta = existingProvider?.meta && typeof existingProvider.meta === 'object'
    ? existingProvider.meta as Record<string, unknown>
    : {};
  const meta: Record<string, unknown> = {
    ...existingMeta,
    authMode,
    noAuth,
    apiFormat: formData.apiFormat,
    apiKeyField: formData.apiKeyField,
    isFullUrl: formData.isFullUrl,
  };

  if (promptCacheKey) {
    meta.promptCacheKey = promptCacheKey;
  } else {
    delete meta.promptCacheKey;
  }
  if (authMode === 'oauth_proxy') {
    meta.providerType = selectedPreset?.providerType || 'codex_oauth';
    if (formData.codexFastMode) {
      meta.codexFastMode = true;
    } else {
      delete meta.codexFastMode;
    }
    meta.authBinding = {
      source: 'managed_account',
      authProvider: 'codex_oauth',
    };
    if (formData.codexAccountId.trim()) {
      meta.authBinding = {
        source: 'managed_account',
        authProvider: 'codex_oauth',
        accountId: formData.codexAccountId.trim(),
      };
    }
  } else {
    delete meta.providerType;
    delete meta.authBinding;
    delete meta.codexFastMode;
  }

  return {
    ...existingProvider,
    id,
    name,
    settingsConfig: { env },
    icon: selectedPreset?.icon ?? existingProvider?.icon,
    iconColor: selectedPreset?.iconColor ?? existingProvider?.iconColor,
    websiteUrl: selectedPreset?.websiteUrl || websiteUrl || undefined,
    notes: notes || undefined,
    meta,
    inFailoverQueue: existingProvider?.inFailoverQueue ?? false,
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
  noAuth?: boolean;
  authMode?: 'api_key' | 'oauth_proxy';
  providerType?: string;
  authBinding?: { accountId?: string };
  codexFastMode?: boolean;
} {
  return (provider.meta || {}) as {
    apiFormat?: ApiFormat;
    apiKeyField?: ApiKeyField;
    isFullUrl?: boolean;
    promptCacheKey?: string;
    noAuth?: boolean;
    authMode?: 'api_key' | 'oauth_proxy';
    providerType?: string;
    authBinding?: { accountId?: string };
    codexFastMode?: boolean;
  };
}

function findApiKeyField(_env: Record<string, string>): ApiKeyField {
  // Respect existing persisted shape first to avoid flipping field selection
  // during edit/save cycles on legacy providers without meta.apiKeyField.
  if (Object.prototype.hasOwnProperty.call(_env, 'ANTHROPIC_API_KEY')) {
    return 'ANTHROPIC_API_KEY';
  }
  return 'ANTHROPIC_AUTH_TOKEN';
}

function setOptionalEnv(env: Record<string, string>, key: string, value: string) {
  const trimmed = value.trim();
  if (trimmed) {
    env[key] = trimmed;
  } else {
    delete env[key];
  }
}

export function validateProviderDraft(form: ProviderFormData) {
  if (!form.id.trim() || !form.name.trim()) throw new Error('Provider ID and name are required.');
  if (form.baseUrl.trim()) {
    let endpoint: URL;
    try { endpoint = new URL(form.baseUrl.trim()); } catch { throw new Error('Base URL must be a valid absolute URL.'); }
    if (endpoint.protocol !== 'http:' && endpoint.protocol !== 'https:') throw new Error('Base URL must use HTTP or HTTPS.');
  }
}

/** Emit edits relative to the displayed baseline, preserving stored values hidden by form defaults. */
export function buildEditedProvider(form: ProviderFormData, preset: ProviderPreset | null, base?: Provider): Provider {
  const desired = buildProvider(form, preset, base);
  if (!base) return desired;
  const displayed = buildProvider(formFromProvider(base), null, base);
  const object = (value: unknown): value is Record<string, unknown> => value !== null && typeof value === 'object' && !Array.isArray(value);
  const apply = (before: unknown, after: unknown, current: unknown): unknown => {
    if (JSON.stringify(before) === JSON.stringify(after)) return current;
    if (object(before) && object(after)) {
      const result: Record<string, unknown> = object(current) ? { ...current } : {};
      for (const key of new Set([...Object.keys(before), ...Object.keys(after)])) {
        if (!(key in after)) { if (key in before) delete result[key]; }
        else { const next = apply(before[key], after[key], result[key]); if (next === undefined) delete result[key]; else result[key] = next; }
      }
      return result;
    }
    return after;
  };
  return apply(displayed, desired, base) as Provider;
}
