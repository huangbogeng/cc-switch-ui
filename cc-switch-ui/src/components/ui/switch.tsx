import type { ButtonHTMLAttributes } from 'react';
import { cn } from '@/lib/utils';

type Props = Omit<ButtonHTMLAttributes<HTMLButtonElement>, 'onChange'> & {
  checked: boolean;
  mixed?: boolean;
  onCheckedChange: (checked: boolean) => void;
};

export function Switch({ checked, mixed = false, onCheckedChange, className, ...props }: Props) {
  return <button {...props} type="button" role="switch" aria-checked={checked}
    onClick={() => onCheckedChange(!checked)}
    className={cn('inline-flex h-6 w-10 shrink-0 items-center rounded-full p-0.5 transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background disabled:cursor-not-allowed disabled:opacity-50 motion-reduce:transition-none', checked || mixed ? 'bg-primary' : 'bg-muted-foreground/30', className)}>
    <span className={cn('h-5 w-5 rounded-full bg-white shadow-sm transition-transform motion-reduce:transition-none', mixed ? 'translate-x-2' : checked ? 'translate-x-4' : 'translate-x-0')} />
  </button>;
}
