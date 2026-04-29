import { CheckCircle2, ExternalLink, Pencil, Trash2 } from 'lucide-react';
import type { Provider } from '@/api';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent } from '@/components/ui/card';
import { providerApiFormat, providerAuthLabel, providerAuthMode, providerBaseUrl, providerInitial } from '@/lib/provider';

interface ProviderCardProps {
  provider: Provider;
  active: boolean;
  onSwitch: (id: string) => void;
  onEdit: (provider: Provider) => void;
  onDelete: (id: string) => void;
}

export function ProviderCard({ provider, active, onSwitch, onEdit, onDelete }: ProviderCardProps) {
  const authMode = providerAuthMode(provider);
  const apiFormat = providerApiFormat(provider);
  const baseUrl = providerBaseUrl(provider);

  return (
    <Card className={active ? 'ring-1 ring-primary/45' : undefined}>
      <CardContent className="p-4">
        <div className="grid gap-4 sm:grid-cols-[minmax(0,1fr)_auto] sm:items-start">
          <div className="grid min-w-0 grid-cols-[44px_minmax(0,1fr)] gap-3">
            <div className="flex h-11 w-11 shrink-0 items-center justify-center rounded-2xl bg-primary/15 text-base font-semibold text-primary">
              {providerInitial(provider)}
            </div>
            <div className="min-w-0">
              <div className="grid grid-cols-[minmax(0,1fr)_auto] items-center gap-2">
                <h2 className="truncate text-sm font-semibold leading-5 text-foreground">{provider.name}</h2>
                {active && (
                  <Badge variant="success" className="gap-1 leading-4">
                    <CheckCircle2 className="h-3 w-3" />
                    Active
                  </Badge>
                )}
              </div>
              <div className="mt-2 flex flex-wrap gap-2">
                <Badge variant={authMode === 'oauth_proxy' ? 'success' : 'outline'} className="leading-4">
                  {providerAuthLabel(provider)}
                </Badge>
                <Badge variant="outline" className="leading-4">
                  {apiFormat}
                </Badge>
              </div>
              {baseUrl && (
                <div className="mt-2 truncate font-mono text-xs leading-4 text-muted-foreground">
                  {baseUrl}
                </div>
              )}
              {provider.websiteUrl && (
                <a
                  href={provider.websiteUrl}
                  target="_blank"
                  rel="noopener noreferrer"
                  className="mt-1 grid min-w-0 grid-cols-[minmax(0,1fr)_12px] items-center gap-1 text-xs leading-4 text-primary hover:underline"
                >
                  <span className="truncate">{provider.websiteUrl}</span>
                  <ExternalLink className="h-3 w-3 shrink-0" />
                </a>
              )}
              {provider.notes && (
                <p className="mt-2 line-clamp-2 text-sm leading-5 text-muted-foreground">{provider.notes}</p>
              )}
            </div>
          </div>
          <div className="grid grid-cols-[auto_32px_32px] items-center justify-start gap-1 sm:justify-end">
            <Button size="sm" variant={active ? 'secondary' : 'outline'} onClick={() => onSwitch(provider.id)}>
              Switch
            </Button>
            <Button size="icon" variant="ghost" className="h-8 w-8" onClick={() => onEdit(provider)}>
              <Pencil className="h-4 w-4" />
            </Button>
            <Button
              size="icon"
              variant="ghost"
              className="h-8 w-8 text-destructive hover:text-destructive"
              onClick={() => onDelete(provider.id)}
            >
              <Trash2 className="h-4 w-4" />
            </Button>
          </div>
        </div>
      </CardContent>
    </Card>
  );
}
