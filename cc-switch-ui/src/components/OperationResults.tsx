import { useState } from 'react';
import { ToastNotice } from '@/components/ToastNotice';
import type { OperationReport } from '@/api';

export function OperationResults({ report, action }: { report: OperationReport; action: 'Import' | 'Sync' | 'Restore' }) {
  const [dismissedReport, setDismissedReport] = useState<OperationReport | null>(null);
  if (dismissedReport === report) return null;

  const failed = !report.success && report.failed === report.items.length;
  const outcome = report.success ? 'completed' : failed ? 'failed' : 'partially completed';
  const message = `${action} ${outcome}: ${action !== 'Sync' ? `${report.imported} ${action === 'Restore' ? 'restored' : 'imported'}, ${report.updated} updated${report.applied ? `, ${report.applied} applied` : ''}` : `${report.applied} applied`}, ${report.skipped} skipped, ${report.failed} failed.`;

  return <ToastNotice tone={report.success ? 'success' : failed ? 'error' : 'warning'} message={message} onDismiss={() => setDismissedReport(report)}>
    <details open={!report.success} className="mt-2 text-xs text-muted-foreground">
      <summary className="cursor-pointer">Item results</summary>
      <div className="mt-2 max-h-48 space-y-2 overflow-y-auto">
        {action === 'Import' && <p>Import preserves existing records. When Import and enable is checked, only new records are applied. If applying fails, records are retained; use Sync to retry.</p>}
        {report.failed > 0 && <p>Completed changes are kept. Fix the listed errors, then retry {action === 'Import' ? 'Import for unsaved items, or Sync for records already saved' : action}. Already managed imports are skipped; unchanged Skills files are not replaced.</p>}
        <ul className="space-y-2">
          {report.items.map((item, index) => <li key={`${item.id}:${index}`} className="break-words"><span className="font-medium">{item.id}</span>: {item.outcome}{item.error ? ` — ${item.error}` : ''}</li>)}
        </ul>
      </div>
    </details>
  </ToastNotice>;
}
