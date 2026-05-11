import { useCallback, useEffect, useState } from 'react';
import { Plus, RefreshCw, Trash2, Package, Download } from 'lucide-react';
import {
  listSkills,
  saveSkill,
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
import { Label } from '@/components/ui/label';
import { Badge } from '@/components/ui/badge';
import { cn } from '@/lib/utils';

function emptyForm(): Skill {
  return {
    id: '',
    name: '',
    description: '',
    directory: '',
    appType: 'claude_code',
    enabled: true,
    installedAt: Date.now(),
  };
}

export default function SkillsPage() {
  const [skills, setSkills] = useState<Skill[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const [showForm, setShowForm] = useState(false);
  const [form, setForm] = useState(emptyForm());
  const [saving, setSaving] = useState(false);
  const [formError, setFormError] = useState('');

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

  const handleSave = async () => {
    setSaving(true);
    setFormError('');
    try {
      if (!form.id.trim() || !form.directory.trim()) {
        setFormError('ID and Directory are required');
        setSaving(false);
        return;
      }
      await saveSkill(form);
      setShowForm(false);
      setForm(emptyForm());
      await loadSkills();
    } catch (e) {
      setFormError(e instanceof Error ? e.message : 'Save failed');
    } finally {
      setSaving(false);
    }
  };

  const handleDelete = async (id: string) => {
    if (!confirm('Delete this skill?')) return;
    try {
      await deleteSkill(id);
      await loadSkills();
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Delete failed');
    }
  };

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

  const handleToggle = async (skill: Skill) => {
    try {
      const res = await toggleSkill(skill.id);
      setSkills((prev) =>
        prev.map((s) => (s.id === skill.id ? { ...s, enabled: res.enabled } : s))
      );
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Toggle failed');
    }
  };

  const openCreate = () => {
    setForm(emptyForm());
    setFormError('');
    setShowForm(true);
  };

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
            <Button size="sm" onClick={openCreate} className="gap-1.5">
              <Plus className="h-3.5 w-3.5" /> Add
            </Button>
          </div>
        }
      />

      {error && (
        <div className="rounded-lg border border-red-500/20 bg-red-500/10 px-4 py-3 text-sm text-red-400">{error}</div>
      )}

      {skills.length === 0 ? (
        <Card>
          <CardContent className="flex flex-col items-center justify-center py-16 text-center">
            <Package className="mb-3 h-10 w-10 text-muted-foreground/40" />
            <p className="text-sm font-medium text-muted-foreground">No skills installed</p>
            <p className="mt-1 text-xs text-muted-foreground/60">Add skills to sync them to ~/.claude/skills/</p>
          </CardContent>
        </Card>
      ) : (
        <div className="space-y-3">
          {skills.map((skill) => (
            <Card key={skill.id} className="group transition-all hover:border-primary/20">
              <CardContent className="flex items-start justify-between p-4">
                <div className="min-w-0 space-y-1">
                  <div className="flex items-center gap-2">
                    <span className="font-semibold text-sm">{skill.name || skill.id}</span>
                    <button onClick={() => handleToggle(skill)} className="cursor-pointer">
                      <Badge variant={skill.enabled ? 'default' : 'outline'} className="text-[10px] px-1.5 py-0 hover:opacity-80">
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
                      <span>{skill.repoOwner}/{skill.repoName}</span>
                    )}
                  </div>
                </div>
                <Button
                  variant="ghost"
                  size="icon"
                  onClick={() => handleDelete(skill.id)}
                  className="h-8 w-8 opacity-0 group-hover:opacity-100 transition-opacity text-muted-foreground hover:text-red-400"
                >
                  <Trash2 className="h-4 w-4" />
                </Button>
              </CardContent>
            </Card>
          ))}
        </div>
      )}

      {showForm && (
        <Card className={cn("border-primary/30")}>
          <CardContent className="space-y-4 p-4">
            <div className="grid grid-cols-2 gap-4">
              <div className="space-y-2">
                <Label htmlFor="skill-id">ID</Label>
                <Input
                  id="skill-id"
                  placeholder="e.g. local:my-skill"
                  value={form.id}
                  onChange={(e) => setForm({ ...form, id: e.target.value })}
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="skill-name">Name</Label>
                <Input
                  id="skill-name"
                  placeholder="e.g. My Skill"
                  value={form.name}
                  onChange={(e) => setForm({ ...form, name: e.target.value })}
                />
              </div>
            </div>
            <div className="space-y-2">
              <Label htmlFor="skill-directory">Directory</Label>
              <Input
                id="skill-directory"
                placeholder="e.g. my-skill (directory name in ~/.cc-switch/skills/)"
                value={form.directory}
                onChange={(e) => setForm({ ...form, directory: e.target.value })}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="skill-desc">Description</Label>
              <Input
                id="skill-desc"
                placeholder="Optional description"
                value={form.description ?? ''}
                onChange={(e) => setForm({ ...form, description: e.target.value })}
              />
            </div>
            <div className="grid grid-cols-2 gap-4">
              <div className="space-y-2">
                <Label htmlFor="skill-repo-owner">Repo Owner</Label>
                <Input
                  id="skill-repo-owner"
                  placeholder="e.g. anthropics"
                  value={form.repoOwner ?? ''}
                  onChange={(e) => setForm({ ...form, repoOwner: e.target.value })}
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="skill-repo-name">Repo Name</Label>
                <Input
                  id="skill-repo-name"
                  placeholder="e.g. skills"
                  value={form.repoName ?? ''}
                  onChange={(e) => setForm({ ...form, repoName: e.target.value })}
                />
              </div>
            </div>
            {formError && (
              <div className="rounded-lg border border-red-500/20 bg-red-500/10 px-3 py-2 text-xs text-red-400">{formError}</div>
            )}
            <div className="flex items-center justify-end gap-2">
              <Button variant="ghost" size="sm" onClick={() => setShowForm(false)}>Cancel</Button>
              <Button size="sm" onClick={handleSave} disabled={saving}>
                {saving ? 'Saving...' : 'Save'}
              </Button>
            </div>
          </CardContent>
        </Card>
      )}
    </div>
  );
}
