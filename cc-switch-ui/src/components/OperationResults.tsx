import type { OperationReport } from '@/api';

export function OperationResults({ report, action }: { report: OperationReport; action: 'Import' | 'Sync' }) {
  return (
    <div className="space-y-3 rounded-lg border p-4 text-sm" role="status">
      <p className="font-medium">{action} {report.success ? 'completed' : report.failed === report.items.length ? 'failed' : 'partially completed'}: {action === 'Import' ? `${report.imported} imported, ${report.updated} updated` : `${report.applied} applied`}, {report.skipped} skipped, {report.failed} failed.</p>
      {action === 'Import' && <p className="text-muted-foreground">Import saves managed records and preserves existing records. Use Sync to apply the reviewed configuration.</p>}
      {report.failed > 0 && <p className="text-muted-foreground">Completed changes are kept. Fix the listed errors, then retry {action}. Already managed imports are skipped; unchanged Skills files are not replaced.</p>}
      <details open={report.failed > 0}>
        <summary className="cursor-pointer">Item results</summary>
        <ul className="mt-2 space-y-2">
          {report.items.map((item, index) => <li key={`${item.id}:${index}`} className="break-words"><span className="font-medium">{item.id}</span>: {item.outcome}{item.error ? ` — ${item.error}` : ''}</li>)}
        </ul>
      </details>
    </div>
  );
}
