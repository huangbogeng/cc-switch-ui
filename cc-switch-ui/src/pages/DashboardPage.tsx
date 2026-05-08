import { useCallback, useEffect, useState } from 'react';
import {
  getCopilotUsage,
  listProviders, switchProvider, getCurrentProviderId,
  getProxyStatus, startProxy, stopProxy, setProxyTarget,
  getProxyUsageSummary,
  type Provider,
} from '../api';
import type { CopilotUsageResponse } from '../api';
import { PageHeader } from '@/components/PageHeader';
import {
  CurrentProviderCard,
  ProviderGrid,
  ProxyCard,
  ProxyUsageCard,
  UsageCard,
} from '@/components/dashboard/DashboardPanels';
import { sortProviders } from '@/lib/provider';

export default function DashboardPage() {
  const [currentProviderId, setCurrentProviderId] = useState<string | null>(null);
  const [providers, setProviders] = useState<Record<string, Provider>>({});
  const [loadingProviders, setLoadingProviders] = useState(true);

  const [usage, setUsage] = useState<CopilotUsageResponse | null>(null);

  const [proxyUsage, setProxyUsage] = useState<{
    total_input_tokens: number;
    total_output_tokens: number;
    total_requests: number;
  } | null>(null);

  const [proxyStatus, setProxyStatus] = useState<{
    running: boolean;
    listen_addr: string | null;
    upstream_url: string;
    http_proxy_url: string | null;
    active_target_provider_id: string | null;
    active_target_provider_name: string | null;
  } | null>(null);
  const [proxyError, setProxyError] = useState('');

  const loadUsage = useCallback(async () => {
    try {
      const data = await getCopilotUsage();
      setUsage(data);
    } catch {
      // Silently fail - user may not be authenticated
    }
  }, []);

  const loadProviders = useCallback(async () => {
    try {
      const data = await listProviders();
      setProviders(data.providers);
      const current = await getCurrentProviderId().catch(() => ({ current_provider_id: null }));
      setCurrentProviderId(current.current_provider_id);
    } catch (e) {
      console.error('Providers error:', e);
    } finally {
      setLoadingProviders(false);
    }
  }, []);

  const loadProxyStatus = useCallback(async () => {
    try {
      const status = await getProxyStatus();
      setProxyStatus(status);
    } catch (e) {
      console.error('Proxy status error:', e);
    }
  }, []);

  const loadProxyUsage = useCallback(async () => {
    try {
      const data = await getProxyUsageSummary();
      setProxyUsage({
        total_input_tokens: data.totals.input_tokens,
        total_output_tokens: data.totals.output_tokens,
        total_requests: data.totals.request_count,
      });
    } catch (e) {
      console.error('Proxy usage error:', e);
    }
  }, []);

  const loadAll = useCallback(async () => {
    await Promise.all([
      loadUsage(),
      loadProviders(),
      loadProxyStatus(),
      loadProxyUsage(),
    ]);
  }, [loadUsage, loadProviders, loadProxyStatus, loadProxyUsage]);

  useEffect(() => {
    Promise.resolve().then(loadAll);
    const interval = setInterval(loadAll, 5000);
    return () => clearInterval(interval);
  }, [loadAll]);

  const handleSwitchProvider = async (id: string) => {
    try {
      await switchProvider(id);
      setCurrentProviderId(id);
      // If local route is running, restart it for the new provider.
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
          <CurrentProviderCard loading={loadingProviders} provider={currentProvider} />
          <ProxyCard
            status={proxyStatus}
            targetProviders={providerList}
            error={proxyError}
            onToggle={handleToggleRoute}
            onTargetChange={handleRouteTargetChange}
          />
          {usage && <UsageCard usage={usage} />}
          {proxyUsage && proxyUsage.total_requests > 0 && (
            <ProxyUsageCard
              totalInputTokens={proxyUsage.total_input_tokens}
              totalOutputTokens={proxyUsage.total_output_tokens}
              totalRequests={proxyUsage.total_requests}
            />
          )}
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
