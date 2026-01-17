import { ReactNode } from 'react'
import Link from 'next/link'
import { Card } from '@/components/ui/card'
import { Button } from '@/components/ui/button'

interface HelpLinkProps {
  href: string
  children: ReactNode
  external?: boolean
}

export function HelpLink({ href, children, external }: HelpLinkProps) {
  return (
    <Button variant="outline" size="sm" asChild>
      <Link href={href} {...(external ? { target: "_blank", rel: "noreferrer" } : {})}>
        {children}
      </Link>
    </Button>
  )
}

interface HelpCardProps {
  title?: string
  description?: string
  children: ReactNode
}

export function HelpCard({ title = "Need Help?", description, children }: HelpCardProps) {
  return (
    <Card className="p-6 bg-muted/30 border-border">
      <h3 className="text-lg font-semibold mb-3">{title}</h3>
      {description && (
        <p className="text-muted-foreground mb-4">{description}</p>
      )}
      <div className="flex flex-wrap gap-3">
        {children}
      </div>
    </Card>
  )
}
