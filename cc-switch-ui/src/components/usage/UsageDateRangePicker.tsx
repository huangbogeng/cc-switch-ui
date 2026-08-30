import { useEffect, useState } from 'react';
import { Calendar, ChevronDown } from 'lucide-react';
import type { UsageRangePreset, UsageRangeSelection } from '@/lib/usageRange';
import {
  dateInputToTimestamp,
  formatRangeLabel,
  resolveUsageRange,
  toDateInputValue,
} from '@/lib/usageRange';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';

const PRESETS: { value: Exclude<UsageRangePreset, 'custom'>; label: string }[] = [
  { value: 'today', label: 'Today' },
  { value: '1d', label: '24h' },
  { value: '7d', label: '7d' },
  { value: '14d', label: '14d' },
  { value: '30d', label: '30d' },
];

interface Props {
  selection: UsageRangeSelection;
  onChange: (selection: UsageRangeSelection) => void;
}

export default function UsageDateRangePicker({ selection, onChange }: Props) {
  const [open, setOpen] = useState(false);
  const resolved = resolveUsageRange(selection);
  const [customStart, setCustomStart] = useState(toDateInputValue(resolved.startDate));
  const [customEnd, setCustomEnd] = useState(toDateInputValue(resolved.endDate));
  const [customError, setCustomError] = useState('');

  useEffect(() => {
    if (selection.preset !== 'custom') return;
    setCustomStart(toDateInputValue(resolved.startDate));
    setCustomEnd(toDateInputValue(resolved.endDate));
  }, [selection.preset, resolved.startDate, resolved.endDate]);

  const applyCustomRange = () => {
    const start = dateInputToTimestamp(customStart);
    const end = dateInputToTimestamp(customEnd, true);
    if (start === null || end === null) {
      setCustomError('Choose both a start and end date.');
      return;
    }
    if (start > end) {
      setCustomError('Start date must be before the end date.');
      return;
    }
    onChange({ preset: 'custom', customStartDate: start, customEndDate: end });
    setCustomError('');
    setOpen(false);
  };

  return (
    <div className="relative">
      <button
        type="button"
        aria-haspopup="dialog"
        aria-expanded={open}
        onClick={() => setOpen((value) => !value)}
        className="flex items-center gap-2 rounded-xl border border-white/10 bg-black/20 px-3 py-1.5 text-sm text-muted-foreground transition-colors hover:text-foreground"
      >
        <Calendar className="h-3.5 w-3.5" />
        {formatRangeLabel(selection, resolved)}
        <ChevronDown className="h-3 w-3" />
      </button>

      {open && (
        <>
          <button
            type="button"
            aria-label="Close date range picker"
            className="fixed inset-0 z-40 cursor-default"
            onClick={() => setOpen(false)}
          />
          <div
            role="dialog"
            aria-label="Usage date range"
            className="absolute right-0 top-full z-50 mt-1 min-w-[280px] rounded-xl border border-white/10 bg-card p-3 shadow-2xl"
          >
            <div className="grid grid-cols-5 gap-1">
              {PRESETS.map((preset) => (
                <button
                  key={preset.value}
                  type="button"
                  onClick={() => {
                    onChange({ preset: preset.value });
                    setOpen(false);
                  }}
                  className={`rounded-lg px-2 py-1.5 text-xs transition-colors ${
                    selection.preset === preset.value
                      ? 'bg-primary/15 font-semibold text-primary'
                      : 'text-muted-foreground hover:bg-white/5 hover:text-foreground'
                  }`}
                >
                  {preset.label}
                </button>
              ))}
            </div>

            <div className="mt-3 space-y-3 border-t border-white/10 pt-3">
              <div className="grid grid-cols-2 gap-2">
                <div className="space-y-1">
                  <Label htmlFor="usage-start-date">Start</Label>
                  <Input
                    id="usage-start-date"
                    type="date"
                    value={customStart}
                    onChange={(event) => setCustomStart(event.target.value)}
                  />
                </div>
                <div className="space-y-1">
                  <Label htmlFor="usage-end-date">End</Label>
                  <Input
                    id="usage-end-date"
                    type="date"
                    value={customEnd}
                    onChange={(event) => setCustomEnd(event.target.value)}
                  />
                </div>
              </div>
              {customError && <p role="alert" className="text-xs text-destructive">{customError}</p>}
              <Button type="button" size="sm" className="w-full" onClick={applyCustomRange}>
                Apply custom range
              </Button>
            </div>
          </div>
        </>
      )}
    </div>
  );
}
