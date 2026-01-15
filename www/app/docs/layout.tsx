import type React from "react"
import {
  Sidebar,
  SidebarContent,
  SidebarGroup,
  SidebarGroupContent,
  SidebarGroupLabel,
  SidebarHeader,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
  SidebarProvider,
  SidebarRail,
  SidebarTrigger,
} from "@/components/ui/sidebar"
import { BookOpen, Code2, FileText, Rocket, Zap } from "lucide-react"
import Link from "next/link"
import Image from "next/image"

const docsNavigation = [
  {
    title: "Getting Started",
    icon: Rocket,
    items: [
      { title: "Introduction", href: "/docs" },
      { title: "Installation", href: "/docs/install" },
      { title: "Quick Start", href: "/docs/getting-started" },
      { title: "Project Structure", href: "/docs/project-structure" },
    ],
  },
  {
    title: "Language Guide",
    icon: BookOpen,
    items: [
      { title: "Syntax Basics", href: "/docs/syntax" },
      { title: "Type System", href: "/docs/types" },
      { title: "Functions", href: "/docs/functions" },
      { title: "Classes & Objects", href: "/docs/classes" },
      { title: "Generics", href: "/docs/generics" },
      { title: "Pattern Matching", href: "/docs/pattern-matching" },
      { title: "Error Handling", href: "/docs/error-handling" },
    ],
  },
  {
    title: "Advanced Topics",
    icon: Zap,
    items: [
      { title: "Modules & Imports", href: "/docs/modules" },
      { title: "Concurrency", href: "/docs/concurrency" },
      { title: "Memory Management", href: "/docs/memory" },
      { title: "Metaprogramming", href: "/docs/metaprogramming" },
    ],
  },
  {
    title: "Standard Library",
    icon: Code2,
    items: [
      { title: "Overview", href: "/reference/stdlib" },
      { title: "Collections", href: "/reference/collections" },
      { title: "File System", href: "/reference/filesystem" },
      { title: "JSON & YAML", href: "/reference/json-yaml" },
      { title: "Process Management", href: "/reference/process" },
    ],
  },
  {
    title: "Contributing",
    icon: FileText,
    items: [
      { title: "Contributing Guide", href: "/docs/contributing" },
      { title: "Code Style", href: "/docs/code-style" },
      { title: "Testing", href: "/docs/testing" },
    ],
  },
]

export default function DocsLayout({ children }: { children: React.ReactNode }) {
  return (
    <SidebarProvider defaultOpen={true}>
      <div className="flex min-h-screen w-full texture-grid">
        <Sidebar collapsible="icon" variant="sidebar" className="texture-grid-fine">
          <SidebarHeader className="border-b border-border">
            <Link href="/" className="flex items-center gap-2.5 px-2 py-1 group">
              <Image
                src="/tea-logo.svg"
                alt="Tea Logo"
                width={20}
                height={20}
                className="group-hover:opacity-80 transition-opacity"
              />
              <span className="text-xl font-bold text-foreground group-hover:text-accent transition-colors">Tea</span>
            </Link>
          </SidebarHeader>

          <SidebarContent>
            {docsNavigation.map((section) => (
              <SidebarGroup key={section.title}>
                <SidebarGroupLabel>
                  <section.icon className="mr-2 h-4 w-4" />
                  {section.title}
                </SidebarGroupLabel>
                <SidebarGroupContent>
                  <SidebarMenu>
                    {section.items.map((item) => (
                      <SidebarMenuItem key={item.href}>
                        <SidebarMenuButton asChild>
                          <Link href={item.href}>{item.title}</Link>
                        </SidebarMenuButton>
                      </SidebarMenuItem>
                    ))}
                  </SidebarMenu>
                </SidebarGroupContent>
              </SidebarGroup>
            ))}
          </SidebarContent>

          <SidebarRail />
        </Sidebar>

        <div className="flex-1 flex flex-col">
          <header className="sticky top-0 z-40 border-b border-border bg-background/95 backdrop-blur supports-[backdrop-filter]:bg-background/60 texture-brushed">
            <div className="container flex h-14 items-center gap-4 px-4">
              <SidebarTrigger />
              <div className="flex-1" />
              <nav className="flex items-center gap-4">
                <Link
                  href="/examples"
                  className="text-sm text-muted-foreground hover:text-foreground transition-colors"
                >
                  Examples
                </Link>
                <Link
                  href="/reference"
                  className="text-sm text-muted-foreground hover:text-foreground transition-colors"
                >
                  Reference
                </Link>
                <Link
                  href="/playground"
                  className="text-sm text-muted-foreground hover:text-foreground transition-colors"
                >
                  Playground
                </Link>
                <Link
                  href="https://github.com/special-tea/tea"
                  target="_blank"
                  className="text-sm text-muted-foreground hover:text-foreground transition-colors"
                >
                  GitHub
                </Link>
              </nav>
            </div>
          </header>

          <main className="flex-1">
            <div className="container max-w-5xl py-8 px-4">{children}</div>
          </main>

          <footer className="border-t border-border mt-auto texture-grid-fine">
            <div className="container py-6 px-4">
              <div className="flex flex-col md:flex-row items-center justify-between gap-4 text-sm text-muted-foreground">
                <p>© 2025 Tea Language. Open source under MIT License.</p>
                <div className="flex items-center gap-4">
                  <Link href="/docs/contributing" className="hover:text-accent transition-colors">
                    Contribute
                  </Link>
                  <Link href="https://github.com/special-tea/tea" className="hover:text-accent transition-colors">
                    GitHub
                  </Link>
                  <Link href="/community" className="hover:text-accent transition-colors">
                    Community
                  </Link>
                </div>
              </div>
            </div>
          </footer>
        </div>
      </div>
    </SidebarProvider>
  )
}
