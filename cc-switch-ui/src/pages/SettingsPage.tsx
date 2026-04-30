import { useState, useEffect } from 'react';
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

  // Load proxy port on mount
  useEffect(() => {
    loadProxyPort();
  }, []);

  const loadProxyPort = async () => {
    try {
      const data = await getProxyPort();
      setProxyPortState(data.port);
    } catch (e) {
      console.error('Failed to load proxy port:', e);
    } finally {
      setPortLoading(false);
    }
  };

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
    <div>
      <PageHeader
        title="Settings"
        description="Adjust local interface preferences."
      />

      <div className="grid gap-4 xl:grid-cols-[1fr_360px]">
        <div className="space-y-4">
          <Card>
            <CardHeader className="border-b border-white/10">
              <CardTitle>Language</CardTitle>
              <p className="text-sm leading-5 text-muted-foreground">Choose the interface language.</p>
            </CardHeader>
            <CardContent>
              <div className="flex items-center gap-4">
                <div className="inline-flex rounded-2xl border border-white/10 bg-white/[0.035] p-1">
                  {[
                    ['en', 'English'],
                    ['zh', '中文'],
                  ].map(([value, label]) => (
                    <button
                      key={value}
                      type="button"
                      onClick={() => setLanguage(value as Language)}
                      className={cn(
                        "rounded-xl px-4 py-2 text-sm font-medium leading-5 transition",
                        language === value
                          ? "bg-primary text-primary-foreground shadow-lg shadow-primary/20"
                          : "text-muted-foreground hover:text-foreground"
                      )}
                    >
                      {label}
                    </button>
                  ))}
                </div>
                <Button variant="outline" size="sm" onClick={handleLanguageReset}>
                  <RotateCcw className="h-4 w-4" />
                  Reset
                </Button>
              </div>
            </CardContent>
          </Card>

          <Card>
            <CardHeader className="border-b border-white/10">
              <CardTitle>Local Proxy Port</CardTitle>
              <p className="text-sm leading-5 text-muted-foreground">Set the default local port used by the proxy server.</p>
            </CardHeader>
            <CardContent className="space-y-4">
              {portError && (
                <div className="rounded-2xl border border-destructive/30 bg-destructive/10 px-3 py-2 text-sm text-destructive">
                  {portError}
                </div>
              )}

              {portLoading ? (
                <div className="text-sm text-muted-foreground">Loading...</div>
              ) : (
                <div className="flex items-end gap-4">
                  <div className="space-y-2">
                    <Label htmlFor="proxy-port-setting">Proxy Listen Port</Label>
                    <Input
                      id="proxy-port-setting"
                      type="number"
                      value={proxyPort}
                      onChange={(e) => setProxyPortState(parseInt(e.target.value) || 15721)}
                      min={1024}
                      max={65535}
                      className="w-32"
                    />
                  </div>
                  <Button onClick={handlePortSave} disabled={portLoading}>
                    {portSaved ? <Check className="h-4 w-4" /> : <Save className="h-4 w-4" />}
                    {portSaved ? 'Saved' : 'Save'}
                  </Button>
                </div>
              )}
              <p className="text-xs leading-4 text-muted-foreground">Default: 15721</p>
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