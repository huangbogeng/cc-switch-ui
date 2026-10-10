import { ToastNotice } from '@/components/ToastNotice';
import { useConfirm } from '@/lib/useConfirm';
import { useEffect, useMemo, useState } from 'react';
import { Plus } from 'lucide-react';
import {
  saveProvider,
  recoverProviderOperation,
  deleteProvider,
  switchProvider,
  getCodexOAuthStatus,
  setProxyTarget,
  startProxy,
  stopProxy,
  type Provider,
} from '../api';
import type { CodexAccount } from '../api';
import type { ProviderPreset } from '@/config/providerPresets';
import { LocalRoutePanel } from '@/components/providers/LocalRoutePanel';
import { ProviderList } from '@/components/providers/ProviderList';
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
import { useQueryClient } from '@tanstack/react-query';
import { providerKeys, useProviders, useCurrentProvider, useRouteStatus, useRefreshProviders } from '@/lib/useProviders';
import { errorMessage } from '@/lib/errors';
import { useUnsavedChanges } from '@/lib/useUnsavedChanges';

export default function ProvidersPage() {
  const queryClient = useQueryClient();
  const providersQuery = useProviders();
  const currentQuery = useCurrentProvider();
  const routeQuery = useRouteStatus();
  const refreshProviders = useRefreshProviders();
  const providers = providersQuery.data?.providers;
  const currentProviderId = currentQuery.data?.current_provider_id ?? null;
  const loading = providersQuery.isLoading || currentQuery.isLoading;
  const { confirmAction, confirmation } = useConfirm();
  const [operationError, setError] = useState('');
  const queryError = providersQuery.error ?? currentQuery.error;
  const [dismissedQueryError, setDismissedQueryError] = useState<unknown>(null);
  const error = operationError || (queryError !== dismissedQueryError ? errorMessage(queryError, '') : '');
  const [showForm, setShowForm] = useState(false);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [selectedPreset, setSelectedPreset] = useState<ProviderPreset | null>(null);
  const [draftFormData, setDraftFormData] = useState<ProviderFormData>(emptyProviderForm);
  const [saving, setSaving] = useState(false);
  const [formError, setFormError] = useState('');
  const [editingBase, setEditingBase] = useState<Provider>();
  const [operationNotice, setOperationNotice] = useState('');
  const saveRecoveryRequired = currentQuery.data?.recovery_required ?? false;
  const [codexAccounts, setCodexAccounts] = useState<CodexAccount[]>([]);
  const proxyStatus = routeQuery.error ? null : routeQuery.data ?? null;
  const [operationProxyError, setProxyError] = useState('');
  const [dismissedRouteError, setDismissedRouteError] = useState<unknown>(null);
  const proxyError = operationProxyError || (routeQuery.error !== dismissedRouteError ? errorMessage(routeQuery.error, '') : '');
  const [pendingAction, setPendingAction] = useState<string | null>(null);
  const busy = pendingAction !== null || saving;
  useUnsavedChanges(false, busy);
  const unavailable = !loading && (!providersQuery.data || !currentQuery.data);
  const providerList = useMemo(() => sortProviders(providers ?? {}), [providers]);

  const applyProviders = async () => {
    try {
      await refreshProviders();
      setError('');
    } catch (e) {
      setError(errorMessage(e, 'Failed to refresh providers'));
    }
  };
  const loadProxyStatus = async () => {
    try {
      await queryClient.invalidateQueries({ queryKey: providerKeys.route }, { throwOnError: true });
      setProxyError('');
    } catch (e) {
      setProxyError(errorMessage(e, 'Failed to refresh local route status'));
    }
  };

  useEffect(() => {
    const ctrl = new AbortController();
    getCodexOAuthStatus({ signal: ctrl.signal })
      .then((status) => setCodexAccounts(status.accounts))
      .catch((e) => {
        if (e instanceof DOMException && e.name === 'AbortError') return;
        setCodexAccounts([]);
        setError(e instanceof Error ? e.message : 'Failed to load Codex accounts');
      });
    return () => ctrl.abort();
  }, []);

  const handleStartProxy = async (providerId: string) => {
    try {
      setPendingAction('route');
      setProxyError('');
      await setProxyTarget(providerId);
      const result = await startProxy();
      if (!result.success) throw new Error(result.error);
      await loadProxyStatus();
    } catch (e) {
      await loadProxyStatus();
      await applyProviders();
      setProxyError(e instanceof Error ? e.message : 'Failed to start local route');
    } finally {
      setPendingAction(null);
    }
  };

  const handleStopProxy = async () => {
    try {
      setPendingAction('route');
      setProxyError('');
      const result = await stopProxy();
      if (!result.success) throw new Error(result.error);
      await loadProxyStatus();
    } catch (e) {
      await loadProxyStatus();
      await applyProviders();
      setProxyError(e instanceof Error ? e.message : 'Failed to stop local route');
    } finally {
      setPendingAction(null);
    }
  };

  const handleProxyTargetChange = async (providerId: string) => {
    try {
      setPendingAction('route');
      setProxyError('');
      await setProxyTarget(providerId);
      await loadProxyStatus();
    } catch (e) {
      await loadProxyStatus();
      await applyProviders();
      setProxyError(e instanceof Error ? e.message : 'Failed to update route target');
    } finally {
      setPendingAction(null);
    }
  };

  const handleSwitch = async (id: string) => {
    try {
      setPendingAction(`switch:${id}`);
      setError('');
      setOperationNotice('');
      const result = await switchProvider(id);
      setOperationNotice([result.message, ...(result.warnings ?? [])].filter(Boolean).join(' ') || 'Direct provider selected.');
      await applyProviders();
      await loadProxyStatus();
    } catch (e) {
      await applyProviders();
      await loadProxyStatus();
      setError(e instanceof Error ? e.message : 'Switch failed');
    } finally {
      setPendingAction(null);
    }
  };

  const handlePresetSelect = (preset: ProviderPreset) => {
    setFormError('');
    setSelectedPreset(preset);
    setEditingId(null);
    setEditingBase(undefined);
    setDraftFormData(formFromPreset(preset));
    setShowForm(true);
  };

  const handleAdd = () => {
    setFormError('');
    setSelectedPreset(null);
    setEditingId(null);
    setEditingBase(undefined);
    setDraftFormData(emptyProviderForm);
    setShowForm(true);
  };

  const handleEdit = (provider: Provider) => {
    setFormError('');
    setEditingId(provider.id);
    setEditingBase(structuredClone(provider));
    setSelectedPreset(null);
    setDraftFormData(formFromProvider(provider));
    setShowForm(true);
  };

  const handleDelete = async (id: string) => {
    if (id === currentProviderId) {
      setError('Select another direct Provider before deleting this one.');
      return;
    }
    if (id === proxyStatus?.active_target_provider_id) {
      setError('Choose another route target before deleting this Provider.');
      return;
    }
    if (!await confirmAction('Delete this provider?', { title: 'Delete managed item?', confirmLabel: 'Delete', destructive: true })) return;
    try {
      setPendingAction(`delete:${id}`);
      setError(''); setOperationNotice('');
      await deleteProvider(id);
      setOperationNotice('Provider deleted.');
      await applyProviders();
      await loadProxyStatus();
    } catch (e) {
      await applyProviders();
      await loadProxyStatus();
      setError(e instanceof Error ? e.message : 'Delete failed');
    } finally {
      setPendingAction(null);
    }
  };

  const handleRecoverSave = async () => {
    setPendingAction('recover');
    setError(''); setOperationNotice('');
    try {
      const result = await recoverProviderOperation();
      setOperationNotice([result.message, ...(result.warnings ?? [])].filter(Boolean).join(' '));
      await applyProviders();
    } catch (e) {
      await applyProviders();
      await loadProxyStatus();
      setError(e instanceof Error ? e.message : 'Recovery failed');
    } finally {
      setPendingAction(null);
    }
  };

  const handleSubmit = async (formData: ProviderFormData) => {
    setSaving(true);
    setError('');
    setFormError('');
    setOperationNotice('');

    try {
      const id = formData.id.trim();
      const name = formData.name.trim();
      if (!id || !name) {
        throw new Error('Provider ID and name are required.');
      }
      if (formData.baseUrl.trim()) {
        let endpoint: URL;
        try {
          endpoint = new URL(formData.baseUrl.trim());
        } catch {
          throw new Error('Base URL must be a valid absolute URL.');
        }
        if (endpoint.protocol !== 'http:' && endpoint.protocol !== 'https:') {
          throw new Error('Base URL must use HTTP or HTTPS.');
        }
      }
      if (formData.authMode !== 'oauth_proxy' && !formData.noAuth && !formData.apiKey.trim()) {
        const providerName = selectedPreset?.name || name;
        const shouldContinue = await confirmAction(
          `${providerName} is missing an API key.\n\nSave anyway? Switching or routing through this provider may fail until you add credentials.`,
          { title: 'Save without credentials?', confirmLabel: 'Save anyway' },
        );
        if (!shouldContinue) {
          setSaving(false);
          return;
        }
      }
      const result = await saveProvider(buildProvider(formData, selectedPreset, editingBase), editingBase);
      setOperationNotice([result.message, ...(result.warnings ?? [])].filter(Boolean).join(' '));
      setShowForm(false);
      await applyProviders();
    } catch (e) {
      setFormError(e instanceof Error ? e.message : 'Save failed');
      await applyProviders();
    } finally {
      setSaving(false);
    }
  };

  return (
    <div>
      {confirmation}
      <PageHeader
        title="Providers"
        description="Your connections, direct configuration, and local route."
        action={
          <Button onClick={handleAdd} disabled={busy || showForm}>
            <Plus className="h-4 w-4" />
            Add Provider
          </Button>
        }
      />

      {error && <ToastNotice tone="error" message={error} onDismiss={() => { setError(''); setDismissedQueryError(queryError); }} />}


      {saveRecoveryRequired && (
        <div role="alert" className="mb-4 rounded-xl border px-4 py-3 text-sm">
          An unfinished provider operation needs recovery before switching or changing the local route.
          <Button className="ml-3" onClick={() => void handleRecoverSave()} disabled={saving || pendingAction !== null}>Recover operation</Button>
        </div>
      )}

      {operationNotice && <ToastNotice message={operationNotice} onDismiss={() => setOperationNotice('')} />}


      {proxyError && <ToastNotice tone="error" message={proxyError} onDismiss={() => { setProxyError(''); setDismissedRouteError(routeQuery.error); }} />}


      {loading ? (
        <Card>
          <CardContent className="p-12 flex flex-col items-center justify-center text-sm leading-5 text-muted-foreground">
            <div className="h-8 w-8 rounded-full border-2 border-primary border-t-transparent animate-spin mb-4"></div>
            Loading providers...
          </CardContent>
        </Card>
      ) : unavailable ? (
        <Card>
          <CardContent className="flex flex-col items-center justify-center gap-3 px-6 py-12 text-center">
            <p className="text-sm font-medium">Unable to load provider configuration</p>
            <p className="max-w-sm text-sm text-muted-foreground">Check the connection and retry to see your saved providers and current selection.</p>
            <Button variant="outline" disabled={busy || providersQuery.isFetching || currentQuery.isFetching}
              onClick={() => void applyProviders()}>{providersQuery.isFetching || currentQuery.isFetching ? 'Retrying...' : 'Retry'}</Button>
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
            <Button onClick={handleAdd} disabled={busy || showForm} className="mt-6 shadow-[0_0_20px_rgba(var(--primary),0.15)]">
              <Plus className="h-4 w-4 mr-2" />
              Add First Provider
            </Button>
          </CardContent>
        </Card>
      ) : (
        <div className="grid items-start gap-6 xl:grid-cols-[minmax(0,1fr)_320px]">
          <ProviderList providers={providerList} currentProviderId={currentProviderId} busy={busy || showForm || unavailable}
            onSwitch={handleSwitch} onEdit={handleEdit} onDelete={handleDelete} />
          <aside aria-label="Local route controls" className="min-w-0 xl:sticky xl:top-6">
            <LocalRoutePanel
              providers={providerList}
              currentProviderId={currentProviderId}
              proxyRunning={proxyStatus?.running ?? false}
              recoveryRequired={proxyStatus?.recovery_required ?? false}
              statusUnavailable={routeQuery.isLoading || Boolean(routeQuery.error)}
              proxyTargetId={proxyStatus?.active_target_provider_id ?? currentProviderId ?? providerList[0]?.id ?? null}
              busy={busy || showForm || unavailable}
              onTargetChange={(providerId) => void handleProxyTargetChange(providerId)}
              onStartProxy={(providerId) => void handleStartProxy(providerId)}
              onStopProxy={handleStopProxy}
            />
          </aside>
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
