import { ReactNode } from 'react'
import { Card } from '@/components/ui/card'

interface TypeRowProps {
  type: string
  description: string
  example: string
}

export function TypeRow({ type, description, example }: TypeRowProps) {
  return (
    <tr className="border-b border-border/50 last:border-0">
      <td className="py-2 pr-4 text-accent font-mono">{type}</td>
      <td className="py-2 pr-4 text-muted-foreground">{description}</td>
      <td className="py-2 font-mono"><code>{example}</code></td>
    </tr>
  )
}

interface TypeTableProps {
  children: ReactNode
}

export function TypeTable({ children }: TypeTableProps) {
  return (
    <Card className="p-6 bg-card border-border">
      <div className="overflow-x-auto">
        <table className="w-full text-sm">
          <thead>
            <tr className="border-b border-border">
              <th className="text-left py-2 pr-4 text-accent">Type</th>
              <th className="text-left py-2 pr-4">Description</th>
              <th className="text-left py-2">Example</th>
            </tr>
          </thead>
          <tbody>
            {children}
          </tbody>
        </table>
      </div>
    </Card>
  )
}
