import { useCallback, useEffect, useId, useRef, useState } from 'react';
import { createPortal } from 'react-dom';
import { Button } from '@/components/ui/button';
import { useDialog } from './useDialog';
import { useUnsavedChanges } from './useUnsavedChanges';

type Options = { title?: string; confirmLabel?: string; destructive?: boolean };
type Request = Options & { description: string };

export function useConfirm() {
  const [request, setRequest] = useState<Request | null>(null);
  const resolveRef = useRef<((confirmed: boolean) => void) | null>(null);
  const titleId = useId();
  const descriptionId = useId();
  const finish = useCallback((confirmed: boolean) => {
    resolveRef.current?.(confirmed);
    resolveRef.current = null;
    setRequest(null);
  }, []);
  const dialogRef = useDialog(request !== null, () => finish(false));
  useUnsavedChanges(false, request !== null);
  useEffect(() => () => { resolveRef.current?.(false); resolveRef.current = null; }, []);
  const confirmAction = useCallback((description: string, options: Options = {}) => {
    resolveRef.current?.(false);
    setRequest({ description, ...options });
    return new Promise<boolean>((resolve) => { resolveRef.current = resolve; });
  }, []);

  const confirmation = request && createPortal(
    <div className="dialog-overlay fixed inset-0 z-[80] flex items-center justify-center bg-black/65 px-4 py-6"
      onClick={(event) => { if (event.target === event.currentTarget) finish(false); }}>
      <div ref={dialogRef} tabIndex={-1} role="alertdialog" aria-modal="true" aria-labelledby={titleId} aria-describedby={descriptionId}
        className="max-h-[90dvh] w-full max-w-lg overflow-y-auto rounded-2xl border border-white/10 bg-popover p-6 shadow-2xl">
        <h2 id={titleId} className="text-lg font-semibold">{request.title ?? 'Review action'}</h2>
        <p id={descriptionId} className="mt-3 whitespace-pre-line text-sm leading-6 text-muted-foreground">{request.description}</p>
        <div className="mt-6 flex justify-end gap-2">
          <Button variant="outline" onClick={() => finish(false)}>Cancel</Button>
          <Button variant={request.destructive ? 'destructive' : 'default'} onClick={() => finish(true)}>{request.confirmLabel ?? 'Continue'}</Button>
        </div>
      </div>
    </div>, document.body,
  );
  return { confirmAction, confirmation };
}
