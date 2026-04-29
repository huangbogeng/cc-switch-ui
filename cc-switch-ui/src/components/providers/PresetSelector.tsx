import { KeyRound } from 'lucide-react';
import { type ProviderPreset, providerPresets } from '../../config/providerPresets';
import { Badge } from '@/components/ui/badge';
import { cn } from '@/lib/utils';

interface Props {
  onSelect: (preset: ProviderPreset) => void;
  selectedId?: string;
}

export function PresetSelector({ onSelect, selectedId }: Props) {
  return (
    <div className="grid gap-3 sm:grid-cols-2">
      {providerPresets.map((preset) => {
        const selected = selectedId === preset.id;
        const usesOAuthProxy = preset.authMode === 'oauth_proxy';

        return (
          <button
            key={preset.id}
            type="button"
            onClick={() => onSelect(preset)}
            className={cn(
              "min-w-0 rounded-2xl border p-4 text-left transition",
              selected
                ? "border-primary/70 bg-primary/10 shadow-inner shadow-primary/10"
                : "border-white/10 bg-white/[0.035] hover:border-white/20 hover:bg-white/[0.06]"
            )}
          >
            <div className="grid grid-cols-[40px_minmax(0,1fr)] items-start gap-3">
              <span
                className="flex h-10 w-10 shrink-0 items-center justify-center rounded-xl text-sm font-semibold text-white shadow-lg"
                style={{ background: preset.iconColor }}
              >
                {preset.name[0]}
              </span>
              <span className="min-w-0">
                <span className="grid grid-cols-[minmax(0,1fr)_auto] items-center gap-2">
                  <span className="truncate text-sm font-semibold leading-5 text-foreground">{preset.name}</span>
                  {usesOAuthProxy && (
                    <Badge variant="outline" className="shrink-0 gap-1 text-[10px] leading-4">
                      <KeyRound className="h-3 w-3" />
                      OAuth Proxy
                    </Badge>
                  )}
                </span>
                {preset.description && (
                  <span className="mt-1 block text-xs leading-5 text-muted-foreground">
                    {preset.description}
                  </span>
                )}
              </span>
            </div>
          </button>
        );
      })}
    </div>
  );
}
