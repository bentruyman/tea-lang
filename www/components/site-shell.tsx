import { ReactNode } from "react"

import { Card } from "@/components/ui/card"
import { SectionNav } from "@/components/section-nav"
import Link from "next/link"

import { repo } from "@/lib/site"
import { cn } from "@/lib/utils"

export function SiteFooter() {
  return (
    <footer className="border-t border-border/70 bg-background/70">
      <div className="container mx-auto px-4 py-10">
        <div className="surface-quiet texture-grid-fine flex flex-col gap-5 rounded-[1.75rem] border border-border/70 px-6 py-6 text-sm text-muted-foreground md:flex-row md:items-end md:justify-between">
          <div className="max-w-xl space-y-2">
            <p className="text-[0.7rem] font-semibold uppercase tracking-[0.24em] text-primary">Tea Language</p>
            <p className="font-display text-2xl leading-none text-foreground">
              A strongly typed scripting language for native tools.
            </p>
            <p>© {new Date().getFullYear()} Tea Language.</p>
          </div>
          <div className="flex items-center gap-5">
            <Link href="/docs/contributing" className="hover:text-foreground">
              Contributing
            </Link>
            <Link href="/community" className="hover:text-foreground">
              Community
            </Link>
            <Link href={repo.url} target="_blank" rel="noreferrer" className="hover:text-foreground">
              Repository
            </Link>
          </div>
        </div>
      </div>
    </footer>
  )
}

export function SectionLayout({
  sections,
  children,
}: {
  sections: { title: string; items: { title: string; href: string }[] }[]
  children: ReactNode
}) {
  return (
    <div className="container mx-auto grid gap-10 px-4 py-10 lg:grid-cols-[280px_minmax(0,1fr)] lg:py-14">
      <aside className="hidden lg:block">
        <div className="sticky top-24">
          <SectionNav sections={sections} />
        </div>
      </aside>
      <div className="min-w-0">{children}</div>
    </div>
  )
}

export function PageIntro({
  eyebrow,
  title,
  description,
}: {
  eyebrow?: string
  title: string
  description: string
}) {
  return (
    <div className="space-y-3">
      {eyebrow ? (
        <p className="text-xs font-semibold uppercase tracking-[0.2em] text-primary">{eyebrow}</p>
      ) : null}
      <h1 className="max-w-4xl font-display text-4xl font-semibold tracking-tight text-balance md:text-[3.6rem]">
        {title}
      </h1>
      <p className="max-w-3xl text-base leading-8 text-muted-foreground md:text-lg">{description}</p>
    </div>
  )
}

export function ContentSection({ children }: { children: ReactNode }) {
  return <div className="content-section space-y-5">{children}</div>
}

export function ContentPage({
  eyebrow,
  title,
  description,
  sourcePaths: _sourcePaths,
  children,
}: {
  eyebrow?: string
  title: string
  description: string
  sourcePaths?: string[]
  children: ReactNode
}) {
  return (
    <article className="max-w-4xl space-y-10">
      <div className="space-y-6">
        <PageIntro eyebrow={eyebrow} title={title} description={description} />
        <div className="site-divider" />
      </div>
      <div className="prose-shell">{children}</div>
    </article>
  )
}

export function SectionCardGrid({
  items,
  className,
  cardClassName,
}: {
  items: { href: string; title: string; summary: string }[]
  className?: string
  cardClassName?: string
}) {
  return (
    <div className={cn("grid gap-4 md:grid-cols-2", className)}>
      {items.map((item) => (
        <Link key={item.href} href={item.href}>
          <Card
            className={cn(
              "h-full gap-3 p-6 transition-all duration-200 hover:-translate-y-0.5 hover:border-primary/20 hover:bg-background/78",
              cardClassName,
            )}
          >
            <h2 className="text-lg font-semibold">{item.title}</h2>
            <p className="text-sm leading-6 text-muted-foreground">{item.summary}</p>
          </Card>
        </Link>
      ))}
    </div>
  )
}

export function GroupedSectionCardGrid({
  sections,
}: {
  sections: { title: string; items: { href: string; title: string; summary: string }[] }[]
}) {
  return (
    <div className="space-y-8">
      {sections.map((section, sectionIdx) => {
        const isFeatured = sectionIdx === 0
        return (
          <div key={section.title}>
            {sectionIdx > 0 && <div className="divider-section" />}
            <div
              className={cn(
                "rounded-[1.5rem] border border-border/70 p-5 md:p-6",
                isFeatured
                  ? "section-band surface-feature texture-hatch"
                  : "surface-quiet texture-grid-fine",
              )}
            >
              <div className="relative z-10">
                <p className="text-xs font-semibold uppercase tracking-[0.2em] text-primary">
                  {section.title}
                </p>
                <div className="mt-4 grid gap-4 md:grid-cols-2">
                  {section.items.map((item) => (
                    <Link key={item.href} href={item.href}>
                      <Card
                        className={cn(
                          "h-full gap-3 p-5 transition-all duration-200 hover:-translate-y-0.5 hover:border-primary/20",
                          isFeatured
                            ? "glow-accent bg-background/90 hover:bg-background/95"
                            : "hover:bg-background/78",
                        )}
                      >
                        <h3 className="font-display text-xl font-semibold tracking-tight">{item.title}</h3>
                        <p className="text-sm leading-6 text-muted-foreground">{item.summary}</p>
                      </Card>
                    </Link>
                  ))}
                </div>
              </div>
            </div>
          </div>
        )
      })}
    </div>
  )
}
