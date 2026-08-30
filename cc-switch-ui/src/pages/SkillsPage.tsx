import { useCallback, useEffect, useMemo, useState } from 'react';
import { Plus, RefreshCw, Package, Download, Search } from 'lucide-react';
import {
  listSkills,
  deleteSkill,
  syncSkills,
  importSkills,
  toggleSkill,
  type Skill,
} from '../api';
import { PageHeader } from '@/components/PageHeader';
import { Button } from '@/components/ui/button';
import { Card, CardContent } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { SkillGroup } from '@/components/skills/SkillGroup';
import { SkillFormDialog } from '@/components/skills/SkillFormDialog';
import { cacheGet, cacheSet } from '@/lib/fetchCache';
import { ErrorAlert } from '@/components/ErrorAlert';

const DEFAULT_COLLECTION = 'Other';

function groupByCollection(skills: Skill[]): [string, Skill[]][] {
  const groups = new Map<string, Skill[]>();
  for (const s of skills) {
    const coll = s.collection || DEFAULT_COLLECTION;
    if (!groups.has(coll)) groups.set(coll, []);
    groups.get(coll)!.push(s);
  }
  return [...groups.entries()].sort((a, b) => b[1].length - a[1].length);
}

export default function SkillsPage() {
  const cached = cacheGet<Skill[]>('skills');
  const [skills, setSkills] = useState<Skill[]>(cached ?? []);
  const [loading, setLoading] = useState(!cached);
  const [error, setError] = useState('');
  const [searchQuery, setSearchQuery] = useState('');
  const [showForm, setShowForm] = useState(false);
  const [syncing, setSyncing] = useState(false);
  const [importing, setImporting] = useState(false);
  const [editingSkill, setEditingSkill] = useState<Skill | null>(null);
  const [pendingSkillId, setPendingSkillId] = useState<string | null>(null);
  const [notice, setNotice] = useState('');

  const loadSkills = useCallback(async (signal?: AbortSignal) => {
    try {
      const data = await listSkills({ signal });
      setSkills(data.skills);
      cacheSet('skills', data.skills);
    } catch (e) {
      if (e instanceof DOMException && e.name === 'AbortError') return;
      setError(e instanceof Error ? e.message : 'Failed to load');
    } finally {
      if (!signal?.aborted) setLoading(false);
    }
  }, []);

  useEffect(() => {
    const ctrl = new AbortController();
    loadSkills(ctrl.signal);
    return () => ctrl.abort();
  }, [loadSkills]);

  const filtered = useMemo(() => {
    if (!searchQuery.trim()) return skills;
    const q = searchQuery.toLowerCase();
    return skills.filter(
      (s) =>
        s.name.toLowerCase().includes(q) ||
        (s.description ?? '').toLowerCase().includes(q),
    );
  }, [skills, searchQuery]);

  const groups = useMemo(() => groupByCollection(filtered), [filtered]);
  const isSearching = searchQuery.trim().length > 0;

  const handleDelete = useCallback(async (id: string) => {
    if (!confirm('Delete this skill?')) return;
    try {
      setPendingSkillId(id);
      setError('');
      setNotice('');
      await deleteSkill(id);
      await loadSkills();
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Delete failed');
    } finally {
      setPendingSkillId(null);
    }
  }, [loadSkills]);

  const handleSync = async () => {
    setSyncing(true);
    try {
      setNotice('');
      await syncSkills();
      setError('');
      await loadSkills();
      setNotice('Skills synced to Claude Code.');
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Sync failed');
    } finally {
      setSyncing(false);
    }
  };

  const handleImport = async () => {
    setImporting(true);
    try {
      setNotice('');
      const res = await importSkills();
      setError('');
      setNotice(`Imported ${res.imported} skill${res.imported === 1 ? '' : 's'}.`);
      if (res.imported > 0) {
        await loadSkills();
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Import failed');
    } finally {
      setImporting(false);
    }
  };

  const handleToggle = useCallback(async (skill: Skill) => {
    try {
      setPendingSkillId(skill.id);
      setError('');
      setNotice('');
      const res = await toggleSkill(skill.id);
      setSkills((prev) => {
        const next = prev.map((item) =>
          item.id === skill.id ? { ...item, enabled: res.enabled } : item,
        );
        cacheSet('skills', next);
        return next;
      });
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Toggle failed');
    } finally {
      setPendingSkillId(null);
    }
  }, []);

  if (loading) {
    return (
      <div className="space-y-6">
        <PageHeader title="Skills" description="Manage installed skills" />
        <div className="flex items-center justify-center py-20 text-muted-foreground">Loading...</div>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <PageHeader
        title="Skills"
        description="Skills synced to ~/.claude/skills/"
        action={
          <div className="flex items-center gap-2">
            <Button variant="outline" size="sm" onClick={handleImport} className="gap-1.5" disabled={importing || syncing || pendingSkillId !== null}>
              <Download className={importing ? 'animate-spin h-3.5 w-3.5' : 'h-3.5 w-3.5'} /> {importing ? 'Importing...' : 'Import'}
            </Button>
            <Button variant="outline" size="sm" onClick={handleSync} className="gap-1.5" disabled={syncing || importing || pendingSkillId !== null}>
              <RefreshCw className={syncing ? 'animate-spin h-3.5 w-3.5' : 'h-3.5 w-3.5'} /> {syncing ? 'Syncing...' : 'Sync'}
            </Button>
            <Button size="sm" onClick={() => { setEditingSkill(null); setShowForm(true); }} disabled={syncing || importing || pendingSkillId !== null} className="gap-1.5">
              <Plus className="h-3.5 w-3.5" /> Add
            </Button>
          </div>
        }
      />

      {error && (
        <ErrorAlert message={error} />
      )}
      {notice && <div role="status" className="rounded-lg border border-emerald-500/20 bg-emerald-500/10 px-4 py-3 text-sm text-emerald-400">{notice}</div>}

      <div className="relative">
        <Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground/50" />
        <Input
          placeholder="Search skills by name or description..."
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
          className="pl-9"
        />
      </div>

      {skills.length === 0 ? (
        <Card>
          <CardContent className="flex flex-col items-center justify-center py-16 text-center">
            <Package className="mb-3 h-10 w-10 text-muted-foreground/40" />
            <p className="text-sm font-medium text-muted-foreground">No skills installed</p>
            <p className="mt-1 text-xs text-muted-foreground/60">Add skills to sync them to ~/.claude/skills/</p>
          </CardContent>
        </Card>
      ) : groups.length === 0 ? (
        <Card>
          <CardContent className="flex flex-col items-center justify-center py-16 text-center">
            <Search className="mb-3 h-10 w-10 text-muted-foreground/40" />
            <p className="text-sm font-medium text-muted-foreground">No skills match "{searchQuery}"</p>
            <p className="mt-1 text-xs text-muted-foreground/60">Try a different search term</p>
          </CardContent>
        </Card>
      ) : (
        <div className="overflow-y-auto" style={{ height: 'calc(100vh - 250px)' }}>
          <div className="space-y-2">
            {groups.map(([collection, collSkills]) => (
              <SkillGroup
                key={collection}
                collection={collection}
                skills={collSkills}
                isSearching={isSearching}
                onToggle={handleToggle}
                onDelete={handleDelete}
                onEdit={(skill) => { setEditingSkill(skill); setShowForm(true); }}
                busySkillId={pendingSkillId}
              />
            ))}
          </div>
        </div>
      )}

      <SkillFormDialog
        key={`skill-form-${showForm}-${editingSkill?.id ?? 'new'}`}
        open={showForm}
        initialSkill={editingSkill}
        onClose={() => { setShowForm(false); setEditingSkill(null); }}
        onSaved={loadSkills}
      />
    </div>
  );
}
