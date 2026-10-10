import { useState } from 'react';
import { listFileBackups, type FileBackup } from '@/api';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { ErrorAlert } from '@/components/ErrorAlert';

export function FileBackupsPanel() {
  const [items, setItems] = useState<FileBackup[]>([]);
  const [loaded, setLoaded] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');
  const load = async () => {
    setBusy(true);
    setError('');
    try { setItems((await listFileBackups()).backups); setLoaded(true); }
    catch (e) { setError(e instanceof Error ? e.message : 'Failed to load file backups'); }
    finally { setBusy(false); }
  };
  return (
    <Card>
      <CardHeader>
        <CardTitle>Configuration file backups</CardTitle>
        <p className="text-xs text-muted-foreground">Original files saved before CC Switch first changed them. These full-file snapshots can contain credentials. Repeated changes keep the first snapshot.</p>
      </CardHeader>
      <CardContent className="space-y-4">
        <Button variant="outline" size="sm" onClick={() => void load()} disabled={busy}>{busy ? 'Loading...' : loaded ? 'Refresh file backups' : 'Load file backups'}</Button>
        {error && <ErrorAlert message={error} />}
        {error && loaded && <p className="text-xs text-muted-foreground">The list below is from the previous successful refresh.</p>}
        {loaded && items.map((item) => (
          <div key={item.id} className="space-y-2 rounded-xl border border-white/[0.07] p-4 text-sm">
            <h3 className="font-semibold">{item.label}</h3>
            <p className="break-all font-mono text-xs text-muted-foreground">Live file: {item.sourcePath}</p>
            {item.status === 'missing' && <p>No first-write backup for this path yet.</p>}
            {item.status === 'invalid' && <ErrorAlert message={`Backup unavailable: ${item.error ?? 'Snapshot validation failed'}`} />}
            {item.status === 'originally-absent' && <p>The file did not originally exist. No original file was saved. Review the current file before removing any configuration.</p>}
            {item.status === 'available' && <>
              <p>{item.sizeBytes} bytes{item.modifiedAt !== null ? ` · marker modified ${new Date(item.modifiedAt * 1000).toLocaleString()}` : ''}</p>
              <p className="break-all font-mono text-xs">Original file: {item.originalPath}</p>
            </>}
            {item.status !== 'missing' && <p className="break-all font-mono text-xs text-muted-foreground">Source marker: {item.markerPath}</p>}
          </div>
        ))}
        <details className="text-sm">
          <summary className="cursor-pointer font-medium">Recover a complete configuration file manually</summary>
          <ol className="mt-3 list-decimal space-y-2 pl-5 text-muted-foreground">
            <li>Stop the local route and recover unfinished provider operations, then stop CC Switch and other writers.</li>
            <li>Keep a separate copy of the current live file.</li>
            <li>For an available backup, copy the Original file to its displayed Live file path. The complete file is replaced, including credentials, permissions and unrelated settings.</li>
            <li>Restart CC Switch and review the file and managed records before switching a provider or syncing MCP. Database records stay unchanged; these operations can apply them again.</li>
          </ol>
        </details>
        <p className="text-xs text-muted-foreground">To restore only API Timeout MS and nonessential traffic, use <button type="button" className="text-primary underline" onClick={() => document.getElementById('claude-user-settings')?.scrollIntoView({ behavior: 'smooth' })}>Restore user settings</button>. It preserves credentials and other live settings.</p>
      </CardContent>
    </Card>
  );
}
