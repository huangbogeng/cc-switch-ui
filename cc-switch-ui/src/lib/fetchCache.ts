const cache = new Map<string, { data: unknown; expiresAt: number }>();

const DEFAULT_TTL = 60_000; // 1 minute

export function cacheGet<T>(key: string): T | undefined {
  const entry = cache.get(key);
  if (!entry) return undefined;
  if (Date.now() > entry.expiresAt) {
    cache.delete(key);
    return undefined;
  }
  return entry.data as T;
}

export function cacheSet<T>(key: string, data: T, ttl = DEFAULT_TTL) {
  cache.set(key, { data, expiresAt: Date.now() + ttl });
}

export function cacheHas(key: string): boolean {
  const entry = cache.get(key);
  return entry !== undefined && Date.now() <= entry.expiresAt;
}

export function cacheDelete(key: string) {
  cache.delete(key);
}

export function cacheClear() {
  cache.clear();
}
