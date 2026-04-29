import { CheckCircle2, Circle, Copy, ExternalLink, Globe, Loader2, Server, Zap } from 'lucide-react';
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
  active_target_provider_id: string | null;
  active_target_provider_name: string | null;
}

export function CurrentProviderCard({
  loading,
  provider,
}: {
  loading: boolean;
  provider: Provider | null;
}) {
  return (
    <Card className="xl:col-span-2">
      <CardHeader className="border-b border-white/10">
        <CardTitle className="grid grid-cols-[16px_minmax(0,1fr)] items-center gap-2 text-sm font-medium text-muted-foreground">
          <Globe className="h-4 w-4" />
          Current Provider
        </CardTitle>
      </CardHeader>
      <CardContent>
        {loading ? (
          <div className="flex h-20 items-center justify-center">
            <Loader2 className="h-5 w-5 animate-spin text-muted-foreground" />
          </div>
        ) : provider ? (
          <div className="grid gap-4 sm:grid-cols-[minmax(0,1fr)_auto] sm:items-center">
            <div className="grid min-w-0 grid-cols-[48px_minmax(0,1fr)] items-center gap-4">
              <div className="flex h-12 w-12 shrink-0 items-center justify-center rounded-2xl bg-primary/20">
                <span className="text-lg font-bold text-primary">{providerInitial(provider)}</span>
              </div>
              <div className="min-w-0">
                <div className="truncate text-lg font-semibold leading-7">{provider.name}</div>
                <div className="truncate text-sm leading-5 text-muted-foreground">
                  {provider.websiteUrl || 'Custom Provider'}
                </div>
              </div>
            </div>
            <Badge variant="success" className="justify-self-start gap-1 sm:justify-self-end">
              <CheckCircle2 className="h-3 w-3" />
              Active
            </Badge>
          </div>
        ) : (
          <div className="py-6 text-center text-muted-foreground">
            <p>No provider selected</p>
            <p className="mt-1 text-sm">Select one below to get started</p>
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
                {status?.authenticated ? accountName : 'OAuth Proxy provider only'}
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
  const proxyUrl = status?.listen_addr ? `${status.listen_addr}/v1/chat/completions` : '';

  return (
    <Card>
      <CardHeader className="border-b border-white/10">
        <CardTitle className="grid grid-cols-[16px_minmax(0,1fr)] items-center gap-2 text-sm font-medium text-muted-foreground">
          <Server className="h-4 w-4" />
          Proxy Server
        </CardTitle>
      </CardHeader>
      <CardContent className="space-y-4">
        {error && (
          <div className="rounded-2xl border border-destructive/30 bg-destructive/10 px-3 py-2 text-sm leading-5 text-destructive">
            {error}
          </div>
        )}
        <div className="space-y-2">
          <label htmlFor="proxy-target" className="text-xs font-medium leading-4 text-muted-foreground">
            Active Target
          </label>
          <select
            id="proxy-target"
            value={status?.active_target_provider_id || ''}
            onChange={(event) => onTargetChange(event.target.value)}
            disabled={status?.running || targetProviders.length === 0}
            className="h-10 w-full rounded-xl border border-input bg-white/[0.04] px-3 text-sm leading-5 text-foreground shadow-inner shadow-black/10 outline-none transition focus:border-primary/70 focus:ring-4 focus:ring-primary/15 disabled:cursor-not-allowed disabled:opacity-60"
          >
            <option value="">Select OAuth Proxy provider</option>
            {targetProviders.map((provider) => (
              <option key={provider.id} value={provider.id}>
                {provider.name}
              </option>
            ))}
          </select>
          <p className="truncate text-xs leading-4 text-muted-foreground">
            {status?.active_target_provider_name || 'No proxy target selected'}
          </p>
        </div>
        <div className="grid grid-cols-[minmax(0,1fr)_auto] items-center gap-3">
          <div className="grid min-w-0 grid-cols-[12px_minmax(0,1fr)] items-center gap-3">
            <div className={`h-3 w-3 rounded-full ${status?.running ? 'animate-pulse bg-emerald-500' : 'bg-muted'}`} />
            <span className="truncate text-sm font-medium leading-5">
              {status?.running ? 'Running' : 'Stopped'}
            </span>
          </div>
          <Button size="sm" variant={status?.running ? 'destructive' : 'default'} onClick={onToggle}>
            {status?.running ? 'Stop' : 'Start'}
          </Button>
        </div>
        {status?.running && proxyUrl && (
          <div className="flex items-center gap-2 rounded-2xl bg-white/[0.04] p-3">
            <code className="flex-1 truncate font-mono text-xs text-primary">{proxyUrl}</code>
            <Button
              size="icon"
              variant="ghost"
              className="h-7 w-7"
              onClick={() => navigator.clipboard.writeText(proxyUrl)}
            >
              <Copy className="h-3 w-3" />
            </Button>
          </div>
        )}
      </CardContent>
    </Card>
  );
}

export function UsageCard({ usage }: { usage: CopilotUsageResponse }) {
  return (
    <Card>
      <CardHeader className="border-b border-white/10">
        <CardTitle className="grid grid-cols-[minmax(0,1fr)_auto] items-center gap-3">
          <span className="grid min-w-0 grid-cols-[16px_minmax(0,1fr)] items-center gap-2 text-sm font-medium text-muted-foreground">
            <Zap className="h-4 w-4" />
            <span className="truncate">Copilot Usage</span>
          </span>
          <Badge variant="outline" className="text-xs leading-4">{usage.copilot_plan}</Badge>
        </CardTitle>
      </CardHeader>
      <CardContent className="space-y-4">
        <UsageMeter label="Chat" quota={usage.quota_snapshots.chat} />
        <UsageMeter label="Completions" quota={usage.quota_snapshots.completions} />
        <p className="text-center text-xs text-muted-foreground">Resets: {usage.quota_reset_date}</p>
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
    <div className="space-y-4">
      <h2 className="text-lg font-semibold leading-7">All Providers</h2>
      <div className="grid grid-cols-2 gap-3 sm:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5">
        {providers.map((provider) => (
          <button
            key={provider.id}
            onClick={() => onSwitch(provider.id)}
            className={`
              relative min-w-0 rounded-2xl border p-4 text-left transition-all duration-200
              ${provider.id === currentProviderId
                ? 'border-primary bg-primary/10 shadow-inner shadow-primary/10'
                : 'border-white/10 bg-card/80 hover:bg-white/[0.06]'
              }
            `}
          >
            {provider.id === currentProviderId && (
              <div className="absolute right-2 top-2">
                <CheckCircle2 className="h-4 w-4 text-primary" />
              </div>
            )}
            <div className="mb-3 flex h-10 w-10 items-center justify-center rounded-xl bg-secondary">
              <span className="text-lg font-bold text-foreground">{providerInitial(provider)}</span>
            </div>
            <div className="truncate text-sm font-medium leading-5">{provider.name}</div>
            <div className="mt-0.5 truncate text-xs leading-4 text-muted-foreground">
              {providerAuthLabel(provider)}
            </div>
            <div className={`mt-2 h-1.5 w-10 rounded-full ${
              providerAuthMode(provider) === 'oauth_proxy' ? 'bg-emerald-500' : 'bg-primary'
            }`} />
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
