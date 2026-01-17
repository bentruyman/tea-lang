import Link from 'next/link'
import { ArrowRight, FileCode, Zap, Terminal, BookOpen, Code2, Rocket, LucideIcon } from 'lucide-react'

const iconMap: Record<string, LucideIcon> = {
  file: FileCode,
  zap: Zap,
  terminal: Terminal,
  book: BookOpen,
  code: Code2,
  rocket: Rocket,
}

interface GridLinkProps {
  title: string
  description: string
  href: string
  icon?: string
}

export function GridLink({ title, description, href, icon = 'file' }: GridLinkProps) {
  const IconComponent = iconMap[icon] || FileCode

  return (
    <Link
      href={href}
      className="flex items-start gap-4 p-4 rounded-lg border border-border bg-card hover:bg-muted/50 transition-colors group"
    >
      <div className="h-10 w-10 rounded-lg bg-accent/10 flex items-center justify-center shrink-0">
        <IconComponent className="h-5 w-5 text-accent" />
      </div>
      <div className="flex-1">
        <h3 className="font-semibold mb-1 group-hover:text-accent transition-colors">{title}</h3>
        <p className="text-sm text-muted-foreground">{description}</p>
      </div>
      <ArrowRight className="h-5 w-5 text-muted-foreground group-hover:text-accent transition-colors shrink-0 mt-2" />
    </Link>
  )
}
