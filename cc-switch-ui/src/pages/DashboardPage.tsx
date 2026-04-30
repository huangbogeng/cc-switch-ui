import { useCallback, useEffect, useState } from 'react';
import {
  getCopilotUsage,
  listProviders, switchProvider, getCurrentProviderId,
  getProxyStatus, startProxy, stopProxy, setProxyTarget,
  type Provider,
} from '../api';
import type { CopilotUsageResponse } from '../api';
import { PageHeader } from '@/components/PageHeader';
import {
  CurrentProviderCard,
  ProviderGrid,
  ProxyCard,
  UsageCard,
} from '@/components/dashboard/DashboardPanels';
import { providerAuthMode, sortProviders } from '@/lib/provider';

export default function DashboardPage() {
  const [currentProviderId, setCurrentProviderId] = useState<string | null>(null);
  const [providers, setProviders] = useState<Record<string, Provider>>({});
  const [loadingProviders, setLoadingProviders] = useState(true);

  const [usage, setUsage] = useState<CopilotUsageResponse | null>(null);

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
    } catch (e) {
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

  const loadAll = useCallback(async () => {
    await Promise.all([
      loadUsage(),
      loadProviders(),
      loadProxyStatus(),
    ]);
  }, [loadUsage, loadProviders, loadProxyStatus]);

  useEffect(() => {
    Promise.resolve().then(loadAll);
    const interval = setInterval(loadAll, 5000);
    return () => clearInterval(interval);
  }, [loadAll]);

  const handleSwitchProvider = async (id: string) => {
    try {
      await switchProvider(id);
      setCurrentProviderId(id);
    } catch (e) {
      console.error('Switch provider error:', e);
    }
  };

  const handleProxyTargetChange = async (id: string) => {
    if (!id) return;
    try {
      setProxyError('');
      const result = await setProxyTarget(id);
      if (!result.success) {
        throw new Error(result.error || 'Failed to set proxy target');
      }
      loadProxyStatus();
    } catch (e) {
      setProxyError(e instanceof Error ? e.message : 'Failed to set proxy target');
    }
  };

  const handleToggleProxy = async () => {
    try {
      setProxyError('');
      if (proxyStatus?.running) {
        const result = await stopProxy();
        if (!result.success) throw new Error(result.error || 'Failed to stop proxy');
      } else {
        const result = await startProxy();
        if (!result.success) throw new Error(result.error || 'Failed to start proxy');
      }
      loadProxyStatus();
    } catch (e) {
      setProxyError(e instanceof Error ? e.message : 'Proxy operation failed');
    }
  };

  const currentProvider = currentProviderId ? providers[currentProviderId] : null;
  const providerList = sortProviders(providers);
  const proxyTargetProviders = providerList.filter((p) => providerAuthMode(p) === 'oauth_proxy');

  return (
    <div>
      <PageHeader
        title="Dashboard"
        description="Monitor active provider and proxy status."
      />
      <div className="space-y-6">
        <div className="grid grid-cols-1 gap-4 xl:grid-cols-2">
          <CurrentProviderCard loading={loadingProviders} provider={currentProvider} />
          <ProxyCard
            status={proxyStatus}
            targetProviders={proxyTargetProviders}
            error={proxyError}
            onToggle={handleToggleProxy}
            onTargetChange={handleProxyTargetChange}
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