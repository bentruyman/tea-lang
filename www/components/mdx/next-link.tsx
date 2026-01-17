import Link from 'next/link'
import { ArrowRight } from 'lucide-react'

interface NextLinkProps {
  title: string
  description: string
  href: string
}

export function NextLink({ title, description, href }: NextLinkProps) {
  return (
    <Link
      href={href}
      className="flex items-center justify-between p-4 rounded-lg border border-border bg-card hover:bg-muted/50 transition-colors group panel-inset"
    >
      <div>
        <h3 className="font-semibold mb-1 group-hover:text-accent transition-colors">{title}</h3>
        <p className="text-sm text-muted-foreground">{description}</p>
      </div>
      <ArrowRight className="h-5 w-5 text-muted-foreground group-hover:text-accent transition-colors" />
    </Link>
  )
}
