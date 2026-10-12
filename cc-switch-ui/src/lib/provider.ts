import type { Provider } from '@/api';

export type ProviderAuthMode = 'api_key' | 'oauth_proxy';
export type ProviderApiFormat = 'anthropic' | 'openai_chat' | 'openai_responses' | 'gemini_native';

// Keep fallback order aligned with ProviderRegistry and ProtocolAdapter.
const adapterTypes = ['claude', 'codex_oauth', 'copilot', 'deepseek', 'gemini', 'minimax', 'openai_responses', 'openrouter', 'siliconflow'];
function providerAdapterType(provider: Provider): string | undefined {
  const meta = provider.meta as { providerType?: string } | null;
  for (const value of [meta?.providerType, provider.id]) {
    if (typeof value !== 'string' || !value) continue;
    if (adapterTypes.includes(value)) return value;
    const match = adapterTypes.find(type => value.includes(type));
    if (match) return match;
  }
  return undefined;
}

export function sortProviders(providers: Record<string, Provider>) {
  return Object.values(providers).sort(compareProviders);
}

export function providerHost(url?: string) {
  if (!url) return 'Custom';
  try {
    return new URL(url).hostname;
  } catch {
    return url;
  }
}

export function providerInitial(provider?: Pick<Provider, 'name'> | null) {
  return provider?.name?.[0] || 'P';
}

export function providerAuthMode(provider: Provider): ProviderAuthMode {
  const meta = provider.meta as {
    authMode?: ProviderAuthMode;
    providerType?: string;
    authBinding?: { authProvider?: string; auth_provider?: string };
  } | null;
  if (
    meta?.authMode === 'oauth_proxy' ||
    providerAdapterType(provider) === 'codex_oauth' ||
    meta?.authBinding?.authProvider === 'codex_oauth' ||
    meta?.authBinding?.auth_provider === 'codex_oauth'
  ) {
    return 'oauth_proxy';
  }
  return 'api_key';
}

export function providerAuthLabel(provider: Provider) {
  if (providerAuthMode(provider) === 'oauth_proxy') return 'OAuth Account';
  const meta = provider.meta as { apiKeyField?: string } | null;
  return meta?.apiKeyField === 'ANTHROPIC_API_KEY' ? 'API Key' : 'Auth Token';
}

export function providerApiFormat(provider: Provider): string {
  if (providerAuthMode(provider) === 'oauth_proxy') return 'openai_responses';
  const meta = provider.meta as { apiFormat?: unknown } | null;
  if (typeof meta?.apiFormat === 'string') return meta.apiFormat;
  const settings = provider.settingsConfig as { api_format?: unknown } | null;
  if (typeof settings?.api_format === 'string') return settings.api_format;
  switch (providerAdapterType(provider)) {
    case 'deepseek': case 'minimax': case 'openrouter': case 'siliconflow': case 'copilot':
      return 'openai_chat';
    case 'openai_responses': return 'openai_responses';
    case 'gemini': return 'gemini_native';
    default: return 'anthropic';
  }
}

export function providerRouteUnsupportedReason(provider: Provider): string | null {
  const format = providerApiFormat(provider);
  if (['anthropic', 'openai_chat', 'openai_responses'].includes(format)) return null;
  if (format === 'gemini_native') return 'Gemini Native is not supported by the local route. Choose an Anthropic-compatible endpoint.';
  return 'This upstream protocol is not supported by the local route.';
}

export function providerBaseUrl(provider: Provider) {
  const settings = provider.settingsConfig as { env?: Record<string, unknown> } | null;
  const value = settings?.env?.ANTHROPIC_BASE_URL;
  return typeof value === 'string' ? value : '';
}

function compareProviders(a: Provider, b: Provider) {
  const aIndex = a.sortIndex ?? Number.POSITIVE_INFINITY;
  const bIndex = b.sortIndex ?? Number.POSITIVE_INFINITY;
  if (aIndex !== bIndex) return aIndex - bIndex;

  const aCreated = a.createdAt ?? Number.POSITIVE_INFINITY;
  const bCreated = b.createdAt ?? Number.POSITIVE_INFINITY;
  if (aCreated !== bCreated) return aCreated - bCreated;

  const nameCompare = a.name.localeCompare(b.name, undefined, { sensitivity: 'base' });
  if (nameCompare !== 0) return nameCompare;

  return a.id.localeCompare(b.id);
}
