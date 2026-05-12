import { TrendingUp } from 'lucide-react';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import type { ProxyUsageTrend } from '@/api';

function formatDate(dateStr: string): string {
  const d = new Date(dateStr);
  return d.toLocaleDateString(undefined, { month: 'short', day: 'numeric' });
}

interface Props {
  trend: ProxyUsageTrend[];
  loading: boolean;
}

export default function UsageTrendChart({ trend, loading }: Props) {
  if (loading) {
    return (
      <Card className="border-white/10 bg-card/60 backdrop-blur-sm shadow-xl">
        <CardHeader className="border-b border-white/5 bg-black/20 pb-4">
          <CardTitle className="text-sm font-semibold text-muted-foreground">30-Day Trend</CardTitle>
        </CardHeader>
        <CardContent className="p-6">
          <div className="h-48 animate-pulse rounded-xl bg-white/5" />
        </CardContent>
      </Card>
    );
  }

  if (trend.length === 0) {
    return (
      <Card className="border-white/10 bg-card/60 backdrop-blur-sm shadow-xl">
        <CardHeader className="border-b border-white/5 bg-black/20 pb-4">
          <CardTitle className="text-sm font-semibold text-muted-foreground flex items-center gap-2">
            <TrendingUp className="h-4 w-4" />
            30-Day Trend
          </CardTitle>
        </CardHeader>
        <CardContent className="p-6">
          <div className="flex h-48 items-center justify-center text-sm text-muted-foreground">
            No data yet. Sync session logs to see usage trends.
          </div>
        </CardContent>
      </Card>
    );
  }

  const max = Math.max(...trend.map((d) => d.total_input_tokens + d.total_output_tokens), 1);

  return (
    <Card className="border-white/10 bg-card/60 backdrop-blur-sm shadow-xl overflow-hidden">
      <CardHeader className="border-b border-white/5 bg-black/20 pb-4">
        <CardTitle className="text-sm font-semibold text-muted-foreground flex items-center gap-2">
          <TrendingUp className="h-4 w-4" />
          30-Day Trend
        </CardTitle>
      </CardHeader>
      <CardContent className="p-6">
        <div className="flex items-end gap-1.5 h-48 overflow-x-auto pb-2">
          {trend.slice().reverse().map((d) => {
            const inpPct = (d.total_input_tokens / max) * 100;
            const outPct = (d.total_output_tokens / max) * 100;
            return (
              <div key={d.day} className="flex flex-col items-center gap-1 flex-shrink-0 min-w-[32px] h-full">
                <div className="flex items-end gap-0.5 h-full w-full justify-center">
                  <div
                    className="w-3 rounded-t-sm bg-gradient-to-t from-primary/70 to-primary/30 transition-all"
                    style={{ height: `${Math.max(inpPct, 0.5)}%` }}
                    title={`Input: ${d.total_input_tokens.toLocaleString()}`}
                  />
                  <div
                    className="w-3 rounded-t-sm bg-gradient-to-t from-amber-500/70 to-amber-500/30 transition-all"
                    style={{ height: `${Math.max(outPct, 0.5)}%` }}
                    title={`Output: ${d.total_output_tokens.toLocaleString()}`}
                  />
                </div>
                <span className="text-[10px] text-muted-foreground font-medium whitespace-nowrap">
                  {formatDate(d.day)}
                </span>
              </div>
            );
          })}
        </div>
      </CardContent>
    </Card>
  );
}
