import { useEffect, useMemo, useState } from 'react';
import { RefreshCw, ListFilter, Activity, BarChart3, DollarSign } from 'lucide-react';
import { PageHeader } from '@/components/PageHeader';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { Button } from '@/components/ui/button';
import { resolveUsageRange } from '@/lib/usageRange';
import type { UsageRangeSelection } from '@/lib/usageRange';
import { useUsageSummary, useProviderStats, useModelStats, useSyncSession } from '@/lib/useUsage';
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

  const { data: summary, isLoading: summaryLoading } = useUsageSummary(refreshMs);
  const { data: providerStats, isLoading: providerLoading } = useProviderStats(
    resolved.startDate,
    resolved.endDate,
  );
  const { data: modelStats, isLoading: modelLoading } = useModelStats(
    resolved.startDate,
    resolved.endDate,
  );

  // Session sync for direct-connect mode
  const syncMutation = useSyncSession();
  const [syncDone, setSyncDone] = useState(false);

  useEffect(() => {
    if (syncDone) return;
    syncMutation.mutate(undefined, {
      onSettled: () => setSyncDone(true),
    });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [syncDone]);

  // Cycle through refresh intervals
  const REFRESH_OPTIONS = [0, 5_000, 10_000, 30_000, 60_000] as const;
  const cycleRefresh = () => {
    const idx = REFRESH_OPTIONS.indexOf(refreshMs as (typeof REFRESH_OPTIONS)[number]);
    const next = REFRESH_OPTIONS[(idx + 1) % REFRESH_OPTIONS.length];
    setRefreshMs(next);
    queryClient.invalidateQueries({ queryKey: usageKeys.all });
  };

  const refreshLabel = refreshMs > 0 ? `${refreshMs / 1000}s` : 'OFF';

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

      <UsageSummaryCards data={summary} loading={summaryLoading} />
      <UsageTrendChart trend={summary?.trend ?? []} loading={summaryLoading} />

      <DataSourceBar />

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
          <RequestLogTable params={logParams} />
        </TabsContent>

        <TabsContent value="providers" className="mt-4">
          <ProviderStatsTable data={providerStats?.providers} loading={providerLoading} />
        </TabsContent>

        <TabsContent value="models" className="mt-4">
          <ModelStatsTable data={modelStats?.models} loading={modelLoading} />
        </TabsContent>

        <TabsContent value="pricing" className="mt-4">
          <ModelPricingPanel />
        </TabsContent>
      </Tabs>
    </div>
  );
}
