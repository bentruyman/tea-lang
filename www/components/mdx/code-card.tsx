import { ReactNode } from 'react'
import { Card } from '@/components/ui/card'
import { CodeHighlighter } from './code-highlighter'

interface CodeCardProps {
  title?: string
  description?: string
  children: ReactNode
  language?: string
  className?: string
}

function extractText(children: ReactNode): string {
  if (typeof children === 'string') return children
  if (typeof children === 'number') return String(children)
  if (Array.isArray(children)) return children.map(extractText).join('')
  return ''
}

export function CodeCard({ title, description, children, language = 'tea', className = '' }: CodeCardProps) {
  const code = extractText(children)

  return (
    <Card className={`p-6 bg-card border-border ${className}`}>
      {title && (
        <h3 className="font-semibold text-lg mb-3 text-accent">{title}</h3>
      )}
      {description && (
        <p className="text-sm text-muted-foreground mb-4">{description}</p>
      )}
      <CodeHighlighter code={code} language={language} />
    </Card>
  )
}
