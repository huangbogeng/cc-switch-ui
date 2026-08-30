import { Badge } from '@/components/ui/badge';
import { Database, FileText } from 'lucide-react';
import type { DataSourceSummary } from '@/api';

const SOURCE_ICONS: Record<string, React.ReactNode> = {
  proxy: <Database className="h-3 w-3" />,
  session_log: <FileText className="h-3 w-3" />,
};

const SOURCE_LABELS: Record<string, string> = {
  proxy: 'Proxy',
  session_log: 'Session Log',
};

export default function DataSourceBar({ sources }: { sources?: DataSourceSummary[] }) {
  if (!sources?.length) return null;

  return (
    <div className="flex flex-wrap items-center gap-3 rounded-lg border bg-muted/30 px-4 py-2 text-sm text-muted-foreground">
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
  );
}
