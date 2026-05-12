import { useState } from 'react';
import { Calendar, ChevronDown } from 'lucide-react';
import type { UsageRangePreset, UsageRangeSelection } from '@/lib/usageRange';
import { resolveUsageRange, formatRangeLabel } from '@/lib/usageRange';

const PRESETS: { value: UsageRangePreset; label: string }[] = [
  { value: 'today', label: 'Today' },
  { value: '1d', label: '24h' },
  { value: '7d', label: '7d' },
  { value: '14d', label: '14d' },
  { value: '30d', label: '30d' },
];

interface Props {
  selection: UsageRangeSelection;
  onChange: (sel: UsageRangeSelection) => void;
}

export default function UsageDateRangePicker({ selection, onChange }: Props) {
  const [open, setOpen] = useState(false);

  const resolved = resolveUsageRange(selection);
  const label = formatRangeLabel(selection, resolved);

  return (
    <div className="relative">
      <button
        type="button"
        onClick={() => setOpen(!open)}
        className="flex items-center gap-2 rounded-xl border border-white/10 bg-black/20 px-3 py-1.5 text-sm text-muted-foreground hover:text-foreground transition-colors"
      >
        <Calendar className="h-3.5 w-3.5" />
        {label}
        <ChevronDown className="h-3 w-3" />
      </button>

      {open && (
        <>
          <div className="fixed inset-0 z-40" onClick={() => setOpen(false)} />
          <div className="absolute right-0 top-full mt-1 z-50 min-w-[240px] rounded-xl border border-white/10 bg-card p-2 shadow-2xl backdrop-blur-xl">
            <div className="flex flex-col gap-0.5">
              {PRESETS.map((p) => (
                <button
                  key={p.value}
                  type="button"
                  onClick={() => {
                    onChange({ preset: p.value });
                    setOpen(false);
                  }}
                  className={`rounded-lg px-3 py-1.5 text-sm text-left transition-colors ${
                    selection.preset === p.value
                      ? 'bg-primary/15 text-primary font-semibold'
                      : 'text-muted-foreground hover:bg-white/5 hover:text-foreground'
                  }`}
                >
                  {p.label}
                </button>
              ))}
            </div>

            <div className="mt-2 border-t border-white/5 pt-2">
              <button
                type="button"
                onClick={() => {
                  onChange({ preset: 'custom' });
                  setOpen(false);
                }}
                className={`w-full rounded-lg px-3 py-1.5 text-sm text-left transition-colors ${
                  selection.preset === 'custom'
                    ? 'bg-primary/15 text-primary font-semibold'
                    : 'text-muted-foreground hover:bg-white/5 hover:text-foreground'
                }`}
              >
                Custom Range
              </button>
            </div>
          </div>
        </>
      )}
    </div>
  );
}
