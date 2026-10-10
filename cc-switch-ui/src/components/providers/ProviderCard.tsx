import { ProviderLogo } from './ProviderLogo';
import { resolveProviderLogo } from '@/lib/providerLogo';
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
} from '@/lib/provider';

interface ProviderCardProps {
  provider: Provider;
  active: boolean;
  onSwitch: (id: string) => void;
  onEdit: (provider: Provider) => void;
  onDelete: (id: string) => void;
  busy?: boolean;
  embedded?: boolean;
}

export function ProviderCard({
  provider,
  active,
  onSwitch,
  onEdit,
  onDelete,
  busy = false,
  embedded = false,
}: ProviderCardProps) {
  const authMode = providerAuthMode(provider);
  const apiFormat = providerApiFormat(provider);
  const baseUrl = providerBaseUrl(provider);

  return (
    <Card className={`min-w-0 transition-colors duration-150 ${embedded ? `rounded-none border-0 shadow-none ${active ? 'bg-primary/[0.06]' : 'hover:bg-white/[0.02]'}` : active ? 'border-primary/30 bg-primary/[0.04]' : 'hover:border-white/15'}`}>
      <CardContent className="p-5 sm:px-6">
        <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
          <div className="flex min-w-0 flex-1 items-start gap-3">
            <ProviderLogo icon={resolveProviderLogo(provider)} />
            <div className="min-w-0 flex-1">
              <h2 title={provider.name} className="truncate text-[15px] font-semibold leading-6">{provider.name}</h2>
              <div className="flex min-w-0 flex-wrap items-center gap-x-2 gap-y-1 text-xs text-muted-foreground">
                {baseUrl && <span title={baseUrl} className="max-w-full truncate">{providerHost(baseUrl)}</span>}
                <span aria-hidden="true" className="text-white/20">·</span>
                <span>{formatApiFormatLabel(apiFormat)}</span>
                <Badge variant="outline" className={`px-1.5 py-0 text-[11px] ${authMode === 'oauth_proxy' ? 'border-emerald-500/20 text-emerald-400' : 'text-muted-foreground'}`}>
                  {providerAuthLabel(provider)}
                </Badge>
                {provider.websiteUrl && <a href={provider.websiteUrl} target="_blank" rel="noopener noreferrer" aria-label={`Website for ${provider.name}`} className="inline-flex items-center gap-1 text-muted-foreground hover:text-primary">
                  Website <ExternalLink className="h-3 w-3" />
                </a>}
              </div>
              {provider.notes && <p className="mt-1 line-clamp-2 text-xs leading-5 text-muted-foreground">{provider.notes}</p>}
            </div>
          </div>
          <div className="flex shrink-0 items-center justify-end gap-1 border-t border-white/5 pt-2 sm:border-0 sm:pt-0">
            <Button size="sm" variant={active ? 'secondary' : 'outline'} className="mr-1 min-w-[78px]" onClick={() => onSwitch(provider.id)} disabled={active || busy}>
              {active ? 'Selected' : 'Select'}
            </Button>
            <Button aria-label={`Edit ${provider.name}`} size="icon" variant="ghost" className="h-9 w-9 rounded-lg" onClick={() => onEdit(provider)} disabled={busy}>
              <PencilLine className="h-4 w-4" />
            </Button>
            <Button aria-label={`Delete ${provider.name}`} size="icon" variant="ghost" className="h-9 w-9 rounded-lg hover:bg-destructive/10 hover:text-destructive" onClick={() => onDelete(provider.id)} disabled={busy}>
              <Trash2 className="h-4 w-4" />
            </Button>
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
