import { useEffect, useLayoutEffect, useMemo, useRef, useState, type CSSProperties, type KeyboardEvent } from 'react';
import { createPortal } from 'react-dom';
import { Check, ChevronDown } from 'lucide-react';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Button } from '@/components/ui/button';
import { useDialog } from '@/lib/useDialog';

export function ModelMappingInput({ id, label, value, options, disabled = false, onChange }: {
  id: string; label: string; value: string; options: string[]; disabled?: boolean;
  onChange: (value: string) => void;
}) {
  const [open, setOpen] = useState(false);
  const [search, setSearch] = useState('');
  const [limit, setLimit] = useState(100);
  const [position, setPosition] = useState<CSSProperties>();
  const anchor = useRef<HTMLDivElement>(null);
  const menu = useDialog(open, () => setOpen(false));
  const hasOptions = options.length > 0;
  const candidates = useMemo(() => {
    const unique = new Set(options.map(option => option.trim()).filter(Boolean));
    const choices = [...unique];
    if (value.trim() && !unique.has(value.trim())) choices.unshift(value.trim());
    return choices;
  }, [options, value]);
  const matches = candidates.filter(option => option.toLowerCase().includes(search.trim().toLowerCase()));
  const shown = matches.slice(0, limit);
  const popupId = `${id}-menu`;
  const listId = `${id}-choices`;

  useEffect(() => { if (disabled || !hasOptions) setOpen(false); }, [disabled, hasOptions]);
  useLayoutEffect(() => {
    if (!open) return;
    const align = () => {
      const rect = anchor.current?.getBoundingClientRect();
      if (!rect) return;
      const below = window.innerHeight - rect.bottom - 12;
      const above = rect.top - 12;
      const downward = below >= Math.min(280, above);
      const width = Math.min(rect.width, window.innerWidth - 16);
      setPosition({
        position: 'fixed', width,
        left: Math.max(8, Math.min(rect.left, window.innerWidth - width - 8)),
        ...(downward ? { top: rect.bottom + 4 } : { bottom: window.innerHeight - rect.top + 4 }),
        maxHeight: Math.max(0, Math.min(320, downward ? below : above)),
      });
    };
    align();
    window.addEventListener('resize', align);
    document.addEventListener('scroll', align, true);
    const dismiss = (event: PointerEvent) => {
      if (event.target instanceof Node && !anchor.current?.contains(event.target) && !menu.current?.contains(event.target)) setOpen(false);
    };
    document.addEventListener('pointerdown', dismiss);
    return () => {
      window.removeEventListener('resize', align);
      document.removeEventListener('scroll', align, true);
      document.removeEventListener('pointerdown', dismiss);
    };
  }, [open, menu]);

  const expand = () => {
    if (disabled || !hasOptions) return;
    setSearch(''); setLimit(100); setOpen(true);
  };
  const navigate = (event: KeyboardEvent) => {
    const keys = ['ArrowDown', 'ArrowUp', 'Home', 'End'];
    if (!keys.includes(event.key)) return;
    const elements = Array.from(menu.current?.querySelectorAll<HTMLButtonElement>('[role="option"]') ?? []);
    if (!elements.length) return;
    event.preventDefault();
    const index = elements.indexOf(document.activeElement as HTMLButtonElement);
    const next = event.key === 'Home' ? 0 : event.key === 'End' ? elements.length - 1
      : event.key === 'ArrowDown' ? Math.min(index + 1, elements.length - 1) : index < 0 ? elements.length - 1 : Math.max(index - 1, 0);
    elements[next].focus();
    elements[next].scrollIntoView?.({ block: 'nearest' });
  };

  return <div className="min-w-0 space-y-2">
    <Label htmlFor={id}>{label}</Label>
    <div ref={anchor} className="relative min-w-0">
      <Input id={id} role="combobox" aria-haspopup="dialog" aria-expanded={open} aria-controls={open ? popupId : undefined}
        value={value} disabled={disabled} className="pr-11" onChange={event => onChange(event.target.value)}
        onKeyDown={event => { if (event.key === 'ArrowDown' && hasOptions) { event.preventDefault(); expand(); } }} />
      <button type="button" aria-label={`Choose ${label} model`} aria-haspopup="dialog" aria-expanded={open}
        disabled={disabled || !hasOptions} title={hasOptions ? 'Choose from fetched models' : 'Fetch models to load choices'}
        className="absolute inset-y-0 right-0 flex w-10 items-center justify-center rounded-r-lg border-l border-input text-muted-foreground hover:bg-accent/40 hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-40"
        onClick={() => { if (open) setOpen(false); else expand(); }}>
        <ChevronDown className="h-4 w-4" />
      </button>
    </div>
    {open && createPortal(<div ref={menu} id={popupId} role="dialog" aria-modal="true" aria-label={`${label} model choices`} tabIndex={-1}
      style={{
        ...(position ?? { visibility: 'hidden' }),
        // The shared modal scale animation would move the anchored menu edges.
        animation: 'none',
      }} className="z-[70] flex min-h-0 flex-col overflow-hidden rounded-lg border border-border bg-popover text-popover-foreground shadow-xl">
      <div className="shrink-0 border-b border-border p-2">
        <Input aria-label={`Search ${label} models`} placeholder="Search fetched models..." value={search}
          aria-controls={listId} onChange={event => { setSearch(event.target.value); setLimit(100); }}
          onKeyDown={event => { if (event.key === 'ArrowDown' || event.key === 'ArrowUp') navigate(event); }} />
      </div>
      <div id={listId} role="listbox" aria-label={`${label} models`} className="min-h-0 overflow-y-auto p-1" onKeyDown={navigate}>
        {shown.map(option => <button key={option} type="button" role="option" aria-selected={option === value.trim()} title={option}
          className="flex w-full min-w-0 items-center gap-2 rounded-md px-2 py-2 text-left hover:bg-accent focus:bg-accent focus:outline-none"
          onClick={() => { onChange(option); setOpen(false); }}>
          <span className="min-w-0 flex-1 truncate font-mono text-xs">{option}</span>
          {option === value.trim() && <Check aria-hidden="true" className="h-4 w-4 shrink-0 text-primary" />}
        </button>)}
        {!shown.length && <p className="px-2 py-3 text-xs text-muted-foreground">No matching models. You can still type a model ID.</p>}
      </div>
      <div className="flex shrink-0 items-center justify-between gap-2 border-t border-border px-3 py-2 text-xs text-muted-foreground">
        <span>{matches.length} models</span>
        {matches.length > limit && <Button type="button" variant="ghost" size="sm" onClick={() => setLimit(count => count + 100)}>Show more</Button>}
      </div>
    </div>, document.body)}
  </div>;
}
