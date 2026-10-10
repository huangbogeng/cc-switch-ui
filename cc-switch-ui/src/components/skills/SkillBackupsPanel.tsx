import { useState } from 'react';
import { ApiError, listSkillBackups, restoreSkillBackup, type SkillBackup } from '@/api';
import { Card, CardContent } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Select } from '@/components/ui/select';
import { Label } from '@/components/ui/label';
import { ErrorAlert } from '@/components/ErrorAlert';
import { useUnsavedChanges } from '@/lib/useUnsavedChanges';

export function SkillBackupsPanel({ disabled, onRestored, onBusyChange }: {
  disabled: boolean;
  onRestored: () => void | Promise<void>;
  onBusyChange: (busy: boolean) => void;
}) {
  const [backups, setBackups] = useState<SkillBackup[]>([]);
  const [selected, setSelected] = useState('');
  const [loaded, setLoaded] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');
  const [notice, setNotice] = useState('');
  useUnsavedChanges(false, busy);

  const refresh = async () => {
    const data = await listSkillBackups();
    setBackups(data.backups);
    setSelected((previous) => data.backups.some((backup) => backup.id === previous) ? previous : (data.backups[0]?.id ?? ''));
    setLoaded(true);
  };

  const start = () => { setBusy(true); onBusyChange(true); setError(''); setNotice(''); };
  const finish = () => { setBusy(false); onBusyChange(false); };
  const load = async () => {
    start();
    try { await refresh(); }
    catch (e) { setError(e instanceof Error ? e.message : 'Failed to load backups'); }
    finally { finish(); }
  };

  const restore = async () => {
    const backup = backups.find((item) => item.id === selected);
    if (!backup || !window.confirm(`Restore source files for ${backup.directory}? Current files will be backed up. Enabled skills will be synced; disabled skills stay disabled. A deleted skill will be recreated and enabled.`)) return;
    start();
    try {
      const result = await restoreSkillBackup(backup.id);
      setNotice(result.enabled ? 'Skill restored and synced to Claude Code.' : 'Source files restored. This skill remains disabled.');
      await onRestored();
      try { await refresh(); }
      catch (e) { setError(`Restore completed, but backup refresh failed: ${e instanceof Error ? e.message : 'Refresh failed'}`); }
    } catch (e) {
      if (e instanceof ApiError && e.saved) { await onRestored(); }
      setError(e instanceof Error ? e.message : 'Restore failed');
    } finally { finish(); }
  };

  return (
    <Card>
      <CardContent className="space-y-3 p-4">
        <div className="flex items-start justify-between gap-4">
          <div>
            <h2 className="text-sm font-semibold">Skill backups</h2>
            <p className="mt-1 text-xs text-muted-foreground">Recover files from before replacement, disabling, or deletion. Selected backups are kept; the current source files are backed up before recovery.</p>
          </div>
          <Button size="sm" variant="outline" onClick={load} disabled={disabled || busy}>{busy ? 'Working...' : loaded ? 'Refresh backups' : 'Load backups'}</Button>
        </div>
        {error && <ErrorAlert message={error} />}
        {notice && <p role="status" className="text-sm text-emerald-400">{notice}</p>}
        {loaded && backups.length === 0 && <p className="text-sm text-muted-foreground">No skill backups available.</p>}
        {backups.length > 0 && (
          <div className="flex flex-wrap items-end gap-3">
            <div className="min-w-0 flex-1 space-y-1">
              <Label htmlFor="skill-backup">Backup version</Label>
              <Select id="skill-backup" value={selected} onChange={(event) => setSelected(event.target.value)} disabled={disabled || busy}>
                {backups.map((backup) => <option key={backup.id} value={backup.id}>{backup.directory} · {backup.origin === 'live' ? 'Claude Code' : 'Source'} · {backup.id.slice(-8)}{backup.modifiedAt > 0 ? ` · modified ${new Date(backup.modifiedAt * 1000).toLocaleString()}` : ''}</option>)}
              </Select>
            </div>
            <Button size="sm" onClick={restore} disabled={disabled || busy || !selected}>Restore files</Button>
          </div>
        )}
      </CardContent>
    </Card>
  );
}
