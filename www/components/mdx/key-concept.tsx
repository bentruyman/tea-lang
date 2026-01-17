import { ReactNode } from 'react'
import { CheckCircle2 } from 'lucide-react'
import { Card } from '@/components/ui/card'

interface KeyConceptProps {
  title: string
  children: ReactNode
}

export function KeyConcept({ title, children }: KeyConceptProps) {
  return (
    <li className="flex items-start gap-3">
      <CheckCircle2 className="h-5 w-5 text-accent mt-0.5 shrink-0" />
      <div>
        <strong className="text-foreground">{title}:</strong>
        <span className="text-muted-foreground"> {children}</span>
      </div>
    </li>
  )
}

interface KeyConceptsCardProps {
  title?: string
  children: ReactNode
}

export function KeyConceptsCard({ title = "Key Concepts Demonstrated", children }: KeyConceptsCardProps) {
  return (
    <Card className="p-6 bg-card border-border">
      <h3 className="text-lg font-semibold mb-3">{title}</h3>
      <ul className="space-y-3">
        {children}
      </ul>
    </Card>
  )
}
