import type { ModelStatsItem } from '@/api';

function fmt(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`;
  return n.toLocaleString();
}

interface Props {
  data: ModelStatsItem[] | undefined;
  loading: boolean;
}

export default function ModelStatsTable({ data, loading }: Props) {
  if (loading) {
    return <div className="h-48 animate-pulse rounded-xl bg-white/5" />;
  }

  if (!data || data.length === 0) {
    return <div className="py-8 text-center text-sm text-muted-foreground">No model data</div>;
  }

  return (
    <div className="space-y-2">
      {data.map((m) => (
        <div
          key={m.model}
          className="flex items-center justify-between rounded-xl border border-white/5 bg-white/[0.02] p-3 transition-colors hover:bg-white/[0.04]"
        >
          <div className="min-w-0 flex-1">
            <div className="font-semibold text-foreground truncate">{m.model || 'unknown'}</div>
            <div className="text-xs text-muted-foreground mt-0.5">
              {m.request_count.toLocaleString()} requests
            </div>
          </div>
          <div className="text-right flex-shrink-0 ml-4">
            <div className="text-sm font-mono text-muted-foreground">
              {fmt(m.total_input_tokens)} / {fmt(m.total_output_tokens)}
            </div>
            <div className="text-[10px] text-muted-foreground/60">in / out</div>
          </div>
        </div>
      ))}
    </div>
  );
}
