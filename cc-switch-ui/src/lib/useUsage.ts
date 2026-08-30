import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import {
  getProxyUsageSummary,
  getProviderStats,
  getModelStats,
  getRequestLogs,
  getRequestLogDetail,
  getModelPricing,
  upsertModelPricing,
  deleteModelPricing,
  getCopilotUsage,
  syncSessionUsage,
  getDataSourceBreakdown,
  type LogsQueryParams,
  type ModelPricingItem,
} from '@/api';

const DEFAULT_REFETCH_MS = 30_000;

export const usageKeys = {
  all: ['usage'] as const,
  summary: (startDate?: number, endDate?: number) =>
    [...usageKeys.all, 'summary', startDate ?? 0, endDate ?? 0] as const,
  providerStats: (startDate?: number, endDate?: number) =>
    [...usageKeys.all, 'provider-stats', startDate ?? 0, endDate ?? 0] as const,
  modelStats: (startDate?: number, endDate?: number) =>
    [...usageKeys.all, 'model-stats', startDate ?? 0, endDate ?? 0] as const,
  logs: (params: LogsQueryParams) =>
    [...usageKeys.all, 'logs', JSON.stringify(params)] as const,
  logDetail: (id: number) =>
    [...usageKeys.all, 'detail', id] as const,
  copilotUsage: () => [...usageKeys.all, 'copilot-usage'] as const,
  sources: (startDate?: number, endDate?: number) =>
    [...usageKeys.all, 'sources', startDate ?? 0, endDate ?? 0] as const,
};

export function useUsageSummary(
  startDate?: number,
  endDate?: number,
  refetchInterval = DEFAULT_REFETCH_MS,
) {
  return useQuery({
    queryKey: usageKeys.summary(startDate, endDate),
    queryFn: ({ signal }) => getProxyUsageSummary(startDate, endDate, signal),
    refetchInterval,
  });
}

export function useProviderStats(
  startDate?: number,
  endDate?: number,
  refetchInterval = DEFAULT_REFETCH_MS,
) {
  return useQuery({
    queryKey: usageKeys.providerStats(startDate, endDate),
    queryFn: ({ signal }) => getProviderStats(startDate, endDate, signal),
    refetchInterval,
  });
}

export function useModelStats(
  startDate?: number,
  endDate?: number,
  refetchInterval = DEFAULT_REFETCH_MS,
) {
  return useQuery({
    queryKey: usageKeys.modelStats(startDate, endDate),
    queryFn: ({ signal }) => getModelStats(startDate, endDate, signal),
    refetchInterval,
  });
}

export function useRequestLogs(params: LogsQueryParams, refetchInterval = DEFAULT_REFETCH_MS) {
  return useQuery({
    queryKey: usageKeys.logs(params),
    queryFn: ({ signal }) => getRequestLogs(params, signal),
    refetchInterval,
  });
}

export function useCopilotUsage() {
  return useQuery({
    queryKey: usageKeys.copilotUsage(),
    queryFn: ({ signal }) => getCopilotUsage(signal),
    retry: false,
    refetchInterval: 30_000,
  });
}

export function useRequestLogDetail(id: number | null) {
  return useQuery({
    queryKey: usageKeys.logDetail(id ?? 0),
    queryFn: ({ signal }) => getRequestLogDetail(id!, signal),
    enabled: id !== null,
  });
}

export function useModelPricing() {
  return useQuery({
    queryKey: [...usageKeys.all, 'pricing'] as const,
    queryFn: ({ signal }) => getModelPricing(signal),
  });
}

export function useUpsertModelPricing() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (pricing: ModelPricingItem) => upsertModelPricing(pricing),
    onSuccess: () => qc.invalidateQueries({ queryKey: [...usageKeys.all, 'pricing'] }),
  });
}

export function useDeleteModelPricing() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (modelId: string) => deleteModelPricing(modelId),
    onSuccess: () => qc.invalidateQueries({ queryKey: [...usageKeys.all, 'pricing'] }),
  });
}

export function useSyncSession() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: syncSessionUsage,
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: usageKeys.all });
    },
  });
}

export function useDataSourceBreakdown(
  startDate?: number,
  endDate?: number,
  refetchInterval = DEFAULT_REFETCH_MS,
) {
  return useQuery({
    queryKey: usageKeys.sources(startDate, endDate),
    queryFn: ({ signal }) => getDataSourceBreakdown(startDate, endDate, signal),
    refetchInterval,
  });
}
