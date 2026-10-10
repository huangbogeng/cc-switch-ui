import { Flame } from 'lucide-react';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import type { ProxyUsageTrend } from '@/api';

interface Props {
  trend: ProxyUsageTrend[];
  loading: boolean;
  title?: string;
  /** Selected range start, unix seconds. Bounds the heatmap so inactive days still show. */
  rangeStart?: number;
  /** Selected range end, unix seconds. */
  rangeEnd?: number;
}

interface Cell {
  /** UTC midnight of the day, in epoch ms. */
  ms: number;
  day: string;
  count: number;
  tokens: number;
}

const CELL_TONES = [
  'bg-white/[0.05]',
  'bg-primary/25',
  'bg-primary/45',
  'bg-primary/70',
  'bg-primary',
];

const WEEKDAY_LABELS = ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat'];
const MONTH_LABELS = [
  'Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun',
  'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec',
];

function pad2(n: number): string {
  return String(n).padStart(2, '0');
}

// The backend groups requests by UTC day (`date(created_at, 'unixepoch')`), so
// all calendar math here is anchored to UTC to stay aligned with the data.
function parseDay(day: string): number | null {
  const m = /^(\d{4})-(\d{2})-(\d{2})$/.exec(day);
  if (!m) return null;
  return Date.UTC(Number(m[1]), Number(m[2]) - 1, Number(m[3]));
}

function dayKey(ms: number): string {
  const d = new Date(ms);
  return `${d.getUTCFullYear()}-${pad2(d.getUTCMonth() + 1)}-${pad2(d.getUTCDate())}`;
}

function startOfUtcDay(seconds: number): number {
  const d = new Date(seconds * 1000);
  return Date.UTC(d.getUTCFullYear(), d.getUTCMonth(), d.getUTCDate());
}

function intensity(value: number, max: number): number {
  if (value <= 0 || max <= 0) return 0;
  const ratio = value / max;
  if (ratio > 0.75) return 4;
  if (ratio > 0.5) return 3;
  if (ratio > 0.25) return 2;
  return 1;
}

function buildCells(trend: ProxyUsageTrend[], rangeStart?: number, rangeEnd?: number): Cell[] {
  const byDay = new Map<string, ProxyUsageTrend>();
  const dataDays: number[] = [];
  for (const item of trend) {
    const ms = parseDay(item.day);
    if (ms === null) continue;
    byDay.set(item.day, item);
    dataDays.push(ms);
  }

  let start: number;
  let end: number;
  if (rangeStart !== undefined && rangeEnd !== undefined) {
    start = startOfUtcDay(rangeStart);
    end = startOfUtcDay(rangeEnd);
    if (end < start) [start, end] = [end, start];
  } else if (dataDays.length > 0) {
    start = Math.min(...dataDays);
    end = Math.max(...dataDays);
  } else {
    return [];
  }

  const DAY_MS = 86_400_000;
  const cells: Cell[] = [];
  for (let ms = start; ms <= end; ms += DAY_MS) {
    const key = dayKey(ms);
    const item = byDay.get(key);
    cells.push({
      ms,
      day: key,
      count: item?.request_count ?? 0,
      tokens: (item?.total_input_tokens ?? 0) + (item?.total_output_tokens ?? 0),
    });
  }
  return cells;
}

export default function UsageHeatmap({ trend, loading, title = 'Activity', rangeStart, rangeEnd }: Props) {
  if (loading) {
    return (
      <Card>
        <CardHeader>
          <CardTitle className="text-[15px] font-semibold text-foreground">{title}</CardTitle>
        </CardHeader>
        <CardContent className="p-5 sm:p-6">
          <div className="h-[120px] animate-pulse rounded-2xl bg-white/5" />
        </CardContent>
      </Card>
    );
  }

  const cells = buildCells(trend, rangeStart, rangeEnd);

  if (cells.length === 0) {
    return (
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2 text-[15px] font-semibold text-foreground">
            <Flame className="h-4 w-4" />
            {title}
          </CardTitle>
        </CardHeader>
        <CardContent className="p-5 sm:p-6">
          <div className="flex h-[120px] items-center justify-center rounded-2xl border border-dashed border-white/10 bg-white/[0.02] text-sm text-muted-foreground">
            No activity yet. Sync session logs to build the heatmap.
          </div>
        </CardContent>
      </Card>
    );
  }

  const maxCount = cells.reduce((max, cell) => Math.max(max, cell.count), 0);
  const totalRequests = cells.reduce((sum, cell) => sum + cell.count, 0);
  const activeDays = cells.filter((cell) => cell.count > 0).length;

  // Lay the days out as GitHub-style week columns (7 rows each).
  const leading = new Date(cells[0].ms).getUTCDay();
  const grid: (Cell | null)[] = [...Array<Cell | null>(leading).fill(null), ...cells];
  while (grid.length % 7 !== 0) grid.push(null);

  const weeks: (Cell | null)[][] = [];
  for (let i = 0; i < grid.length; i += 7) weeks.push(grid.slice(i, i + 7));

  // Show a month label above the first week that enters a new month.
  let lastMonth = -1;
  const monthLabels = weeks.map((week) => {
    const first = week.find((cell): cell is Cell => cell !== null);
    if (!first) return '';
    const month = new Date(first.ms).getUTCMonth();
    if (month !== lastMonth) {
      lastMonth = month;
      return MONTH_LABELS[month];
    }
    return '';
  });

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex flex-wrap items-center justify-between gap-3">
          <div className="flex items-center gap-2 text-[15px] font-semibold text-foreground">
            <Flame className="h-4 w-4" />
            <span>{title}</span>
          </div>
          <div className="flex flex-wrap items-center gap-2 text-[11px] text-muted-foreground/70">
            <span className="font-mono tabular-nums">{totalRequests.toLocaleString()}</span>
            <span className="text-xs font-normal text-muted-foreground">requests</span>
            <span className="text-muted-foreground/25">·</span>
            <span className="font-mono tabular-nums">{activeDays}</span>
            <span className="text-xs font-normal text-muted-foreground">active days</span>
          </div>
        </CardTitle>
      </CardHeader>
      <CardContent className="p-5 sm:p-6">
        <div className="overflow-x-auto">
          <div className="inline-flex min-w-full flex-col gap-2">
            <div className="flex gap-[3px] pl-[26px]">
              {monthLabels.map((label, index) => (
                <div key={index} className="w-[13px] text-[9px] leading-none text-muted-foreground/50">
                  {label}
                </div>
              ))}
            </div>
            <div className="flex gap-1.5">
              <div className="flex flex-col gap-[3px] pr-1">
                {WEEKDAY_LABELS.map((label, index) => (
                  <div
                    key={label}
                    className="flex h-[13px] items-center text-[9px] leading-none text-muted-foreground/45"
                    style={{ width: 22, visibility: index % 2 === 1 ? 'visible' : 'hidden' }}
                  >
                    {label}
                  </div>
                ))}
              </div>
              <div className="flex gap-[3px]">
                {weeks.map((week, weekIndex) => (
                  <div key={weekIndex} className="flex flex-col gap-[3px]">
                    {week.map((cell, dayIndex) => {
                      if (!cell) {
                        return <div key={dayIndex} className="h-[13px] w-[13px]" />;
                      }
                      const level = intensity(cell.count, maxCount);
                      const dateLabel = new Date(cell.ms).toLocaleDateString(undefined, {
                        month: 'short',
                        day: 'numeric',
                        timeZone: 'UTC',
                      });
                      return (
                        <div
                          key={cell.day}
                          className={`h-[13px] w-[13px] rounded-[3px] ring-1 ring-inset ring-white/5 ${CELL_TONES[level]}`}
                          title={`${dateLabel} · ${cell.count.toLocaleString()} requests · ${cell.tokens.toLocaleString()} tokens`}
                        />
                      );
                    })}
                  </div>
                ))}
              </div>
            </div>
          </div>
        </div>

        <div className="mt-4 flex items-center justify-end gap-1.5 text-[10px] text-muted-foreground/55">
          <span>Less</span>
          {CELL_TONES.map((tone, index) => (
            <span key={index} className={`h-[11px] w-[11px] rounded-[3px] ring-1 ring-inset ring-white/5 ${tone}`} />
          ))}
          <span>More</span>
        </div>
      </CardContent>
    </Card>
  );
}
