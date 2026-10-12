import { useEffect, useRef, useState } from 'react';
import { Loader2 } from 'lucide-react';
import { fetchCodexModels } from '@/api';
import { Button } from '@/components/ui/button';
import type { ModelRole } from './ModelPickerDialog';

const roles: [ModelRole, string][] = [['model', 'Main'], ['haikuModel', 'Haiku'], ['sonnetModel', 'Sonnet'], ['opusModel', 'Opus']];

export function CodexDiagnostics({ accountId, resolvedAccountId, canCheck, configured, disabled = false }: {
  accountId?: string; resolvedAccountId?: string; canCheck: boolean;
  configured: Record<ModelRole, string>; disabled?: boolean;
}) {
  const [checking, setChecking] = useState(false);
  const [models, setModels] = useState<string[] | null>(null);
  const [error, setError] = useState('');
  const request = useRef(0);
  const context = JSON.stringify([accountId, resolvedAccountId, canCheck]);
  useEffect(() => {
    request.current += 1; setModels(null); setError(''); setChecking(false);
    return () => { request.current += 1; };
  }, [context]);

  const check = async () => {
    if (disabled || checking || !canCheck) return;
    const current = ++request.current;
    setChecking(true); setError(''); setModels(null);
    try {
      const result = await fetchCodexModels(accountId?.trim() || undefined);
      if (current !== request.current) return;
      setModels([...new Set(result.models.map(model => model.id.trim()).filter(Boolean))]);
    } catch (failure) {
      if (current === request.current) setError(failure instanceof Error ? failure.message : 'Codex OAuth connection check failed.');
    } finally {
      if (current === request.current) setChecking(false);
    }
  };

  return <div className="space-y-3 rounded-xl border border-white/[0.07] bg-white/[0.025] px-4 py-3">
    <p className="text-sm text-muted-foreground">Codex OAuth uses a fixed Responses connection. Check the selected account's access to its model catalog.</p>
    <Button type="button" size="sm" variant="outline" disabled={disabled || checking || !canCheck} onClick={() => void check()}>
      {checking && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
      {checking ? 'Checking OAuth...' : 'Check OAuth connection'}
    </Button>
    {!canCheck && <p className="text-xs text-muted-foreground">Connect an account on the OAuth page and select a connected account to run diagnostics.</p>}
    {error && <p role="alert" className="text-sm text-destructive">{error}</p>}
    {models !== null && <div className="space-y-3">
      <p role="status" className="text-sm text-foreground">OAuth model catalog reached successfully. {models.length} models returned.</p>
      <dl className="divide-y divide-border rounded-lg border border-border px-3">
        {roles.map(([key, label]) => {
          const model = configured[key].trim();
          const listed = models.includes(model.replace(/\[1m\]$/i, '').trim());
          return <div key={key} className="grid min-w-0 gap-1 py-2 sm:grid-cols-[4rem_minmax(0,1fr)_auto] sm:items-center">
            <dt className="text-xs text-muted-foreground">{label}</dt>
            <dd className="min-w-0 break-all font-mono text-xs">{model || 'Unset'}</dd>
            <dd className="text-xs text-muted-foreground">{!model ? 'Unset' : listed ? 'Listed in catalog' : 'Not listed in catalog'}</dd>
          </div>;
        })}
      </dl>
      <p className="text-xs text-muted-foreground">Catalog entries describe the account's model list. Verify generation access with a request through the local route; manually configured models may work without being listed.</p>
    </div>}
  </div>;
}
