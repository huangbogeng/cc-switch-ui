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
import { Button } from '@/components/ui/button';
import { Card, CardContent } from '@/components/ui/card';
import { ArrowUpRight } from 'lucide-react';

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
    <div className="space-y-6">
      <PageHeader
        title="Dashboard"
        description="Your connections and activity, at a glance."
        action={<Button variant="outline" asChild><a href="#providers">Manage providers<ArrowUpRight className="h-4 w-4" /></a></Button>}
      />
      {queryError && (
        <ErrorAlert message={errorMessage(queryError, 'Failed to load dashboard')} />
      )}
      <div className="space-y-6">
        <DashboardHeroCard
          currentProvider={currentProvider}
          routeTarget={routeTarget}
          status={proxyStatus}
          loading={providersQuery.isLoading || currentQuery.isLoading || routeQuery.isLoading}
          providerUnavailable={Boolean(providersQuery.error || currentQuery.error)}
        />
        <section aria-label="Activity in the last 30 days" className="space-y-3">
          <div className="flex items-center justify-between"><h2 className="text-sm font-semibold">Recent activity</h2><span className="text-xs text-muted-foreground">Last 30 days</span></div>
          <Card><CardContent className="grid grid-cols-1 divide-y divide-white/[0.07] p-0 sm:px-0 sm:grid-cols-3 sm:divide-x sm:divide-y-0">
            {[
              ['Requests', usageSummaryQuery.data?.totals.request_count],
              ['Input tokens', usageSummaryQuery.data?.totals.input_tokens],
              ['Output tokens', usageSummaryQuery.data?.totals.output_tokens],
            ].map(([label, value]) => <div key={label} className="px-5 py-4 sm:px-6">
              <div className="text-xs text-muted-foreground">{label}</div>
              <div className="mt-1 text-2xl font-semibold tracking-tight tabular-nums">{usageSummaryQuery.isLoading ? '…' : typeof value === 'number' ? value.toLocaleString() : '—'}</div>
            </div>)}
          </CardContent></Card>
        </section>
        <div className="grid grid-cols-1 gap-6 xl:grid-cols-[minmax(0,1fr)_320px]">
          <div className={usage ? 'min-w-0' : 'min-w-0 xl:col-span-2'}>
            <UsageTrendChart trend={usageSummaryQuery.data?.trend ?? []} loading={usageSummaryQuery.isLoading} />
          </div>
          {usage && <UsageCard usage={usage} />}
        </div>
      </div>
    </div>
  );
}
