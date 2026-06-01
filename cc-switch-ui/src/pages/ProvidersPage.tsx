import { useCallback, useEffect, useState } from 'react';
import { Plus } from 'lucide-react';
import {
  listProviders,
  saveProvider,
  deleteProvider,
  switchProvider,
  getCurrentProviderId,
  getCodexOAuthStatus,
  setProxyTarget,
  getProxyStatus,
  startProxy,
  stopProxy,
  type Provider,
} from '../api';
import type { CodexAccount } from '../api';
import type { ProviderPreset } from '@/config/providerPresets';
import { LocalRoutePanel } from '@/components/providers/LocalRoutePanel';
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
import { sortProviders } from '@/lib/provider';
import { cacheGet, cacheSet } from '@/lib/fetchCache';

export default function ProvidersPage() {
  const cached = cacheGet<{
    providers: Record<string, Provider>;
    currentProviderId: string | null;
  }>('providers');
  const [providers, setProviders] = useState<Record<string, Provider>>(
    cached?.providers ?? {},
  );
  const [currentProviderId, setCurrentProviderId] = useState<string | null>(
    cached?.currentProviderId ?? null,
  );
  const [loading, setLoading] = useState(!cached);
  const [error, setError] = useState('');
  const [showForm, setShowForm] = useState(false);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [selectedPreset, setSelectedPreset] = useState<ProviderPreset | null>(null);
  const [draftFormData, setDraftFormData] = useState<ProviderFormData>(emptyProviderForm);
  const [saving, setSaving] = useState(false);
  const [formError, setFormError] = useState('');
  const [codexAccounts, setCodexAccounts] = useState<CodexAccount[]>([]);
  const [proxyStatus, setProxyStatus] = useState<{
    running: boolean;
    listen_addr: string | null;
    active_target_provider_id: string | null;
  } | null>(null);
  const [proxyError, setProxyError] = useState('');

  const providerList = sortProviders(providers);

  const applyProviders = async (signal?: AbortSignal) => {
    try {
      const [providerData, currentData] = await Promise.all([
        listProviders({ signal }),
        getCurrentProviderId({ signal }).catch(() => ({ current_provider_id: null })),
      ]);
      setProviders(providerData.providers);
      setCurrentProviderId(currentData.current_provider_id);
      cacheSet('providers', {
        providers: providerData.providers,
        currentProviderId: currentData.current_provider_id,
      });
    } catch (e) {
      if (e instanceof DOMException && e.name === 'AbortError') return;
      setError(e instanceof Error ? e.message : 'Failed to load');
    } finally {
      if (!signal?.aborted) setLoading(false);
    }
  };

  const loadProxyStatus = useCallback(async (signal?: AbortSignal) => {
    try {
      const status = await getProxyStatus({ signal });
      setProxyStatus(status);
    } catch (e) {
      if (e instanceof DOMException && e.name === 'AbortError') return;
      setProxyStatus(null);
    }
  }, []);

  useEffect(() => {
    const ctrl = new AbortController();

    Promise.all([
      listProviders({ signal: ctrl.signal }),
      getCurrentProviderId({ signal: ctrl.signal }).catch(() => ({ current_provider_id: null })),
    ])
      .then(([providerData, currentData]) => {
        if (ctrl.signal.aborted) return;
        setProviders(providerData.providers);
        setCurrentProviderId(currentData.current_provider_id);
        cacheSet('providers', {
          providers: providerData.providers,
          currentProviderId: currentData.current_provider_id,
        });
      })
      .catch((e) => {
        if (e instanceof DOMException && e.name === 'AbortError') return;
        if (ctrl.signal.aborted) return;
        setError(e instanceof Error ? e.message : 'Failed to load');
      })
      .finally(() => {
        if (ctrl.signal.aborted) return;
        setLoading(false);
      });

    return () => ctrl.abort();
  }, []);

  useEffect(() => {
    const ctrl = new AbortController();
    getCodexOAuthStatus({ signal: ctrl.signal })
      .then((status) => setCodexAccounts(status.accounts))
      .catch(() => setCodexAccounts([]));
    return () => ctrl.abort();
  }, []);

  useEffect(() => {
    const ctrl = new AbortController();
    Promise.resolve().then(() => loadProxyStatus(ctrl.signal));
    return () => ctrl.abort();
  }, [loadProxyStatus]);

  const handleStartProxy = async (providerId: string) => {
    try {
      setProxyError('');
      await setProxyTarget(providerId);
      const result = await startProxy();
      if (!result.success) throw new Error(result.error);
      await loadProxyStatus();
    } catch (e) {
      setProxyError(e instanceof Error ? e.message : 'Failed to start local route');
    }
  };

  const handleStopProxy = async () => {
    try {
      setProxyError('');
      const result = await stopProxy();
      if (!result.success) throw new Error(result.error);
      await loadProxyStatus();
    } catch (e) {
      setProxyError(e instanceof Error ? e.message : 'Failed to stop local route');
    }
  };

  const handleSwitch = async (id: string) => {
    try {
      await switchProvider(id);
      setCurrentProviderId(id);
      if (proxyStatus?.running) {
        const shouldSwitchTarget = confirm(
          'Local route takeover is active.\n\nAlso switch the active route target to this provider?'
        );
        if (shouldSwitchTarget) {
          await setProxyTarget(id);
        }
      }
      await applyProviders();
      await loadProxyStatus();
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Switch failed');
    }
  };

  const handlePresetSelect = (preset: ProviderPreset) => {
    setFormError('');
    setSelectedPreset(preset);
    setEditingId(null);
    setDraftFormData(formFromPreset(preset));
    setShowForm(true);
  };

  const handleAdd = () => {
    setFormError('');
    setSelectedPreset(null);
    setEditingId(null);
    setDraftFormData(emptyProviderForm);
    setShowForm(true);
  };

  const handleEdit = (provider: Provider) => {
    setFormError('');
    setEditingId(provider.id);
    setSelectedPreset(null);
    setDraftFormData(formFromProvider(provider));
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

  const handleSubmit = async (formData: ProviderFormData) => {
    setSaving(true);
    setError('');
    setFormError('');

    try {
      const id = formData.id.trim();
      const name = formData.name.trim();
      if (!id || !name) {
        throw new Error('Provider ID and name are required.');
      }
      if (formData.authMode !== 'oauth_proxy' && !formData.apiKey.trim()) {
        const providerName = selectedPreset?.name || name;
        const shouldContinue = confirm(
          `${providerName} is missing an API key.\n\nSave anyway? Switching or routing through this provider may fail until you add credentials.`
        );
        if (!shouldContinue) {
          setSaving(false);
          return;
        }
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
        description="Manage provider configs. Local route control is separated below."
        action={
          <Button onClick={handleAdd}>
            <Plus className="h-4 w-4" />
            Add Provider
          </Button>
        }
      />

      {error && (
        <div role="alert" className="mb-4 rounded-2xl border border-destructive/30 bg-destructive/10 px-4 py-3 text-sm text-destructive">
          {error}
        </div>
      )}

      {proxyError && (
        <div role="alert" className="mb-4 rounded-xl border border-destructive/30 bg-destructive/10 px-4 py-2.5 text-sm text-destructive shadow-sm">
          {proxyError}
        </div>
      )}

      {!loading && providerList.length > 0 && (
        <LocalRoutePanel
          providers={providerList}
          currentProviderId={currentProviderId}
          proxyRunning={proxyStatus?.running ?? false}
          proxyTargetId={proxyStatus?.active_target_provider_id ?? null}
          onStartProxy={() => {
            const startId = currentProviderId ?? providerList[0]?.id ?? '';
            if (!startId) {
              setProxyError('Select a provider before starting the local route');
              return;
            }
            void handleStartProxy(startId);
          }}
          onStopProxy={handleStopProxy}
        />
      )}

      {loading ? (
        <Card className="border-white/5 bg-card/40">
          <CardContent className="p-12 flex flex-col items-center justify-center text-sm leading-5 text-muted-foreground">
            <div className="h-8 w-8 rounded-full border-2 border-primary border-t-transparent animate-spin mb-4"></div>
            Loading providers...
          </CardContent>
        </Card>
      ) : providerList.length === 0 ? (
        <Card className="border-dashed border-2 border-white/10 bg-white/[0.01]">
          <CardContent className="p-16 flex flex-col items-center justify-center text-center">
            <div className="h-16 w-16 rounded-2xl bg-white/5 flex items-center justify-center mb-4">
              <Plus className="h-8 w-8 text-muted-foreground/50" />
            </div>
            <div className="text-lg font-semibold leading-5 text-foreground mb-2">No providers yet</div>
            <div className="text-sm leading-5 text-muted-foreground max-w-sm">Add a preset or configure a custom endpoint to start managing your Claude Code routes.</div>
            <Button onClick={handleAdd} className="mt-6 shadow-[0_0_20px_rgba(var(--primary),0.15)]">
              <Plus className="h-4 w-4 mr-2" />
              Add First Provider
            </Button>
          </CardContent>
        </Card>
      ) : (
        <div className="grid gap-4 lg:grid-cols-2">
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
        key={`${editingId ?? 'new'}:${selectedPreset?.id ?? 'custom'}:${draftFormData.id}:${showForm ? 'open' : 'closed'}`}
        open={showForm}
        editingId={editingId}
        selectedPreset={selectedPreset}
        initialFormData={draftFormData}
        saving={saving}
        error={formError}
        codexAccounts={codexAccounts}
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
