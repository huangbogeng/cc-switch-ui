import { useEffect, useRef, useState } from 'react';
import { Check, CheckCircle2, ChevronDown, Circle, Copy, ExternalLink, Globe, Loader2, Server, Zap } from 'lucide-react';
import type { CodexAccount, CopilotAccount, CopilotUsageResponse, Provider } from '@/api';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Progress } from '@/components/ui/progress';
import { providerAuthLabel, providerAuthMode, providerInitial } from '@/lib/provider';
import { usagePercent } from '@/lib/usage';

interface CopilotStatus {
  authenticated: boolean;
  accounts: CopilotAccount[];
  default_account_id: string | null;
}

interface CodexStatus {
  authenticated: boolean;
  accounts: CodexAccount[];
}

interface ProxyStatus {
  running: boolean;
  listen_addr: string | null;
  upstream_url: string;
  http_proxy_url: string | null;
  active_target_provider_id: string | null;
  active_target_provider_name: string | null;
}

export function CurrentProviderCard({
  loading,
  provider,
  usage24h,
  routeRuntime,
  routeError,
}: {
  loading: boolean;
  provider: Provider | null;
  usage24h: {
    requestCount: number;
    inputTokens: number;
    outputTokens: number;
  } | null;
  routeRuntime: {
    running: boolean;
    listenAddr: string | null;
    activeTargetProviderId: string | null;
  } | null;
  routeError?: string;
}) {
  const formatCompact = (value: number) => {
    if (value >= 1_000_000) return `${(value / 1_000_000).toFixed(1)}M`;
    if (value >= 1_000) return `${(value / 1_000).toFixed(1)}K`;
    return value.toString();
  };

  const isCurrentTarget = !!provider && routeRuntime?.activeTargetProviderId === provider.id;
  const authMode = provider ? providerAuthMode(provider) : 'api_key';
  const avatarStyle = provider?.iconColor
    ? { backgroundColor: provider.iconColor, borderColor: `${provider.iconColor}66` }
    : undefined;

  return (
    <Card className="overflow-hidden relative group border-white/10 bg-gradient-to-br from-card to-card/50 shadow-xl">
      <div className="absolute inset-0 bg-gradient-to-br from-primary/10 via-transparent to-transparent opacity-0 transition-opacity duration-500 group-hover:opacity-100 pointer-events-none" />
      <CardHeader className="border-b border-white/5 bg-black/20 pb-4">
        <CardTitle className="flex items-center gap-2.5 text-sm font-semibold text-muted-foreground tracking-tight">
          <div className="rounded-md bg-white/5 p-1.5 shadow-inner">
            <Globe className="h-4 w-4 text-primary" />
          </div>
          Current Provider
        </CardTitle>
      </CardHeader>
      <CardContent className="p-6">
        {loading ? (
          <div className="flex h-24 items-center justify-center">
            <Loader2 className="h-6 w-6 animate-spin text-primary/50" />
          </div>
        ) : provider ? (
          <div className="space-y-5">
            <div className="flex flex-col sm:flex-row sm:items-start justify-between gap-6">
              <div className="flex items-start gap-4 min-w-0">
                <div
                  className="flex h-14 w-14 shrink-0 items-center justify-center rounded-2xl shadow-inner text-white"
                  style={avatarStyle}
                >
                  <span className="text-2xl font-bold tracking-tight drop-shadow-sm">{providerInitial(provider)}</span>
                </div>
                <div className="min-w-0 flex-1">
                  <div className="flex items-center gap-2 mb-1.5">
                    <div className="truncate text-xl font-bold tracking-tight text-foreground">{provider.name}</div>
                    <Badge
                      variant={authMode === 'oauth_proxy' ? 'success' : 'outline'}
                      className={`text-[10px] uppercase tracking-wider font-bold ${
                        authMode === 'oauth_proxy'
                          ? 'bg-emerald-500/10 text-emerald-400 border-emerald-500/20'
                          : 'bg-white/5 text-muted-foreground border-white/10'
                      }`}
                    >
                      {providerAuthLabel(provider)}
                    </Badge>
                  </div>
                  <div className="space-y-1.5">
                    {provider.websiteUrl ? (
                      <a
                        href={provider.websiteUrl}
                        target="_blank"
                        rel="noopener noreferrer"
                        className="inline-flex items-center gap-1.5 text-[12px] font-medium text-primary/80 hover:text-primary transition-colors hover:underline"
                      >
                        <span className="truncate max-w-[260px]">{provider.websiteUrl}</span>
                        <ExternalLink className="h-3 w-3 shrink-0" />
                      </a>
                    ) : (
                      <div className="truncate text-sm font-medium text-muted-foreground">Custom Provider</div>
                    )}
                  </div>
                </div>
              </div>
              <Badge variant="success" className="justify-self-start sm:justify-self-end gap-1.5 py-1 px-3 shadow-[0_0_10px_rgba(16,185,129,0.2)]">
                <span className="relative flex h-2 w-2">
                  <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-emerald-400 opacity-75"></span>
                  <span className="relative inline-flex rounded-full h-2 w-2 bg-emerald-500"></span>
                </span>
                Active
              </Badge>
            </div>

            <div className="grid grid-cols-3 gap-2 rounded-xl border border-white/5 bg-white/[0.02] p-3">
              <div>
                <div className="text-[10px] font-bold uppercase tracking-wider text-muted-foreground/70">24h Requests</div>
                <div className="mt-1 font-mono text-base font-bold text-foreground tabular-nums">
                  {usage24h ? formatCompact(usage24h.requestCount) : '-'}
                </div>
              </div>
              <div>
                <div className="text-[10px] font-bold uppercase tracking-wider text-muted-foreground/70">24h Input</div>
                <div className="mt-1 font-mono text-base font-bold text-primary tabular-nums">
                  {usage24h ? formatCompact(usage24h.inputTokens) : '-'}
                </div>
              </div>
              <div>
                <div className="text-[10px] font-bold uppercase tracking-wider text-muted-foreground/70">24h Output</div>
                <div className="mt-1 font-mono text-base font-bold text-amber-500 tabular-nums">
                  {usage24h ? formatCompact(usage24h.outputTokens) : '-'}
                </div>
              </div>
            </div>

            {routeRuntime?.running && (
              <div className="grid grid-cols-1 gap-2 rounded-xl border border-white/5 bg-black/20 p-3">
                <div className="flex items-center justify-between">
                  <span className="text-xs font-medium text-muted-foreground">Local Route</span>
                  <span className={`text-xs font-semibold text-emerald-400`}>
                    Running
                  </span>
                </div>
                <div className="flex items-center justify-between">
                  <span className="text-xs font-medium text-muted-foreground">Current Target</span>
                  <span className={`text-xs font-semibold ${isCurrentTarget ? 'text-primary' : 'text-muted-foreground'}`}>
                    {isCurrentTarget ? 'This Provider' : 'Different Provider'}
                  </span>
                </div>
                {routeRuntime?.listenAddr && (
                  <div className="truncate text-[11px] font-mono text-muted-foreground/80">
                    {routeRuntime.listenAddr}
                  </div>
                )}
                {routeError && (
                  <div className="truncate text-[11px] text-destructive">
                    {routeError}
                  </div>
                )}
              </div>
            )}
          </div>
        ) : (
          <div className="py-8 flex flex-col items-center justify-center text-center text-muted-foreground bg-white/[0.02] rounded-xl border border-dashed border-white/10">
            <Globe className="h-8 w-8 text-muted-foreground/30 mb-3" />
            <p className="font-medium text-foreground">No provider selected</p>
            <p className="mt-1 text-sm text-muted-foreground/70">Select one below to get started</p>
          </div>
        )}
      </CardContent>
    </Card>
  );
}

export function OAuthStatusCard({
  title = 'OAuth Status',
  providerName = 'GitHub Copilot',
  status,
  pending,
  onConnect,
}: {
  title?: string;
  providerName?: string;
  status: CopilotStatus | null;
  pending: boolean;
  onConnect: () => void;
}) {
  const accountName = status?.accounts.find((a) => a.id === status.default_account_id)?.login || 'Connected';

  return (
    <Card>
      <CardHeader className="border-b border-white/10">
        <CardTitle className="grid grid-cols-[16px_minmax(0,1fr)] items-center gap-2 text-sm font-medium text-muted-foreground">
          <Server className="h-4 w-4" />
          {title}
        </CardTitle>
      </CardHeader>
      <CardContent className="space-y-3">
        <div className="grid grid-cols-[minmax(0,1fr)_auto] items-center gap-3 rounded-2xl bg-white/[0.04] p-3">
          <div className="grid min-w-0 grid-cols-[32px_minmax(0,1fr)] items-center gap-3">
            {status?.authenticated ? (
              <div className="flex h-8 w-8 items-center justify-center rounded-full bg-emerald-500/20">
                <CheckCircle2 className="h-4 w-4 text-emerald-500" />
              </div>
            ) : (
              <div className="flex h-8 w-8 items-center justify-center rounded-full bg-muted">
                <Circle className="h-4 w-4 text-muted-foreground" />
              </div>
            )}
            <div className="min-w-0">
              <div className="truncate text-sm font-medium leading-5">{providerName}</div>
              <div className="truncate text-xs leading-4 text-muted-foreground">
                {status?.authenticated ? accountName : 'Not connected'}
              </div>
            </div>
          </div>
          {!pending && (
            <Button size="sm" variant={status?.authenticated ? 'ghost' : 'default'} onClick={onConnect}>
              {status?.authenticated ? 'Manage' : 'Connect'}
            </Button>
          )}
        </div>
      </CardContent>
    </Card>
  );
}

export function CodexOAuthStatusCard({
  status,
  pending,
  onConnect,
  onSetDefault,
  onRemove,
}: {
  status: CodexStatus | null;
  pending: boolean;
  onConnect: () => void;
  onSetDefault: (accountId: string) => void;
  onRemove: (accountId: string) => void;
}) {
  const accountName = status?.accounts.find((a) => a.is_default)?.login || status?.accounts[0]?.login || 'Connected';

  return (
    <Card>
      <CardHeader className="border-b border-white/10">
        <CardTitle className="grid grid-cols-[16px_minmax(0,1fr)] items-center gap-2 text-sm font-medium text-muted-foreground">
          <Server className="h-4 w-4" />
          Codex OAuth
        </CardTitle>
      </CardHeader>
      <CardContent className="space-y-3">
        <div className="grid grid-cols-[minmax(0,1fr)_auto] items-center gap-3 rounded-2xl bg-white/[0.04] p-3">
          <div className="grid min-w-0 grid-cols-[32px_minmax(0,1fr)] items-center gap-3">
            {status?.authenticated ? (
              <div className="flex h-8 w-8 items-center justify-center rounded-full bg-emerald-500/20">
                <CheckCircle2 className="h-4 w-4 text-emerald-500" />
              </div>
            ) : (
              <div className="flex h-8 w-8 items-center justify-center rounded-full bg-muted">
                <Circle className="h-4 w-4 text-muted-foreground" />
              </div>
            )}
            <div className="min-w-0">
              <div className="truncate text-sm font-medium leading-5">ChatGPT Codex</div>
              <div className="truncate text-xs leading-4 text-muted-foreground">
                {status?.authenticated ? accountName : 'OAuth route provider only'}
              </div>
            </div>
          </div>
          {!pending && (
            <Button size="sm" variant={status?.authenticated ? 'ghost' : 'default'} onClick={onConnect}>
              {status?.authenticated ? 'Reconnect' : 'Connect'}
            </Button>
          )}
        </div>
        {status?.accounts.map((account) => (
          <div key={account.id} className="grid grid-cols-[minmax(0,1fr)_auto_auto] items-center gap-2 rounded-2xl border border-white/10 px-3 py-2">
            <div className="min-w-0">
              <div className="truncate text-sm leading-5 text-foreground">{account.login}</div>
              <div className="truncate text-xs leading-4 text-muted-foreground">{account.id}</div>
            </div>
            <Button
              size="sm"
              variant={account.is_default ? 'secondary' : 'outline'}
              onClick={() => onSetDefault(account.id)}
              disabled={account.is_default}
            >
              {account.is_default ? 'Default' : 'Set Default'}
            </Button>
            <Button size="sm" variant="ghost" onClick={() => onRemove(account.id)}>
              Remove
            </Button>
          </div>
        ))}
      </CardContent>
    </Card>
  );
}

export function ProxyCard({
  status,
  targetProviders,
  error,
  onToggle,
  onTargetChange,
}: {
  status: ProxyStatus | null;
  targetProviders: Provider[];
  error?: string;
  onToggle: () => void;
  onTargetChange: (providerId: string) => void;
}) {
  const proxyUrl = status?.listen_addr ? `${status.listen_addr}/v1/messages` : '';

  return (
    <Card className="border-white/10 shadow-lg bg-card/80 backdrop-blur-sm">
      <CardHeader className="border-b border-white/5 bg-black/10 pb-4">
        <CardTitle className="flex items-center gap-2.5 text-sm font-semibold text-muted-foreground tracking-tight">
          <div className="rounded-md bg-white/5 p-1.5 shadow-inner">
            <Server className="h-4 w-4 text-emerald-500" />
          </div>
          Local Route
        </CardTitle>
      </CardHeader>
      <CardContent className="space-y-5 pt-5">
        {error && (
          <div className="rounded-xl border border-destructive/30 bg-destructive/10 px-3 py-2.5 text-sm text-destructive shadow-sm">
            {error}
          </div>
        )}
        <div className="space-y-2.5">
          <div className="text-[11px] font-bold uppercase tracking-wider text-muted-foreground/80">
            Route Target
          </div>
          <RouteTargetMenu
            providers={targetProviders}
            selectedId={status?.active_target_provider_id || ''}
            disabled={status?.running || targetProviders.length === 0}
            onChange={onTargetChange}
          />
          <div className="flex flex-col gap-1">
            <p className="truncate text-xs font-medium text-muted-foreground">
              {status?.active_target_provider_name || 'No route selected'}
            </p>
            {status?.http_proxy_url && (
              <p className="truncate font-mono text-[11px] text-primary/80 bg-primary/10 w-fit px-1.5 py-0.5 rounded-md">
                via {status.http_proxy_url}
              </p>
            )}
          </div>
        </div>
        <div className="flex items-center justify-between rounded-xl bg-white/[0.02] border border-white/5 p-3">
          <div className="flex items-center gap-3">
            <div className={`h-2.5 w-2.5 rounded-full ${status?.running ? 'animate-pulse bg-emerald-500 shadow-[0_0_8px_rgba(16,185,129,0.8)]' : 'bg-muted-foreground/40'}`} />
            <span className="text-sm font-semibold tracking-tight">
              {status?.running ? 'Running' : 'Stopped'}
            </span>
          </div>
          <Button size="sm" variant={status?.running ? 'destructive' : 'default'} onClick={onToggle} className="rounded-lg shadow-sm">
            {status?.running ? 'Stop' : 'Start'}
          </Button>
        </div>
        {status?.running && proxyUrl && (
          <div className="flex items-center gap-2 rounded-xl bg-black/30 border border-white/5 p-2 pl-3 shadow-inner">
            <code className="flex-1 truncate font-mono text-[11px] text-emerald-400">{proxyUrl}</code>
            <Button
              size="icon"
              variant="ghost"
              className="h-7 w-7 rounded-lg hover:bg-white/10 text-muted-foreground hover:text-foreground"
              onClick={() => navigator.clipboard.writeText(proxyUrl)}
            >
              <Copy className="h-3.5 w-3.5" />
            </Button>
          </div>
        )}
      </CardContent>
    </Card>
  );
}

function RouteTargetMenu({
  providers,
  selectedId,
  disabled,
  onChange,
}: {
  providers: Provider[];
  selectedId: string;
  disabled: boolean;
  onChange: (providerId: string) => void;
}) {
  const [open, setOpen] = useState(false);
  const menuRef = useRef<HTMLDivElement | null>(null);
  const selectedProvider = providers.find((provider) => provider.id === selectedId);

  useEffect(() => {
    if (!open) return;
    const handlePointerDown = (event: PointerEvent) => {
      if (!menuRef.current?.contains(event.target as Node)) {
        setOpen(false);
      }
    };
    window.addEventListener('pointerdown', handlePointerDown);
    return () => window.removeEventListener('pointerdown', handlePointerDown);
  }, [open]);

  return (
    <div ref={menuRef} className="relative">
      <button
        type="button"
        disabled={disabled}
        onClick={() => setOpen((value) => !value)}
        className="grid h-12 w-full grid-cols-[32px_minmax(0,1fr)_16px] items-center gap-3 rounded-xl border border-white/10 bg-black/25 px-3 text-left shadow-inner shadow-black/20 outline-none transition hover:border-white/20 hover:bg-white/[0.06] focus-visible:border-primary/50 focus-visible:ring-2 focus-visible:ring-primary/20 disabled:cursor-not-allowed disabled:opacity-50"
      >
        <div className="flex h-8 w-8 items-center justify-center rounded-lg border border-white/10 bg-white/[0.06] text-xs font-bold text-foreground">
          {selectedProvider ? providerInitial(selectedProvider) : '-'}
        </div>
        <div className="min-w-0">
          <div className="truncate text-sm font-semibold leading-5 text-foreground">
            {selectedProvider?.name || 'Select provider route'}
          </div>
          <div className="truncate text-[11px] font-medium uppercase tracking-wider text-muted-foreground/70">
            {selectedProvider ? providerAuthLabel(selectedProvider) : 'No route selected'}
          </div>
        </div>
        <ChevronDown className={`h-4 w-4 text-muted-foreground transition-transform ${open ? 'rotate-180' : ''}`} />
      </button>

      {open && (
        <div className="absolute z-30 mt-2 max-h-72 w-full overflow-y-auto rounded-xl border border-white/10 bg-card/95 p-1.5 shadow-2xl shadow-black/40 backdrop-blur-xl">
          {providers.map((provider) => {
            const selected = provider.id === selectedId;
            return (
              <button
                key={provider.id}
                type="button"
                onClick={() => {
                  onChange(provider.id);
                  setOpen(false);
                }}
                className={`grid w-full grid-cols-[32px_minmax(0,1fr)_18px] items-center gap-3 rounded-lg px-2.5 py-2 text-left transition ${
                  selected
                    ? 'bg-primary/15 text-foreground'
                    : 'text-muted-foreground hover:bg-white/[0.06] hover:text-foreground'
                }`}
              >
                <div className={`flex h-8 w-8 items-center justify-center rounded-lg border text-xs font-bold ${
                  selected
                    ? 'border-primary/30 bg-primary/20 text-primary'
                    : 'border-white/10 bg-white/[0.04] text-foreground/80'
                }`}>
                  {providerInitial(provider)}
                </div>
                <div className="min-w-0">
                  <div className="truncate text-sm font-semibold leading-5">{provider.name}</div>
                  <div className="truncate text-[11px] font-medium uppercase tracking-wider opacity-70">
                    {providerAuthLabel(provider)}
                  </div>
                </div>
                {selected && <Check className="h-4 w-4 text-primary" />}
              </button>
            );
          })}
        </div>
      )}
    </div>
  );
}

export function UsageCard({ usage }: { usage: CopilotUsageResponse }) {
  return (
    <Card className="border-white/10 shadow-lg bg-card/80 backdrop-blur-sm">
      <CardHeader className="border-b border-white/5 bg-black/10 pb-4">
        <CardTitle className="flex items-center justify-between gap-3">
          <div className="flex items-center gap-2.5 text-sm font-semibold text-muted-foreground tracking-tight">
            <div className="rounded-md bg-white/5 p-1.5 shadow-inner">
              <Zap className="h-4 w-4 text-amber-500" />
            </div>
            <span>Copilot Usage</span>
          </div>
          <Badge variant="outline" className="text-[10px] uppercase tracking-wider font-bold bg-white/5 border-white/10">{usage.copilot_plan}</Badge>
        </CardTitle>
      </CardHeader>
      <CardContent className="space-y-5 pt-5">
        <UsageMeter label="Chat" quota={usage.quota_snapshots.chat} />
        <UsageMeter label="Completions" quota={usage.quota_snapshots.completions} />
        <div className="pt-2 border-t border-white/5">
          <p className="text-center text-[11px] font-medium text-muted-foreground/70">Resets: {usage.quota_reset_date}</p>
        </div>
      </CardContent>
    </Card>
  );
}

function UsageMeter({
  label,
  quota,
}: {
  label: string;
  quota: { entitlement: number; remaining: number; unlimited: boolean };
}) {
  return (
    <div className="space-y-2">
      <div className="grid grid-cols-[minmax(0,1fr)_auto] items-baseline gap-3 text-sm leading-5">
        <span className="text-muted-foreground">{label}</span>
        <span className="font-mono text-xs tabular-nums">
          {quota.remaining.toLocaleString()} / {quota.entitlement.toLocaleString()}
        </span>
      </div>
      <Progress
        value={usagePercent(quota.remaining, quota.entitlement, quota.unlimited)}
        className="h-2"
        indicatorClassName={quota.unlimited ? 'bg-emerald-500' : undefined}
      />
    </div>
  );
}

export function ProviderGrid({
  providers,
  currentProviderId,
  onSwitch,
}: {
  providers: Provider[];
  currentProviderId: string | null;
  onSwitch: (id: string) => void;
}) {
  return (
    <div className="space-y-5">
      <div className="flex items-center gap-3">
        <h2 className="text-lg font-bold tracking-tight">All Providers</h2>
        <div className="h-[1px] flex-1 bg-gradient-to-r from-white/10 to-transparent" />
      </div>
      <div className="grid grid-cols-2 gap-4 sm:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5">
        {providers.map((provider) => (
          <button
            key={provider.id}
            onClick={() => onSwitch(provider.id)}
            className={`
              relative min-w-0 rounded-2xl border p-4 text-left transition-all duration-300 outline-none focus-visible:ring-2 focus-visible:ring-primary/50 group overflow-hidden
              ${provider.id === currentProviderId
                ? 'border-primary/50 bg-gradient-to-b from-primary/10 to-primary/5 shadow-[0_4px_20px_-4px_rgba(var(--primary),0.2)] hover:from-primary/15 hover:to-primary/10'
                : 'border-white/5 bg-card/40 hover:bg-white/[0.08] hover:border-white/10 hover:shadow-lg'
              }
            `}
          >
            {provider.id === currentProviderId && (
              <div className="absolute right-3 top-3 animate-in zoom-in duration-300">
                <CheckCircle2 className="h-5 w-5 text-primary drop-shadow-[0_0_8px_rgba(var(--primary),0.5)]" />
              </div>
            )}
            <div className={`mb-4 flex h-12 w-12 items-center justify-center rounded-xl shadow-inner transition-colors duration-300
              ${provider.id === currentProviderId ? 'bg-primary/20 border border-primary/20' : 'bg-white/5 border border-white/5 group-hover:bg-white/10'}
            `}>
              <span className={`text-xl font-bold ${provider.id === currentProviderId ? 'text-primary' : 'text-foreground/80 group-hover:text-foreground'}`}>
                {providerInitial(provider)}
              </span>
            </div>
            <div className="truncate text-[15px] font-bold tracking-tight leading-5">{provider.name}</div>
            <div className="mt-1 truncate text-[11px] font-medium uppercase tracking-wider text-muted-foreground/80">
              {providerAuthLabel(provider)}
            </div>
            <div className="mt-3 flex items-center gap-1.5">
              <div className={`h-1.5 w-1.5 rounded-full ${providerAuthMode(provider) === 'oauth_proxy' ? 'bg-emerald-500' : 'bg-primary'
                } shadow-[0_0_8px_currentColor]`} />
              <div className={`h-1.5 flex-1 rounded-full opacity-20 ${providerAuthMode(provider) === 'oauth_proxy' ? 'bg-emerald-500' : 'bg-primary'
                }`} />
            </div>
          </button>
        ))}
      </div>
    </div>
  );
}

export function DeviceOAuthModal({
  open,
  title,
  verificationUri,
  userCode,
  onAuthorized,
}: {
  open: boolean;
  title: string;
  verificationUri: string;
  userCode: string;
  onAuthorized: () => void;
}) {
  if (!open) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/55 px-4 backdrop-blur-sm">
      <div className="w-full max-w-md rounded-3xl border border-white/10 bg-card/95 p-6 shadow-2xl shadow-black/30">
        <h2 className="mb-2 text-xl font-semibold">{title}</h2>
        <p className="mb-6 text-sm text-muted-foreground">Visit the URL and enter the code to authorize</p>
        <a
          href={verificationUri}
          target="_blank"
          rel="noopener noreferrer"
          className="mb-4 flex min-w-0 items-center gap-2 text-primary hover:underline"
        >
          <ExternalLink className="h-4 w-4" />
          <span className="truncate">{verificationUri}</span>
        </a>
        <div className="mb-6 text-center">
          <div className="rounded-2xl bg-secondary/50 py-6 font-mono text-5xl font-bold tracking-widest text-primary">
            {userCode}
          </div>
        </div>
        <Button onClick={onAuthorized} className="w-full">
          I've Authorized
        </Button>
      </div>
    </div>
  );
}
