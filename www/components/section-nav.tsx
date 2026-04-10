"use client"

import Link from "next/link"
import { usePathname } from "next/navigation"

import { cn } from "@/lib/utils"

type SectionItem = {
  title: string
  href: string
}

type SectionGroup = {
  title: string
  items: SectionItem[]
}

export function SectionNav({ sections }: { sections: SectionGroup[] }) {
  const pathname = usePathname()

  return (
    <nav className="surface-quiet texture-grid-fine rounded-[1.5rem] border border-border/70 p-4">
      <div className="site-divider mb-5" />
      <div className="space-y-6">
        {sections.map((section) => (
          <div key={section.title} className="space-y-2">
            <p className="px-3 text-xs font-semibold uppercase tracking-[0.2em] text-muted-foreground">
              {section.title}
            </p>
            <div className="space-y-1">
              {section.items.map((item) => {
                const isActive = pathname === item.href
                return (
                  <Link
                    key={item.href}
                    href={item.href}
                    aria-current={isActive ? "page" : undefined}
                    className={cn(
                      "block rounded-xl px-3 py-2.5 text-sm transition-all",
                      isActive
                        ? "bg-primary text-primary-foreground shadow-sm"
                        : "text-muted-foreground hover:bg-background/85 hover:text-foreground",
                    )}
                  >
                    {item.title}
                  </Link>
                )
              })}
            </div>
          </div>
        ))}
      </div>
    </nav>
  )
}
