import { ReactNode } from 'react'
import { Card } from '@/components/ui/card'

interface IntrinsicRowProps {
  intrinsic: string
  description: string
}

export function IntrinsicRow({ intrinsic, description }: IntrinsicRowProps) {
  return (
    <tr className="border-b border-border/50 last:border-0">
      <td className="py-2 pr-4 text-accent font-mono">{intrinsic}</td>
      <td className="py-2 text-muted-foreground">{description}</td>
    </tr>
  )
}

interface IntrinsicTableProps {
  children: ReactNode
}

export function IntrinsicTable({ children }: IntrinsicTableProps) {
  return (
    <Card className="p-6 bg-card border-border">
      <h3 className="font-semibold text-lg mb-3 text-accent">Common Intrinsics</h3>
      <div className="overflow-x-auto">
        <table className="w-full text-sm">
          <thead>
            <tr className="border-b border-border">
              <th className="text-left py-2 pr-4 text-accent">Intrinsic</th>
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
