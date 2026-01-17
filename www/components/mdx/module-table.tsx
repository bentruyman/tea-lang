import { ReactNode } from 'react'
import { Card } from '@/components/ui/card'

interface ModuleRowProps {
  module: string
  description: string
}

export function ModuleRow({ module, description }: ModuleRowProps) {
  return (
    <tr className="border-b border-border/50 last:border-0">
      <td className="py-2 pr-4 font-mono">{module}</td>
      <td className="py-2 text-muted-foreground">{description}</td>
    </tr>
  )
}

interface ModuleTableProps {
  children: ReactNode
}

export function ModuleTable({ children }: ModuleTableProps) {
  return (
    <Card className="p-6 bg-card border-border">
      <div className="overflow-x-auto">
        <table className="w-full text-sm">
          <thead>
            <tr className="border-b border-border">
              <th className="text-left py-2 pr-4 text-accent">Module</th>
              <th className="text-left py-2">Description</th>
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
