import { ReactNode } from 'react'
import { CheckCircle2 } from 'lucide-react'
import { Card } from '@/components/ui/card'

interface PrerequisiteItemProps {
  title: string
  children?: ReactNode
}

export function PrerequisiteItem({ title, children }: PrerequisiteItemProps) {
  return (
    <li className="flex items-start gap-3">
      <CheckCircle2 className="h-5 w-5 text-accent mt-0.5 shrink-0" />
      <span>
        <strong className="text-foreground">{title}</strong>
        {children && <span className="text-muted-foreground"> - {children}</span>}
      </span>
    </li>
  )
}

interface PrerequisiteListProps {
  description?: string
  children: ReactNode
}

export function PrerequisiteList({ description, children }: PrerequisiteListProps) {
  return (
    <Card className="p-6 bg-card border-border">
      {description && (
        <p className="text-muted-foreground mb-4">{description}</p>
      )}
      <ul className="space-y-2">
        {children}
      </ul>
    </Card>
  )
}
