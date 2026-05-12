export type UsageRangePreset = 'today' | '1d' | '7d' | '14d' | '30d' | 'custom';

export interface UsageRangeSelection {
  preset: UsageRangePreset;
  customStartDate?: number; // unix seconds
  customEndDate?: number;   // unix seconds
}

export interface ResolvedRange {
  startDate: number;
  endDate: number;
}

const DAY_S = 86400;

function startOfDay(ms: number): number {
  const d = new Date(ms);
  return Math.floor(new Date(d.getFullYear(), d.getMonth(), d.getDate()).getTime() / 1000);
}

export function resolveUsageRange(sel: UsageRangeSelection, now: number = Date.now()): ResolvedRange {
  const end = Math.floor(now / 1000);
  switch (sel.preset) {
    case 'today':
      return { startDate: startOfDay(now), endDate: end };
    case '1d':
      return { startDate: end - DAY_S, endDate: end };
    case '7d':
      return { startDate: startOfDay(now - 6 * DAY_S * 1000), endDate: end };
    case '14d':
      return { startDate: startOfDay(now - 13 * DAY_S * 1000), endDate: end };
    case '30d':
      return { startDate: startOfDay(now - 29 * DAY_S * 1000), endDate: end };
    case 'custom':
      return {
        startDate: sel.customStartDate ?? end - DAY_S,
        endDate: sel.customEndDate ?? end,
      };
  }
}

export function formatRangeLabel(sel: UsageRangeSelection, resolved: ResolvedRange): string {
  if (sel.preset !== 'custom') {
    const labels: Record<UsageRangePreset, string> = {
      today: 'Today',
      '1d': '24h',
      '7d': '7d',
      '14d': '14d',
      '30d': '30d',
      custom: '',
    };
    return labels[sel.preset];
  }
  const fmt = (ts: number) =>
    new Date(ts * 1000).toLocaleDateString(undefined, { month: 'short', day: 'numeric' });
  return `${fmt(resolved.startDate)} – ${fmt(resolved.endDate)}`;
}
