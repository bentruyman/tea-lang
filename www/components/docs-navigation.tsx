"use client"

import { usePathname } from "next/navigation"
import Link from "next/link"
import { cn } from "@/lib/utils"

interface NavItem {
  title: string
  href: string
}

interface DocsNavigationProps {
  items: NavItem[]
  title?: string
}

export function DocsNavigation({ items, title }: DocsNavigationProps) {
  const pathname = usePathname()

  return (
    <nav className="space-y-1">
      {title && <h4 className="font-semibold text-sm text-muted-foreground mb-2 px-2">{title}</h4>}
      {items.map((item) => {
        const isActive = pathname === item.href
        return (
          <Link
            key={item.href}
            href={item.href}
            className={cn(
              "block px-2 py-1.5 text-sm rounded-md transition-colors",
              isActive
                ? "bg-accent text-accent-foreground font-medium"
                : "text-muted-foreground hover:text-foreground hover:bg-muted/50",
            )}
          >
            {item.title}
          </Link>
        )
      })}
    </nav>
  )
}
