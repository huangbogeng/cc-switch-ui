import { Server, Zap } from 'lucide-react';
import type { Provider } from '@/api';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent } from '@/components/ui/card';
import { Select } from '@/components/ui/select';
import { Label } from '@/components/ui/label';
import { providerApiFormat, providerBaseUrl, providerHost } from '@/lib/provider';

interface LocalRoutePanelProps {
  providers: Provider[];
  currentProviderId: string | null;
  proxyRunning: boolean;
  proxyTargetId: string | null;
  busy: boolean;
  onTargetChange: (providerId: string) => void;
  onStartProxy: (providerId: string) => void;
  onStopProxy: () => void;
}

export function LocalRoutePanel({
  providers,
  currentProviderId,
  proxyRunning,
  proxyTargetId,
  busy,
  onTargetChange,
  onStartProxy,
  onStopProxy,
}: LocalRoutePanelProps) {
  const currentProvider = providers.find((provider) => provider.id === currentProviderId) ?? null;
  const targetProvider = providers.find((provider) => provider.id === proxyTargetId) ?? null;
  const targetMatchesSelection =
    !proxyRunning || !currentProvider || !targetProvider || currentProvider.id === targetProvider.id;

  return (
    <Card className="mb-6 overflow-hidden border-white/10 bg-gradient-to-br from-amber-500/8 via-transparent to-sky-500/8">
      <CardContent className="p-5">
        <div className="flex flex-col gap-5 lg:flex-row lg:items-start lg:justify-between">
          <div className="min-w-0 flex-1 space-y-4">
            <div className="flex flex-wrap items-center gap-2">
              <Badge className="gap-1.5 border-amber-500/20 bg-amber-500/15 text-amber-300">
                <Server className="h-3 w-3" />
                Local Route
              </Badge>
              <Badge variant="outline" className={proxyRunning ? 'border-emerald-500/20 bg-emerald-500/15 text-emerald-300' : 'border-white/10 bg-white/5 text-muted-foreground'}>
                {proxyRunning ? 'Running' : 'Stopped'}
              </Badge>
            </div>

            <div className="grid gap-3 lg:grid-cols-2">
              <RouteSummary
                label="Direct Config"
                providerName={currentProvider?.name ?? 'None'}
                hint={
                  currentProvider
                    ? `${currentProvider.id} · ${formatApiFormatLabel(providerApiFormat(currentProvider))}`
                    : 'Used when route is stopped'
                }
                meta={currentProvider ? providerHost(providerBaseUrl(currentProvider)) : null}
                tone="neutral"
              />
              <RouteSummary
                label="Local Route Takeover"
                providerName={targetProvider?.name ?? 'None'}
                hint={
                  targetProvider
                    ? `${targetProvider.id} · ${formatApiFormatLabel(providerApiFormat(targetProvider))}`
                    : 'Uses the selected provider after startup'
                }
                meta={
                  proxyRunning
                    ? 'Live traffic is forwarded here'
                    : 'Dormant until route is started'
                }
                tone={proxyRunning ? 'warning-active' : 'warning'}
              />
            </div>

            <div className="rounded-xl border border-amber-500/15 bg-amber-500/8 px-3 py-2 text-xs text-amber-100/85">
              Selecting a provider sets direct config. Starting the route takes over live config and forwards traffic to the selected provider.
            </div>
          </div>

          <div className="w-full lg:max-w-[360px]">
            <div className="rounded-2xl border border-white/10 bg-black/20 p-4 shadow-inner shadow-black/20">
              <div className="flex items-center justify-between gap-3">
                <div className="text-[10px] uppercase tracking-[0.22em] text-slate-300/55">Route Control</div>
                <Badge
                  variant="outline"
                  className={
                    proxyRunning
                      ? 'border-emerald-500/20 bg-emerald-500/15 text-emerald-300'
                      : 'border-white/10 bg-white/5 text-muted-foreground'
                  }
                >
                  {proxyRunning ? 'Takeover Active' : 'Standby'}
                </Badge>
              </div>

              <div className="mt-4 space-y-2">
                <div className="text-base font-semibold text-white">
                  {proxyRunning ? 'Local route is currently controlling live traffic.' : 'Start local route takeover for the selected provider.'}
                </div>
                <div className="text-sm leading-6 text-slate-300/75">
                  {currentProvider
                    ? `Selected provider: ${currentProvider.name}`
                    : 'Select a provider first. Starting the route will use that provider as the takeover target.'}
                </div>
              </div>

              <div className="mt-4 space-y-2">
                <Label htmlFor="route-target">Route Target</Label>
                <Select
                  id="route-target"
                  value={proxyTargetId ?? ''}
                  onChange={(event) => onTargetChange(event.target.value)}
                  disabled={busy}
                >
                  <option value="" disabled>Select a provider</option>
                  {providers.map((provider) => (
                    <option key={provider.id} value={provider.id}>{provider.name}</option>
                  ))}
                </Select>
                <p className="text-xs text-slate-300/65">
                  This target is independent from the direct Provider selection.
                </p>
              </div>

              <div className="mt-4">
                {proxyRunning ? (
                  <Button variant="destructive" className="w-full" onClick={onStopProxy} disabled={busy}>
                    {busy ? 'Updating...' : 'Stop Route'}
                  </Button>
                ) : (
                  <Button
                    className="w-full bg-amber-600 text-white hover:bg-amber-700"
                    onClick={() => proxyTargetId && onStartProxy(proxyTargetId)}
                    disabled={!proxyTargetId || busy}
                  >
                    <Zap className="mr-2 h-4 w-4" />
                    {busy ? 'Starting...' : 'Start Route'}
                  </Button>
                )}
              </div>

              {proxyRunning && !targetMatchesSelection && currentProvider && targetProvider && (
                <div className="mt-4 rounded-xl border border-white/10 bg-white/[0.04] px-3 py-3 text-sm text-slate-200">
                  <div className="text-[10px] uppercase tracking-[0.18em] text-slate-300/55">Takeover Target</div>
                  <div className="mt-1 font-medium">
                    Route is still forwarding to <span className="text-amber-300">{targetProvider.name}</span>.
                  </div>
                  <div className="mt-1 text-xs text-slate-300/70">
                    Select a provider and confirm if you also want to switch the active route target.
                  </div>
                </div>
              )}

              {!proxyRunning && targetProvider && (
                <div className="mt-4 rounded-xl border border-amber-500/15 bg-amber-500/8 px-3 py-2 text-xs text-amber-100/85">
                  Starting the route will use <span className="font-semibold">{targetProvider.name}</span> as the takeover target.
                </div>
              )}
            </div>
          </div>
        </div>
      </CardContent>
    </Card>
  );
}

function RouteSummary({
  label,
  providerName,
  hint,
  meta,
  tone,
}: {
  label: string;
  providerName: string;
  hint: string;
  meta?: string | null;
  tone: 'neutral' | 'warning' | 'warning-active';
}) {
  const toneClass =
    tone === 'warning-active'
      ? 'border-amber-400/30 bg-gradient-to-br from-amber-500/18 to-amber-500/6 shadow-[0_0_24px_rgba(245,158,11,0.16)]'
      : tone === 'warning'
        ? 'border-amber-500/20 bg-amber-500/8'
        : 'border-slate-400/15 bg-slate-500/8';
  const labelClass =
    tone === 'neutral' ? 'text-slate-300/75' : 'text-amber-200/80';
  const providerClass =
    tone === 'neutral' ? 'text-slate-50' : 'text-amber-50';
  const textClass =
    tone === 'neutral' ? 'text-slate-300/70' : 'text-amber-100/75';

  return (
    <div className={`flex min-h-[128px] flex-col justify-between rounded-2xl border px-4 py-4 ${toneClass}`}>
      <div className="space-y-2">
        <div className={`text-[10px] font-bold uppercase tracking-[0.2em] ${labelClass}`}>{label}</div>
        <div className={`break-words text-base font-semibold leading-6 ${providerClass}`}>{providerName}</div>
      </div>
      <div className="space-y-1.5">
        <div className={`text-xs leading-5 ${textClass}`}>{hint}</div>
        {meta ? <div className={`font-mono text-[11px] leading-5 ${textClass}`}>{meta}</div> : null}
      </div>
    </div>
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
