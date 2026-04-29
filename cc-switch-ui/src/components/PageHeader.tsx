import type { ReactNode } from "react"

interface PageHeaderProps {
  title: string
  description: string
  action?: ReactNode
}

export function PageHeader({ title, description, action }: PageHeaderProps) {
  return (
    <div className="mb-6 grid gap-4 sm:grid-cols-[minmax(0,1fr)_auto] sm:items-start">
      <div className="min-w-0">
        <h1 className="text-2xl font-semibold leading-8 tracking-normal text-foreground">{title}</h1>
        <p className="mt-1 max-w-2xl text-sm leading-5 text-muted-foreground">{description}</p>
      </div>
      {action && <div className="flex shrink-0 items-center justify-start gap-2 sm:justify-end">{action}</div>}
    </div>
  )
}
