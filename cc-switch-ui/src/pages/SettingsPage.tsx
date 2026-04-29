import { useState } from 'react';
import { Check, RotateCcw, Save } from 'lucide-react';
import { PageHeader } from '@/components/PageHeader';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { cn } from '@/lib/utils';

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
              <p className="text-sm leading-5 text-muted-foreground">Set the default local port used by the proxy server.</p>
            </CardHeader>
            <CardContent>
              <div className="max-w-xs space-y-2">
                <Label htmlFor="proxy-port">Proxy Listen Port</Label>
                <Input
                  id="proxy-port"
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
