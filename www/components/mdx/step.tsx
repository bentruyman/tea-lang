import { ReactNode } from 'react'
import { Card } from '@/components/ui/card'

interface StepProps {
  number: number
  title: string
  children: ReactNode
}

export function Step({ number, title, children }: StepProps) {
  return (
    <Card className="p-6 bg-card border-border">
      <div className="flex items-center gap-3 mb-4">
        <div className="h-8 w-8 rounded-full bg-accent text-accent-foreground flex items-center justify-center font-bold">
          {number}
        </div>
        <h3 className="text-xl font-semibold">{title}</h3>
      </div>
      <div className="space-y-4">
        {children}
      </div>
    </Card>
  )
}
