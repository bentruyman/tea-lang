import { ReactNode } from 'react'
import { Card } from '@/components/ui/card'

interface FeatureCardProps {
  title: string
  description: string
  children?: ReactNode
  className?: string
}

export function FeatureCard({ title, description, children, className = '' }: FeatureCardProps) {
  return (
    <Card className={`p-6 bg-card border-border panel-inset ${className}`}>
      <h3 className="font-semibold text-lg mb-3 text-accent">{title}</h3>
      <p className="text-sm text-muted-foreground leading-relaxed mb-4">
        {description}
      </p>
      {children && (
        <pre className="bg-muted p-3 rounded-md overflow-x-auto texture-grid-fine">
          <code className="font-mono text-xs">{children}</code>
        </pre>
      )}
    </Card>
  )
}
