import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { RefreshCw, Database, FileText } from 'lucide-react';
import { useDataSourceBreakdown, useSyncSession } from '@/lib/useUsage';

const SOURCE_ICONS: Record<string, React.ReactNode> = {
  proxy: <Database className="h-3 w-3" />,
  session_log: <FileText className="h-3 w-3" />,
};

const SOURCE_LABELS: Record<string, string> = {
  proxy: 'Proxy',
  session_log: 'Session Log',
};

export default function DataSourceBar() {
  const { data: sources, isLoading } = useDataSourceBreakdown();
  const syncMutation = useSyncSession();

  if (isLoading || !sources || sources.length <= 1) {
    return null;
  }

  const handleSync = () => {
    syncMutation.mutate();
  };

  return (
    <div className="flex items-center justify-between rounded-lg border bg-muted/30 px-4 py-2 text-sm text-muted-foreground">
      <div className="flex items-center gap-3">
        <span className="text-xs font-medium">Data Sources:</span>
        {sources.map((source) => (
          <Badge
            key={source.dataSource}
            variant="secondary"
            className="gap-1 px-2 py-0.5 text-xs font-normal"
          >
            {SOURCE_ICONS[source.dataSource] || <Database className="h-3 w-3" />}
            {SOURCE_LABELS[source.dataSource] || source.dataSource}
            <span className="font-mono tabular-nums">{source.requestCount.toLocaleString()}</span>
          </Badge>
        ))}
      </div>
      <Button
        variant="outline"
        size="sm"
        className="h-7 gap-1.5 text-xs"
        onClick={handleSync}
        disabled={syncMutation.isPending}
      >
        <RefreshCw
          className={`h-3 w-3 ${syncMutation.isPending ? 'animate-spin' : ''}`}
        />
        {syncMutation.isPending ? 'Syncing...' : 'Sync Session Logs'}
      </Button>
    </div>
  );
}
