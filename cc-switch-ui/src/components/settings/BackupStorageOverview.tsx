import { useState } from 'react';
import { getBackupStorage, type BackupLocation } from '@/api';
import { Button } from '@/components/ui/button';
import { Card, CardHeader, CardTitle, CardContent } from '@/components/ui/card';
import { ToastNotice } from '@/components/ToastNotice';
function size(bytes: number) { if (bytes < 1024) return `${bytes} B`; if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KiB`; return `${(bytes / (1024 * 1024)).toFixed(1)} MiB`; }
export function BackupStorageOverview() {
  const [locations, setLocations] = useState<BackupLocation[] | null>(null);
  const [busy, setBusy] = useState(false); const [error, setError] = useState(''); const [notice, setNotice] = useState('');
  const inspect = async () => {
    if (busy) return; setBusy(true); setError(''); setLocations(null);
    try { setLocations((await getBackupStorage()).locations); }
    catch (e) { setError(e instanceof Error ? e.message : 'Backup inventory unavailable'); }
    finally { setBusy(false); }
  };
  const copy = async (path: string) => { try { await navigator.clipboard.writeText(path); setNotice('Backup location copied.'); } catch { setError('Could not copy the location. Select and copy the path below.'); } };
  return <Card><CardHeader><CardTitle>Backup storage overview</CardTitle><p className="text-xs text-muted-foreground">Find database, file, Skills and installation backups in one place. Inventory checks read metadata only.</p></CardHeader>
    <CardContent className="space-y-4">
      {error && <ToastNotice tone="error" message={error} onDismiss={() => setError('')} />}
      {notice && <ToastNotice message={notice} onDismiss={() => setNotice('')} />}
      <Button variant="outline" disabled={busy} onClick={() => void inspect()}>{busy ? 'Inspecting...' : locations ? 'Refresh inventory' : 'Inspect backup storage'}</Button>
      {locations && <><p className="text-xs text-muted-foreground">Counts are top-level entries, including metadata or incomplete upgrade folders. Sizes are logical file bytes; times are the latest observed file or folder modification. Inventory does not validate a backup for restoration.</p>
        <div className="grid gap-3 lg:grid-cols-2">{locations.map(item => <div key={item.id} className="min-w-0 rounded-xl border border-border p-4 space-y-3">
          <h3 className="text-sm font-semibold">{item.label}</h3><p className="text-xs text-muted-foreground">{item.scope}</p>
          <p className="text-xs">{item.status === 'missing' ? 'No backup directory yet.' : item.status === 'unavailable' ? 'Location unavailable.' : `${item.entries} top-level entries · ${size(item.sizeBytes)}${item.status === 'partial' ? ' · Incomplete totals' : ''}`}</p>
          {item.modifiedAt !== null && <p className="text-xs text-muted-foreground">Last modified: {new Date(item.modifiedAt * 1000).toLocaleString()}</p>}
          {item.path && <div className="space-y-2"><p className="break-all select-all font-mono text-xs">{item.path}</p><Button size="sm" variant="ghost" onClick={() => void copy(item.path!)}>Copy location</Button></div>}
          <p className="text-xs text-muted-foreground">{item.recovery}</p>
          {item.warnings.length > 0 && <ul className="space-y-1 text-xs text-destructive">{item.warnings.map((warning, index) => <li key={index}>{warning}</li>)}</ul>}
        </div>)}</div>
      </>}
    </CardContent></Card>;
}
