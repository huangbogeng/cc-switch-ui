import { useEffect, useId, useState } from 'react';
import { listImportCandidates, listRemovedItems, type ManagedKind, type ImportCandidate } from '@/api';
import { useDialog } from '@/lib/useDialog';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { errorMessage } from '@/lib/errors';

export function ManagedImportDialog({ kind, mode, onClose, onApply }: {
  kind: ManagedKind; mode: 'Import' | 'Restore'; onClose: () => void; onApply: (ids: string[]) => Promise<void>;
}) {
  const [items, setItems] = useState<ImportCandidate[]>([]);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [search, setSearch] = useState('');
  const [scanVersion, setScanVersion] = useState(0);
  const [visibleCount, setVisibleCount] = useState(80);
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');
  const titleId = useId();
  const dialogRef = useDialog(true, onClose, !busy);
  useEffect(() => {
    let active = true;
    setLoading(true); setError(''); setSelected(new Set()); setItems([]); setVisibleCount(80);
    const request = mode === 'Restore' ? listRemovedItems(kind).then(items => ({ items, report: null })) : listImportCandidates(kind);
    request.then(data => {
      if (!active) return;
      setItems(data.items);
      if (data.report?.failed) setError(data.report.items.filter(item => item.error).map(item => `${item.id}: ${item.error}`).join('\n'));
    }).catch(e => { if (active) setError(errorMessage(e, 'Scan failed')); }).finally(() => { if (active) setLoading(false); });
    return () => { active = false; };
  }, [kind, mode, scanVersion]);
  const visible = items.filter(item => `${item.name} ${item.id} ${item.source}`.toLowerCase().includes(search.trim().toLowerCase()));
  const apply = async () => {
    if (busy || selected.size === 0 || selected.size > 2000) return;
    setBusy(true);
    try { await onApply([...selected]); onClose(); }
    catch (e) { setError(errorMessage(e, `${mode} failed`)); }
    finally { setBusy(false); }
  };
  return <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-4" onClick={() => { if (!busy) onClose(); }}>
    <div ref={dialogRef} role="dialog" aria-modal="true" aria-labelledby={titleId} tabIndex={-1} className="dialog-panel flex w-full max-w-xl flex-col" onClick={e => e.stopPropagation()}>
      <div className="border-b border-border p-5 sm:p-6">
        <h2 id={titleId} className="text-lg font-semibold">{mode === 'Restore' ? 'Restore removed items' : `Import ${kind === 'skill' ? 'Skills' : 'MCP servers'}`}</h2>
        <p className="mt-1 text-sm text-muted-foreground">{mode === 'Restore' ? 'Restore selected records as disabled. Enable them when ready.' : 'Choose what to manage. Existing states are preserved; removed items are excluded. Imported items start enabled; use Sync to apply them.'}</p>
      </div>
      <div className="dialog-body space-y-4 p-5 sm:p-6">
        {mode === 'Import' && items.some(item => item.source === 'Plugin') && <p className="text-xs text-muted-foreground">Plugin originals remain installed. These controls manage the copy in Claude Code’s skills directory.</p>}
        <Input aria-label="Search import candidates" placeholder="Search..." value={search} onChange={e => { setSearch(e.target.value); setVisibleCount(80); }} disabled={busy} />
        {error && <div className="space-y-2"><p role="alert" className="whitespace-pre-line text-sm text-destructive">{error}</p><Button variant="outline" size="sm" disabled={busy || loading} onClick={() => setScanVersion(value => value + 1)}>Rescan</Button><p className="text-xs text-muted-foreground">Rescan refreshes the list and clears your selection.</p></div>}
        {loading ? <p className="text-sm text-muted-foreground">Scanning...</p> : <>
          <div className="flex items-center justify-between text-sm text-muted-foreground">
            <span>{selected.size} selected · {items.length} available</span>
            <Button variant="ghost" size="sm" disabled={busy || visible.length === 0} onClick={() => setSelected(previous => { const next = new Set(previous); const all = visible.every(item => next.has(item.id)); visible.forEach(item => { if (all) next.delete(item.id); else next.add(item.id); }); if (next.size > 2000) { setError('Select up to 2,000 items at a time.'); return previous; } return next; })}>Select matching</Button>
          </div>
          <div className="max-h-72 divide-y divide-border overflow-y-auto rounded-xl border border-border">
            {visible.slice(0, visibleCount).map(item => <label key={item.id} className="flex cursor-pointer items-start gap-3 px-4 py-3 hover:bg-accent/30">
              <input type="checkbox" className="mt-1 accent-primary" checked={selected.has(item.id)} disabled={busy || (!selected.has(item.id) && selected.size >= 2000)} onChange={() => setSelected(previous => { const next = new Set(previous); if (next.has(item.id)) next.delete(item.id); else next.add(item.id); return next; })} />
              <span className="min-w-0"><span className="block break-words text-sm font-medium">{item.name}</span><span className="block break-words text-xs text-muted-foreground">{item.id} · {item.source}</span></span>
            </label>)}
            {visible.length === 0 && <p className="px-4 py-8 text-center text-sm text-muted-foreground">{error && items.length === 0 ? 'Items could not be loaded. Use Rescan to retry.' : items.length ? 'No matching items.' : mode === 'Restore' ? 'No removed items.' : 'No new items to import.'}</p>}
          </div>
          {visible.length > visibleCount && <Button variant="outline" disabled={busy} onClick={() => setVisibleCount(count => count + 80)}>Show next {Math.min(80, visible.length - visibleCount)} items</Button>}
        </>}
      </div>
      <div className="dialog-actions flex justify-end gap-2 border-t border-border p-5 sm:p-6">
        <Button variant="outline" disabled={busy} onClick={onClose}>Cancel</Button>
        <Button disabled={busy || loading || selected.size === 0 || selected.size > 2000} onClick={apply}>{busy ? 'Applying...' : `${mode} selected`}</Button>
      </div>
    </div>
  </div>;
}
