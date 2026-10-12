import { useState } from 'react';
import { getEnvironmentDiagnostics, type EnvironmentDiagnostics } from '@/api';
import { Card, CardHeader, CardTitle, CardContent } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { ToastNotice } from '@/components/ToastNotice';

export function EnvironmentDiagnosticsPanel() {
  const [result, setResult] = useState<EnvironmentDiagnostics | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');
  const inspect = async () => {
    if (busy) return;
    setBusy(true); setError(''); setResult(null);
    try { setResult(await getEnvironmentDiagnostics()); }
    catch (e) { setError(e instanceof Error ? e.message : 'Diagnostics unavailable'); }
    finally { setBusy(false); }
  };
  return <Card>
    <CardHeader><CardTitle>Configuration overrides</CardTitle><p className="text-xs text-muted-foreground">Check environment variables that may explain why a provider change is not taking effect. Values remain private.</p></CardHeader>
    <CardContent className="space-y-4">
      {error && <ToastNotice tone="error" message={error} onDismiss={() => setError('')} />}
      <Button variant="outline" disabled={busy} onClick={() => void inspect()}>{busy ? 'Checking...' : result ? 'Check again' : 'Check overrides'}</Button>
      {result && <>
        <p className="text-xs text-muted-foreground">This checks the server’s inherited environment and common shell startup files. Shell assignments are potential overrides; files are not executed. Your terminal or client may inherit a different environment.</p>
        {result.findings.length === 0 && <p className="text-sm">No supported override variables found in the inspected sources.</p>}
        <ul className="divide-y divide-border">{result.findings.map(item => <li key={`${item.source}:${item.variable}`} className="py-3 space-y-1"><p className="text-sm font-medium">{item.application} · <span className="font-mono">{item.variable}</span></p><p className="break-all text-xs text-muted-foreground">{item.source} · {item.kind === 'process' ? 'Inherited by server' : 'Potential shell assignment'}</p></li>)}</ul>
        {result.warnings.length > 0 && <div className="text-sm"><p>Some sources could not be checked:</p><ul className="text-xs text-muted-foreground">{result.warnings.map(warning => <li key={warning}>{warning}</li>)}</ul></div>}
        {result.findings.length > 0 && <p className="text-xs text-muted-foreground">Review the named variables in the terminal that launches your client, then start a new client session. This check changes no files.</p>}
      </>}
    </CardContent>
  </Card>;
}
