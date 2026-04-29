import { useEffect, useState, type FormEvent } from 'react';
import { Plus } from 'lucide-react';
import {
  listProviders,
  saveProvider,
  deleteProvider,
  switchProvider,
  getCurrentProviderId,
  getCodexOAuthStatus,
  setProxyTarget,
  type Provider,
} from '../api';
import type { CodexAccount } from '../api';
import type { ProviderPreset } from '../config/providerPresets';
import { ProviderCard } from '@/components/providers/ProviderCard';
import { ProviderFormDialog } from '@/components/providers/ProviderFormDialog';
import {
  buildProvider,
  emptyProviderForm,
  formFromPreset,
  formFromProvider,
  type ProviderFormData,
} from '@/components/providers/providerForm';
import { PageHeader } from '@/components/PageHeader';
import { Button } from '@/components/ui/button';
import { Card, CardContent } from '@/components/ui/card';
import { providerAuthMode, sortProviders } from '@/lib/provider';

export default function ProvidersPage() {
  const [providers, setProviders] = useState<Record<string, Provider>>({});
  const [currentProviderId, setCurrentProviderId] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const [showForm, setShowForm] = useState(false);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [selectedPreset, setSelectedPreset] = useState<ProviderPreset | null>(null);
  const [formData, setFormData] = useState<ProviderFormData>(emptyProviderForm);
  const [saving, setSaving] = useState(false);
  const [formError, setFormError] = useState('');
  const [codexAccounts, setCodexAccounts] = useState<CodexAccount[]>([]);

  const providerList = sortProviders(providers);

  const applyProviders = async () => {
    try {
      const [providerData, currentData] = await Promise.all([
        listProviders(),
        getCurrentProviderId().catch(() => ({ current_provider_id: null })),
      ]);
      setProviders(providerData.providers);
      setCurrentProviderId(currentData.current_provider_id);
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to load');
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    let active = true;

    Promise.all([
      listProviders(),
      getCurrentProviderId().catch(() => ({ current_provider_id: null })),
    ])
      .then(([providerData, currentData]) => {
        if (!active) return;
        setProviders(providerData.providers);
        setCurrentProviderId(currentData.current_provider_id);
      })
      .catch((e) => {
        if (!active) return;
        setError(e instanceof Error ? e.message : 'Failed to load');
      })
      .finally(() => {
        if (!active) return;
        setLoading(false);
      });

    return () => {
      active = false;
    };
  }, []);

  useEffect(() => {
    getCodexOAuthStatus()
      .then((status) => setCodexAccounts(status.accounts))
      .catch(() => setCodexAccounts([]));
  }, []);

  const handlePresetSelect = (preset: ProviderPreset) => {
    setFormError('');
    setSelectedPreset(preset);
    setEditingId(null);
    setFormData(formFromPreset(preset));
    setShowForm(true);
  };

  const handleAdd = () => {
    setFormError('');
    setSelectedPreset(null);
    setEditingId(null);
    setFormData(emptyProviderForm);
    setShowForm(true);
  };

  const handleEdit = (provider: Provider) => {
    setFormError('');
    setEditingId(provider.id);
    setSelectedPreset(null);
    setFormData(formFromProvider(provider));
    setShowForm(true);
  };

  const handleDelete = async (id: string) => {
    if (!confirm('Delete this provider?')) return;
    try {
      await deleteProvider(id);
      applyProviders();
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Delete failed');
    }
  };

  const handleSwitch = async (id: string) => {
    try {
      await switchProvider(id);
      if (providers[id] && providerAuthMode(providers[id]) === 'oauth_proxy') {
        await setProxyTarget(id);
      }
      setCurrentProviderId(id);
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Switch failed');
    }
  };

  const handleSubmit = async (e: FormEvent) => {
    e.preventDefault();
    setSaving(true);
    setError('');
    setFormError('');

    try {
      const id = formData.id.trim();
      const name = formData.name.trim();
      if (!id || !name) {
        throw new Error('Provider ID and name are required.');
      }
      if (formData.authMode !== 'oauth_proxy' && selectedPreset && !formData.apiKey.trim()) {
        throw new Error(`${selectedPreset.name} requires an API key.`);
      }
      await saveProvider(buildProvider(formData, selectedPreset));
      setShowForm(false);
      await applyProviders();
    } catch (e) {
      setFormError(e instanceof Error ? e.message : 'Save failed');
    } finally {
      setSaving(false);
    }
  };

  return (
    <div>
      <PageHeader
        title="Providers"
        description="Create, switch, and maintain Claude Code provider routes."
        action={
          <Button onClick={handleAdd}>
            <Plus className="h-4 w-4" />
            Add Provider
          </Button>
        }
      />

      {error && (
        <div className="mb-4 rounded-2xl border border-destructive/30 bg-destructive/10 px-4 py-3 text-sm text-destructive">
          {error}
        </div>
      )}

      {loading ? (
        <Card>
          <CardContent className="p-8 text-center text-sm leading-5 text-muted-foreground">Loading providers...</CardContent>
        </Card>
      ) : providerList.length === 0 ? (
        <Card>
          <CardContent className="p-8 text-center">
            <div className="text-sm font-medium leading-5 text-foreground">No providers yet</div>
            <div className="mt-1 text-sm leading-5 text-muted-foreground">Add a preset or configure a custom endpoint.</div>
          </CardContent>
        </Card>
      ) : (
        <div className="grid gap-3 lg:grid-cols-2">
          {providerList.map((provider) => (
            <ProviderCard
              key={provider.id}
              provider={provider}
              active={provider.id === currentProviderId}
              onSwitch={handleSwitch}
              onEdit={handleEdit}
              onDelete={handleDelete}
            />
          ))}
        </div>
      )}

      <ProviderFormDialog
        open={showForm}
        editingId={editingId}
        selectedPreset={selectedPreset}
        formData={formData}
        saving={saving}
        error={formError}
        codexAccounts={codexAccounts}
        onChange={(next) => {
          setFormData(next);
          if (formError) setFormError('');
        }}
        onPresetSelect={handlePresetSelect}
        onCancel={() => {
          setShowForm(false);
          setFormError('');
        }}
        onSubmit={handleSubmit}
      />
    </div>
  );
}
