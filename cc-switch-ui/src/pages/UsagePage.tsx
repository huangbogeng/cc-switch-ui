import { useMemo, useState } from 'react';
import { RefreshCw, ListFilter, Activity, BarChart3, DollarSign } from 'lucide-react';
import { PageHeader } from '@/components/PageHeader';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { Button } from '@/components/ui/button';
import { resolveUsageRange } from '@/lib/usageRange';
import type { UsageRangeSelection } from '@/lib/usageRange';
import { useUsageSummary, useProviderStats, useModelStats, useSyncSession, useDataSourceBreakdown } from '@/lib/useUsage';
import UsageSummaryCards from '@/components/usage/UsageSummaryCards';
import UsageTrendChart from '@/components/usage/UsageTrendChart';
import UsageDateRangePicker from '@/components/usage/UsageDateRangePicker';
import ProviderStatsTable from '@/components/usage/ProviderStatsTable';
import ModelStatsTable from '@/components/usage/ModelStatsTable';
import RequestLogTable from '@/components/usage/RequestLogTable';
import ModelPricingPanel from '@/components/usage/ModelPricingPanel';
import DataSourceBar from '@/components/usage/DataSourceBar';
import type { LogsQueryParams } from '@/api';
import { useQueryClient } from '@tanstack/react-query';
import { usageKeys } from '@/lib/useUsage';
import { ErrorAlert } from '@/components/ErrorAlert';
import { errorMessage } from '@/lib/errors';
import { formatRangeLabel } from '@/lib/usageRange';

export default function UsagePage() {
  const queryClient = useQueryClient();
  const [range, setRange] = useState<UsageRangeSelection>({ preset: '30d' });
  const [refreshMs, setRefreshMs] = useState(30_000);

  const resolved = useMemo(() => resolveUsageRange(range), [range]);
  const logParams: LogsQueryParams = useMemo(
    () => ({
      start_date: resolved.startDate,
      end_date: resolved.endDate,
    }),
    [resolved],
  );

  const summaryQuery = useUsageSummary(resolved.startDate, resolved.endDate, refreshMs);
  const providerQuery = useProviderStats(
    resolved.startDate,
    resolved.endDate,
    refreshMs,
  );
  const modelQuery = useModelStats(
    resolved.startDate,
    resolved.endDate,
    refreshMs,
  );
  const sourcesQuery = useDataSourceBreakdown(resolved.startDate, resolved.endDate, refreshMs);

  // Session sync for direct-connect mode
  const syncMutation = useSyncSession();

  // Cycle through refresh intervals
  const REFRESH_OPTIONS = [0, 5_000, 10_000, 30_000, 60_000] as const;
  const cycleRefresh = () => {
    const idx = REFRESH_OPTIONS.indexOf(refreshMs as (typeof REFRESH_OPTIONS)[number]);
    const next = REFRESH_OPTIONS[(idx + 1) % REFRESH_OPTIONS.length];
    setRefreshMs(next);
    queryClient.invalidateQueries({ queryKey: usageKeys.all });
  };

  const refreshLabel = refreshMs > 0 ? `${refreshMs / 1000}s` : 'OFF';
  const queryError = summaryQuery.error ?? providerQuery.error ?? modelQuery.error ?? sourcesQuery.error;
  const rangeLabel = formatRangeLabel(range, resolved);

  return (
    <div className="space-y-8 animate-in fade-in slide-in-from-bottom-4 duration-500">
      <PageHeader
        title="Usage"
        description="Monitor local route usage across all providers."
        action={
          <div className="flex items-center gap-2">
            <UsageDateRangePicker selection={range} onChange={setRange} />
            <Button
              variant="outline"
              size="sm"
              onClick={() => syncMutation.mutate()}
              disabled={syncMutation.isPending}
              className="gap-1.5"
            >
              <RefreshCw className={syncMutation.isPending ? 'animate-spin h-3.5 w-3.5' : 'h-3.5 w-3.5'} />
              {syncMutation.isPending ? 'Syncing...' : 'Sync'}
            </Button>
            <Button
              variant="ghost"
              size="sm"
              className="h-8 px-2 text-xs text-muted-foreground"
              title="Toggle auto-refresh"
              onClick={cycleRefresh}
            >
              <RefreshCw className="mr-1 h-3.5 w-3.5" />
              {refreshLabel}
            </Button>
          </div>
        }
      />

      {queryError && (
        <ErrorAlert message={errorMessage(queryError, 'Failed to load usage data')} />
      )}
      {syncMutation.error && (
        <ErrorAlert message={errorMessage(syncMutation.error, 'Failed to sync session logs')} />
      )}

      <UsageSummaryCards data={summaryQuery.data} loading={summaryQuery.isLoading} />
      <UsageTrendChart
        trend={summaryQuery.data?.trend ?? []}
        loading={summaryQuery.isLoading}
        title={`${rangeLabel} Token Trend`}
      />

      <DataSourceBar sources={sourcesQuery.data} />

      <Tabs defaultValue="logs" className="w-full">
        <TabsList className="bg-muted/50">
          <TabsTrigger value="logs" className="gap-2">
            <ListFilter className="h-4 w-4" />
            Request Logs
          </TabsTrigger>
          <TabsTrigger value="providers" className="gap-2">
            <Activity className="h-4 w-4" />
            Providers
          </TabsTrigger>
          <TabsTrigger value="models" className="gap-2">
            <BarChart3 className="h-4 w-4" />
            Models
          </TabsTrigger>
          <TabsTrigger value="pricing" className="gap-2">
            <DollarSign className="h-4 w-4" />
            Pricing
          </TabsTrigger>
        </TabsList>

        <TabsContent value="logs" className="mt-4">
          <RequestLogTable
            key={`${resolved.startDate}:${resolved.endDate}`}
            params={logParams}
            refreshMs={refreshMs}
          />
        </TabsContent>

        <TabsContent value="providers" className="mt-4">
          <ProviderStatsTable data={providerQuery.data?.providers} loading={providerQuery.isLoading} />
        </TabsContent>

        <TabsContent value="models" className="mt-4">
          <ModelStatsTable data={modelQuery.data?.models} loading={modelQuery.isLoading} />
        </TabsContent>

        <TabsContent value="pricing" className="mt-4">
          <ModelPricingPanel />
        </TabsContent>
      </Tabs>
    </div>
  );
}
