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
      className="surface-card group flex items-center justify-between rounded-[1.35rem] border border-border/70 p-5 transition-all duration-200 hover:-translate-y-0.5 hover:border-primary/20 hover:bg-background/80"
    >
      <div className="space-y-1">
        <p className="text-[0.68rem] font-semibold uppercase tracking-[0.22em] text-primary">Continue to</p>
        <h3 className="font-display text-2xl font-semibold tracking-tight text-foreground">{title}</h3>
        <p className="text-sm leading-6 text-muted-foreground">{description}</p>
      </div>
      <span className="surface-quiet flex h-11 w-11 items-center justify-center rounded-full border border-border/70 transition-colors group-hover:border-primary/20">
        <ArrowRight className="h-5 w-5 text-muted-foreground transition-all duration-200 group-hover:translate-x-0.5 group-hover:text-primary" />
      </span>
    </Link>
  )
}
