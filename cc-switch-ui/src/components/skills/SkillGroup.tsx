import { memo, useState } from 'react';
import { ChevronRight, Pencil, Trash2 } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { cn } from '@/lib/utils';
import type { Skill } from '@/api';

const MAX_VISIBLE = 30;

function fmtCollectionCount(collection: string, count: number) {
  const label = collection === 'Other' ? 'individual skills' : 'skills';
  return `${count} ${label}`;
}

const SkillItem = memo(function SkillItem({
  skill,
  onToggle,
  onDelete,
  onEdit,
  busy,
}: {
  skill: Skill;
  onToggle: (skill: Skill) => void;
  onDelete: (id: string) => void;
  onEdit: (skill: Skill) => void;
  busy: boolean;
}) {
  return (
    <div className="group flex items-start justify-between px-4 py-2.5 hover:bg-accent/30">
      <div className="min-w-0 flex-1 space-y-0.5">
        <div className="flex items-center gap-2">
          <span className="text-sm font-medium">{skill.name || skill.id}</span>
          <button aria-label={`${skill.enabled ? 'Disable' : 'Enable'} ${skill.name || skill.id}`} disabled={busy} onClick={() => onToggle(skill)} className="cursor-pointer disabled:cursor-wait disabled:opacity-60">
            <Badge
              variant={skill.enabled ? 'default' : 'outline'}
              className="text-[10px] px-1.5 py-0 hover:opacity-80"
            >
              {skill.enabled ? 'enabled' : 'disabled'}
            </Badge>
          </button>
        </div>
        {skill.description && (
          <p className="text-xs text-muted-foreground line-clamp-1">{skill.description}</p>
        )}
        <div className="flex items-center gap-2 text-[11px] text-muted-foreground/60">
          <span className="font-mono">{skill.directory}</span>
          {skill.repoOwner && skill.repoName && (
            <span>
              {skill.repoOwner}/{skill.repoName}
            </span>
          )}
        </div>
      </div>
      <div className="ml-2 flex shrink-0 items-center gap-1">
      <Button
        aria-label={`Edit ${skill.name || skill.id}`}
        variant="ghost"
        size="icon"
        onClick={() => onEdit(skill)}
        disabled={busy}
        className="h-7 w-7 text-muted-foreground hover:text-foreground"
      >
        <Pencil className="h-3.5 w-3.5" />
      </Button>
      <Button
        aria-label={`Delete ${skill.name || skill.id}`}
        variant="ghost"
        size="icon"
        onClick={() => onDelete(skill.id)}
        disabled={busy}
        className="h-7 w-7 text-muted-foreground hover:text-red-400"
      >
        <Trash2 className="h-3.5 w-3.5" />
      </Button>
      </div>
    </div>
  );
});

export function SkillGroup({
  collection,
  skills,
  isSearching,
  onToggle,
  onDelete,
  onEdit,
  busySkillId,
}: {
  collection: string;
  skills: Skill[];
  isSearching: boolean;
  onToggle: (skill: Skill) => void;
  onDelete: (id: string) => void;
  onEdit: (skill: Skill) => void;
  busySkillId?: string | null;
}) {
  const [open, setOpen] = useState(false);
  const [showAll, setShowAll] = useState(false);

  const isOpen = isSearching || open;
  const limitReached = !isSearching && !showAll && skills.length > MAX_VISIBLE;
  const visible = limitReached ? skills.slice(0, MAX_VISIBLE) : skills;
  const hiddenCount = skills.length - visible.length;

  return (
    <div className="overflow-hidden rounded-2xl border border-white/10 bg-card/50">
      <button
        type="button"
        onClick={() => setOpen((prev) => !prev)}
        className="flex w-full items-center justify-between px-4 py-3 text-left hover:bg-accent/50"
      >
        <div className="flex items-center gap-2">
          <ChevronRight
            className={cn(
              'h-4 w-4 text-muted-foreground transition-transform',
              isOpen && 'rotate-90',
            )}
          />
          <span className="text-sm font-semibold">{collection}</span>
          <Badge variant="secondary" className="text-[10px] px-1.5 py-0 font-normal">
            {fmtCollectionCount(collection, skills.length)}
          </Badge>
        </div>
      </button>
      {isOpen && (
        <div className="divide-y divide-border border-t">
          {visible.map((skill) => (
            <SkillItem
              key={skill.id}
              skill={skill}
              onToggle={onToggle}
              onDelete={onDelete}
              onEdit={onEdit}
              busy={busySkillId === skill.id}
            />
          ))}
          {limitReached && (
            <button
              type="button"
              onClick={() => setShowAll(true)}
              className="flex w-full items-center justify-center px-4 py-2.5 text-xs text-muted-foreground hover:bg-accent/30 hover:text-foreground transition-colors"
            >
              Show {hiddenCount} more {hiddenCount === 1 ? 'skill' : 'skills'}
            </button>
          )}
        </div>
      )}
    </div>
  );
}
