import { useState } from 'react';
import { Plus, Pencil, Trash2, X, Check } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { useModelPricing, useUpsertModelPricing, useDeleteModelPricing } from '@/lib/useUsage';
import type { ModelPricingItem } from '@/api';
import { ErrorAlert } from '@/components/ErrorAlert';
import { errorMessage } from '@/lib/errors';

function EditableRow({
  item,
  onSave,
  onCancel,
  modelIdLocked = false,
  busy = false,
}: {
  item: ModelPricingItem;
  onSave: (v: ModelPricingItem) => void;
  onCancel: () => void;
  modelIdLocked?: boolean;
  busy?: boolean;
}) {
  const [draft, setDraft] = useState(item);
  return (
    <tr className="border-b border-border">
      <td className="p-2">
        <Input
          value={draft.modelId}
          onChange={(e) => setDraft({ ...draft, modelId: e.target.value })}
          className="h-8 text-xs"
          placeholder="model-id"
          disabled={modelIdLocked}
        />
      </td>
      <td className="p-2">
        <Input
          value={draft.displayName}
          onChange={(e) => setDraft({ ...draft, displayName: e.target.value })}
          className="h-8 text-xs"
          placeholder="Display Name"
        />
      </td>
      <td className="p-2">
        <Input
          value={draft.inputCostPerMillion}
          onChange={(e) => setDraft({ ...draft, inputCostPerMillion: e.target.value })}
          className="h-8 text-xs w-24"
          placeholder="0"
        />
      </td>
      <td className="p-2">
        <Input
          value={draft.outputCostPerMillion}
          onChange={(e) => setDraft({ ...draft, outputCostPerMillion: e.target.value })}
          className="h-8 text-xs w-24"
          placeholder="0"
        />
      </td>
      <td className="p-2">
        <Input
          value={draft.cacheReadCostPerMillion}
          onChange={(e) => setDraft({ ...draft, cacheReadCostPerMillion: e.target.value })}
          className="h-8 text-xs w-24"
          placeholder="0"
        />
      </td>
      <td className="p-2">
        <Input
          value={draft.cacheCreationCostPerMillion}
          onChange={(e) => setDraft({ ...draft, cacheCreationCostPerMillion: e.target.value })}
          className="h-8 text-xs w-24"
          placeholder="0"
        />
      </td>
      <td className="p-2">
        <div className="flex gap-1">
          <Button aria-label="Save pricing" variant="ghost" size="icon" className="h-7 w-7" onClick={() => onSave(draft)} disabled={busy}>
            <Check className="h-3.5 w-3.5 text-green-500" />
          </Button>
          <Button aria-label="Cancel pricing edit" variant="ghost" size="icon" className="h-7 w-7" onClick={onCancel} disabled={busy}>
            <X className="h-3.5 w-3.5 text-muted-foreground" />
          </Button>
        </div>
      </td>
    </tr>
  );
}

function ReadonlyRow({
  item,
  onEdit,
  onDelete,
  busy,
}: {
  item: ModelPricingItem;
  onEdit: () => void;
  onDelete: () => void;
  busy: boolean;
}) {
  return (
    <tr className="border-b border-border hover:bg-muted/30">
      <td className="p-2 text-xs font-mono">{item.modelId}</td>
      <td className="p-2 text-sm">{item.displayName}</td>
      <td className="p-2 text-xs font-mono">{item.inputCostPerMillion}</td>
      <td className="p-2 text-xs font-mono">{item.outputCostPerMillion}</td>
      <td className="p-2 text-xs font-mono">{item.cacheReadCostPerMillion}</td>
      <td className="p-2 text-xs font-mono">{item.cacheCreationCostPerMillion}</td>
      <td className="p-2">
        <div className="flex gap-1">
          <Button aria-label={`Edit pricing for ${item.modelId}`} variant="ghost" size="icon" className="h-7 w-7" onClick={onEdit} disabled={busy}>
            <Pencil className="h-3.5 w-3.5 text-muted-foreground" />
          </Button>
          <Button aria-label={`Delete pricing for ${item.modelId}`} variant="ghost" size="icon" className="h-7 w-7" onClick={onDelete} disabled={busy}>
            <Trash2 className="h-3.5 w-3.5 text-destructive" />
          </Button>
        </div>
      </td>
    </tr>
  );
}

const emptyItem: ModelPricingItem = {
  modelId: '',
  displayName: '',
  inputCostPerMillion: '0',
  outputCostPerMillion: '0',
  cacheReadCostPerMillion: '0',
  cacheCreationCostPerMillion: '0',
};

export default function ModelPricingPanel() {
  const { data: pricing, isLoading } = useModelPricing();
  const upsert = useUpsertModelPricing();
  const del = useDeleteModelPricing();

  const [editingId, setEditingId] = useState<string | null>(null);
  const [adding, setAdding] = useState(false);
  const [validationError, setValidationError] = useState('');

  const handleSave = (item: ModelPricingItem) => {
    if (!item.modelId.trim()) {
      setValidationError('Model ID is required.');
      return;
    }
    const costs = [
      item.inputCostPerMillion,
      item.outputCostPerMillion,
      item.cacheReadCostPerMillion,
      item.cacheCreationCostPerMillion,
    ];
    if (costs.some((cost) => cost.trim() === '' || !Number.isFinite(Number(cost)) || Number(cost) < 0)) {
      setValidationError('Pricing values must be non-negative numbers.');
      return;
    }
    setValidationError('');
    upsert.mutate(item, {
      onSuccess: () => {
        setEditingId(null);
        setAdding(false);
      },
    });
  };

  const handleDelete = (modelId: string) => {
    if (!confirm(`Delete pricing for ${modelId}?`)) return;
    del.mutate(modelId);
  };

  const list = pricing ?? [];
  const busy = upsert.isPending || del.isPending;

  if (isLoading) {
    return <div className="text-sm text-muted-foreground py-4">Loading pricing...</div>;
  }

  return (
    <Card>
      <CardHeader className="flex flex-row items-center justify-between py-3">
        <CardTitle className="text-sm font-medium">Model Pricing</CardTitle>
        {!adding && (
          <Button variant="outline" size="sm" className="h-7 text-xs" onClick={() => setAdding(true)} disabled={busy}>
            <Plus className="mr-1 h-3.5 w-3.5" />
            Add Model
          </Button>
        )}
      </CardHeader>
      <CardContent className="p-0">
        {(validationError || upsert.error || del.error) && (
          <div className="p-3">
            <ErrorAlert message={validationError || errorMessage(upsert.error ?? del.error, 'Pricing update failed')} />
          </div>
        )}
        <div className="overflow-x-auto">
          <table className="w-full">
            <thead>
              <tr className="border-b border-border text-xs text-muted-foreground">
                <th className="p-2 text-left font-medium">Model ID</th>
                <th className="p-2 text-left font-medium">Display Name</th>
                <th className="p-2 text-right font-medium">Input (per M)</th>
                <th className="p-2 text-right font-medium">Output (per M)</th>
                <th className="p-2 text-right font-medium">Cache Read (per M)</th>
                <th className="p-2 text-right font-medium">Cache Creation (per M)</th>
                <th className="p-2 text-center font-medium w-20">Actions</th>
              </tr>
            </thead>
            <tbody>
              {adding && (
                <EditableRow
                  item={emptyItem}
                  onSave={handleSave}
                  onCancel={() => setAdding(false)}
                  busy={busy}
                />
              )}
              {list.map((item) =>
                editingId === item.modelId ? (
                  <EditableRow
                    key={item.modelId}
                    item={item}
                    onSave={handleSave}
                    onCancel={() => setEditingId(null)}
                    modelIdLocked
                    busy={busy}
                  />
                ) : (
                  <ReadonlyRow
                    key={item.modelId}
                    item={item}
                    onEdit={() => setEditingId(item.modelId)}
                    onDelete={() => handleDelete(item.modelId)}
                    busy={busy}
                  />
                ),
              )}
              {list.length === 0 && !adding && (
                <tr>
                  <td colSpan={7} className="p-4 text-center text-sm text-muted-foreground">
                    No pricing configured. Click "Add Model" to add one.
                  </td>
                </tr>
              )}
            </tbody>
          </table>
        </div>
      </CardContent>
    </Card>
  );
}
