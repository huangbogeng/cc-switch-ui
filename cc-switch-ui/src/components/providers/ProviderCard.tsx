import { ExternalLink, Pencil, Trash2 } from 'lucide-react';
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
    <Card className={`group relative overflow-hidden transition-all duration-300 ${
      active 
        ? 'ring-2 ring-primary/50 bg-gradient-to-br from-primary/5 to-transparent shadow-[0_0_20px_rgba(var(--primary),0.1)]' 
        : 'border-white/5 bg-card/40 hover:bg-card/80 hover:border-white/10 hover:shadow-lg'
    }`}>
      {active && (
        <div className="absolute inset-0 bg-gradient-to-br from-primary/10 via-transparent to-transparent pointer-events-none" />
      )}
      <CardContent className="p-5 relative">
        <div className="flex flex-col sm:flex-row gap-5 sm:items-start justify-between">
          <div className="flex gap-4 min-w-0 flex-1">
            <div className={`flex h-14 w-14 shrink-0 items-center justify-center rounded-2xl shadow-inner transition-colors duration-300 ${
              active ? 'bg-primary/20 border border-primary/20 text-primary' : 'bg-white/5 border border-white/5 text-foreground/80 group-hover:bg-white/10'
            }`}>
              <span className="text-2xl font-bold tracking-tight drop-shadow-sm">{providerInitial(provider)}</span>
            </div>
            <div className="min-w-0 flex-1">
              <div className="flex items-center gap-2 mb-1.5">
                <h2 className="truncate text-lg font-bold tracking-tight text-foreground">{provider.name}</h2>
                {active && (
                  <Badge variant="success" className="gap-1.5 py-0.5 px-2.5 bg-emerald-500/15 text-emerald-400 border-emerald-500/20 shadow-[0_0_10px_rgba(16,185,129,0.15)]">
                    <span className="relative flex h-1.5 w-1.5">
                      <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-emerald-400 opacity-75"></span>
                      <span className="relative inline-flex rounded-full h-1.5 w-1.5 bg-emerald-500"></span>
                    </span>
                    Active
                  </Badge>
                )}
              </div>
              
              <div className="flex flex-wrap gap-2 mb-3">
                <Badge variant={authMode === 'oauth_proxy' ? 'success' : 'outline'} className={`text-[10px] uppercase tracking-wider font-bold ${
                  authMode === 'oauth_proxy' 
                    ? 'bg-emerald-500/10 text-emerald-400 border-emerald-500/20' 
                    : 'bg-white/5 text-muted-foreground border-white/10'
                }`}>
                  {providerAuthLabel(provider)}
                </Badge>
                <Badge variant="outline" className="text-[10px] uppercase tracking-wider font-bold bg-white/5 text-muted-foreground border-white/10">
                  {apiFormat}
                </Badge>
              </div>

              <div className="space-y-1.5">
                {baseUrl && (
                  <div className="truncate font-mono text-[11px] text-muted-foreground/80 bg-black/20 w-fit px-2 py-1 rounded-md border border-white/5">
                    {baseUrl}
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
                <p className="mt-4 line-clamp-2 text-[13px] leading-relaxed text-muted-foreground/90 bg-white/[0.02] p-2.5 rounded-lg border border-white/5">
                  {provider.notes}
                </p>
              )}
            </div>
          </div>

          <div className="flex sm:flex-col items-center sm:items-end justify-end gap-2 pt-2 sm:pt-0 border-t border-white/5 sm:border-0">
            <Button 
              size="sm" 
              variant={active ? 'secondary' : 'default'} 
              className={`w-full sm:w-auto shadow-sm transition-all ${
                !active ? 'bg-primary hover:bg-primary/90 text-primary-foreground shadow-[0_0_15px_rgba(var(--primary),0.2)] hover:shadow-[0_0_20px_rgba(var(--primary),0.3)]' : ''
              }`}
              onClick={() => onSwitch(provider.id)}
            >
              {active ? 'Selected' : 'Switch to this'}
            </Button>
            <div className="flex items-center gap-1">
              <Button size="icon" variant="ghost" className="h-8 w-8 rounded-lg hover:bg-white/10 text-muted-foreground hover:text-foreground transition-colors" onClick={() => onEdit(provider)}>
                <Pencil className="h-3.5 w-3.5" />
              </Button>
              <Button
                size="icon"
                variant="ghost"
                className="h-8 w-8 rounded-lg text-destructive/80 hover:text-destructive hover:bg-destructive/10 transition-colors"
                onClick={() => onDelete(provider.id)}
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
