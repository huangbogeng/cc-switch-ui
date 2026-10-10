import { CheckCircle2, Circle, ExternalLink, Server, Zap } from 'lucide-react';
import type { CodexAccount, CopilotAccount, CopilotUsageResponse, Provider } from '@/api';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Progress } from '@/components/ui/progress';
import { providerApiFormat } from '@/lib/provider';
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
  recovery_required?: boolean;
  listen_addr: string | null;
  upstream_url: string;
  http_proxy_url: string | null;
  active_target_provider_id: string | null;
  active_target_provider_name: string | null;
}

export function DashboardHeroCard({
  currentProvider, routeTarget, status, loading = false, providerUnavailable = false,
}: {
  currentProvider: Provider | null;
  routeTarget: Provider | null;
  status: ProxyStatus | null;
  loading?: boolean;
  providerUnavailable?: boolean;
}) {
  const routeLabel = loading ? 'Loading...' : !status ? 'Unavailable' : status.recovery_required ? 'Recovery needed' : status.running ? 'Running' : 'Stopped';
  const providerLabel = loading ? 'Loading...' : providerUnavailable ? 'Unavailable' : currentProvider?.name ?? 'None selected';
  return (
    <Card>
      <CardContent className="p-0 sm:px-0">
        <div className="grid divide-y divide-white/[0.07] md:grid-cols-3 md:divide-x md:divide-y-0">
          <div className="min-w-0 space-y-2 p-5 sm:p-6">
            <div className="flex h-6 items-center text-xs text-muted-foreground">Direct Config</div>
            <h2 title={providerLabel} className="truncate text-xl font-semibold tracking-tight">{providerLabel}</h2>
            <p className="text-xs leading-5 text-muted-foreground">{currentProvider ? formatApiFormatLabel(providerApiFormat(currentProvider)) : 'Select a provider in Providers.'}</p>
          </div>
          <div className="min-w-0 space-y-2 p-5 sm:p-6">
            <div className="flex h-6 items-center justify-between gap-2">
              <span className="text-xs text-muted-foreground">Local Route</span>
              <Badge variant="outline" className={status?.recovery_required ? 'border-amber-500/20 text-amber-300' : status?.running ? 'border-emerald-500/20 text-emerald-400' : 'text-muted-foreground'}>{routeLabel}</Badge>
            </div>
            <div className="truncate text-xl font-semibold tracking-tight" title={routeTarget?.name ?? status?.active_target_provider_name ?? undefined}>
              {loading ? 'Loading...' : !status ? 'Unavailable' : routeTarget?.name ?? status.active_target_provider_name ?? 'No target'}
            </div>
            <p className="text-xs leading-5 text-muted-foreground">{status?.recovery_required ? 'Restore Direct Config in Providers.' : !status ? 'Route status is not available.' : status.running ? 'Forwarding through the local route.' : 'Start the route in Providers when needed.'}</p>
          </div>
          <div className="min-w-0 space-y-2 p-5 sm:p-6">
            <div className="flex h-6 items-center text-xs text-muted-foreground">Listen Address</div>
            <div className="break-all font-mono text-base font-medium leading-7">{loading ? 'Loading...' : status?.listen_addr ?? 'Unavailable'}</div>
            <p className="text-xs leading-5 text-muted-foreground">Local endpoint for connected clients.</p>
          </div>
        </div>
        {status?.running && <details className="border-t border-white/[0.07] px-5 py-3 sm:px-6">
          <summary className="cursor-pointer text-xs text-muted-foreground hover:text-foreground">Connection details</summary>
          <dl className="mt-3 grid gap-3 text-xs sm:grid-cols-2">
            <div><dt className="text-muted-foreground">Proxy Entry</dt><dd className="mt-1 break-all font-mono">{status.listen_addr ? `${status.listen_addr}/v1/messages` : 'Unavailable'}</dd></div>
            <div><dt className="text-muted-foreground">Upstream Proxy</dt><dd className="mt-1 break-all font-mono">{status.http_proxy_url || 'None'}</dd></div>
          </dl>
        </details>}
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

export function UsageCard({ usage }: { usage: CopilotUsageResponse }) {
  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex flex-wrap items-center justify-between gap-3">
          <div className="flex items-center gap-2 text-[15px] font-semibold text-foreground">
            <div className="text-muted-foreground">
              <Zap className="h-4 w-4 text-amber-500" />
            </div>
            <span>Copilot Usage</span>
          </div>
          <Badge variant="outline" className="text-xs font-medium bg-white/5 border-white/10">{usage.copilot_plan}</Badge>
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
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 px-4 py-6">
      <div className="dialog-panel w-full max-w-md p-5 sm:p-6">
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
          <div className="rounded-2xl bg-secondary/50 py-6 font-mono text-4xl font-semibold sm:text-5xl tracking-widest text-primary">
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
