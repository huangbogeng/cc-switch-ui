import { ExternalLink, PencilLine, Trash2 } from 'lucide-react';
import type { Provider } from '@/api';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent } from '@/components/ui/card';
import {
  providerApiFormat,
  providerAuthLabel,
  providerAuthMode,
  providerBaseUrl,
  providerHost,
  providerInitial,
} from '@/lib/provider';

interface ProviderCardProps {
  provider: Provider;
  active: boolean;
  onSwitch: (id: string) => void;
  onEdit: (provider: Provider) => void;
  onDelete: (id: string) => void;
  busy?: boolean;
}

export function ProviderCard({
  provider,
  active,
  onSwitch,
  onEdit,
  onDelete,
  busy = false,
}: ProviderCardProps) {
  const authMode = providerAuthMode(provider);
  const apiFormat = providerApiFormat(provider);
  const baseUrl = providerBaseUrl(provider);

  return (
    <Card className={`group relative overflow-hidden transition-[background-color,border-color] duration-200 ${
      active
        ? 'border-primary/35 bg-card ring-1 ring-primary/10'
        : 'border-white/[0.07] bg-card hover:border-white/15'
    }`}>
      <CardContent className="p-6 relative">
        <div className="flex flex-col sm:flex-row gap-5 sm:items-start justify-between">
          <div className="flex gap-4 min-w-0 flex-1">
            <div className={`flex h-12 w-12 shrink-0 items-center justify-center rounded-xl transition-colors duration-150 ${
              active ? 'bg-primary/20 border border-primary/20 text-primary' : 'bg-white/5 border border-white/5 text-foreground/80 group-hover:bg-white/10'
            }`}>
              <span className="text-xl font-medium tracking-tight">{providerInitial(provider)}</span>
            </div>
            <div className="min-w-0 flex-1">
              <div className="flex items-center gap-2 mb-1.5">
                <h2 className="truncate text-lg font-semibold tracking-[-0.02em] text-foreground">{provider.name}</h2>
                {active && (
                  <Badge variant="success" className="gap-1.5 py-0.5 px-2.5 bg-emerald-500/15 text-emerald-400 border-emerald-500/20 shadow-[0_0_10px_rgba(16,185,129,0.15)]">
                    <span className="relative flex h-1.5 w-1.5">
                      <span className="absolute inline-flex h-full w-full rounded-full bg-emerald-400 opacity-20"></span>
                      <span className="relative inline-flex rounded-full h-1.5 w-1.5 bg-emerald-500"></span>
                    </span>
                    Selected
                  </Badge>
                )}
              </div>

              <div className="flex flex-wrap gap-2 mb-3">
                <Badge variant={authMode === 'oauth_proxy' ? 'success' : 'outline'} className={`text-xs font-medium ${
                  authMode === 'oauth_proxy'
                    ? 'bg-emerald-500/10 text-emerald-400 border-emerald-500/20'
                    : 'bg-white/5 text-muted-foreground border-white/10'
                }`}>
                  {providerAuthLabel(provider)}
                </Badge>
                <Badge variant="outline" className="bg-white/5 text-muted-foreground border-white/10">
                  {formatApiFormatLabel(apiFormat)}
                </Badge>
              </div>

              <div className="space-y-1.5">
                {baseUrl && (
                  <div className="truncate font-mono text-xs text-muted-foreground bg-white/[0.03] w-fit px-2 py-1 rounded-md border border-white/10">
                    {providerHost(baseUrl)}
                  </div>
                )}
                {provider.websiteUrl && (
                  <a
                    href={provider.websiteUrl}
                    target="_blank"
                    rel="noopener noreferrer"
                    className="inline-flex items-center gap-1.5 text-[12px] font-medium text-primary/80 hover:text-primary transition-colors hover:underline"
                  >
                    <span className="truncate max-w-[200px]">{provider.websiteUrl}</span>
                    <ExternalLink className="h-3 w-3 shrink-0" />
                  </a>
                )}
              </div>

              {provider.notes && (
                <p className="mt-3 line-clamp-2 text-sm leading-relaxed text-muted-foreground">
                  {provider.notes}
                </p>
              )}
            </div>
          </div>

          <div className="flex sm:flex-col items-center sm:items-end justify-end gap-2 pt-2 sm:pt-0 border-t border-white/5 sm:border-0">
            <Button
              size="sm"
              variant={active ? 'secondary' : 'default'}
              className={`w-full sm:w-auto shadow-sm transition-colors ${
                !active ? 'bg-primary hover:bg-primary/90 text-primary-foreground' : ''
              }`}
              onClick={() => onSwitch(provider.id)}
              disabled={active || busy}
            >
              {active ? 'Selected' : 'Select'}
            </Button>
            <div className="flex items-center gap-1">
              <Button aria-label={`Edit ${provider.name}`} size="icon" variant="ghost" className="h-8 w-8 rounded-lg hover:bg-white/10 text-muted-foreground hover:text-foreground transition-colors" onClick={() => onEdit(provider)} disabled={busy}>
                <PencilLine className="h-3.5 w-3.5" />
              </Button>
              <Button
                size="icon"
                variant="ghost"
                className="h-8 w-8 rounded-lg text-destructive/80 hover:text-destructive hover:bg-destructive/10 transition-colors"
                onClick={() => onDelete(provider.id)}
                aria-label={`Delete ${provider.name}`}
                disabled={busy}
              >
                <Trash2 className="h-3.5 w-3.5" />
              </Button>
            </div>
          </div>
        </div>
      </CardContent>
    </Card>
  );
}

function formatApiFormatLabel(apiFormat: string) {
  switch (apiFormat) {
    case 'openai_chat':
      return 'OpenAI Chat';
    case 'openai_responses':
      return 'OpenAI Responses';
    case 'gemini_native':
      return 'Gemini Native';
    default:
      return 'Anthropic';
  }
}
