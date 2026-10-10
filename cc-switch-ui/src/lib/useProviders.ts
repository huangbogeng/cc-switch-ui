import { useQuery, useQueryClient } from '@tanstack/react-query';
import { getCurrentProviderId, getProxyStatus, listProviders } from '@/api';

export const providerKeys = {
  list: ['providers', 'list'] as const,
  current: ['providers', 'current'] as const,
  route: ['providers', 'route'] as const,
};

export function useProviders() {
  return useQuery({
    queryKey: providerKeys.list,
    queryFn: ({ signal }) => listProviders({ signal }),
    staleTime: 60_000,
    refetchInterval: 60_000,
    refetchOnWindowFocus: true,
  });
}

export function useCurrentProvider() {
  return useQuery({
    queryKey: providerKeys.current,
    queryFn: ({ signal }) => getCurrentProviderId({ signal }),
    staleTime: 5_000,
    refetchInterval: 5_000,
    refetchOnWindowFocus: true,
  });
}

export function useRouteStatus() {
  return useQuery({
    queryKey: providerKeys.route,
    queryFn: ({ signal }) => getProxyStatus({ signal }),
    staleTime: 5_000,
    refetchInterval: 5_000,
    refetchOnWindowFocus: true,
  });
}

export function useRefreshProviders() {
  const client = useQueryClient();
  return () => Promise.all([
    client.invalidateQueries({ queryKey: providerKeys.list }, { throwOnError: true }),
    client.invalidateQueries({ queryKey: providerKeys.current }, { throwOnError: true }),
  ]);
}
