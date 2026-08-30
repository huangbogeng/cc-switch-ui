import { TrendingUp } from 'lucide-react';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import type { ProxyUsageTrend } from '@/api';

function formatDate(dateStr: string): string {
  const d = new Date(dateStr);
  return d.toLocaleDateString(undefined, { month: 'short', day: 'numeric' });
}

function formatCompact(value: number): string {
  if (value >= 1_000_000) return `${(value / 1_000_000).toFixed(1)}M`;
  if (value >= 1_000) return `${(value / 1_000).toFixed(1)}K`;
  return value.toString();
}

interface Props {
  trend: ProxyUsageTrend[];
  loading: boolean;
  title?: string;
}

export default function UsageTrendChart({ trend, loading, title = '30-Day Token Trend' }: Props) {
  if (loading) {
    return (
      <Card className="overflow-hidden border-white/10 bg-card/60 shadow-xl backdrop-blur-sm">
        <CardHeader className="border-b border-white/5 bg-black/20 pb-4">
          <CardTitle className="text-sm font-semibold text-muted-foreground">{title}</CardTitle>
        </CardHeader>
        <CardContent className="p-6">
          <div className="h-[320px] animate-pulse rounded-2xl bg-white/5" />
        </CardContent>
      </Card>
    );
  }

  if (trend.length === 0) {
    return (
      <Card className="overflow-hidden border-white/10 bg-card/60 shadow-xl backdrop-blur-sm">
        <CardHeader className="border-b border-white/5 bg-black/20 pb-4">
          <CardTitle className="flex items-center gap-2 text-sm font-semibold text-muted-foreground">
            <TrendingUp className="h-4 w-4" />
            {title}
          </CardTitle>
        </CardHeader>
        <CardContent className="p-6">
          <div className="flex h-[320px] items-center justify-center rounded-2xl border border-dashed border-white/10 bg-white/[0.02] text-sm text-muted-foreground">
            No data yet. Sync session logs to see usage trends.
          </div>
        </CardContent>
      </Card>
    );
  }

  const points = trend.slice().reverse();
  const chartHeight = 220;
  const chartWidth = 720;
  const paddingX = 16;
  const step = points.length > 1 ? (chartWidth - paddingX * 2) / (points.length - 1) : 0;
  const maxValue = Math.max(
    ...points.map((point) => Math.max(point.total_input_tokens, point.total_output_tokens)),
    1,
  );

  const toY = (value: number) => chartHeight - (value / maxValue) * (chartHeight - 24) - 8;

  const inputLine = points
    .map((point, index) => `${paddingX + index * step},${toY(point.total_input_tokens)}`)
    .join(' ');
  const outputLine = points
    .map((point, index) => `${paddingX + index * step},${toY(point.total_output_tokens)}`)
    .join(' ');

  const inputArea = `${inputLine} ${paddingX + (points.length - 1) * step},${chartHeight} ${paddingX},${chartHeight}`;
  const outputArea = `${outputLine} ${paddingX + (points.length - 1) * step},${chartHeight} ${paddingX},${chartHeight}`;

  const ticks = [1, 0.66, 0.33, 0].map((ratio) => ({
    value: Math.round(maxValue * ratio),
    y: 8 + (chartHeight - 24) * (1 - ratio),
  }));

  const labelIndexes = Array.from(
    new Set([
      0,
      Math.max(0, Math.floor((points.length - 1) / 3)),
      Math.max(0, Math.floor(((points.length - 1) * 2) / 3)),
      points.length - 1,
    ]),
  );

  const totalInput = points.reduce((sum, point) => sum + point.total_input_tokens, 0);
  const totalOutput = points.reduce((sum, point) => sum + point.total_output_tokens, 0);

  return (
    <Card className="overflow-hidden border-white/10 bg-card/60 shadow-xl backdrop-blur-sm">
      <CardHeader className="border-b border-white/5 bg-black/20 pb-4">
        <CardTitle className="flex items-center justify-between gap-3">
          <div className="flex items-center gap-2 text-sm font-semibold text-muted-foreground">
            <TrendingUp className="h-4 w-4" />
            <span>{title}</span>
          </div>
          <div className="flex flex-wrap items-center gap-2 text-[11px]">
            <LegendChip label="Input" value={formatCompact(totalInput)} tone="primary" />
            <LegendChip label="Output" value={formatCompact(totalOutput)} tone="warning" />
          </div>
        </CardTitle>
      </CardHeader>
      <CardContent className="space-y-5 p-6">
        <div className="rounded-2xl border border-white/8 bg-[linear-gradient(180deg,rgba(255,255,255,0.04),rgba(255,255,255,0.01))] p-4">
          <div className="grid gap-4 lg:grid-cols-[56px_minmax(0,1fr)]">
            <div className="hidden lg:block">
              <div className="relative h-[220px]">
                {ticks.map((tick) => (
                  <div
                    key={tick.y}
                    className="absolute left-0 right-0 text-right text-[10px] font-medium text-muted-foreground/55"
                    style={{ top: tick.y - 8 }}
                  >
                    {formatCompact(tick.value)}
                  </div>
                ))}
              </div>
            </div>

            <div className="overflow-x-auto">
              <div className="min-w-[720px]">
                <svg viewBox={`0 0 ${chartWidth} ${chartHeight + 4}`} className="h-[220px] w-full">
                  <defs>
                    <linearGradient id="usage-input-fill" x1="0" x2="0" y1="0" y2="1">
                      <stop offset="0%" stopColor="rgba(59,130,246,0.30)" />
                      <stop offset="100%" stopColor="rgba(59,130,246,0.03)" />
                    </linearGradient>
                    <linearGradient id="usage-output-fill" x1="0" x2="0" y1="0" y2="1">
                      <stop offset="0%" stopColor="rgba(245,158,11,0.26)" />
                      <stop offset="100%" stopColor="rgba(245,158,11,0.03)" />
                    </linearGradient>
                  </defs>

                  {ticks.map((tick) => (
                    <line
                      key={tick.y}
                      x1={paddingX}
                      y1={tick.y}
                      x2={chartWidth - paddingX}
                      y2={tick.y}
                      stroke="rgba(255,255,255,0.08)"
                      strokeDasharray="4 6"
                    />
                  ))}

                  <polygon points={outputArea} fill="url(#usage-output-fill)" />
                  <polygon points={inputArea} fill="url(#usage-input-fill)" />

                  <polyline
                    points={outputLine}
                    fill="none"
                    stroke="rgba(245,158,11,0.95)"
                    strokeWidth="3"
                    strokeLinecap="round"
                    strokeLinejoin="round"
                  />
                  <polyline
                    points={inputLine}
                    fill="none"
                    stroke="rgba(59,130,246,0.95)"
                    strokeWidth="3"
                    strokeLinecap="round"
                    strokeLinejoin="round"
                  />

                  {points.map((point, index) => {
                    const x = paddingX + index * step;
                    return (
                      <g key={point.day}>
                        <circle cx={x} cy={toY(point.total_input_tokens)} r="3.5" fill="rgba(59,130,246,1)" />
                        <circle cx={x} cy={toY(point.total_output_tokens)} r="3.5" fill="rgba(245,158,11,1)" />
                      </g>
                    );
                  })}
                </svg>

                <div className="mt-3 grid grid-cols-4 gap-2 text-[10px] font-medium text-muted-foreground/65">
                  {labelIndexes.map((index) => (
                    <div
                      key={`${points[index]?.day}-${index}`}
                      className={index === labelIndexes[labelIndexes.length - 1] ? 'text-right' : ''}
                    >
                      {points[index] ? formatDate(points[index].day) : ''}
                    </div>
                  ))}
                </div>
              </div>
            </div>
          </div>
        </div>

        <div className="grid gap-3 sm:grid-cols-3">
          <TrendStat
            label="Latest Input"
            value={formatCompact(points[points.length - 1]?.total_input_tokens ?? 0)}
            tone="primary"
          />
          <TrendStat
            label="Latest Output"
            value={formatCompact(points[points.length - 1]?.total_output_tokens ?? 0)}
            tone="warning"
          />
          <TrendStat
            label="Latest Requests"
            value={formatCompact(points[points.length - 1]?.request_count ?? 0)}
          />
        </div>
      </CardContent>
    </Card>
  );
}

function LegendChip({
  label,
  value,
  tone,
}: {
  label: string;
  value: string;
  tone: 'primary' | 'warning';
}) {
  const dotClass = tone === 'primary' ? 'bg-primary' : 'bg-amber-500';
  const textClass = tone === 'primary' ? 'text-primary/85' : 'text-amber-300';

  return (
    <div className="inline-flex items-center gap-2 rounded-full border border-white/10 bg-white/[0.04] px-3 py-1.5">
      <span className={`h-2 w-2 rounded-full ${dotClass}`} />
      <span className="uppercase tracking-[0.18em] text-muted-foreground/55">{label}</span>
      <span className={`font-mono font-semibold ${textClass}`}>{value}</span>
    </div>
  );
}

function TrendStat({
  label,
  value,
  tone = 'default',
}: {
  label: string;
  value: string;
  tone?: 'default' | 'primary' | 'warning';
}) {
  const valueClass =
    tone === 'primary' ? 'text-primary' : tone === 'warning' ? 'text-amber-400' : 'text-foreground';

  return (
    <div className="rounded-2xl border border-white/8 bg-white/[0.025] px-4 py-3">
      <div className="text-[10px] font-bold uppercase tracking-[0.2em] text-muted-foreground/55">{label}</div>
      <div className={`mt-2 font-mono text-lg font-semibold tabular-nums ${valueClass}`}>{value}</div>
    </div>
  );
}
