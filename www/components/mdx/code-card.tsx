import { ReactNode } from 'react'
import { Card } from '@/components/ui/card'

interface CodeCardProps {
  title?: string
  description?: string
  children: ReactNode
  className?: string
}

export function CodeCard({ title, description, children, className = '' }: CodeCardProps) {
  return (
    <Card className={`p-6 bg-card border-border ${className}`}>
      {title && (
        <h3 className="font-semibold text-lg mb-3 text-accent">{title}</h3>
      )}
      {description && (
        <p className="text-sm text-muted-foreground mb-4">{description}</p>
      )}
      <pre className="bg-muted p-4 rounded-md overflow-x-auto">
        <code className="font-mono text-sm">{children}</code>
      </pre>
    </Card>
  )
}
