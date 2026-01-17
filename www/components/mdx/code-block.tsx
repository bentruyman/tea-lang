import { ReactNode, isValidElement } from 'react'
import { CodeHighlighter } from './code-highlighter'

interface CodeBlockProps {
  children: ReactNode
  title?: string
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

export function CodeBlock({ children, title, language = 'tea', className = '' }: CodeBlockProps) {
  const code = extractText(children)

  return (
    <div className={`mb-4 ${className}`}>
      {title && (
        <div className="font-semibold text-accent mb-2">{title}</div>
      )}
      <CodeHighlighter code={code} language={language} />
    </div>
  )
}
