import { ReactNode } from 'react'
import { CheckCircle2 } from 'lucide-react'
import { Card } from '@/components/ui/card'

interface RequirementItemProps {
  children: ReactNode
  optional?: boolean
}

export function RequirementItem({ children, optional }: RequirementItemProps) {
  return (
    <li className="flex items-start gap-2">
      <CheckCircle2 className={`h-5 w-5 mt-0.5 shrink-0 ${optional ? 'text-muted-foreground' : 'text-accent'}`} />
      <span className="text-sm">{children}</span>
    </li>
  )
}

interface RequirementsSectionProps {
  title: string
  children: ReactNode
}

export function RequirementsSection({ title, children }: RequirementsSectionProps) {
  return (
    <div>
      <h3 className="font-semibold text-lg mb-3 text-accent">{title}</h3>
      <ul className="space-y-2">
        {children}
      </ul>
    </div>
  )
}

interface RequirementsCardProps {
  children: ReactNode
}

export function RequirementsCard({ children }: RequirementsCardProps) {
  return (
    <Card className="p-6 bg-card border-border">
      <div className="grid md:grid-cols-2 gap-6">
        {children}
      </div>
    </Card>
  )
}
