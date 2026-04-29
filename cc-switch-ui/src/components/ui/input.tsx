import * as React from "react"
import { cn } from "@/lib/utils"

const Input = React.forwardRef<HTMLInputElement, React.InputHTMLAttributes<HTMLInputElement>>(
  ({ className, ...props }, ref) => (
    <input
      ref={ref}
      className={cn(
        "h-10 w-full rounded-xl border border-input bg-white/[0.04] px-3 text-sm leading-5 text-foreground shadow-inner shadow-black/10 outline-none transition placeholder:text-muted-foreground focus:border-primary/70 focus:ring-4 focus:ring-primary/15 disabled:cursor-not-allowed disabled:opacity-60",
        className
      )}
      {...props}
    />
  )
)
Input.displayName = "Input"

export { Input }
