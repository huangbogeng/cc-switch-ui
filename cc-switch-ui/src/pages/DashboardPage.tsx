import { useMemo } from 'react';
import { useProviders, useCurrentProvider, useRouteStatus } from '@/lib/useProviders';
import { PageHeader } from '@/components/PageHeader';
import {
  DashboardHeroCard,
  UsageCard,
} from '@/components/dashboard/DashboardPanels';
import UsageTrendChart from '@/components/usage/UsageTrendChart';
import { sortProviders } from '@/lib/provider';
import { useCopilotUsage, useUsageSummary } from '@/lib/useUsage';
import { resolveUsageRange } from '@/lib/usageRange';
import { ErrorAlert } from '@/components/ErrorAlert';
import { errorMessage } from '@/lib/errors';

export default function DashboardPage() {
  const providersQuery = useProviders();
  const currentQuery = useCurrentProvider();
  const routeQuery = useRouteStatus();
  const providers = providersQuery.data?.providers;
  const currentProviderId = currentQuery.data?.current_provider_id;
  const proxyStatus = routeQuery.error ? null : routeQuery.data ?? null;
  const usageRange = useMemo(() => resolveUsageRange({ preset: '30d' }), []);
  const { data: usage } = useCopilotUsage();
  const usageSummaryQuery = useUsageSummary(usageRange.startDate, usageRange.endDate);
  const queryError = providersQuery.error ?? currentQuery.error ?? routeQuery.error ?? usageSummaryQuery.error;
  const providerList = useMemo(() => sortProviders(providers ?? {}), [providers]);
  const currentProvider = currentProviderId ? providers?.[currentProviderId] ?? null : null;
  const routeTarget = providerList.find((provider) => provider.id === proxyStatus?.active_target_provider_id) ?? null;

  return (
    <div className="space-y-8 animate-in fade-in slide-in-from-bottom-4 duration-500">
      <PageHeader
        title="Dashboard"
        description="Read-only runtime overview. Manage providers and local route from the Providers page."
      />
      {queryError && (
        <ErrorAlert message={errorMessage(queryError, 'Failed to load dashboard')} />
      )}
      <div className="space-y-8">
        <DashboardHeroCard
          currentProvider={currentProvider}
          routeTarget={routeTarget}
          status={proxyStatus}
        />
        <div className="grid grid-cols-1 gap-6 xl:grid-cols-[minmax(0,1.25fr)_360px]">
          <div className={usage ? '' : 'xl:col-span-2'}>
            <UsageTrendChart trend={usageSummaryQuery.data?.trend ?? []} loading={usageSummaryQuery.isLoading} />
          </div>
          {usage && <UsageCard usage={usage} />}
        </div>
      </div>
    </div>
  );
}
