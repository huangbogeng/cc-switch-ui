import { useCallback, useEffect, useState } from 'react';
import { BarChart3, TrendingUp, Zap, Clock, Activity } from 'lucide-react';
import { PageHeader } from '@/components/PageHeader';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Progress } from '@/components/ui/progress';
import {
  getProxyUsageSummary,
  getProxyUsageTrend,
  type ProxyUsageSummaryResponse,
  type ProxyUsageTrendResponse,
} from '@/api';

function formatTokens(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`;
  return n.toString();
}

function formatDate(dateStr: string): string {
  const d = new Date(dateStr);
  return d.toLocaleDateString('en-US', { month: 'short', day: 'numeric' });
}

function isMiniMaxUsage(item: { provider_id: string; model: string }) {
  const provider = item.provider_id.toLowerCase();
  const model = item.model.toLowerCase();
  return provider.includes('minimax') || model.includes('minimax');
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
  accent: string;
}) {
  return (
    <div className="relative group overflow-hidden rounded-2xl border border-white/10 bg-gradient-to-b from-card to-card/50 p-5 shadow-xl transition-all duration-300 hover:border-white/20 hover:shadow-2xl">
      <div className={`absolute inset-0 bg-gradient-to-br from-${accent}/10 via-transparent to-transparent opacity-0 group-hover:opacity-100 transition-opacity duration-500`} />
      <div className="flex items-start justify-between">
        <div className={`rounded-xl bg-${accent}/15 p-2.5 shadow-inner`}>
          <Icon className={`h-5 w-5 text-${accent}`} />
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
          <div
            className="w-full rounded-t-md bg-gradient-to-t from-primary/60 to-primary/30 transition-all duration-300 hover:from-primary/80 hover:to-primary/50"
            style={{ height: `${inputHeight}%` }}
          />
        </div>
        <div className="relative flex-1 flex flex-col justify-end">
          <div
            className="w-full rounded-t-md bg-gradient-to-t from-amber-500/60 to-amber-500/30 transition-all duration-300 hover:from-amber-500/80 hover:to-amber-500/50"
            style={{ height: `${outputHeight}%` }}
          />
        </div>
      </div>
      <div className="text-[10px] text-muted-foreground font-medium">{formatDate(day)}</div>
    </div>
  );
}

function MiniMaxMetric({ label, value, sub }: { label: string; value: string; sub?: string }) {
  return (
    <div className="rounded-2xl border border-white/5 bg-black/20 p-4 shadow-inner">
      <div className="text-[10px] font-bold uppercase tracking-wider text-muted-foreground/70">{label}</div>
      <div className="mt-2 font-mono text-xl font-bold tabular-nums text-foreground">{value}</div>
      {sub && <div className="mt-0.5 text-[11px] font-medium text-muted-foreground">{sub}</div>}
    </div>
  );
}

export default function UsagePage() {
  const [summary, setSummary] = useState<ProxyUsageSummaryResponse | null>(null);
  const [trend, setTrend] = useState<ProxyUsageTrendResponse | null>(null);
  const [loading, setLoading] = useState(true);

  const loadUsage = useCallback(async () => {
    try {
      const [summaryData, trendData] = await Promise.all([
        getProxyUsageSummary(),
        getProxyUsageTrend(),
      ]);
      setSummary(summaryData);
      setTrend(trendData);
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

  const maxTrendValue = trend?.trend.reduce((max, d) => {
    const dayMax = d.total_input_tokens + d.total_output_tokens;
    return dayMax > max ? dayMax : max;
  }, 0) || 0;

  const topProvider = summary?.summary[0];
  const minimaxItems = summary?.summary.filter(isMiniMaxUsage) || [];
  const minimaxTotals = minimaxItems.reduce(
    (totals, item) => ({
      input: totals.input + item.total_input_tokens,
      output: totals.output + item.total_output_tokens,
      requests: totals.requests + item.request_count,
    }),
    { input: 0, output: 0, requests: 0 }
  );
  const minimaxModels = new Set(minimaxItems.map((item) => item.model).filter(Boolean)).size;

  return (
    <div className="space-y-8 animate-in fade-in slide-in-from-bottom-4 duration-500">
      <PageHeader
        title="Usage"
        description="Monitor local route request usage and trends."
      />

      {loading ? (
        <div className="flex h-64 items-center justify-center">
          <Activity className="h-8 w-8 animate-spin text-primary/50" />
        </div>
      ) : summary?.total_requests === 0 ? (
        <div className="flex flex-col items-center justify-center rounded-3xl border border-dashed border-white/10 bg-white/[0.02] py-20 text-center">
          <div className="rounded-2xl bg-white/5 p-4 shadow-inner">
            <BarChart3 className="h-10 w-10 text-muted-foreground/30" />
          </div>
          <div className="mt-6 text-lg font-semibold text-foreground">No usage data yet</div>
          <div className="mt-2 max-w-sm text-sm text-muted-foreground">
            Route requests will be tracked here once the local route starts and handles traffic.
          </div>
        </div>
      ) : (
        <>
          <Card className="overflow-hidden border-white/10 bg-gradient-to-br from-card to-card/50 shadow-xl">
            <CardHeader className="border-b border-white/5 bg-black/20 pb-4">
              <CardTitle className="flex items-center justify-between gap-3 text-sm font-semibold text-muted-foreground tracking-tight">
                <div className="flex items-center gap-2.5">
                  <div className="rounded-md bg-white/5 p-1.5 shadow-inner">
                    <Activity className="h-4 w-4 text-emerald-500" />
                  </div>
                  MiniMax Usage
                </div>
                <span className="rounded-lg border border-emerald-500/20 bg-emerald-500/10 px-2 py-1 text-[10px] font-bold uppercase tracking-wider text-emerald-400">
                  Tokens only
                </span>
              </CardTitle>
            </CardHeader>
            <CardContent className="p-6">
              {minimaxTotals.requests === 0 ? (
                <div className="rounded-2xl border border-dashed border-white/10 bg-white/[0.02] px-5 py-8 text-center">
                  <div className="text-sm font-semibold text-foreground">No MiniMax usage yet</div>
                  <div className="mt-1 text-sm text-muted-foreground">
                    Start a MiniMax route request and token usage will appear here.
                  </div>
                </div>
              ) : (
                <div className="grid gap-4 sm:grid-cols-4">
                  <MiniMaxMetric label="Requests" value={minimaxTotals.requests.toLocaleString()} />
                  <MiniMaxMetric label="Input" value={formatTokens(minimaxTotals.input)} sub="tokens" />
                  <MiniMaxMetric label="Output" value={formatTokens(minimaxTotals.output)} sub="tokens" />
                  <MiniMaxMetric label="Models" value={minimaxModels.toLocaleString()} />
                </div>
              )}
            </CardContent>
          </Card>

          <div className="grid grid-cols-2 gap-4 lg:grid-cols-4">
            <StatCard
              icon={Zap}
              label="Total Input"
              value={formatTokens(summary?.total_input_tokens || 0)}
              sub="tokens processed"
              accent="primary"
            />
            <StatCard
              icon={TrendingUp}
              label="Total Output"
              value={formatTokens(summary?.total_output_tokens || 0)}
              sub="tokens generated"
              accent="amber-500"
            />
            <StatCard
              icon={Clock}
              label="Total Requests"
              value={(summary?.total_requests || 0).toLocaleString()}
              sub="route requests"
              accent="emerald-500"
            />
            <StatCard
              icon={Activity}
              label="Active Provider"
              value={topProvider?.provider_id || 'None'}
              sub={topProvider ? `${topProvider.request_count} requests` : undefined}
              accent="violet-500"
            />
          </div>

          {trend && trend.trend.length > 0 && (
            <Card className="border-white/10 bg-card/60 backdrop-blur-sm shadow-xl overflow-hidden">
              <CardHeader className="border-b border-white/5 bg-black/20 pb-4">
                <CardTitle className="flex items-center gap-2.5 text-sm font-semibold text-muted-foreground tracking-tight">
                  <div className="rounded-md bg-white/5 p-1.5 shadow-inner">
                    <BarChart3 className="h-4 w-4 text-primary" />
                  </div>
                  30-Day Trend
                </CardTitle>
              </CardHeader>
              <CardContent className="p-6">
                <div className="flex items-center gap-6 mb-6">
                  <div className="flex items-center gap-2">
                    <div className="h-3 w-3 rounded-sm bg-primary/60" />
                    <span className="text-xs font-medium text-muted-foreground">Input</span>
                  </div>
                  <div className="flex items-center gap-2">
                    <div className="h-3 w-3 rounded-sm bg-amber-500/60" />
                    <span className="text-xs font-medium text-muted-foreground">Output</span>
                  </div>
                </div>
                <div className="grid grid-cols-[repeat(auto-fill,minmax(48px,1fr))] gap-2">
                  {trend.trend.slice().reverse().map((d) => (
                    <TrendBar
                      key={d.day}
                      day={d.day}
                      input={d.total_input_tokens}
                      output={d.total_output_tokens}
                      max={maxTrendValue}
                    />
                  ))}
                </div>
              </CardContent>
            </Card>
          )}

          {summary && summary.summary.length > 0 && (
            <Card className="border-white/10 bg-card/60 backdrop-blur-sm shadow-xl">
              <CardHeader className="border-b border-white/5 bg-black/20 pb-4">
                <CardTitle className="flex items-center justify-between gap-3 text-sm font-semibold text-muted-foreground tracking-tight">
                  <div className="flex items-center gap-2.5">
                    <div className="rounded-md bg-white/5 p-1.5 shadow-inner">
                      <Activity className="h-4 w-4 text-emerald-500" />
                    </div>
                    Provider Breakdown
                  </div>
                </CardTitle>
              </CardHeader>
              <CardContent className="p-6">
                <div className="space-y-4">
                  {summary.summary.map((item, i) => (
                    <div key={`${item.provider_id}-${item.model}`} className={`group relative overflow-hidden rounded-xl border p-4 transition-all duration-200 hover:border-white/10 hover:bg-white/[0.04] ${
                      isMiniMaxUsage(item)
                        ? 'border-emerald-500/20 bg-emerald-500/[0.04]'
                        : 'border-white/5 bg-white/[0.02]'
                    }`}>
                      <div className="flex items-center justify-between">
                        <div className="flex items-center gap-3">
                          <div className="flex h-9 w-9 items-center justify-center rounded-lg bg-gradient-to-br from-primary/20 to-primary/5 border border-primary/20 shadow-sm">
                            <span className="text-sm font-bold text-primary">{i + 1}</span>
                          </div>
                          <div>
                            <div className="font-semibold text-foreground">{item.provider_id}</div>
                            <div className="flex flex-wrap items-center gap-2 text-xs text-muted-foreground">
                              <span>{item.model}</span>
                              {isMiniMaxUsage(item) && (
                                <span className="rounded-md bg-emerald-500/10 px-1.5 py-0.5 text-[10px] font-bold uppercase tracking-wider text-emerald-400">
                                  MiniMax
                                </span>
                              )}
                            </div>
                          </div>
                        </div>
                        <div className="text-right">
                          <div className="font-mono text-sm font-semibold tabular-nums text-foreground">
                            {formatTokens(item.total_input_tokens + item.total_output_tokens)}
                          </div>
                          <div className="text-xs text-muted-foreground">
                            {item.request_count} requests
                          </div>
                        </div>
                      </div>
                      <div className="mt-3 grid grid-cols-2 gap-3">
                        <div>
                          <div className="flex justify-between text-xs mb-1">
                            <span className="text-muted-foreground">Input</span>
                            <span className="font-mono text-muted-foreground">{formatTokens(item.total_input_tokens)}</span>
                          </div>
                          <Progress
                            value={item.total_input_tokens > 0 ? 100 : 0}
                            className="h-1.5 bg-white/5"
                            indicatorClassName="bg-primary/70"
                          />
                        </div>
                        <div>
                          <div className="flex justify-between text-xs mb-1">
                            <span className="text-muted-foreground">Output</span>
                            <span className="font-mono text-muted-foreground">{formatTokens(item.total_output_tokens)}</span>
                          </div>
                          <Progress
                            value={item.total_output_tokens > 0 ? 100 : 0}
                            className="h-1.5 bg-white/5"
                            indicatorClassName="bg-amber-500/70"
                          />
                        </div>
                      </div>
                    </div>
                  ))}
                </div>
              </CardContent>
            </Card>
          )}
        </>
      )}
    </div>
  );
}
