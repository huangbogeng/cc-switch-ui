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
  const [skills, setSkills] = useState<Skill[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const [searchQuery, setSearchQuery] = useState('');
  const [showForm, setShowForm] = useState(false);

  const loadSkills = useCallback(async () => {
    try {
      const data = await listSkills();
      setSkills(data.skills);
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to load');
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => { loadSkills(); }, [loadSkills]);

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
      await deleteSkill(id);
      await loadSkills();
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Delete failed');
    }
  }, [loadSkills]);

  const handleSync = async () => {
    try {
      await syncSkills();
      setError('');
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Sync failed');
    }
  };

  const handleImport = async () => {
    try {
      const res = await importSkills();
      setError('');
      if (res.imported > 0) {
        await loadSkills();
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Import failed');
    }
  };

  const handleToggle = useCallback(async (skill: Skill) => {
    try {
      const res = await toggleSkill(skill.id);
      setSkills((prev) =>
        prev.map((s) => (s.id === skill.id ? { ...s, enabled: res.enabled } : s)),
      );
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Toggle failed');
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
            <Button variant="outline" size="sm" onClick={handleImport} className="gap-1.5">
              <Download className="h-3.5 w-3.5" /> Import
            </Button>
            <Button variant="outline" size="sm" onClick={handleSync} className="gap-1.5">
              <RefreshCw className="h-3.5 w-3.5" /> Sync
            </Button>
            <Button size="sm" onClick={() => setShowForm(true)} className="gap-1.5">
              <Plus className="h-3.5 w-3.5" /> Add
            </Button>
          </div>
        }
      />

      {error && (
        <div className="rounded-lg border border-red-500/20 bg-red-500/10 px-4 py-3 text-sm text-red-400">{error}</div>
      )}

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
              />
            ))}
          </div>
        </div>
      )}

      <SkillFormDialog
        key={`skill-form-${showForm}`}
        open={showForm}
        onClose={() => setShowForm(false)}
        onSaved={loadSkills}
      />
    </div>
  );
}
