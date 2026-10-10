import { useEffect, useRef, useState, type ReactNode } from 'react';
import { createPortal } from 'react-dom';
import { AlertCircle, CheckCircle2, X } from 'lucide-react';
import { Button } from '@/components/ui/button';

export function NotificationViewport() {
  return <div id="cc-switch-notifications" aria-label="Notifications" role="region"
    className="pointer-events-none fixed right-4 top-4 z-[100] flex max-h-[calc(100dvh-2rem)] w-[calc(100%-2rem)] max-w-sm flex-col gap-2 overflow-y-auto sm:right-6 sm:top-6" />;
}

interface Props {
  message: string;
  tone?: 'success' | 'error' | 'warning';
  onDismiss: () => void;
  duration?: number;
  children?: ReactNode;
}

export function ToastNotice({ message, tone = 'success', onDismiss, duration = tone === 'success' ? 5000 : 8000, children }: Props) {
  const [hovered, setHovered] = useState(false);
  const [focused, setFocused] = useState(false);
  const [dismissedMessage, setDismissedMessage] = useState<string | null>(null);
  const paused = hovered || focused;
  const dismiss = useRef(onDismiss);
  useEffect(() => { dismiss.current = onDismiss; }, [onDismiss]);
  useEffect(() => {
    if (paused || !message || dismissedMessage === message) return;
    const timer = window.setTimeout(() => { setDismissedMessage(message); dismiss.current(); }, duration);
    return () => window.clearTimeout(timer);
  }, [message, duration, paused, dismissedMessage]);
  if (dismissedMessage === message) return null;
  const close = () => { setDismissedMessage(message); dismiss.current(); };
  const viewport = document.getElementById('cc-switch-notifications');
  const color = tone === 'success' ? 'text-emerald-400' : tone === 'error' ? 'text-destructive' : 'text-amber-300';
  const Icon = tone === 'success' ? CheckCircle2 : AlertCircle;
  return createPortal(
    <div role={tone === 'error' ? 'alert' : 'status'}
      onMouseEnter={() => setHovered(true)} onMouseLeave={() => setHovered(false)}
      onFocus={() => setFocused(true)} onBlur={(event) => { if (!event.currentTarget.contains(event.relatedTarget)) setFocused(false); }}
      className={`pointer-events-auto flex items-start gap-3 rounded-xl border border-white/10 bg-card px-4 py-3 text-sm shadow-lg notification-enter ${viewport ? '' : 'fixed right-4 top-4 z-[100] max-w-sm'}`}>
      <Icon aria-hidden="true" className={`mt-0.5 h-4 w-4 shrink-0 ${color}`} />
      <div className="min-w-0 flex-1"><span className="whitespace-pre-line break-words leading-5">{message}</span>{children}</div>
      <Button variant="ghost" size="icon" className="-mr-1 -mt-1 h-7 w-7 shrink-0 rounded-lg" aria-label="Dismiss notification" onClick={close}><X className="h-3.5 w-3.5" /></Button>
    </div>, viewport ?? document.body,
  );
}
