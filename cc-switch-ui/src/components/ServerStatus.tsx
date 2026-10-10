import { useRouteStatus } from '@/lib/useProviders';
import { cn } from '@/lib/utils';

export function ServerStatus() {
  const query = useRouteStatus();
  const label = query.isError ? 'Server unavailable' : query.isPending ? 'Connecting…' : 'Local Server';
  return <div role="status" className="mt-1.5 flex items-center gap-2" title={query.isError ? 'Unable to refresh server status' : undefined}>
    <div aria-hidden="true" className={cn('h-2 w-2 rounded-full',
      query.isError ? 'bg-destructive' : query.isPending ? 'bg-muted-foreground' : 'bg-emerald-500')} />
    <span className="truncate text-sm font-medium text-foreground">{label}</span>
  </div>;
}
