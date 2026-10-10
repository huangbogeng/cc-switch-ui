import { ToastNotice } from '@/components/ToastNotice';
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
import UsageHeatmap from '@/components/usage/UsageHeatmap';
import UsageDateRangePicker from '@/components/usage/UsageDateRangePicker';
import ProviderStatsTable from '@/components/usage/ProviderStatsTable';
import ModelStatsTable from '@/components/usage/ModelStatsTable';
import RequestLogTable from '@/components/usage/RequestLogTable';
import ModelPricingPanel from '@/components/usage/ModelPricingPanel';
import DataSourceBar from '@/components/usage/DataSourceBar';
import type { LogsQueryParams } from '@/api';
import { useQueryClient } from '@tanstack/react-query';
import { usageKeys } from '@/lib/useUsage';
import { Card, CardContent } from '@/components/ui/card';
import { useUnsavedChanges } from '@/lib/useUnsavedChanges';
import type { SessionSyncResult } from '@/api';
import { errorMessage } from '@/lib/errors';
import { formatRangeLabel } from '@/lib/usageRange';

export default function UsagePage() {
  const queryClient = useQueryClient();
  const [range, setRange] = useState<UsageRangeSelection>({ preset: '30d' });
  const [activeTab, setActiveTab] = useState('logs');
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
    activeTab === 'providers',
  );
  const modelQuery = useModelStats(
    resolved.startDate,
    resolved.endDate,
    refreshMs,
    activeTab === 'models',
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
  const [dismissedError, setDismissedError] = useState<unknown>(null);
  const [dismissedSync, setDismissedSync] = useState<SessionSyncResult | null>(null);
  useUnsavedChanges(false, syncMutation.isPending);
  const queryError = summaryQuery.error ?? (activeTab === 'providers' ? providerQuery.error : activeTab === 'models' ? modelQuery.error : null) ?? sourcesQuery.error;
  const refreshing = summaryQuery.isFetching || sourcesQuery.isFetching || (activeTab === 'providers' && providerQuery.isFetching) || (activeTab === 'models' && modelQuery.isFetching);
  const refresh = () => { void queryClient.invalidateQueries({ queryKey: usageKeys.all }); };
  const summaryUnavailable = summaryQuery.error && !summaryQuery.data;
  const rangeLabel = formatRangeLabel(range, resolved);

  return (
    <div className="space-y-6">
      <PageHeader
        title="Usage"
        description="Monitor local route usage across all providers."
        action={
          <div className="flex flex-wrap items-center gap-2">
            <UsageDateRangePicker selection={range} onChange={setRange} />
            <Button variant="outline" size="sm" onClick={refresh} disabled={refreshing}>{refreshing ? 'Refreshing...' : 'Refresh'}</Button>
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

      {queryError && queryError !== dismissedError && (
        <ToastNotice tone="error" message={errorMessage(queryError, 'Failed to load usage data')} onDismiss={() => setDismissedError(queryError)} />
      )}
      {syncMutation.error && (
        <ToastNotice tone="error" message={errorMessage(syncMutation.error, 'Failed to sync session logs')} onDismiss={() => syncMutation.reset()} />
      )}

      {syncMutation.data && syncMutation.data !== dismissedSync && <ToastNotice
        tone={syncMutation.data.errors.length ? 'warning' : 'success'}
        message={`Session sync ${syncMutation.data.errors.length ? 'partially completed' : 'completed'}: ${syncMutation.data.imported} imported, ${syncMutation.data.skipped} skipped, ${syncMutation.data.errors.length} ${syncMutation.data.errors.length === 1 ? 'error' : 'errors'}.`}
        onDismiss={() => setDismissedSync(syncMutation.data ?? null)}>
        {syncMutation.data.errors.length > 0 && <details className="mt-2 text-xs"><summary className="cursor-pointer">Sync errors</summary><ul className="mt-2 max-h-48 overflow-y-auto space-y-1">{syncMutation.data.errors.map((message, index) => <li key={index} className="break-words">{message}</li>)}</ul></details>}
      </ToastNotice>}

      <UsageSummaryCards data={summaryQuery.data} loading={summaryQuery.isLoading} />
      {summaryUnavailable ? <Card><CardContent className="flex flex-col items-center gap-3 px-6 py-12 text-center">
        <p className="text-sm text-muted-foreground">Usage activity is unavailable.</p>
        <Button variant="outline" disabled={summaryQuery.isFetching} onClick={() => void summaryQuery.refetch()}>Retry usage activity</Button>
      </CardContent></Card> : <>
      <UsageTrendChart
        trend={summaryQuery.data?.trend ?? []}
        loading={summaryQuery.isLoading}
        title={`${rangeLabel} Token Trend`}
      />

      <UsageHeatmap
        trend={summaryQuery.data?.trend ?? []}
        loading={summaryQuery.isLoading}
        title={`${rangeLabel} Activity`}
        rangeStart={resolved.startDate}
        rangeEnd={resolved.endDate}
      />

      </>}

      <DataSourceBar sources={sourcesQuery.data} />

      <Tabs value={activeTab} onValueChange={setActiveTab} className="w-full">
        <TabsList className="w-full flex-nowrap justify-start overflow-x-auto bg-muted/50 sm:w-auto [&>button]:shrink-0">
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
          {providerQuery.error && !providerQuery.data ? <div className="flex flex-col items-center gap-3 py-8 text-sm text-muted-foreground">
            <p>Provider statistics are unavailable.</p>
            <Button variant="outline" disabled={providerQuery.isFetching} onClick={() => void providerQuery.refetch()}>Retry provider statistics</Button>
          </div> : <ProviderStatsTable data={providerQuery.data?.providers} loading={providerQuery.isLoading} />}
        </TabsContent>

        <TabsContent value="models" className="mt-4">
          {modelQuery.error && !modelQuery.data ? <div className="flex flex-col items-center gap-3 py-8 text-sm text-muted-foreground">
            <p>Model statistics are unavailable.</p>
            <Button variant="outline" disabled={modelQuery.isFetching} onClick={() => void modelQuery.refetch()}>Retry model statistics</Button>
          </div> : <ModelStatsTable data={modelQuery.data?.models} loading={modelQuery.isLoading} />}
        </TabsContent>

        <TabsContent value="pricing" className="mt-4">
          <ModelPricingPanel />
        </TabsContent>
      </Tabs>
    </div>
  );
}
