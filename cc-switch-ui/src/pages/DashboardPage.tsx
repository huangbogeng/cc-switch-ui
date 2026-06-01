import { useCallback, useEffect, useState } from 'react';
import {
  listProviders, getCurrentProviderId,
  getProxyStatus,
  type Provider,
} from '../api';
import { PageHeader } from '@/components/PageHeader';
import {
  DashboardHeroCard,
  UsageCard,
} from '@/components/dashboard/DashboardPanels';
import UsageTrendChart from '@/components/usage/UsageTrendChart';
import { sortProviders } from '@/lib/provider';
import { useCopilotUsage, useUsageSummary } from '@/lib/useUsage';
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
  const { data: usage } = useCopilotUsage();
  const { data: usageSummary, isLoading: usageSummaryLoading } = useUsageSummary();

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

  const currentProvider = currentProviderId ? providers[currentProviderId] : null;
  const providerList = sortProviders(providers);
  const routeTarget =
    providerList.find((provider) => provider.id === proxyStatus?.active_target_provider_id) ?? null;

  return (
    <div className="space-y-8 animate-in fade-in slide-in-from-bottom-4 duration-500">
      <PageHeader
        title="Dashboard"
        description="Read-only runtime overview. Manage providers and local route from the Providers page."
      />
      <div className="space-y-8">
        <DashboardHeroCard
          currentProvider={currentProvider}
          routeTarget={routeTarget}
          status={proxyStatus}
        />
        <div className="grid grid-cols-1 gap-6 xl:grid-cols-[minmax(0,1.25fr)_360px]">
          <div className={usage ? '' : 'xl:col-span-2'}>
            <UsageTrendChart trend={usageSummary?.trend ?? []} loading={usageSummaryLoading} />
          </div>
          {usage && <UsageCard usage={usage} />}
        </div>
      </div>
    </div>
  );
}
