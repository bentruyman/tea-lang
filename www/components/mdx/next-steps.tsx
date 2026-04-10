import type { ReactNode } from "react"

interface NextStepsProps {
  title?: string
  children: ReactNode
}

export function NextSteps({ title = "Next steps", children }: NextStepsProps) {
  return (
    <div className="space-y-4">
      <h2 className="font-display text-3xl font-semibold tracking-tight">{title}</h2>
      <div className="grid gap-4 md:grid-cols-2">{children}</div>
    </div>
  )
}
