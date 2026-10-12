import { ToastNotice } from '@/components/ToastNotice';
import { useDialog } from '@/lib/useDialog';
import { useConfirm } from '@/lib/useConfirm';
import { useCallback, useEffect, useState } from 'react';
import {
  getClaudeUserSettings, saveClaudeUserSettings, restoreClaudeUserSettings, previewClaudeUserSettings,
  type ClaudeUserSettings, type ClaudeUserValues, type ClaudeSettingsPreview,
} from '@/api';
import { useUnsavedChanges } from '@/lib/useUnsavedChanges';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Select } from '@/components/ui/select';
import { ErrorAlert } from '@/components/ErrorAlert';

const defaults: ClaudeUserValues = { timeoutMs: null, nonessentialTraffic: null };

export function ClaudeUserSettingsPanel() {
  const { confirmAction, confirmation } = useConfirm();
  const [conflict, setConflict] = useState<{ keys: string[]; revision: string } | null>(null);
  const [preview, setPreview] = useState<ClaudeSettingsPreview | null>(null);
  const [snapshot, setSnapshot] = useState<ClaudeUserSettings | null>(null);
  const [values, setValues] = useState<ClaudeUserValues>(defaults);
  const [baseline, setBaseline] = useState(defaults);
  const [busy, setBusy] = useState(true);
  const conflictRef = useDialog(conflict !== null, () => setConflict(null), !busy);
  const [needsReload, setNeedsReload] = useState(false);
  const [error, setError] = useState('');
  const [notice, setNotice] = useState('');
  const dirty = JSON.stringify(values) !== JSON.stringify(baseline);
  const canDiscard = useUnsavedChanges(dirty, busy);

  const load = useCallback(async (signal?: AbortSignal) => {
    const data = await getClaudeUserSettings({ signal });
    if (signal?.aborted) return;
    setPreview(null);
    setSnapshot(data);
    setValues(data.values);
    setBaseline(data.values);
    setNeedsReload(false);
  }, []);

  useEffect(() => {
    const ctrl = new AbortController();
    load(ctrl.signal)
      .catch((e) => { if (!ctrl.signal.aborted) setError(e instanceof Error ? e.message : 'Failed to load user settings'); })
      .finally(() => { if (!ctrl.signal.aborted) setBusy(false); });
    return () => ctrl.abort();
  }, [load]);

  const reload = async () => {
    if (!canDiscard()) return;
    setBusy(true);
    setError('');
    try { await load(); }
    catch (e) { setError(e instanceof Error ? e.message : 'Failed to reload user settings'); }
    finally { setBusy(false); }
  };

  const inspectChanges = async () => {
    if (!snapshot || busy || needsReload) return;
    setBusy(true); setError(''); setPreview(null);
    try { setPreview(await previewClaudeUserSettings(snapshot.revision, values, snapshot.values)); }
    catch (e) { setError(e instanceof Error ? e.message : 'Preview unavailable'); }
    finally { setBusy(false); }
  };

  const submit = async (restore: boolean, policy: 'refuse' | 'keepMine' | 'keepTheirs' = 'refuse') => {
    if (!snapshot || busy || needsReload) return;
    setError('');
    setNotice('');
    if (restore) {
      const original = snapshot.initialValues;
      if (!original || !await confirmAction(`Restore these two user settings from the original configuration backup?\n\nAPI Timeout MS: ${original.timeoutMs ?? 'Claude Code default'}\nNonessential traffic: ${original.nonessentialTraffic === '1' ? 'Disabled' : original.nonessentialTraffic === '0' ? 'Allowed' : original.nonessentialTraffic ?? 'Claude Code default'}\n\nAny unsaved changes in this panel will be discarded. Provider credentials and other settings are preserved.`, { title: 'Restore configuration?', confirmLabel: 'Restore' })) return;
    } else if (values.timeoutMs !== baseline.timeoutMs && values.timeoutMs !== null
      && (!/^[0-9]+$/.test(values.timeoutMs) || BigInt(values.timeoutMs) <= 0n || BigInt(values.timeoutMs) > 18446744073709551615n)) {
      setError('API timeout must be a positive integer in milliseconds, or blank to use Claude Code default.');
      return;
    }
    setBusy(true);
    try {
      if (restore) await restoreClaudeUserSettings(snapshot.revision);
      else await saveClaudeUserSettings(snapshot.revision, values, snapshot.values, policy, conflict?.revision);
      setConflict(null);
      setPreview(null);
      setNotice(restore ? 'User settings restored to the live file.' : 'User settings saved to the live file.');
      // A successful write is complete even if the follow-up read fails.
      setNeedsReload(true);
      if (!restore) setBaseline(values);
      else setBaseline(snapshot.initialValues!);
      if (restore) setValues(snapshot.initialValues!);
      try { await load(); }
      catch (e) { setError(`The write completed, but refreshing failed: ${e instanceof Error ? e.message : 'Unknown error'}. Reload from file before editing again.`); }
    } catch (e) {
      if (!restore && e instanceof Error && 'conflictKeys' in e && 'conflictRevision' in e && Array.isArray(e.conflictKeys) && typeof e.conflictRevision === 'string') {
        setConflict({ keys: e.conflictKeys as string[], revision: e.conflictRevision });
        return;
      }
      setError(`${e instanceof Error ? e.message : 'Failed to update user settings'}. Reload from file to review the latest values, then retry.`);
    } finally { setBusy(false); }
  };

  return (
    <Card>
      {confirmation}
      {conflict && <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-4">
        <div ref={conflictRef} role="dialog" aria-modal="true" aria-labelledby="settings-conflict-title" tabIndex={-1} className="dialog-panel w-full max-w-lg p-6 space-y-4">
          <h2 id="settings-conflict-title" className="text-lg font-semibold">Settings changed externally</h2>
          <p className="text-sm text-muted-foreground">Choose which conflicting changes to keep. Your other edits will be saved; fields you did not edit keep their latest values.</p>
          <ul className="space-y-1 text-xs font-mono">{conflict.keys.map(key => <li key={key}>{key}</li>)}</ul>
          {conflict.keys.length === 0 && <p className="text-sm">The file changed again. Review and confirm before retrying.</p>}
          <div className="flex flex-wrap justify-end gap-2">
            <Button variant="outline" disabled={busy} onClick={() => setConflict(null)}>Cancel</Button>
            <Button variant="outline" disabled={busy} onClick={() => void submit(false, 'keepTheirs')}>Keep external changes</Button>
            <Button disabled={busy} onClick={() => void submit(false, 'keepMine')}>Use my changes</Button>
          </div>
        </div>
      </div>}
      <CardHeader>
        <CardTitle>Claude Code User Settings</CardTitle>
        <p className="text-xs text-muted-foreground">Saved directly to the live file. Provider switching preserves these settings. Start a new Claude Code session to pick up changes.</p>
      </CardHeader>
      <CardContent className="space-y-4">
        {snapshot && <p className="break-all font-mono text-xs text-muted-foreground">{snapshot.path}</p>}
        {error && <ErrorAlert message={error} />}
        {notice && <ToastNotice message={notice} onDismiss={() => setNotice('')} />}
        {snapshot && <fieldset disabled={busy || needsReload || conflict !== null} className="space-y-4">
          <div className="space-y-2">
            <Label htmlFor="claude-api-timeout">API Timeout MS</Label>
            <Input id="claude-api-timeout" inputMode="numeric" value={values.timeoutMs ?? ''}
              placeholder="Claude Code default" onChange={(e) => { setNotice(''); setPreview(null); setValues({ ...values, timeoutMs: e.target.value || null }); }} />
            <p className="text-xs text-muted-foreground">Leave blank to remove the override.</p>
          </div>
          <div className="space-y-2">
            <Label htmlFor="claude-nonessential-traffic">Nonessential traffic</Label>
            <Select id="claude-nonessential-traffic" value={values.nonessentialTraffic ?? 'default'}
              onChange={(e) => { setNotice(''); setPreview(null); setValues({ ...values, nonessentialTraffic: e.target.value === 'default' ? null : e.target.value }); }}>
              <option value="default">Use Claude Code default</option>
              <option value="1">Disable nonessential traffic</option>
              <option value="0">Allow nonessential traffic</option>
              {values.nonessentialTraffic !== null && !['0', '1'].includes(values.nonessentialTraffic)
                && <option value={values.nonessentialTraffic}>Current value: {values.nonessentialTraffic}</option>}
            </Select>
          </div>
        </fieldset>}
        {preview && <div className="rounded-xl border border-border p-4 space-y-3">
          <p className="text-sm font-medium">Changes to the live file</p>
          <p className="break-all text-xs font-mono text-muted-foreground">{preview.path}</p>
          {preview.changes.length === 0 ? <p className="text-sm">The live file already has these values.</p> : <ul className="space-y-2 text-xs">{preview.changes.map(change => <li key={change.field}><p className="font-mono">{change.field}</p><p className="text-muted-foreground">{change.before ?? 'Claude Code default'} → {change.after ?? 'Claude Code default'}</p></li>)}</ul>}
          <p className="text-xs text-muted-foreground">These are global user settings. Provider credentials and other fields are preserved. Preview changes no files; saving checks the latest file again.</p>
        </div>}
        <div className="flex flex-wrap gap-2">
          <Button variant="outline" disabled={!snapshot || busy || needsReload || !dirty || conflict !== null} onClick={() => void inspectChanges()}>Preview changes</Button>
          <Button disabled={!snapshot || busy || needsReload || !dirty || conflict !== null} onClick={() => void submit(false)}>Save User Settings</Button>
          <Button variant="secondary" disabled={busy} onClick={() => void reload()}>Reload from file</Button>
          <Button variant="secondary" disabled={!snapshot?.initialValues || busy || needsReload} onClick={() => void submit(true)}>Restore user settings</Button>
        </div>
        <p className="text-xs text-muted-foreground">Restore uses the configuration saved before CC Switch first changed this file and changes only the two fields above.</p>
      </CardContent>
    </Card>
  );
}
