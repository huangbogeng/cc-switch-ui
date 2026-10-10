import { ToastNotice } from '@/components/ToastNotice';
import { useConfirm } from '@/lib/useConfirm';
import { useCallback, useEffect, useState } from 'react';
import { finishManagedDeletion, getManagedDeletion, type PendingDeletion } from '@/api';
import { Button } from '@/components/ui/button';
import { ErrorAlert } from '@/components/ErrorAlert';
import { useUnsavedChanges } from '@/lib/useUnsavedChanges';

export function DeletionRecoveryPanel({ refreshKey, disabled, onBusyChange, onResolved, onPendingChange }: {
  refreshKey: number; disabled: boolean; onBusyChange: (busy: boolean) => void;
  onResolved: () => void | Promise<void>; onPendingChange: (pending: boolean) => void;
}) {
  const { confirmAction, confirmation } = useConfirm();
  const [pending, setPending] = useState<PendingDeletion | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');
  const [notice, setNotice] = useState('');
  useUnsavedChanges(false, busy);
  const load = useCallback(async (signal?: AbortSignal) => {
    const data = await getManagedDeletion({ signal });
    if (!signal?.aborted) { setPending(data.pending); onPendingChange(data.pending !== null); }
  }, [onPendingChange]);
  useEffect(() => {
    const ctrl = new AbortController();
    load(ctrl.signal).catch((e) => { if (!ctrl.signal.aborted) setError(e instanceof Error ? e.message : 'Failed to load deletion recovery'); });
    return () => ctrl.abort();
  }, [load, refreshKey]);
  const refresh = async () => {
    setBusy(true); onBusyChange(true); setError(''); setNotice('');
    try { await load(); }
    catch (e) { setError(e instanceof Error ? e.message : 'Failed to refresh deletion recovery'); }
    finally { setBusy(false); onBusyChange(false); }
  };
  const finish = async (keepFiles: boolean) => {
    if (!pending) return;
    if (keepFiles && !await confirmAction(`Keep the current live files for ${pending.name} and finish deletion of the managed record? The files remain unmanaged and can still be active in Claude Code. This does not restore the deleted record.`, { title: 'Keep current files?', confirmLabel: 'Keep files' })) return;
    setBusy(true); onBusyChange(true); setError(''); setNotice('');
    try {
      await finishManagedDeletion(pending.operationId, keepFiles);
      setPending(null); onPendingChange(false);
      setNotice(keepFiles ? 'Deletion finished. Current live files were kept unmanaged.' : 'Deletion finished. Captured live configuration was removed; Skills source files and backups are retained.');
      try { await onResolved(); await load(); }
      catch (e) { setError(`Deletion finished, but refreshing records or recovery status failed: ${e instanceof Error ? e.message : 'Unknown error'}`); }
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to finish deletion');
      try { await load(); } catch { /* Keep the pending snapshot and original error for retry. */ }
    } finally { setBusy(false); onBusyChange(false); }
  };
  if (!pending && !error) return notice ? <ToastNotice message={notice} onDismiss={() => setNotice('')} /> : null;
  return (
    <div className="space-y-3 rounded-lg border p-4 text-sm">
      {confirmation}
      <div className="flex items-center justify-between gap-3">
        <h2 className="font-semibold">Deletion recovery</h2>
        <Button size="sm" variant="outline" disabled={disabled || busy} onClick={() => void refresh()}>Refresh deletion status</Button>
      </div>
      {error && <ErrorAlert message={error} />}
      {notice && <ToastNotice message={notice} onDismiss={() => setNotice('')} />}
      {pending && <>
        <p role="status">{pending.kind === 'mcp' ? 'MCP server' : 'Skill'} {pending.name} ({pending.id}): record deleted, live cleanup unfinished.</p>
        <p className="text-muted-foreground">MCP/Skills changes are blocked until this deletion finishes. Retry removes only the captured configuration; changed files are preserved and reported as a conflict. Keep current files leaves them unmanaged and potentially active.</p>
        <div className="flex flex-wrap gap-2">
          <Button disabled={disabled || busy} onClick={() => void finish(false)}>Retry live cleanup</Button>
          <Button variant="secondary" disabled={disabled || busy} onClick={() => void finish(true)}>Keep current files</Button>
        </div>
      </>}
    </div>
  );
}
