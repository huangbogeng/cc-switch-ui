import { useCallback, useEffect, useState } from 'react';
import {
  listProviders, switchProvider, getCurrentProviderId,
  getProxyStatus, startProxy, stopProxy, setProxyTarget,
  type Provider,
} from '../api';
import { PageHeader } from '@/components/PageHeader';
import {
  CurrentProviderCard,
  ProviderGrid,
  ProxyCard,
  UsageCard,
} from '@/components/dashboard/DashboardPanels';
import { sortProviders } from '@/lib/provider';
import { useUsageSummary, useCopilotUsage } from '@/lib/useUsage';
import { cacheGet, cacheSet } from '@/lib/fetchCache';

interface DashboardProxyStatus {
  running: boolean;
  listen_addr: string | null;
  upstream_url: string;
  http_proxy_url: string | null;
  active_target_provider_id: string | null;
  active_target_provider_name: string | null;
}

const CACHE_KEY = 'dashboard';

export default function DashboardPage() {
  const cached = cacheGet<{
    providers: Record<string, Provider>;
    currentProviderId: string | null;
    proxyStatus: DashboardProxyStatus | null;
  }>(CACHE_KEY);

  const [currentProviderId, setCurrentProviderId] = useState<string | null>(
    cached?.currentProviderId ?? null,
  );
  const [providers, setProviders] = useState<Record<string, Provider>>(
    cached?.providers ?? {},
  );
  const [loadingProviders, setLoadingProviders] = useState(!cached);
  const [proxyStatus, setProxyStatus] = useState<DashboardProxyStatus | null>(
    cached?.proxyStatus ?? null,
  );
  const [proxyError, setProxyError] = useState('');

  const { data: usage } = useCopilotUsage();
  const { data: proxyUsage } = useUsageSummary(30_000);

  const loadProviders = useCallback(async (signal?: AbortSignal) => {
    try {
      const data = await listProviders({ signal });
      setProviders(data.providers);
      const current = await getCurrentProviderId({ signal }).catch(
        () => ({ current_provider_id: null }),
      );
      setCurrentProviderId(current.current_provider_id);
    } catch (e) {
      if (e instanceof DOMException && e.name === 'AbortError') return;
      console.error('Providers error:', e);
    } finally {
      if (!signal?.aborted) setLoadingProviders(false);
    }
  }, []);

  const loadProxyStatus = useCallback(async (signal?: AbortSignal) => {
    try {
      const status = await getProxyStatus({ signal });
      setProxyStatus(status);
    } catch (e) {
      if (e instanceof DOMException && e.name === 'AbortError') return;
      console.error('Proxy status error:', e);
    }
  }, []);

  const loadAll = useCallback(async (signal?: AbortSignal) => {
    await Promise.all([
      loadProviders(signal),
      loadProxyStatus(signal),
    ]);
  }, [loadProviders, loadProxyStatus]);

  useEffect(() => {
    const ctrl = new AbortController();

    Promise.resolve().then(() => loadAll(ctrl.signal));
    const interval = setInterval(() => loadAll(ctrl.signal), 5000);

    return () => {
      ctrl.abort();
      clearInterval(interval);
    };
  }, [loadAll]);

  // Update cache when data changes after initial load
  useEffect(() => {
    if (!loadingProviders) {
      cacheSet(CACHE_KEY, { providers, currentProviderId, proxyStatus });
    }
  }, [providers, currentProviderId, proxyStatus, loadingProviders]);

  const handleSwitchProvider = async (id: string) => {
    try {
      await switchProvider(id);
      setCurrentProviderId(id);
      if (proxyStatus?.running) {
        await stopProxy();
        await startProxy();
        await loadProxyStatus();
      }
    } catch (e) {
      console.error('Switch provider error:', e);
    }
  };

  const handleRouteTargetChange = async (providerId: string) => {
    if (!providerId) return;
    try {
      setProxyError('');
      const result = await setProxyTarget(providerId);
      if (!result.success) throw new Error(result.error || 'Failed to set route target');
      await loadProxyStatus();
    } catch (e) {
      setProxyError(e instanceof Error ? e.message : 'Failed to set route target');
    }
  };

  const handleToggleRoute = async () => {
    try {
      setProxyError('');
      if (proxyStatus?.running) {
        const result = await stopProxy();
        if (!result.success) throw new Error(result.error || 'Failed to stop local route');
      } else {
        const result = await startProxy();
        if (!result.success) throw new Error(result.error || 'Failed to start local route');
      }
      await loadProxyStatus();
    } catch (e) {
      setProxyError(e instanceof Error ? e.message : 'Local route operation failed');
    }
  };

  const currentProvider = currentProviderId ? providers[currentProviderId] : null;
  const providerList = sortProviders(providers);

  return (
    <div className="space-y-8 animate-in fade-in slide-in-from-bottom-4 duration-500">
      <PageHeader
        title="Dashboard"
        description="Monitor active provider and local route status."
      />
      {proxyError && (
        <div className="rounded-xl border border-destructive/30 bg-destructive/10 px-4 py-3 text-sm text-destructive shadow-sm">
          {proxyError}
        </div>
      )}
      <div className="space-y-8">
        <div className="grid grid-cols-1 gap-6 xl:grid-cols-2">
          <CurrentProviderCard
            loading={loadingProviders}
            provider={currentProvider}
            usage24h={
              currentProvider
                ? (() => {
                    // Proxy mode: match by provider_id
                    const byProvider = (proxyUsage?.providers || []).find(
                      (item) => item.provider_id === currentProvider.id,
                    );
                    if (byProvider) {
                      return {
                        requestCount: byProvider.request_count,
                        inputTokens: byProvider.input_tokens,
                        outputTokens: byProvider.output_tokens,
                      };
                    }
                    // Direct mode: show total session usage
                    if (proxyUsage?.totals && proxyUsage.totals.request_count > 0) {
                      return {
                        requestCount: proxyUsage.totals.request_count,
                        inputTokens: proxyUsage.totals.input_tokens,
                        outputTokens: proxyUsage.totals.output_tokens,
                      };
                    }
                    return null;
                  })()
                : null
            }
            routeRuntime={
              proxyStatus
                ? {
                    running: proxyStatus.running,
                    listenAddr: proxyStatus.listen_addr,
                    activeTargetProviderId: proxyStatus.active_target_provider_id,
                  }
                : null
            }
            routeError={proxyError || undefined}
          />
          <ProxyCard
            status={proxyStatus}
            targetProviders={providerList}
            error={proxyError}
            onToggle={handleToggleRoute}
            onTargetChange={handleRouteTargetChange}
          />
          {usage && <UsageCard usage={usage} />}
        </div>

        <ProviderGrid
          providers={providerList}
          currentProviderId={currentProviderId}
          onSwitch={handleSwitchProvider}
        />
      </div>
    </div>
  );
}
