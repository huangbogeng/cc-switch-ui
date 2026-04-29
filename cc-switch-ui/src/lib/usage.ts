export function usagePercent(remaining: number, entitlement: number, unlimited: boolean) {
  if (unlimited) return 100;
  if (!entitlement) return 0;
  return Math.max(0, Math.min(100, (remaining / entitlement) * 100));
}
