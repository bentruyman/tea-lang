import { ReactNode } from 'react'
import Link from 'next/link'
import { Card } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { ArrowRight, Rocket, BookOpen, Code2, Download, Zap, FileText, Users, Cog, LucideIcon } from 'lucide-react'

const iconMap: Record<string, LucideIcon> = {
  rocket: Rocket,
  book: BookOpen,
  code: Code2,
  download: Download,
  zap: Zap,
  file: FileText,
  users: Users,
  cog: Cog,
}

interface QuickLinkCardProps {
  title: string
  description: string
  href: string
  icon?: string
  buttonText?: string
}

export function QuickLinkCard({
  title,
  description,
  href,
  icon = 'rocket',
  buttonText = 'Learn More'
}: QuickLinkCardProps) {
  const IconComponent = iconMap[icon] || Rocket

  return (
    <Card className="p-6 bg-card border-border hover:bg-muted/50 transition-colors panel-inset hover:glow-accent">
      <div className="flex items-center gap-3 mb-3">
        <div className="h-10 w-10 rounded-lg bg-accent/10 flex items-center justify-center">
          <IconComponent className="h-5 w-5 text-accent" />
        </div>
        <h3 className="font-semibold text-lg">{title}</h3>
      </div>
      <p className="text-sm text-muted-foreground mb-4 leading-relaxed">
        {description}
      </p>
      <Button variant="ghost" size="sm" className="gap-2 text-accent hover:text-accent" asChild>
        <Link href={href}>
          {buttonText}
          <ArrowRight className="h-4 w-4" />
        </Link>
      </Button>
    </Card>
  )
}
