import { useState } from 'react';
import { ToastNotice } from '@/components/ToastNotice';
import type { OperationReport } from '@/api';

export function OperationResults({ report, action }: { report: OperationReport; action: 'Import' | 'Sync' }) {
  const [dismissedReport, setDismissedReport] = useState<OperationReport | null>(null);
  if (dismissedReport === report) return null;

  const failed = !report.success && report.failed === report.items.length;
  const outcome = report.success ? 'completed' : failed ? 'failed' : 'partially completed';
  const message = `${action} ${outcome}: ${action === 'Import' ? `${report.imported} imported, ${report.updated} updated` : `${report.applied} applied`}, ${report.skipped} skipped, ${report.failed} failed.`;

  return <ToastNotice tone={report.success ? 'success' : failed ? 'error' : 'warning'} message={message} onDismiss={() => setDismissedReport(report)}>
    <details open={!report.success} className="mt-2 text-xs text-muted-foreground">
      <summary className="cursor-pointer">Item results</summary>
      <div className="mt-2 max-h-48 space-y-2 overflow-y-auto">
        {action === 'Import' && <p>Import saves managed records and preserves existing records. Use Sync to apply the reviewed configuration.</p>}
        {report.failed > 0 && <p>Completed changes are kept. Fix the listed errors, then retry {action}. Already managed imports are skipped; unchanged Skills files are not replaced.</p>}
        <ul className="space-y-2">
          {report.items.map((item, index) => <li key={`${item.id}:${index}`} className="break-words"><span className="font-medium">{item.id}</span>: {item.outcome}{item.error ? ` — ${item.error}` : ''}</li>)}
        </ul>
      </div>
    </details>
  </ToastNotice>;
}
