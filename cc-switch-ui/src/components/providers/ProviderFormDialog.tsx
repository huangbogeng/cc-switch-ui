import type { FormEvent } from 'react';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Textarea } from '@/components/ui/textarea';
import type { ProviderPreset } from '@/config/providerPresets';
import { PresetSelector } from './PresetSelector';
import type { ProviderFormData } from './providerForm';

interface ProviderFormDialogProps {
  open: boolean;
  editingId: string | null;
  selectedPreset: ProviderPreset | null;
  formData: ProviderFormData;
  saving: boolean;
  onChange: (formData: ProviderFormData) => void;
  onPresetSelect: (preset: ProviderPreset) => void;
  onCancel: () => void;
  onSubmit: (event: FormEvent) => void;
}

export function ProviderFormDialog({
  open,
  editingId,
  selectedPreset,
  formData,
  saving,
  onChange,
  onPresetSelect,
  onCancel,
  onSubmit,
}: ProviderFormDialogProps) {
  if (!open) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/55 px-4 py-6 backdrop-blur-sm">
      <Card className="max-h-[90vh] w-full max-w-2xl overflow-y-auto">
        <CardContent className="p-6">
          <div className="mb-5">
            <h2 className="text-xl font-semibold leading-7 text-foreground">
              {editingId ? 'Edit Provider' : selectedPreset ? `Add ${selectedPreset.name}` : 'Add Custom Provider'}
            </h2>
            <p className="mt-1 text-sm leading-5 text-muted-foreground">
              Keep routing credentials local to this admin service.
            </p>
          </div>

          {!editingId && !selectedPreset && (
            <div className="mb-6 space-y-4">
              <PresetSelector onSelect={onPresetSelect} />
              <div className="grid grid-cols-[minmax(0,1fr)_auto_minmax(0,1fr)] items-center gap-3 text-xs leading-4 text-muted-foreground">
                <div className="h-px flex-1 bg-border" />
                or enter manually
                <div className="h-px flex-1 bg-border" />
              </div>
            </div>
          )}

          {selectedPreset && (
            <div className="mb-5 rounded-2xl border border-white/10 bg-white/[0.04] p-4">
              <Badge style={{ background: selectedPreset.iconColor }} className="border-transparent text-white">
                {selectedPreset.name}
              </Badge>
              {selectedPreset.description && (
                <p className="mt-2 text-sm leading-5 text-muted-foreground">{selectedPreset.description}</p>
              )}
            </div>
          )}

          <form onSubmit={onSubmit} className="space-y-4">
            <div className="grid gap-4 sm:grid-cols-2">
              <div className="space-y-2">
                <Label htmlFor="provider-id">Provider ID</Label>
                <Input
                  id="provider-id"
                  value={formData.id}
                  onChange={(e) => onChange({ ...formData, id: e.target.value })}
                  required
                  disabled={!!editingId}
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="provider-name">Name</Label>
                <Input
                  id="provider-name"
                  value={formData.name}
                  onChange={(e) => onChange({ ...formData, name: e.target.value })}
                  required
                />
              </div>
            </div>

            {(selectedPreset || formData.websiteUrl) && (
              <div className="space-y-2">
                <Label htmlFor="api-key">API Key</Label>
                <Input
                  id="api-key"
                  type="password"
                  value={formData.apiKey}
                  onChange={(e) => onChange({ ...formData, apiKey: e.target.value })}
                  required={!!selectedPreset}
                />
              </div>
            )}

            {!selectedPreset && (
              <div className="space-y-2">
                <Label htmlFor="base-url">Base URL</Label>
                <Input
                  id="base-url"
                  placeholder="Optional, for custom providers"
                  value={formData.websiteUrl}
                  onChange={(e) => onChange({ ...formData, websiteUrl: e.target.value })}
                />
              </div>
            )}

            <div className="space-y-2">
              <Label htmlFor="notes">Notes</Label>
              <Textarea
                id="notes"
                value={formData.notes}
                onChange={(e) => onChange({ ...formData, notes: e.target.value })}
                placeholder="Optional"
              />
            </div>

            <div className="flex justify-end gap-2 pt-2">
              <Button type="button" variant="ghost" onClick={onCancel}>
                Cancel
              </Button>
              <Button type="submit" disabled={saving}>
                {saving ? 'Saving...' : 'Save'}
              </Button>
            </div>
          </form>
        </CardContent>
      </Card>
    </div>
  );
}
