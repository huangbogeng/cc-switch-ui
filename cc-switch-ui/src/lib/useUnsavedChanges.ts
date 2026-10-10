import { useEffect, useId } from 'react';

const editors = new Map<string, { dirty: boolean; busy: boolean }>();

export function confirmDiscardChanges(): boolean {
  if ([...editors.values()].some((editor) => editor.busy)) return false;
  return ![...editors.values()].some((editor) => editor.dirty)
    || window.confirm('Discard unsaved changes?');
}

export function useUnsavedChanges(dirty: boolean, busy = false) {
  const id = useId();
  useEffect(() => {
    editors.set(id, { dirty, busy });
    const beforeUnload = (event: BeforeUnloadEvent) => {
      if (!dirty && !busy) return;
      event.preventDefault();
      event.returnValue = '';
    };
    window.addEventListener('beforeunload', beforeUnload);
    return () => {
      editors.delete(id);
      window.removeEventListener('beforeunload', beforeUnload);
    };
  }, [id, dirty, busy]);

  return () => !busy && (!dirty || window.confirm('Discard unsaved changes?'));
}
