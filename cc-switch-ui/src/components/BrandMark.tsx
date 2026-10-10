import { cn } from '@/lib/utils';

export function BrandMark({ className }: { className?: string }) {
  return <img src={`${import.meta.env.BASE_URL}brand-mark.svg`} alt="" aria-hidden="true"
    width={48} height={48} className={cn('h-10 w-10 shrink-0', className)} />;
}
