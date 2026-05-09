import * as React from 'react';
import { ChevronDown } from 'lucide-react';
import { cn } from '@/lib/utils';

export type SelectProps = React.SelectHTMLAttributes<HTMLSelectElement>;

const Select = React.forwardRef<HTMLSelectElement, SelectProps>(
  ({ className, children, ...props }, ref) => {
    return (
      <div className="relative">
        <select
          ref={ref}
          className={cn(
            'flex h-10 w-full appearance-none rounded-xl border border-white/10 bg-black/20 px-3 pr-9 text-sm text-foreground shadow-inner transition outline-none',
            'focus:border-primary/50 focus:ring-2 focus:ring-primary/20',
            'disabled:cursor-not-allowed disabled:opacity-60',
            className,
          )}
          {...props}
        >
          {children}
        </select>
        <ChevronDown className="pointer-events-none absolute right-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
      </div>
    );
  },
);

Select.displayName = 'Select';

export { Select };
