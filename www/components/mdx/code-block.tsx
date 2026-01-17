import { ReactNode } from 'react'

interface CodeBlockProps {
  children: ReactNode
  title?: string
  className?: string
}

export function CodeBlock({ children, title, className = '' }: CodeBlockProps) {
  return (
    <div className={`mb-4 ${className}`}>
      {title && (
        <div className="font-semibold text-accent mb-2">{title}</div>
      )}
      <pre className="bg-muted p-4 rounded-md overflow-x-auto">
        <code className="font-mono text-sm">{children}</code>
      </pre>
    </div>
  )
}
