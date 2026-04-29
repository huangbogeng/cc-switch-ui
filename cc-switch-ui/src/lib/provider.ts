import type { Provider } from '@/api';

export type ProviderAuthMode = 'api_key' | 'oauth_proxy';

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
    meta?.providerType === 'codex_oauth' ||
    meta?.authBinding?.authProvider === 'codex_oauth' ||
    meta?.authBinding?.auth_provider === 'codex_oauth'
  ) {
    return 'oauth_proxy';
  }
  return 'api_key';
}

export function providerAuthLabel(provider: Provider) {
  return providerAuthMode(provider) === 'oauth_proxy' ? 'OAuth Proxy' : 'API Key';
}

export function providerApiFormat(provider: Provider) {
  const meta = provider.meta as { apiFormat?: string } | null;
  return meta?.apiFormat || 'anthropic';
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
