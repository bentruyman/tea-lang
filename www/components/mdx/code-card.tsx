import { ReactNode, isValidElement, Children } from 'react'
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
  if (children === null || children === undefined) return ''
  if (typeof children === 'string') return children
  if (typeof children === 'number') return String(children)
  if (typeof children === 'boolean') return ''
  if (Array.isArray(children)) return children.map(extractText).join('')
  if (isValidElement(children)) {
    const props = children.props as { children?: ReactNode }
    return extractText(props.children)
  }
  // Handle iterator/iterable children
  if (typeof children === 'object' && Symbol.iterator in children) {
    return Array.from(children as Iterable<ReactNode>).map(extractText).join('')
  }
  return String(children)
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
