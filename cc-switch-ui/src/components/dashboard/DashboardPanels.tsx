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
  listen_addr: string | null;
  upstream_url: string;
  http_proxy_url: string | null;
  active_target_provider_id: string | null;
  active_target_provider_name: string | null;
}

export function DashboardHeroCard({
  currentProvider,
  routeTarget,
  status,
}: {
  currentProvider: Provider | null;
  routeTarget: Provider | null;
  status: ProxyStatus | null;
}) {
  const proxyUrl = status?.listen_addr ? `${status.listen_addr}/v1/messages` : 'Unavailable';

  return (
    <Card className="relative overflow-hidden border-white/10 bg-[radial-gradient(circle_at_top_left,_rgba(245,158,11,0.14),_transparent_34%),radial-gradient(circle_at_top_right,_rgba(59,130,246,0.16),_transparent_30%),linear-gradient(135deg,rgba(12,18,32,0.98),rgba(8,12,20,0.96))] shadow-[0_24px_80px_-40px_rgba(15,23,42,0.95)]">
      <div className="pointer-events-none absolute inset-0 bg-[linear-gradient(120deg,transparent,rgba(255,255,255,0.04),transparent)] opacity-60" />
      <CardContent className="relative p-6 sm:p-8">
        <div className="grid gap-8 xl:grid-cols-[minmax(0,1.25fr)_360px]">
          <div className="space-y-6">
            <div className="flex flex-wrap items-center gap-2">
              <Badge className="border-amber-400/20 bg-amber-400/12 text-amber-200">Runtime Cockpit</Badge>
              <Badge
                variant="outline"
                className={
                  status?.running
                    ? 'border-emerald-500/25 bg-emerald-500/12 text-emerald-300'
                    : 'border-white/10 bg-white/5 text-slate-300/75'
                }
              >
                {status?.running ? 'Local Route Running' : 'Local Route Stopped'}
              </Badge>
            </div>

            <div className="space-y-2">
              <h2 className="max-w-3xl text-3xl font-semibold tracking-tight text-white sm:text-4xl">
                Dashboard is now read-only. Providers owns all configuration and route control.
              </h2>
              <p className="max-w-2xl text-sm leading-6 text-slate-300/75">
                Use this page to confirm the live runtime shape: direct config, route takeover target, and the local entry point currently exposed to Claude Code.
              </p>
            </div>

            <div className="grid gap-3 sm:grid-cols-3">
              <HeroStat
                label="Direct Config"
                value={currentProvider?.name || 'None'}
                meta={currentProvider ? `${currentProvider.id} · ${formatApiFormatLabel(providerApiFormat(currentProvider))}` : 'No provider selected'}
                tone="neutral"
              />
              <HeroStat
                label="Route Takeover"
                value={routeTarget?.name || 'None'}
                meta={
                  routeTarget
                    ? `${routeTarget.id} · ${formatApiFormatLabel(providerApiFormat(routeTarget))}`
                    : 'No route target selected'
                }
                tone={status?.running ? 'warning-active' : 'warning'}
              />
              <HeroStat
                label="Listen Address"
                value={status?.listen_addr || 'Unavailable'}
                meta={status?.running ? 'Traffic is currently routed through the local proxy' : 'Proxy endpoint will appear here after startup'}
                tone="neutral"
                mono
              />
            </div>
          </div>

          <div className="space-y-3 rounded-3xl border border-white/10 bg-black/25 p-4 shadow-inner shadow-black/20">
            <div className="flex items-center justify-between">
              <div className="text-xs uppercase tracking-[0.24em] text-slate-300/55">Runtime Signals</div>
              <div className={`h-2.5 w-2.5 rounded-full ${status?.running ? 'bg-emerald-400 shadow-[0_0_16px_rgba(74,222,128,0.95)]' : 'bg-slate-500/70'}`} />
            </div>
            <HeroSignal
              label="Route Status"
              value={status?.running ? 'Takeover active' : 'Direct config active'}
            />
            <HeroSignal
              label="Proxy Entry"
              value={proxyUrl}
              mono
            />
            <HeroSignal
              label="Upstream Proxy"
              value={status?.http_proxy_url || 'None'}
              mono
            />
          </div>
        </div>
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

function HeroStat({
  label,
  value,
  meta,
  tone,
  mono = false,
}: {
  label: string;
  value: string;
  meta: string;
  tone: 'neutral' | 'warning' | 'warning-active';
  mono?: boolean;
}) {
  const toneClass =
    tone === 'warning-active'
      ? 'border-amber-400/30 bg-gradient-to-br from-amber-500/18 to-amber-500/6'
      : tone === 'warning'
        ? 'border-amber-500/20 bg-amber-500/8'
        : 'border-white/10 bg-white/[0.04]';
  const textClass = tone === 'neutral' ? 'text-white' : 'text-amber-50';
  const metaClass = tone === 'neutral' ? 'text-slate-300/70' : 'text-amber-100/75';

  return (
    <div className={`rounded-2xl border px-4 py-4 backdrop-blur-sm ${toneClass}`}>
      <div className="text-[10px] font-bold uppercase tracking-[0.22em] text-slate-300/55">{label}</div>
      <div className={`mt-2 truncate text-lg font-semibold ${mono ? 'font-mono text-base' : ''} ${textClass}`}>{value}</div>
      <div className={`mt-1 truncate text-xs ${metaClass}`}>{meta}</div>
    </div>
  );
}

function HeroSignal({
  label,
  value,
  mono = false,
}: {
  label: string;
  value: string;
  mono?: boolean;
}) {
  return (
    <div className="rounded-2xl border border-white/8 bg-white/[0.03] px-3 py-3">
      <div className="text-[10px] font-bold uppercase tracking-[0.2em] text-slate-300/50">{label}</div>
      <div className={`mt-1 truncate text-sm text-slate-100 ${mono ? 'font-mono text-[13px]' : 'font-medium'}`}>{value}</div>
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
