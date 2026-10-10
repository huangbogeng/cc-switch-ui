import { BadgeCheck, XCircle } from 'lucide-react';
import type { ProviderStatsItem } from '@/api';

function fmt(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`;
  return n.toLocaleString();
}

interface Props {
  data: ProviderStatsItem[] | undefined;
  loading: boolean;
}

export default function ProviderStatsTable({ data, loading }: Props) {
  if (loading) {
    return <div className="h-48 animate-pulse rounded-xl bg-white/5" />;
  }

  if (!data || data.length === 0) {
    return <div className="py-8 text-center text-sm text-muted-foreground">No provider data</div>;
  }

  return (
    <div className="data-surface divide-y divide-white/[0.07]">
      {data.map((p) => {
        const total = p.request_count;
        const successRate = total > 0 ? Math.round((p.success_count / total) * 100) : 0;
        return (
          <div
            key={p.provider_id}
            className="data-row"
          >
            <div className="min-w-0 flex-1">
              <div className="flex items-center gap-2">
                <span className="text-[15px] font-semibold text-foreground truncate">{p.provider_id}</span>
                {successRate >= 99 ? (
                  <BadgeCheck className="h-4 w-4 text-emerald-500 flex-shrink-0" />
                ) : successRate < 90 ? (
                  <XCircle className="h-4 w-4 text-red-500 flex-shrink-0" />
                ) : null}
              </div>
              <div className="flex flex-wrap gap-x-4 gap-y-1 mt-1 text-xs text-muted-foreground">
                <span>{total.toLocaleString()} requests</span>
                <span>{successRate}% success</span>
              </div>
            </div>
            <div className="shrink-0 sm:ml-4 sm:text-right">
              <div className="text-sm font-mono text-muted-foreground">
                {fmt(p.total_input_tokens)} / {fmt(p.total_output_tokens)}
              </div>
              <div className="text-[10px] text-muted-foreground/60">in / out</div>
            </div>
          </div>
        );
      })}
    </div>
  );
}
