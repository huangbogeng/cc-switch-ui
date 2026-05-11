import { useState } from 'react';
import { Card, CardContent } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { saveSkill, type Skill } from '@/api';

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
}: {
  open: boolean;
  onClose: () => void;
  onSaved: () => void;
}) {
  const [form, setForm] = useState<Skill>(emptyForm());
  const [saving, setSaving] = useState(false);
  const [formError, setFormError] = useState('');

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
      onSaved();
    } catch (e) {
      setFormError(e instanceof Error ? e.message : 'Save failed');
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/65 px-4 py-6">
      <Card className="w-full max-w-lg overflow-hidden border-primary/30">
        <CardContent className="space-y-4 p-6">
          <div className="space-y-4">
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
          </div>

          {formError && (
            <div className="rounded-lg border border-red-500/20 bg-red-500/10 px-3 py-2 text-xs text-red-400">{formError}</div>
          )}

          <div className="flex items-center justify-end gap-2">
            <Button variant="ghost" size="sm" onClick={onClose}>Cancel</Button>
            <Button size="sm" onClick={handleSave} disabled={saving}>
              {saving ? 'Saving...' : 'Save'}
            </Button>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
