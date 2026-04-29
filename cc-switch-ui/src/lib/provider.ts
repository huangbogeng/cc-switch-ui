import type { Provider } from '@/api';

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
