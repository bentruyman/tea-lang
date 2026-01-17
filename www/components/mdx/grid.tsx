import { ReactNode } from 'react'

interface GridProps {
  children: ReactNode
  className?: string
}

export function TwoColumnGrid({ children, className = '' }: GridProps) {
  return (
    <div className={`grid md:grid-cols-2 gap-6 ${className}`}>
      {children}
    </div>
  )
}

export function ThreeColumnGrid({ children, className = '' }: GridProps) {
  return (
    <div className={`grid md:grid-cols-3 gap-6 ${className}`}>
      {children}
    </div>
  )
}
