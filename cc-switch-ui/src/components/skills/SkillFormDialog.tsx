import { useId, useState } from 'react';
import { Card, CardContent } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { ApiError, saveSkill, type Skill } from '@/api';
import { useDialog } from '@/lib/useDialog';
import { useUnsavedChanges } from '@/lib/useUnsavedChanges';

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

export function SkillFormDialog({
  open,
  onClose,
  onSaved,
  initialSkill,
}: {
  open: boolean;
  onClose: () => void;
  onSaved: () => void | Promise<void>;
  initialSkill?: Skill | null;
}) {
  const [baseline, setBaseline] = useState<Skill>(() => initialSkill ?? emptyForm());
  const [form, setForm] = useState<Skill>(baseline);
  const [saving, setSaving] = useState(false);
  const [formError, setFormError] = useState('');
  const titleId = useId();
  const canDiscard = useUnsavedChanges(open && JSON.stringify(form) !== JSON.stringify(baseline), open && saving);
  const requestClose = () => { if (canDiscard()) onClose(); };
  const dialogRef = useDialog(open, requestClose, !saving);

  if (!open) return null;

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
      setForm(emptyForm());
      onClose();
      await onSaved();
    } catch (e) {
      if (e instanceof ApiError && e.saved) {
        setBaseline(form);
        await onSaved();
      }
      setFormError(e instanceof Error ? e.message : 'Save failed');
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/65 px-4 py-6">
      <Card ref={dialogRef} tabIndex={-1} role="dialog" aria-modal="true" aria-labelledby={titleId} className="w-full max-w-lg overflow-hidden border-primary/30">
        <CardContent className="space-y-4 p-6">
          <h2 id={titleId} className="text-lg font-semibold">{initialSkill ? 'Edit Skill' : 'Add Skill'}</h2>
          <p className="text-sm text-muted-foreground">Save applies enabled skills to Claude Code. These fields describe the record; edit instructions in ~/.cc-switch/skills/{form.directory || '<directory>'}/SKILL.md.</p>
          <div className="space-y-4">
            <div className="grid grid-cols-2 gap-4">
              <div className="space-y-2">
                <Label htmlFor="skill-id">ID</Label>
                <Input
                  id="skill-id"
                  placeholder="e.g. local:my-skill"
                  value={form.id}
                  onChange={(e) => setForm({ ...form, id: e.target.value })}
                  disabled={!!initialSkill || saving}
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="skill-name">Name</Label>
                <Input
                  id="skill-name"
                  disabled={saving}
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
                disabled={!!initialSkill || saving}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="skill-desc">Description</Label>
              <Input
                id="skill-desc"
                  disabled={saving}
                placeholder="Optional description"
                value={form.description ?? ''}
                onChange={(e) => setForm({ ...form, description: e.target.value })}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="skill-collection">Collection</Label>
              <Input
                id="skill-collection"
                  disabled={saving}
                placeholder="e.g. Development"
                value={form.collection ?? ''}
                onChange={(e) => setForm({ ...form, collection: e.target.value })}
              />
            </div>
            <div className="grid grid-cols-2 gap-4">
              <div className="space-y-2">
                <Label htmlFor="skill-repo-owner">Repo Owner</Label>
                <Input
                  id="skill-repo-owner"
                  disabled={saving}
                  placeholder="e.g. anthropics"
                  value={form.repoOwner ?? ''}
                  onChange={(e) => setForm({ ...form, repoOwner: e.target.value })}
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="skill-repo-name">Repo Name</Label>
                <Input
                  id="skill-repo-name"
                  disabled={saving}
                  placeholder="e.g. skills"
                  value={form.repoName ?? ''}
                  onChange={(e) => setForm({ ...form, repoName: e.target.value })}
                />
              </div>
            </div>
          </div>

          {formError && (
            <div className="rounded-lg border border-red-500/20 bg-red-500/10 px-3 py-2 text-xs text-red-400">{formError}</div>
          )}

          <div className="flex items-center justify-end gap-2">
            <Button variant="ghost" size="sm" onClick={requestClose} disabled={saving}>Cancel</Button>
            <Button size="sm" onClick={handleSave} disabled={saving}>
              {saving ? 'Saving...' : 'Save'}
            </Button>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
