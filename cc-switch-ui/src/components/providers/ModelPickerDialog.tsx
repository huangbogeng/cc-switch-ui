import { useEffect, useId, useMemo, useState } from 'react';
import type { FetchedModel } from '@/api';
import { useDialog } from '@/lib/useDialog';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Select } from '@/components/ui/select';
import { Label } from '@/components/ui/label';
export type ModelRole = 'model' | 'haikuModel' | 'sonnetModel' | 'opusModel';
const roles: [ModelRole, string][] = [['model','Main'],['haikuModel','Haiku'],['sonnetModel','Sonnet'],['opusModel','Opus']];
export function ModelPickerDialog({ models, configured, loading, error, onRefresh, onApply, onClose }: {
  models: FetchedModel[]; configured: Record<ModelRole, string>; loading: boolean; error: string;
  onRefresh: () => void; onApply: (role: ModelRole, id: string) => void; onClose: () => void;
}) {
  const [role, setRole] = useState<ModelRole>('model'); const [selected, setSelected] = useState(configured.model);
  const [search, setSearch] = useState(''); const [limit, setLimit] = useState(100);
  const title = useId(); const searchId = useId(); const roleId = useId(); const selectedId = useId(); const radioName = useId();
  const ref = useDialog(true, onClose);
  useEffect(() => { ref.current?.querySelector<HTMLInputElement>('input')?.focus(); }, [ref]);
  const candidates = useMemo(() => {
    const unique = new Map<string, FetchedModel>();
    for (const model of models) { const id = model.id.trim(); if (id && !unique.has(id)) unique.set(id, { ...model, id }); }
    const current = configured[role].trim();
    const result = [...unique.values()];
    if (current && !unique.has(current)) result.unshift({ id:current, ownedBy:null });
    return result;
  }, [models, configured, role]);
  const query = search.trim().toLowerCase(); const visible = candidates.filter(model => `${model.id} ${model.ownedBy ?? ''}`.toLowerCase().includes(query));
  return <div className="fixed inset-0 z-[60] flex items-center justify-center bg-black/50 p-4" onClick={onClose}>
    <div ref={ref} role="dialog" aria-modal="true" aria-labelledby={title} tabIndex={-1} className="dialog-panel w-full max-w-xl p-5 sm:p-6 space-y-4" onClick={e => e.stopPropagation()}>
      <div className="flex items-start justify-between gap-3"><div><h2 id={title} className="text-lg font-semibold">Choose model</h2><p className="mt-1 text-xs text-muted-foreground">Apply one model to a purpose in this draft. Save the provider when ready.</p></div><Button variant="outline" size="sm" disabled={loading} onClick={onRefresh}>{loading ? 'Refreshing...' : 'Refresh models'}</Button></div>
      <div className="grid gap-3 sm:grid-cols-2"><div className="space-y-1"><Label htmlFor={roleId}>Use for</Label><Select id={roleId} value={role} disabled={loading} onChange={e => { const next = e.target.value as ModelRole; setRole(next); setSelected(configured[next]); }}>
        {roles.map(([value,label]) => <option key={value} value={value}>{label}</option>)}
      </Select></div><div className="space-y-1"><Label htmlFor={searchId}>Search models</Label><Input id={searchId} value={search} placeholder="Model ID or owner..." onChange={e => { setSearch(e.target.value); setLimit(100); }} /></div></div>
      {error && <p role="alert" className="text-xs text-destructive">{error} {models.length > 0 && 'The previous model list is retained.'}</p>}
      <p className="text-xs text-muted-foreground">Current {roles.find(([key]) => key === role)?.[1]}: <span className="font-mono">{configured[role] || 'Unset'}</span> · {visible.length} matching models</p>
      <div className="max-h-64 overflow-y-auto divide-y divide-border rounded-xl border border-border">{visible.slice(0,limit).map(model => <label key={model.id} className="flex items-start gap-3 px-4 py-3 cursor-pointer hover:bg-accent/30"><input type="radio" name={radioName} checked={selected === model.id} disabled={loading} onChange={() => setSelected(model.id)} className="mt-1 accent-primary" /><span className="min-w-0"><span className="block break-all font-mono text-sm">{model.id}</span><span className="block break-words text-xs text-muted-foreground">{model.id === configured[role] ? 'Current value' : model.ownedBy || 'Provider model'}</span></span></label>)}
        {visible.length === 0 && <p className="p-6 text-center text-sm text-muted-foreground">No matching models. You can enter a model ID below.</p>}
      </div>
      {visible.length > limit && <Button variant="outline" size="sm" onClick={() => setLimit(value => value + 100)}>Show next {Math.min(100,visible.length-limit)} models</Button>}
      <div className="space-y-1"><Label htmlFor={selectedId}>Model ID</Label><Input id={selectedId} value={selected} disabled={loading} onChange={e => setSelected(e.target.value)} /><p className="text-xs text-muted-foreground">Manual values remain available even when the provider does not list them.</p></div>
      <div className="flex justify-end gap-2"><Button variant="outline" onClick={onClose}>Cancel</Button><Button disabled={loading || !selected.trim()} onClick={() => onApply(role,selected.trim())}>Apply model</Button></div>
    </div>
  </div>;
}
