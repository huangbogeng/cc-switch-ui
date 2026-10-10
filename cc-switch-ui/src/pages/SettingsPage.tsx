import { BrandMark } from '@/components/BrandMark';
import { useCallback, useEffect, useState } from 'react';
import { Check, RotateCcw, Save } from 'lucide-react';
import { PageHeader } from '@/components/PageHeader';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Select } from '@/components/ui/select';
import {
  deleteProxyConfig,
  getProxyConfig,
  getProxyPort,
  setProxyConfig,
  setProxyPort,
  type ProxyConfig,
} from '@/api';
import { cacheGet, cacheSet } from '@/lib/fetchCache';
import { ToastNotice } from '@/components/ToastNotice';
import { errorMessage } from '@/lib/errors';
import { useUnsavedChanges } from '@/lib/useUnsavedChanges';
import { BackupCenter } from '@/components/settings/BackupCenter';
import { ClaudeUserSettingsPanel } from '@/components/settings/ClaudeUserSettingsPanel';

export default function SettingsPage() {
  const cachedPort = cacheGet<number>('settings-proxy-port');
  const cachedProxy = cacheGet<ProxyConfig>('settings-outbound-proxy');

  // Proxy port state
  const [proxyPort, setProxyPortState] = useState(cachedPort ?? 15721);
  const [portLoading, setPortLoading] = useState(!cachedPort);
  const [portReady, setPortReady] = useState(cachedPort != null);
  const [outboundReady, setOutboundReady] = useState(cachedProxy != null);
  const [portSaved, setPortSaved] = useState(false);
  const [portError, setPortError] = useState<string | null>(null);
  const [outboundProxy, setOutboundProxy] = useState<ProxyConfig>(
    cachedProxy ?? {
      enabled: false,
      proxy_type: 'http',
      host: '127.0.0.1',
      port: 10809,
    },
  );
  const [outboundLoading, setOutboundLoading] = useState(!cachedProxy);
  const [outboundSaved, setOutboundSaved] = useState(false);
  const [outboundError, setOutboundError] = useState<string | null>(null);
  const [savingPort, setSavingPort] = useState(false);
  const [savingOutbound, setSavingOutbound] = useState(false);

  const [savedPort, setSavedPort] = useState(cachedPort ?? 15721);
  const [savedOutbound, setSavedOutbound] = useState(JSON.stringify(outboundProxy));
  useUnsavedChanges(
    (!portLoading && proxyPort !== savedPort)
      || (!outboundLoading && JSON.stringify(outboundProxy) !== savedOutbound),
    savingPort || savingOutbound,
  );

  const loadProxyPort = useCallback(async (signal?: AbortSignal) => {
    setPortLoading(true); setPortError(null);
    try {
      const data = await getProxyPort({ signal });
      if (signal?.aborted) return;
      setPortReady(true);
      setProxyPortState(data.port);
      setSavedPort(data.port);
      cacheSet('settings-proxy-port', data.port);
    } catch (e) {
      if (e instanceof DOMException && e.name === 'AbortError') return;
      setPortError(errorMessage(e, 'Failed to load proxy port'));
    } finally {
      if (!signal?.aborted) setPortLoading(false);
    }
  }, []);

  const loadOutboundProxy = useCallback(async (signal?: AbortSignal) => {
    setOutboundLoading(true); setOutboundError(null);
    try {
      const config = await getProxyConfig({ signal });
      if (signal?.aborted) return;
      setOutboundReady(true);
      setOutboundProxy(config);
      setSavedOutbound(JSON.stringify(config));
      cacheSet('settings-outbound-proxy', config);
    } catch (e) {
      if (e instanceof DOMException && e.name === 'AbortError') return;
      setOutboundError(errorMessage(e, 'Failed to load outbound proxy config'));
    } finally {
      if (!signal?.aborted) setOutboundLoading(false);
    }
  }, []);

  useEffect(() => {
    const ctrl = new AbortController();
    void Promise.resolve().then(() =>
      Promise.all([loadProxyPort(ctrl.signal), loadOutboundProxy(ctrl.signal)]),
    );
    return () => ctrl.abort();
  }, [loadProxyPort, loadOutboundProxy]);

  const handlePortSave = async () => {
    if (!portReady || portLoading || savingPort) return;
    setPortSaved(false);
    setPortError(null);
    if (!Number.isInteger(proxyPort) || proxyPort < 1024 || proxyPort > 65535) {
      setPortError('Route port must be an integer between 1024 and 65535.');
      return;
    }
    setSavingPort(true);
    try {
      await setProxyPort(proxyPort);
      setSavedPort(proxyPort);
      cacheSet('settings-proxy-port', proxyPort);
      setPortSaved(true);

    } catch (e) {
      setPortError(e instanceof Error ? e.message : 'Failed to save proxy port');
    } finally {
      setSavingPort(false);
    }
  };

  const handleOutboundSave = async () => {
    if (!outboundReady || outboundLoading || savingOutbound) return;
    setOutboundSaved(false);
    setOutboundError(null);
    if (outboundProxy.enabled && !outboundProxy.host.trim()) {
      setOutboundError('Proxy host is required when the outbound proxy is enabled.');
      return;
    }
    if (!Number.isInteger(outboundProxy.port) || outboundProxy.port < 1 || outboundProxy.port > 65535) {
      setOutboundError('Proxy port must be an integer between 1 and 65535.');
      return;
    }
    setSavingOutbound(true);
    try {
      await setProxyConfig(outboundProxy);
      setSavedOutbound(JSON.stringify(outboundProxy));
      cacheSet('settings-outbound-proxy', outboundProxy);
      setOutboundSaved(true);

    } catch (e) {
      setOutboundError(e instanceof Error ? e.message : 'Failed to save outbound proxy');
    } finally {
      setSavingOutbound(false);
    }
  };

  const handleOutboundReset = async () => {
    if (!outboundReady || outboundLoading || savingOutbound) return;
    setOutboundSaved(false);
    setOutboundError(null);
    setSavingOutbound(true);
    try {
      await deleteProxyConfig();
      const resetConfig: ProxyConfig = {
        enabled: false,
        proxy_type: 'http',
        host: '127.0.0.1',
        port: 10809,
      };
      setOutboundProxy(resetConfig);
      setSavedOutbound(JSON.stringify(resetConfig));
      cacheSet('settings-outbound-proxy', resetConfig);
      setOutboundSaved(true);

    } catch (e) {
      setOutboundError(e instanceof Error ? e.message : 'Failed to reset outbound proxy');
    } finally {
      setSavingOutbound(false);
    }
  };

  return (
    <div className="space-y-6">
      {portError && <ToastNotice tone="error" message={portError} onDismiss={() => setPortError(null)} />}
      {outboundError && <ToastNotice tone="error" message={outboundError} onDismiss={() => setOutboundError(null)} />}
      <PageHeader
        title="Settings"
        description="Manage user settings, backups, and local connections."
      />

      <div className="grid gap-6 items-start xl:grid-cols-[minmax(0,1fr)_320px]">
        <div className="space-y-6">
          <div id="claude-user-settings"><ClaudeUserSettingsPanel /></div>
          <BackupCenter />
          <Card>
            <CardHeader>
              <CardTitle className="text-[15px] font-semibold tracking-tight">Local Route Port</CardTitle>
              <p className="text-xs font-medium text-muted-foreground/80 mt-1.5">Set the local Claude Code endpoint port. Restart the CC Switch server after saving to apply a new port.</p>
            </CardHeader>
            <CardContent className="space-y-5">

              {portLoading ? (
                <div className="flex items-center gap-3 text-sm font-medium text-muted-foreground">
                  <div className="h-4 w-4 rounded-full border-2 border-primary border-t-transparent animate-spin"></div>
                  Loading port config...
                </div>
              ) : !portReady ? (
                <div className="flex flex-wrap items-center gap-3 text-sm text-muted-foreground">
                  <span>Route port configuration is unavailable.</span>
                  <Button variant="outline" onClick={() => void loadProxyPort()}>Retry port configuration</Button>
                </div>
              ) : (
                <div className="flex flex-wrap items-end gap-4">
                  <div className="space-y-2.5 flex-1 max-w-[240px]">
                    <Label htmlFor="proxy-port-setting" className="text-xs font-medium text-muted-foreground">Route Listen Port</Label>
                    <Input
                      id="proxy-port-setting"
                      type="number"
                      value={Number.isFinite(proxyPort) ? proxyPort : ''}
                      disabled={savingPort}
                      onChange={(e) => { setPortSaved(false); setProxyPortState(e.target.valueAsNumber); }}
                      min={1024}
                      max={65535}
                      className="h-10 rounded-lg font-mono"
                    />
                  </div>
                  <Button onClick={handlePortSave} disabled={portLoading || savingPort} className="h-10 rounded-xl shadow-sm px-6">
                    {portSaved ? <Check className="h-4 w-4 mr-2" /> : <Save className="h-4 w-4 mr-2" />}
                    {savingPort ? 'Saving...' : portSaved ? 'Saved' : 'Save Port'}
                  </Button>
                </div>
              )}
              <p className="text-[11px] font-medium text-muted-foreground/60 flex items-center gap-2">
                <span className="inline-block w-1 h-1 rounded-full bg-muted-foreground/40"></span>
                Default port is 15721
              </p>
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardTitle className="text-[15px] font-semibold tracking-tight">Outbound Network Proxy</CardTitle>
              <p className="text-xs font-medium text-muted-foreground/80 mt-1.5">Optional network proxy used when the local route connects to upstream providers and OAuth services.</p>
            </CardHeader>
            <CardContent className="space-y-5">

              {outboundLoading ? (
                <div className="flex items-center gap-3 text-sm font-medium text-muted-foreground">
                  <div className="h-4 w-4 rounded-full border-2 border-primary border-t-transparent animate-spin"></div>
                  Loading outbound proxy...
                </div>
              ) : !outboundReady ? (
                <div className="flex flex-wrap items-center gap-3 text-sm text-muted-foreground">
                  <span>Outbound proxy configuration is unavailable.</span>
                  <Button variant="outline" onClick={() => void loadOutboundProxy()}>Retry outbound configuration</Button>
                </div>
              ) : (
                <>
                  <label className="flex items-center gap-3 text-sm font-medium text-muted-foreground">
                    <input
                      type="checkbox"
                      checked={outboundProxy.enabled}
                      disabled={savingOutbound}
                      onChange={(e) => { setOutboundSaved(false); setOutboundProxy({ ...outboundProxy, enabled: e.target.checked }); }}
                      className="h-4 w-4 rounded border-input accent-primary"
                    />
                    Use outbound network proxy
                  </label>

                  <div className="grid gap-4 sm:grid-cols-[140px_minmax(0,1fr)_120px]">
                    <div className="space-y-2.5">
                      <Label htmlFor="outbound-proxy-type" className="text-xs font-medium text-muted-foreground">Type</Label>
                      <Select
                        id="outbound-proxy-type"
                        value={outboundProxy.proxy_type}
                        onChange={(e) => { setOutboundSaved(false); setOutboundProxy({ ...outboundProxy, proxy_type: e.target.value }); }}
                        disabled={savingOutbound || !outboundProxy.enabled}
                      >
                        <option value="http">HTTP</option>
                        <option value="socks5">SOCKS5</option>
                      </Select>
                    </div>
                    <div className="space-y-2.5">
                      <Label htmlFor="outbound-proxy-host" className="text-xs font-medium text-muted-foreground">Host</Label>
                      <Input
                        id="outbound-proxy-host"
                        value={outboundProxy.host}
                        onChange={(e) => { setOutboundSaved(false); setOutboundProxy({ ...outboundProxy, host: e.target.value }); }}
                        disabled={savingOutbound || !outboundProxy.enabled}
                        placeholder="127.0.0.1"
                        className="h-10 rounded-lg font-mono"
                      />
                    </div>
                    <div className="space-y-2.5">
                      <Label htmlFor="outbound-proxy-port" className="text-xs font-medium text-muted-foreground">Port</Label>
                      <Input
                        id="outbound-proxy-port"
                        type="number"
                        value={Number.isFinite(outboundProxy.port) ? outboundProxy.port : ''}
                        onChange={(e) => { setOutboundSaved(false); setOutboundProxy({ ...outboundProxy, port: e.target.valueAsNumber }); }}
                        disabled={savingOutbound || !outboundProxy.enabled}
                        min={1}
                        max={65535}
                        className="h-10 rounded-lg font-mono"
                      />
                    </div>
                  </div>

                  <div className="flex flex-wrap gap-3">
                    <Button onClick={handleOutboundSave} disabled={savingOutbound} className="h-10 rounded-xl shadow-sm px-6">
                      {outboundSaved ? <Check className="h-4 w-4 mr-2" /> : <Save className="h-4 w-4 mr-2" />}
                      {savingOutbound ? 'Saving...' : outboundSaved ? 'Saved' : 'Save Outbound Proxy'}
                    </Button>
                    <Button variant="outline" onClick={handleOutboundReset} disabled={savingOutbound} className="h-10 rounded-xl border-white/10 hover:bg-white/5">
                      <RotateCcw className="h-3.5 w-3.5 mr-2" />
                      Reset
                    </Button>
                  </div>
                </>
              )}
            </CardContent>
          </Card>
        </div>

        <Card className="h-fit">
          <CardHeader>
            <CardTitle className="text-[15px] font-semibold tracking-tight">About</CardTitle>
          </CardHeader>
          <CardContent className="space-y-5 text-sm text-muted-foreground">
            <div className="flex items-center gap-4">
              <BrandMark className="h-12 w-12" />
              <div>
                <div className="text-base font-semibold tracking-tight text-foreground">CC Switch Web</div>
                <div className="mt-0.5 text-xs font-semibold text-primary/80 bg-primary/10 w-fit px-2 py-0.5 rounded-md">Version {__APP_VERSION__}</div>
              </div>
            </div>
            <div className="h-[1px] bg-gradient-to-r from-white/10 to-transparent" />
            <p className="leading-relaxed font-medium">
              A lightweight pure Web architecture Claude Code provider manager.
              Forked from{' '}
              <a
                href="https://github.com/farion1231/cc-switch"
                target="_blank"
                rel="noopener noreferrer"
                className="text-primary font-semibold hover:text-primary/80 transition-colors hover:underline underline-offset-4"
              >
                cc-switch
              </a>
              .
            </p>
          </CardContent>
        </Card>
      </div>
    </div>
  );
}
