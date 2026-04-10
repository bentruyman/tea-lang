import { ReactNode, isValidElement, Children } from 'react'

import { cn } from '@/lib/utils'

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
    <section className={cn("space-y-3", className)}>
      {title && (
        <div>
          <h2 className="font-display text-[1.9rem] font-semibold tracking-tight text-foreground">{title}</h2>
        </div>
      )}
      {description && (
        <p className="max-w-2xl text-sm leading-7 text-muted-foreground">{description}</p>
      )}
      <CodeHighlighter code={code} language={language} />
    </section>
  )
}
