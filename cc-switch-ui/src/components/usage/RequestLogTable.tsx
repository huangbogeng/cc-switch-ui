import { useMemo, useState } from 'react';
import { ArrowLeft, ArrowRight } from 'lucide-react';
import type { RequestLogDetail } from '@/api';
import { useRequestLogs } from '@/lib/useUsage';
import type { LogsQueryParams } from '@/api';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import RequestLogDetailPanel from '@/components/usage/RequestLogDetailPanel';

function fmtDate(ts: number): string {
  return new Date(ts * 1000).toLocaleString();
}

interface Props {
  params: LogsQueryParams;
  refreshMs?: number;
}

export default function RequestLogTable({ params, refreshMs = 30_000 }: Props) {
  const [page, setPage] = useState(0);
  const [selectedLogId, setSelectedLogId] = useState<number | null>(null);
  const pageSize = 20;

  const queryParams = useMemo(
    () => ({ ...params, page, page_size: pageSize }),
    [params, page],
  );
  const { data, isLoading, isError } = useRequestLogs(queryParams, refreshMs);

  const totalPages = data ? Math.ceil(data.total / data.page_size) : 0;

  return (
    <div className="space-y-3">
      {/* summary + pagination */}
      <div className="flex items-center justify-between">
        <div className="text-sm text-muted-foreground">
          {data ? `${data.total} log entries` : '—'}
          {data && totalPages > 0 && ` · Page ${data.page + 1} of ${totalPages}`}
        </div>
        <div className="flex items-center gap-1">
          <Button
            aria-label="Previous log page"
            variant="ghost"
            size="sm"
            disabled={page <= 0}
            onClick={() => setPage((p) => Math.max(0, p - 1))}
          >
            <ArrowLeft className="h-4 w-4" />
          </Button>
          <span className="text-xs text-muted-foreground px-2 tabular-nums">
            {page + 1} / {Math.max(totalPages, 1)}
          </span>
          <Button
            aria-label="Next log page"
            variant="ghost"
            size="sm"
            disabled={totalPages <= 1 || page >= totalPages - 1}
            onClick={() => setPage((p) => p + 1)}
          >
            <ArrowRight className="h-4 w-4" />
          </Button>
        </div>
      </div>

      {/* table */}
      <div className="overflow-x-auto rounded-xl border border-white/5">
        <table className="w-full text-sm">
          <thead>
            <tr className="border-b border-white/5 bg-black/20 text-left text-xs font-semibold text-muted-foreground uppercase tracking-wider">
              <th className="px-3 py-2">Time</th>
              <th className="px-3 py-2">Provider</th>
              <th className="px-3 py-2">Path</th>
              <th className="px-3 py-2">Model</th>
              <th className="px-3 py-2">Status</th>
              <th className="px-3 py-2 text-right">Tokens (in/out)</th>
            </tr>
          </thead>
          <tbody>
            {isLoading ? (
              <tr>
                <td colSpan={6} className="px-3 py-12 text-center text-muted-foreground">
                  Loading...
                </td>
              </tr>
            ) : isError ? (
              <tr>
                <td colSpan={6} className="px-3 py-12 text-center text-red-400">
                  Failed to load request logs
                </td>
              </tr>
            ) : data && data.data.length === 0 ? (
              <tr>
                <td colSpan={6} className="px-3 py-12 text-center text-muted-foreground">
                  No request logs found
                </td>
              </tr>
            ) : (
              data?.data.map((log) => (
                <RequestLogRow key={log.id} log={log} onClick={() => setSelectedLogId(log.id)} />
              ))
            )}
          </tbody>
        </table>
      </div>

      {selectedLogId !== null && (
        <RequestLogDetailPanel
          logId={selectedLogId}
          onClose={() => setSelectedLogId(null)}
        />
      )}
    </div>
  );
}

function RequestLogRow({ log, onClick }: { log: RequestLogDetail; onClick: () => void }) {
  const statusBadge = () => {
    if (!log.status_code) return <Badge variant="outline">—</Badge>;
    if (log.status_code >= 200 && log.status_code < 300)
      return <Badge variant="success">{log.status_code}</Badge>;
    if (log.status_code >= 400 && log.status_code < 500)
      return <Badge variant="warning">{log.status_code}</Badge>;
    return <Badge variant="danger">{log.status_code}</Badge>;
  };

  return (
    <tr
      className="cursor-pointer border-b border-white/5 transition-colors hover:bg-white/[0.05] focus-within:bg-white/[0.05]"
      onClick={onClick}
    >
      <td className="px-3 py-2 text-xs text-muted-foreground whitespace-nowrap">{fmtDate(log.created_at)}</td>
      <td className="px-3 py-2 font-medium text-foreground truncate max-w-[120px]" title={log.provider_id}>
        {log.provider_id}
      </td>
      <td className="px-3 py-2 text-muted-foreground truncate max-w-[160px] font-mono text-xs" title={log.request_path}>
        {log.request_path}
      </td>
      <td className="px-3 py-2 text-muted-foreground truncate max-w-[140px] text-xs" title={log.request_model ?? ''}>
        {log.request_model || '—'}
      </td>
      <td className="px-3 py-2">{statusBadge()}</td>
      <td className="px-3 py-2 text-right text-xs text-muted-foreground tabular-nums whitespace-nowrap">
        {log.input_tokens != null ? `${log.input_tokens.toLocaleString()} / ${log.output_tokens?.toLocaleString() ?? '?'}` : '—'}
        <button type="button" className="sr-only" onClick={onClick}>Open request detail</button>
      </td>
    </tr>
  );
}
