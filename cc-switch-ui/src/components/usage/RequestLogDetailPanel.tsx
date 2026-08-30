import { X, Copy, Check } from 'lucide-react';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { useRequestLogDetail } from '@/lib/useUsage';
import { useId, useState } from 'react';
import { useDialog } from '@/lib/useDialog';

interface Props {
  logId: number | null;
  onClose: () => void;
}

function CopyButton({ text }: { text: string }) {
  const [copied, setCopied] = useState(false);
  const handleCopy = async () => {
    try {
      await navigator.clipboard.writeText(text);
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    } catch {
      setCopied(false);
    }
  };
  return (
    <Button aria-label="Copy value" variant="ghost" size="icon" className="h-6 w-6" onClick={() => void handleCopy()}>
      {copied ? <Check className="h-3 w-3 text-green-500" /> : <Copy className="h-3 w-3" />}
    </Button>
  );
}

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div className="flex items-start gap-3 py-1.5">
      <span className="text-xs text-muted-foreground w-28 shrink-0 pt-0.5">{label}</span>
      <div className="flex items-center gap-1.5 min-w-0 flex-1">
        {children}
      </div>
    </div>
  );
}

export default function RequestLogDetailPanel({ logId, onClose }: Props) {
  const { data, isLoading } = useRequestLogDetail(logId);
  const titleId = useId();
  const dialogRef = useDialog(true, onClose);

  return (
    <div className="fixed inset-0 z-50 flex justify-end">
      <div className="absolute inset-0 bg-black/30" onClick={onClose} />
      <div ref={dialogRef} tabIndex={-1} role="dialog" aria-modal="true" aria-labelledby={titleId} className="relative w-full max-w-lg bg-background border-l border-border shadow-xl animate-in slide-in-from-right">
        <CardHeader className="flex flex-row items-center justify-between border-b border-border px-4 py-3">
          <CardTitle id={titleId} className="text-sm font-medium">Request Detail</CardTitle>
          <Button aria-label="Close request detail" variant="ghost" size="icon" className="h-7 w-7" onClick={onClose}>
            <X className="h-4 w-4" />
          </Button>
        </CardHeader>
        <CardContent className="p-4 overflow-y-auto max-h-[calc(100vh-60px)]">
          {isLoading ? (
            <div className="text-sm text-muted-foreground py-8 text-center">Loading...</div>
          ) : !data ? (
            <div className="text-sm text-muted-foreground py-8 text-center">No details found</div>
          ) : (
            <div className="space-y-3">
              <Field label="ID">
                <span className="text-xs font-mono">{data.id}</span>
                <CopyButton text={String(data.id)} />
              </Field>

              <Field label="App Type">
                <Badge variant="outline" className="text-xs">{data.app_type}</Badge>
              </Field>

              <Field label="Provider">
                <span className="text-sm font-medium">{data.provider_id}</span>
              </Field>

              <Field label="Request Path">
                <span className="text-xs font-mono break-all">{data.request_path}</span>
                <CopyButton text={data.request_path} />
              </Field>

              <Field label="Model">
                <span className="text-xs">{data.request_model || '—'}</span>
              </Field>

              <Field label="Status Code">
                {data.status_code ? (
                  <Badge
                    variant={
                      data.status_code >= 200 && data.status_code < 300
                        ? 'success'
                        : data.status_code >= 400 && data.status_code < 500
                          ? 'warning'
                          : 'danger'
                    }
                  >
                    {data.status_code}
                  </Badge>
                ) : (
                  <span className="text-xs text-muted-foreground">—</span>
                )}
              </Field>

              <Field label="Success">
                <Badge variant={data.success ? 'success' : 'danger'}>
                  {data.success ? 'Yes' : 'No'}
                </Badge>
              </Field>

              {data.error_message && (
                <Field label="Error">
                  <span className="text-xs text-destructive break-all">{data.error_message}</span>
                  <CopyButton text={data.error_message} />
                </Field>
              )}

              <Field label="Input Tokens">
                <span className="text-sm tabular-nums">{data.input_tokens?.toLocaleString() ?? '—'}</span>
              </Field>

              <Field label="Output Tokens">
                <span className="text-sm tabular-nums">{data.output_tokens?.toLocaleString() ?? '—'}</span>
              </Field>

              <Field label="Timestamp">
                <span className="text-xs text-muted-foreground">
                  {new Date(data.created_at * 1000).toLocaleString()}
                </span>
              </Field>
            </div>
          )}
        </CardContent>
      </div>
    </div>
  );
}
