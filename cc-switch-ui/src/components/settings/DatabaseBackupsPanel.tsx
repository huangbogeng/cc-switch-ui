import { useConfirm } from '@/lib/useConfirm';
import { useCallback, useEffect, useState } from 'react';
import { getDatabaseBackups, createDatabaseBackup, deleteDatabaseBackup, scheduleDatabaseRestore, cancelDatabaseRestore, type DatabaseBackupStatus, type DatabaseBackup } from '@/api';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { ErrorAlert } from '@/components/ErrorAlert';

export function DatabaseBackupsPanel() {
  const { confirmAction, confirmation } = useConfirm();
  const [status, setStatus] = useState<DatabaseBackupStatus | null>(null);
  const [busy, setBusy] = useState(true);
  const [needsReload, setNeedsReload] = useState(false);
  const [error, setError] = useState('');
  const [notice, setNotice] = useState('');
  const load = useCallback(async (signal?: AbortSignal) => {
    const data = await getDatabaseBackups({ signal });
    if (!signal?.aborted) { setStatus(data); setNeedsReload(false); }
  }, []);
  useEffect(() => {
    const controller = new AbortController();
    load(controller.signal).catch((e) => { if (!controller.signal.aborted) setError(e instanceof Error ? e.message : 'Failed to load backups'); })
      .finally(() => { if (!controller.signal.aborted) setBusy(false); });
    return () => controller.abort();
  }, [load]);
  const run = async (action: () => Promise<unknown>, message: string) => {
    if (busy) return;
    setBusy(true); setError(''); setNotice('');
    let completed = false;
    try {
      await action(); completed = true; setNotice(message); await load();
    } catch (e) {
      setError(completed ? 'The action completed, but refreshing failed. Reload backup status before another action.' : e instanceof Error ? e.message : 'Backup action failed');
      if (completed) setNeedsReload(true);
    } finally { setBusy(false); }
  };
  const restore = async (entry: DatabaseBackup) => {
    if (!await confirmAction(`Restore the database backup from ${new Date(entry.createdAt).toLocaleString()} on the next server restart?\n\nProvider, MCP and Skills records, settings stored in the database, and usage history will be replaced. Live Claude files, installed Skills, OAuth logins and device preferences are not restored. Select a direct provider again afterward.\n\nA safety backup of the current database will be kept before replacement. Changes made before restart will only remain in that safety backup.`, { title: 'Restore configuration?', confirmLabel: 'Restore' })) return;
    void run(() => scheduleDatabaseRestore(entry.id), 'Restore staged. Restart the CC Switch server and reload this page to apply it, then select your direct provider. The current database is unchanged until restart.');
  };
  const remove = async (entry: DatabaseBackup) => {
    if (!await confirmAction(`Delete the ${entry.kind === 'safety' ? 'safety' : 'manual'} backup from ${new Date(entry.createdAt).toLocaleString()}?`, { title: 'Delete managed item?', confirmLabel: 'Delete', destructive: true })) return;
    void run(() => deleteDatabaseBackup(entry.id), 'Backup deleted.');
  };
  const disabled = busy || needsReload;
  return <Card>
    {confirmation}
    <CardHeader><CardTitle>Database Backups</CardTitle>
      <p className="text-sm text-muted-foreground">Save provider, MCP and Skills records, database settings and usage history. Claude files, installed Skills and OAuth logins are separate.</p>
    </CardHeader>
    <CardContent className="space-y-4">
      {error && <ErrorAlert message={error} />}
      {notice && <p role="status" className="text-sm">{notice}</p>}
      <div className="flex flex-wrap gap-2">
        <Button disabled={disabled || !status || status.entries.filter((e) => e.kind === 'manual').length >= status.manualLimit} onClick={() => void run(createDatabaseBackup, 'Database backup created.')}>Create backup</Button>
        <Button variant="outline" disabled={busy} onClick={() => void run(load, 'Backup status reloaded.')}>Reload backups</Button>
      </div>
      {status?.pending && <div role="alert" className="rounded-lg border p-3 text-sm space-y-2">
        <p>{status.pending.applying ? 'Restore started. Restart to complete recovery.' : 'A database restore is waiting for server restart. New changes will be replaced when it applies. Reload this page after restarting.'}</p>
        <Button variant="outline" disabled={disabled || status.pending.applying} onClick={() => void run(cancelDatabaseRestore, 'Planned restore cancelled. The current database was kept.')}>Cancel planned restore</Button>
        <Button variant="outline" disabled={busy} onClick={() => window.location.reload()}>Reload app</Button>
      </div>}
      {busy && !status && <p>Loading database backups…</p>}
      {status && status.entries.length === 0 && <p className="text-sm text-muted-foreground">No database backups yet.</p>}
      {status?.entries.map((entry) => <div key={entry.id} className="flex flex-wrap items-center justify-between gap-3 rounded-lg border p-3">
        <div className="text-sm" title={entry.id}>
          <p>{entry.kind === 'safety' ? 'Safety backup' : 'Manual backup'} · {new Date(entry.createdAt).toLocaleString()}</p>
          <p className="text-muted-foreground">{(entry.sizeBytes / 1024).toFixed(1)} KB{entry.protected ? ' · Protected' : ''}</p>
        </div>
        <div className="flex gap-2">
          <Button variant="outline" disabled={disabled || !!status.pending} onClick={() => restore(entry)}>Restore on restart</Button>
          <Button variant="outline" disabled={disabled || entry.protected} onClick={() => remove(entry)}>Delete backup</Button>
        </div>
      </div>)}
      {status && <p className="break-all text-xs text-muted-foreground">Saved in {status.directory}. Backups contain provider credentials. Up to {status.manualLimit} manual backups; the latest safety backup is protected.</p>}
    </CardContent>
  </Card>;
}
