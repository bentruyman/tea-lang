import { ReactNode } from 'react'
import { Card } from '@/components/ui/card'
import { Bug, Lightbulb, Code2, GitBranch, Folder, FileCode, TestTube, type LucideIcon } from 'lucide-react'

type IconName = 'bug' | 'lightbulb' | 'code' | 'git' | 'folder' | 'file' | 'test'

const iconMap: Record<IconName, LucideIcon> = {
  bug: Bug,
  lightbulb: Lightbulb,
  code: Code2,
  git: GitBranch,
  folder: Folder,
  file: FileCode,
  test: TestTube,
}

interface ContributionCardProps {
  icon: IconName
  title: string
  children: ReactNode
}

export function ContributionCard({ icon, title, children }: ContributionCardProps) {
  const Icon = iconMap[icon]

  return (
    <Card className="p-6 bg-card border-border">
      <div className="flex items-start gap-4">
        <div className="h-10 w-10 rounded-lg bg-accent/10 flex items-center justify-center shrink-0">
          <Icon className="h-5 w-5 text-accent" />
        </div>
        <div>
          <h3 className="font-semibold text-lg mb-2">{title}</h3>
          <p className="text-sm text-muted-foreground">{children}</p>
        </div>
      </div>
    </Card>
  )
}

interface ContributionGridProps {
  children: ReactNode
}

export function ContributionGrid({ children }: ContributionGridProps) {
  return (
    <div className="grid md:grid-cols-2 gap-4">
      {children}
    </div>
  )
}

interface DirectoryCardProps {
  icon: IconName
  title: string
  description: string
  children: ReactNode
}

export function DirectoryCard({ icon, title, description, children }: DirectoryCardProps) {
  const Icon = iconMap[icon]

  return (
    <Card className="p-6 bg-card border-border">
      <div className="flex items-start gap-4">
        <div className="h-10 w-10 rounded-lg bg-accent/10 flex items-center justify-center shrink-0">
          <Icon className="h-5 w-5 text-accent" />
        </div>
        <div>
          <h3 className="font-semibold text-lg mb-2">{title}</h3>
          <p className="text-sm text-muted-foreground mb-3">{description}</p>
          {children}
        </div>
      </div>
    </Card>
  )
}
