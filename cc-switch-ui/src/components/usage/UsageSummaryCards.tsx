import { Activity, Type, Braces, CheckCircle } from 'lucide-react';
import type { ProxyUsageSummaryResponse } from '@/api';

function formatNum(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`;
  return n.toLocaleString();
}

interface StatCardProps {
  icon: typeof Activity;
  label: string;
  value: string;
  sub?: string;
  accent: 'primary' | 'amber' | 'emerald' | 'violet';
}

function StatCard({ icon: Icon, label, value, sub, accent }: StatCardProps) {
  const accentMap: Record<string, string> = {
    primary: 'from-primary/10 bg-primary/15 text-primary',
    amber: 'from-amber-500/10 bg-amber-500/15 text-amber-500',
    emerald: 'from-emerald-500/10 bg-emerald-500/15 text-emerald-500',
    violet: 'from-violet-500/10 bg-violet-500/15 text-violet-500',
  };
  const [, iconBg, iconText] = accentMap[accent].split(' ');

  return (
    <div className="relative overflow-hidden rounded-2xl border border-white/10 bg-gradient-to-b from-card to-card/50 p-5 shadow-xl transition-all duration-300 hover:border-white/20 hover:shadow-2xl">
      <div className={`absolute inset-0 bg-gradient-to-br ${accentMap[accent].split(' ')[0]} via-transparent to-transparent opacity-0 group-hover:opacity-100 transition-opacity duration-500`} />
      <div className="flex items-start justify-between">
        <div className={`rounded-xl ${iconBg} p-2.5 shadow-inner`}>
          <Icon className={`h-5 w-5 ${iconText}`} />
        </div>
      </div>
      <div className="mt-4">
        <div className="text-2xl font-bold tracking-tight text-foreground tabular-nums">{value}</div>
        <div className="mt-1 text-sm font-medium text-muted-foreground">{label}</div>
        {sub && <div className="mt-1 text-xs text-muted-foreground/70">{sub}</div>}
      </div>
    </div>
  );
}

interface Props {
  data: ProxyUsageSummaryResponse | undefined;
  loading: boolean;
}

export default function UsageSummaryCards({ data, loading }: Props) {
  if (loading || !data) {
    return (
      <div className="grid grid-cols-2 gap-4 lg:grid-cols-4">
        {[...Array(4)].map((_, i) => (
          <div key={i} className="h-32 animate-pulse rounded-2xl bg-white/5" />
        ))}
      </div>
    );
  }

  const { totals } = data;

  return (
    <div className="space-y-4">
      <div className="grid grid-cols-2 gap-4 lg:grid-cols-4">
        <StatCard
          icon={Activity}
          label="Total Requests"
          value={totals.request_count.toLocaleString()}
          sub="all providers"
          accent="emerald"
        />
        <StatCard
          icon={Type}
          label="Input Tokens"
          value={formatNum(totals.input_tokens)}
          accent="primary"
        />
        <StatCard
          icon={Braces}
          label="Output Tokens"
          value={formatNum(totals.output_tokens)}
          accent="amber"
        />
        <StatCard
          icon={CheckCircle}
          label="Top Provider"
          value={data.providers[0]?.provider_id ?? 'N/A'}
          sub={data.providers[0] ? `${data.providers[0].request_count} requests` : undefined}
          accent="violet"
        />
      </div>

      {data.sources.length > 0 && (
        <div className="flex flex-wrap gap-2">
          {data.sources.map((s) => (
            <div
              key={s.app_type}
              className="rounded-xl border border-white/10 bg-card/50 px-3 py-1.5 text-xs"
            >
              <span className="text-muted-foreground">{s.app_type}: </span>
              <span className="font-medium text-foreground">{s.request_count.toLocaleString()} req</span>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
