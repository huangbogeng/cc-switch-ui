import { useState, useEffect } from 'react';
import { Check, RotateCcw, Save } from 'lucide-react';
import { PageHeader } from '@/components/PageHeader';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { cn } from '@/lib/utils';
import { getProxyConfig, setProxyConfig, deleteProxyConfig, type ProxyConfig } from '@/api';

type Language = 'en' | 'zh';
type Theme = 'dark' | 'light';

interface Settings {
  language: Language;
  theme: Theme;
  proxyPort: number;
}

export default function SettingsPage() {
  const [settings, setSettings] = useState<Settings>(() => {
    const saved = localStorage.getItem('ccswitch_settings');
    return saved ? JSON.parse(saved) : {
      language: (localStorage.getItem('ccswitch_language') as Language) || 'en',
      theme: 'dark',
      proxyPort: 15721,
    };
  });
  const [saved, setSaved] = useState(false);

  // Proxy config state
  const [proxyConfig, setProxyConfigState] = useState<ProxyConfig>({
    enabled: false,
    proxyType: 'http',
    host: '127.0.0.1',
    port: 10809,
  });
  const [proxyLoading, setProxyLoading] = useState(true);
  const [proxySaved, setProxySaved] = useState(false);
  const [proxyError, setProxyError] = useState<string | null>(null);

  // Load proxy config on mount
  useEffect(() => {
    loadProxyConfig();
  }, []);

  const loadProxyConfig = async () => {
    try {
      const config = await getProxyConfig();
      setProxyConfigState(config);
    } catch (e) {
      console.error('Failed to load proxy config:', e);
    } finally {
      setProxyLoading(false);
    }
  };

  const handleSave = () => {
    localStorage.setItem('ccswitch_settings', JSON.stringify(settings));
    localStorage.setItem('ccswitch_language', settings.language);
    setSaved(true);
    setTimeout(() => setSaved(false), 2000);
  };

  const handleReset = () => {
    const defaults = { language: 'en' as Language, theme: 'dark' as Theme, proxyPort: 15721 };
    setSettings(defaults);
    localStorage.removeItem('ccswitch_settings');
    localStorage.removeItem('ccswitch_language');
  };

  const handleProxySave = async () => {
    setProxyError(null);
    try {
      await setProxyConfig(proxyConfig);
      setProxySaved(true);
      setTimeout(() => setProxySaved(false), 2000);
    } catch (e) {
      setProxyError(e instanceof Error ? e.message : 'Failed to save proxy config');
    }
  };

  const handleProxyReset = async () => {
    setProxyError(null);
    try {
      await deleteProxyConfig();
      setProxyConfigState({
        enabled: false,
        proxyType: 'http',
        host: '127.0.0.1',
        port: 10809,
      });
      setProxySaved(true);
      setTimeout(() => setProxySaved(false), 2000);
    } catch (e) {
      setProxyError(e instanceof Error ? e.message : 'Failed to reset proxy config');
    }
  };

  return (
    <div>
      <PageHeader
        title="Settings"
        description="Adjust local interface preferences and proxy defaults."
        action={
          <>
            <Button variant="outline" onClick={handleReset}>
              <RotateCcw className="h-4 w-4" />
              Reset
            </Button>
            <Button onClick={handleSave}>
              {saved ? <Check className="h-4 w-4" /> : <Save className="h-4 w-4" />}
              {saved ? 'Saved' : 'Save'}
            </Button>
          </>
        }
      />

      <div className="grid gap-4 xl:grid-cols-[1fr_360px]">
        <div className="space-y-4">
          <Card>
            <CardHeader className="border-b border-white/10">
              <CardTitle>Language</CardTitle>
              <p className="text-sm leading-5 text-muted-foreground">Choose the interface language used by local preferences.</p>
            </CardHeader>
            <CardContent>
              <div className="inline-flex rounded-2xl border border-white/10 bg-white/[0.035] p-1">
                {[
                  ['en', 'English'],
                  ['zh', '中文'],
                ].map(([value, label]) => (
                  <button
                    key={value}
                    type="button"
                    onClick={() => setSettings({ ...settings, language: value as Language })}
                    className={cn(
                      "rounded-xl px-4 py-2 text-sm font-medium leading-5 transition",
                      settings.language === value
                        ? "bg-primary text-primary-foreground shadow-lg shadow-primary/20"
                        : "text-muted-foreground hover:text-foreground"
                    )}
                  >
                    {label}
                  </button>
                ))}
              </div>
            </CardContent>
          </Card>

          <Card>
            <CardHeader className="border-b border-white/10">
              <CardTitle>Proxy</CardTitle>
              <p className="text-sm leading-5 text-muted-foreground">Configure proxy server for OAuth authentication (Codex/Copilot).</p>
            </CardHeader>
            <CardContent className="space-y-4">
              {proxyError && (
                <div className="rounded-2xl border border-destructive/30 bg-destructive/10 px-3 py-2 text-sm text-destructive">
                  {proxyError}
                </div>
              )}

              {proxyLoading ? (
                <div className="text-sm text-muted-foreground">Loading...</div>
              ) : (
                <>
                  <div className="flex items-center gap-3">
                    <input
                      type="checkbox"
                      id="proxy-enabled"
                      checked={proxyConfig.enabled}
                      onChange={(e) => setProxyConfigState({ ...proxyConfig, enabled: e.target.checked })}
                      className="h-4 w-4 rounded border-input"
                    />
                    <Label htmlFor="proxy-enabled">Enable Proxy</Label>
                  </div>

                  <div className="grid grid-cols-[auto_1fr_auto] gap-3 items-end">
                    <div className="space-y-2">
                      <Label htmlFor="proxy-type">Type</Label>
                      <select
                        id="proxy-type"
                        value={proxyConfig.proxyType}
                        onChange={(e) => setProxyConfigState({ ...proxyConfig, proxyType: e.target.value })}
                        disabled={!proxyConfig.enabled}
                        className="h-10 rounded-xl border border-input bg-white/[0.04] px-3 text-sm shadow-inner shadow-black/10 outline-none transition focus:border-primary/70 focus:ring-4 focus:ring-primary/15 disabled:cursor-not-allowed disabled:opacity-60"
                      >
                        <option value="http">HTTP</option>
                        <option value="socks5">SOCKS5</option>
                      </select>
                    </div>

                    <div className="space-y-2">
                      <Label htmlFor="proxy-host">Host</Label>
                      <Input
                        id="proxy-host"
                        value={proxyConfig.host}
                        onChange={(e) => setProxyConfigState({ ...proxyConfig, host: e.target.value })}
                        disabled={!proxyConfig.enabled}
                        placeholder="127.0.0.1"
                      />
                    </div>

                    <div className="space-y-2 w-24">
                      <Label htmlFor="proxy-port">Port</Label>
                      <Input
                        id="proxy-port"
                        type="number"
                        value={proxyConfig.port}
                        onChange={(e) => setProxyConfigState({ ...proxyConfig, port: parseInt(e.target.value) || 10809 })}
                        disabled={!proxyConfig.enabled}
                        min={1}
                        max={65535}
                      />
                    </div>
                  </div>

                  <div className="flex gap-2 pt-2">
                    <Button onClick={handleProxySave} disabled={!proxyConfig.enabled}>
                      {proxySaved ? <Check className="h-4 w-4" /> : <Save className="h-4 w-4" />}
                      {proxySaved ? 'Saved' : 'Save Proxy'}
                    </Button>
                    <Button variant="outline" onClick={handleProxyReset}>
                      <RotateCcw className="h-4 w-4" />
                      Reset
                    </Button>
                  </div>
                </>
              )}
            </CardContent>
          </Card>

          <Card>
            <CardHeader className="border-b border-white/10">
              <CardTitle>Local Proxy Port</CardTitle>
              <p className="text-sm leading-5 text-muted-foreground">Set the default local port used by the proxy server.</p>
            </CardHeader>
            <CardContent>
              <div className="max-w-xs space-y-2">
                <Label htmlFor="proxy-port-setting">Proxy Listen Port</Label>
                <Input
                  id="proxy-port-setting"
                  type="number"
                  value={settings.proxyPort}
                  onChange={(e) => setSettings({ ...settings, proxyPort: parseInt(e.target.value) || 15721 })}
                  min={1024}
                  max={65535}
                />
                <p className="text-xs leading-4 text-muted-foreground">Default: 15721</p>
              </div>
            </CardContent>
          </Card>
        </div>

        <Card>
          <CardHeader className="border-b border-white/10">
            <CardTitle>About</CardTitle>
          </CardHeader>
          <CardContent className="space-y-4 text-sm text-muted-foreground">
            <div>
              <div className="text-base font-semibold leading-6 text-foreground">CC Switch Web</div>
              <div className="mt-1 text-sm leading-5 text-primary">Version 0.1.0</div>
            </div>
            <p className="leading-6">
              A lightweight pure Web architecture Claude Code provider manager.
              Forked from{' '}
              <a
                href="https://github.com/farion1231/cc-switch"
                target="_blank"
                rel="noopener noreferrer"
                className="text-primary hover:underline"
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
