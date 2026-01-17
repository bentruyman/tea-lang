import { ReactNode } from 'react'
import Link from 'next/link'
import { Card } from '@/components/ui/card'
import { Button } from '@/components/ui/button'

interface InstallStepProps {
  title: string
  children: ReactNode
}

export function InstallStep({ title, children }: InstallStepProps) {
  return (
    <div>
      <h3 className="font-semibold text-accent mb-2">{title}</h3>
      <pre className="bg-muted p-4 rounded-md overflow-x-auto texture-grid-fine">
        <code className="font-mono text-sm text-foreground">{children}</code>
      </pre>
    </div>
  )
}

interface InstallCardProps {
  children: ReactNode
  linkHref?: string
  linkText?: string
}

export function InstallCard({ children, linkHref, linkText }: InstallCardProps) {
  return (
    <Card className="p-6 bg-card border-border corner-brackets panel-inset">
      <div className="space-y-4">
        {children}
      </div>
      {linkHref && linkText && (
        <div className="mt-6">
          <Button className="bg-accent text-accent-foreground hover:bg-accent/90 glow-accent" asChild>
            <Link href={linkHref}>{linkText}</Link>
          </Button>
        </div>
      )}
    </Card>
  )
}
