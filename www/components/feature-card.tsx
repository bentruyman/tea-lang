import Link from "next/link"
import { ArrowRight } from "lucide-react"
import { Card } from "@/components/ui/card"

interface FeatureCardProps {
  title: string
  description: string
  href: string
}

export function FeatureCard({ title, description, href }: FeatureCardProps) {
  return (
    <Link href={href}>
      <Card className="group h-full p-6 bg-card hover:bg-muted/30 border-border/50 hover:border-border transition-all cursor-pointer panel-inset hover:glow-accent">
        <h3 className="font-semibold text-base mb-3 text-foreground group-hover:text-accent transition-colors">
          {title}
        </h3>
        <p className="text-sm text-muted-foreground leading-relaxed mb-4">{description}</p>
        <div className="flex items-center gap-1 text-sm font-medium text-accent group-hover:gap-2 transition-all">
          Learn more
          <ArrowRight className="h-3.5 w-3.5" />
        </div>
      </Card>
    </Link>
  )
}
