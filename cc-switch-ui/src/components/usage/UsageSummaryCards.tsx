import { Activity, Type, Braces, CheckCircle } from 'lucide-react';
import type { ProxyUsageSummaryResponse } from '@/api';
import { Card, CardContent } from '@/components/ui/card';

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
  const iconColor = { primary: 'text-primary', amber: 'text-amber-400', emerald: 'text-emerald-400', violet: 'text-muted-foreground' }[accent];
  return (
    <Card>
      <CardContent className="p-5 sm:p-6">
        <div className="flex items-center gap-2 text-xs text-muted-foreground">
          <Icon className={`h-4 w-4 ${iconColor}`} />{label}
        </div>
        <div title={value} className="mt-3 truncate text-2xl font-semibold tracking-tight tabular-nums">{value}</div>
        <div className="mt-1 min-h-5 text-xs leading-5 text-muted-foreground">{sub ?? '\u00a0'}</div>
      </CardContent>
    </Card>
  );
}

interface Props {
  data: ProxyUsageSummaryResponse | undefined;
  loading: boolean;
}

export default function UsageSummaryCards({ data, loading }: Props) {
  if (loading) {
    return (
      <div className="grid grid-cols-2 gap-4 lg:grid-cols-4">
        {[...Array(4)].map((_, i) => (
          <div key={i} className="h-32 animate-pulse rounded-2xl bg-white/5" />
        ))}
      </div>
    );
  }

  if (!data) {
    return <div className="py-8 text-center text-sm text-muted-foreground">Usage summary unavailable</div>;
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
              className="rounded-xl border border-white/[0.07] bg-white/[0.025] px-3 py-1.5 text-xs"
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
