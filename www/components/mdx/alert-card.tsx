import { ReactNode } from 'react'
import { AlertCircle, Info } from 'lucide-react'

interface AlertCardProps {
  variant?: 'warning' | 'info'
  children: ReactNode
}

export function AlertCard({ variant = 'info', children }: AlertCardProps) {
  const Icon = variant === 'warning' ? AlertCircle : Info

  return (
    <div className="flex items-start gap-3 p-3 bg-muted/50 rounded-md">
      <Icon className="h-5 w-5 text-accent shrink-0 mt-0.5" />
      <p className="text-sm text-muted-foreground">{children}</p>
    </div>
  )
}
