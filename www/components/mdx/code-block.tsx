import { ReactNode } from 'react'
import { CodeHighlighter } from './code-highlighter'

interface CodeBlockProps {
  children: ReactNode
  title?: string
  language?: string
  className?: string
}

function extractText(children: ReactNode): string {
  if (typeof children === 'string') return children
  if (typeof children === 'number') return String(children)
  if (Array.isArray(children)) return children.map(extractText).join('')
  return ''
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
