import { useCallback, useEffect, useState } from 'react';
import { Check, RotateCcw, Save } from 'lucide-react';
import { PageHeader } from '@/components/PageHeader';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { cn } from '@/lib/utils';
import { getProxyPort, setProxyPort } from '@/api';

type Language = 'en' | 'zh';

export default function SettingsPage() {
  const [language, setLanguage] = useState<Language>(
    () => (localStorage.getItem('ccswitch_language') as Language) || 'en'
  );

  // Proxy port state
  const [proxyPort, setProxyPortState] = useState(15721);
  const [portLoading, setPortLoading] = useState(true);
  const [portSaved, setPortSaved] = useState(false);
  const [portError, setPortError] = useState<string | null>(null);

  const loadProxyPort = useCallback(async () => {
    try {
      const data = await getProxyPort();
      setProxyPortState(data.port);
    } catch (e) {
      console.error('Failed to load proxy port:', e);
    } finally {
      setPortLoading(false);
    }
  }, []);

  // Load proxy port on mount
  useEffect(() => {
    void Promise.resolve().then(loadProxyPort);
  }, [loadProxyPort]);

  const handleLanguageReset = () => {
    setLanguage('en');
    localStorage.removeItem('ccswitch_language');
  };

  const handlePortSave = async () => {
    setPortError(null);
    try {
      await setProxyPort(proxyPort);
      localStorage.setItem('ccswitch_proxy_port', proxyPort.toString());
      setPortSaved(true);
      setTimeout(() => setPortSaved(false), 2000);
    } catch (e) {
      setPortError(e instanceof Error ? e.message : 'Failed to save proxy port');
    }
  };

  return (
    <div className="max-w-[1000px] mx-auto">
      <PageHeader
        title="Settings"
        description="Adjust local interface preferences."
      />

      <div className="grid gap-6 xl:grid-cols-[1fr_360px]">
        <div className="space-y-6">
          <Card className="border-white/5 bg-card/40 hover:bg-card/60 transition-colors duration-300">
            <CardHeader className="border-b border-white/5 bg-black/10 pb-4">
              <CardTitle className="text-[15px] font-bold tracking-tight">Language</CardTitle>
              <p className="text-xs font-medium text-muted-foreground/80 mt-1.5">Choose the interface language.</p>
            </CardHeader>
            <CardContent className="pt-6">
              <div className="flex flex-wrap items-center gap-4">
                <div className="inline-flex rounded-xl border border-white/5 bg-black/20 p-1 shadow-inner">
                  {[
                    ['en', 'English'],
                    ['zh', '中文'],
                  ].map(([value, label]) => (
                    <button
                      key={value}
                      type="button"
                      onClick={() => setLanguage(value as Language)}
                      className={cn(
                        "rounded-lg px-4 py-2 text-sm font-semibold transition-all duration-200",
                        language === value
                          ? "bg-primary text-primary-foreground shadow-[0_0_15px_rgba(var(--primary),0.3)]"
                          : "text-muted-foreground hover:text-foreground hover:bg-white/5"
                      )}
                    >
                      {label}
                    </button>
                  ))}
                </div>
                <Button variant="outline" size="sm" onClick={handleLanguageReset} className="rounded-xl border-white/10 hover:bg-white/5">
                  <RotateCcw className="h-3.5 w-3.5 mr-2" />
                  Reset
                </Button>
              </div>
            </CardContent>
          </Card>

          <Card className="border-white/5 bg-card/40 hover:bg-card/60 transition-colors duration-300">
            <CardHeader className="border-b border-white/5 bg-black/10 pb-4">
              <CardTitle className="text-[15px] font-bold tracking-tight">Local Proxy Port</CardTitle>
              <p className="text-xs font-medium text-muted-foreground/80 mt-1.5">Set the default local port used by the proxy server.</p>
            </CardHeader>
            <CardContent className="space-y-5 pt-6">
              {portError && (
                <div className="rounded-xl border border-destructive/30 bg-destructive/10 px-3 py-2.5 text-sm font-medium text-destructive shadow-sm">
                  {portError}
                </div>
              )}

              {portLoading ? (
                <div className="flex items-center gap-3 text-sm font-medium text-muted-foreground">
                  <div className="h-4 w-4 rounded-full border-2 border-primary border-t-transparent animate-spin"></div>
                  Loading port config...
                </div>
              ) : (
                <div className="flex flex-wrap items-end gap-4">
                  <div className="space-y-2.5 flex-1 max-w-[240px]">
                    <Label htmlFor="proxy-port-setting" className="text-[11px] font-bold uppercase tracking-wider text-muted-foreground/80">Proxy Listen Port</Label>
                    <Input
                      id="proxy-port-setting"
                      type="number"
                      value={proxyPort}
                      onChange={(e) => setProxyPortState(parseInt(e.target.value) || 15721)}
                      min={1024}
                      max={65535}
                      className="h-10 rounded-xl border-white/10 bg-black/20 font-mono shadow-inner transition focus:border-primary/50 focus:ring-2 focus:ring-primary/20"
                    />
                  </div>
                  <Button onClick={handlePortSave} disabled={portLoading} className="h-10 rounded-xl shadow-sm px-6">
                    {portSaved ? <Check className="h-4 w-4 mr-2" /> : <Save className="h-4 w-4 mr-2" />}
                    {portSaved ? 'Saved' : 'Save Port'}
                  </Button>
                </div>
              )}
              <p className="text-[11px] font-medium text-muted-foreground/60 flex items-center gap-2">
                <span className="inline-block w-1 h-1 rounded-full bg-muted-foreground/40"></span>
                Default port is 15721
              </p>
            </CardContent>
          </Card>
        </div>

        <Card className="border-white/5 bg-card/40 hover:bg-card/60 transition-colors duration-300 h-fit">
          <CardHeader className="border-b border-white/5 bg-black/10 pb-4">
            <CardTitle className="text-[15px] font-bold tracking-tight">About</CardTitle>
          </CardHeader>
          <CardContent className="space-y-5 pt-6 text-sm text-muted-foreground">
            <div className="flex items-center gap-4">
              <div className="flex h-12 w-12 items-center justify-center rounded-xl bg-gradient-to-br from-primary/20 to-primary/5 border border-primary/20 shadow-inner">
                <span className="font-bold text-primary text-lg tracking-tight">CC</span>
              </div>
              <div>
                <div className="text-base font-bold tracking-tight text-foreground">CC Switch Web</div>
                <div className="mt-0.5 text-xs font-semibold text-primary/80 bg-primary/10 w-fit px-2 py-0.5 rounded-md">Version 0.1.0</div>
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
