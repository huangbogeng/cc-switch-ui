import { useCallback, useEffect, useState } from 'react';
import { BarChart3, TrendingUp, Zap, Clock, Activity } from 'lucide-react';
import { PageHeader } from '@/components/PageHeader';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { getProxyUsageSummary, type ProxyUsageSummaryResponse } from '@/api';

function formatTokens(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`;
  return n.toString();
}

function formatDate(dateStr: string): string {
  const d = new Date(dateStr);
  return d.toLocaleDateString('en-US', { month: 'short', day: 'numeric' });
}

function StatCard({
  icon: Icon,
  label,
  value,
  sub,
  accent,
}: {
  icon: typeof Zap;
  label: string;
  value: string;
  sub?: string;
  accent: 'primary' | 'amber' | 'emerald' | 'violet';
}) {
  const accentClassMap: Record<string, { overlay: string; iconBg: string; iconText: string }> = {
    primary: { overlay: 'from-primary/10', iconBg: 'bg-primary/15', iconText: 'text-primary' },
    amber: { overlay: 'from-amber-500/10', iconBg: 'bg-amber-500/15', iconText: 'text-amber-500' },
    emerald: { overlay: 'from-emerald-500/10', iconBg: 'bg-emerald-500/15', iconText: 'text-emerald-500' },
    violet: { overlay: 'from-violet-500/10', iconBg: 'bg-violet-500/15', iconText: 'text-violet-500' },
  };
  const accentClasses = accentClassMap[accent] ?? accentClassMap.primary;

  return (
    <div className="relative group overflow-hidden rounded-2xl border border-white/10 bg-gradient-to-b from-card to-card/50 p-5 shadow-xl transition-all duration-300 hover:border-white/20 hover:shadow-2xl">
      <div className={`absolute inset-0 bg-gradient-to-br ${accentClasses.overlay} via-transparent to-transparent opacity-0 group-hover:opacity-100 transition-opacity duration-500`} />
      <div className="flex items-start justify-between">
        <div className={`rounded-xl ${accentClasses.iconBg} p-2.5 shadow-inner`}>
          <Icon className={`h-5 w-5 ${accentClasses.iconText}`} />
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

function TrendBar({ day, input, output, max }: { day: string; input: number; output: number; max: number }) {
  const inputHeight = max > 0 ? (input / max) * 100 : 0;
  const outputHeight = max > 0 ? (output / max) * 100 : 0;

  return (
    <div className="flex flex-col items-center gap-1.5">
      <div className="flex items-end gap-0.5 h-24 w-full">
        <div className="relative flex-1 flex flex-col justify-end">
          <div className="w-full rounded-t-md bg-gradient-to-t from-primary/60 to-primary/30" style={{ height: `${inputHeight}%` }} />
        </div>
        <div className="relative flex-1 flex flex-col justify-end">
          <div className="w-full rounded-t-md bg-gradient-to-t from-amber-500/60 to-amber-500/30" style={{ height: `${outputHeight}%` }} />
        </div>
      </div>
      <div className="text-[10px] text-muted-foreground font-medium">{formatDate(day)}</div>
    </div>
  );
}

export default function UsagePage() {
  const [summary, setSummary] = useState<ProxyUsageSummaryResponse | null>(null);
  const [loading, setLoading] = useState(true);

  const loadUsage = useCallback(async () => {
    try {
      setSummary(await getProxyUsageSummary());
    } catch (e) {
      console.error('Failed to load usage:', e);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    Promise.resolve().then(loadUsage);
    const interval = setInterval(loadUsage, 10000);
    return () => clearInterval(interval);
  }, [loadUsage]);

  const maxTrendValue = summary?.trend.reduce((max, d) => Math.max(max, d.total_input_tokens + d.total_output_tokens), 0) || 0;
  const topProvider = summary?.providers[0];
  const topModel = summary?.models[0];

  return (
    <div className="space-y-8 animate-in fade-in slide-in-from-bottom-4 duration-500">
      <PageHeader title="Usage" description="Monitor local route usage across all providers." />

      {loading ? (
        <div className="flex h-64 items-center justify-center">
          <Activity className="h-8 w-8 animate-spin text-primary/50" />
        </div>
      ) : (summary?.totals.request_count || 0) === 0 ? (
        <div className="flex flex-col items-center justify-center rounded-3xl border border-dashed border-white/10 bg-white/[0.02] py-20 text-center">
          <div className="rounded-2xl bg-white/5 p-4 shadow-inner"><BarChart3 className="h-10 w-10 text-muted-foreground/30" /></div>
          <div className="mt-6 text-lg font-semibold text-foreground">No usage data yet</div>
        </div>
      ) : (
        <>
          <div className="grid grid-cols-2 gap-4 lg:grid-cols-4">
            <StatCard icon={Zap} label="Total Input" value={formatTokens(summary?.totals.input_tokens || 0)} sub="tokens" accent="primary" />
            <StatCard icon={TrendingUp} label="Total Output" value={formatTokens(summary?.totals.output_tokens || 0)} sub="tokens" accent="amber" />
            <StatCard icon={Clock} label="Total Requests" value={(summary?.totals.request_count || 0).toLocaleString()} sub="all providers" accent="emerald" />
            <StatCard icon={Activity} label="Top Provider" value={topProvider?.provider_id || 'None'} sub={topProvider ? `${topProvider.request_count} requests` : undefined} accent="violet" />
          </div>

          {summary && summary.trend.length > 0 && (
            <Card className="border-white/10 bg-card/60 backdrop-blur-sm shadow-xl overflow-hidden">
              <CardHeader className="border-b border-white/5 bg-black/20 pb-4"><CardTitle className="text-sm font-semibold text-muted-foreground">30-Day Trend</CardTitle></CardHeader>
              <CardContent className="p-6">
                <div className="grid grid-cols-[repeat(auto-fill,minmax(48px,1fr))] gap-2">
                  {summary.trend.slice().reverse().map((d) => (
                    <TrendBar key={d.day} day={d.day} input={d.total_input_tokens} output={d.total_output_tokens} max={maxTrendValue} />
                  ))}
                </div>
              </CardContent>
            </Card>
          )}

          <div className="grid gap-6 xl:grid-cols-2">
            <Card className="border-white/10 bg-card/60 backdrop-blur-sm shadow-xl">
              <CardHeader className="border-b border-white/5 bg-black/20 pb-4"><CardTitle className="text-sm font-semibold text-muted-foreground">Provider Breakdown</CardTitle></CardHeader>
              <CardContent className="p-6 space-y-3">
                {summary?.providers.map((item) => (
                  <div key={item.provider_id} className="flex items-center justify-between rounded-xl border border-white/5 bg-white/[0.02] p-3">
                    <div>
                      <div className="font-semibold text-foreground">{item.provider_id}</div>
                      <div className="text-xs text-muted-foreground">{item.request_count.toLocaleString()} requests</div>
                    </div>
                    <div className="text-sm font-mono text-muted-foreground">{formatTokens(item.input_tokens + item.output_tokens)}</div>
                  </div>
                ))}
              </CardContent>
            </Card>

            <Card className="border-white/10 bg-card/60 backdrop-blur-sm shadow-xl">
              <CardHeader className="border-b border-white/5 bg-black/20 pb-4"><CardTitle className="text-sm font-semibold text-muted-foreground">Model Breakdown</CardTitle></CardHeader>
              <CardContent className="p-6 space-y-3">
                {summary?.models.map((item) => (
                  <div key={item.model} className="flex items-center justify-between rounded-xl border border-white/5 bg-white/[0.02] p-3">
                    <div>
                      <div className="font-semibold text-foreground">{item.model || 'unknown'}</div>
                      <div className="text-xs text-muted-foreground">{item.request_count.toLocaleString()} requests</div>
                    </div>
                    <div className="text-sm font-mono text-muted-foreground">{formatTokens(item.input_tokens + item.output_tokens)}</div>
                  </div>
                ))}
                {topModel && <div className="pt-2 text-xs text-muted-foreground">Top model: <span className="text-foreground">{topModel.model}</span></div>}
              </CardContent>
            </Card>
          </div>
        </>
      )}
    </div>
  );
}
