import { Card } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import Link from "next/link"
import { Book, Code2, Database, FileText, Zap, Terminal, Package } from "lucide-react"

const referenceCategories = [
  {
    title: "Standard Library",
    description: "Core modules and functions available in every Tea program",
    icon: Package,
    href: "/reference/stdlib",
    items: ["Array", "String", "Math", "IO", "System"],
  },
  {
    title: "Collections",
    description: "Data structures for storing and manipulating collections of data",
    icon: Database,
    href: "/reference/collections",
    items: ["Array", "Map", "Set", "List", "Queue"],
  },
  {
    title: "File System",
    description: "Read, write, and manipulate files and directories",
    icon: FileText,
    href: "/reference/filesystem",
    items: ["File", "Dir", "Path", "Stat"],
  },
  {
    title: "JSON & YAML",
    description: "Parse and generate JSON and YAML data formats",
    icon: Code2,
    href: "/reference/json-yaml",
    items: ["JSON", "YAML"],
  },
  {
    title: "Process Management",
    description: "Execute and manage system processes",
    icon: Terminal,
    href: "/reference/process",
    items: ["Process", "Env", "Args"],
  },
  {
    title: "Networking",
    description: "HTTP clients, servers, and network utilities",
    icon: Zap,
    href: "/reference/networking",
    items: ["HTTP", "Socket", "URL"],
  },
]

export default function ReferencePage() {
  return (
    <div className="min-h-screen bg-background">
      {/* Header */}
      <header className="border-b border-border bg-background/95 backdrop-blur supports-[backdrop-filter]:bg-background/60 sticky top-0 z-50">
        <div className="container mx-auto px-4 h-16 flex items-center justify-between">
          <div className="flex items-center gap-8">
            <Link href="/" className="flex items-center gap-2">
              <div className="h-8 w-8 rounded-md bg-accent flex items-center justify-center">
                <span className="font-bold text-accent-foreground">T</span>
              </div>
              <span className="font-semibold text-xl text-foreground">Tea</span>
            </Link>
            <nav className="hidden md:flex items-center gap-6">
              <Link href="/docs" className="text-sm text-muted-foreground hover:text-foreground transition-colors">
                Docs
              </Link>
              <Link href="/examples" className="text-sm text-muted-foreground hover:text-foreground transition-colors">
                Examples
              </Link>
              <Link href="/reference" className="text-sm text-foreground font-medium">
                Reference
              </Link>
              <Link
                href="/playground"
                className="text-sm text-muted-foreground hover:text-foreground transition-colors"
              >
                Playground
              </Link>
            </nav>
          </div>
          <div className="flex items-center gap-3">
            <Button variant="ghost" size="sm" asChild>
              <Link href="https://github.com/special-tea/tea" target="_blank">
                GitHub
              </Link>
            </Button>
          </div>
        </div>
      </header>

      {/* Main Content */}
      <main className="container mx-auto px-4 py-12">
        <div className="max-w-6xl mx-auto space-y-12">
          {/* Hero */}
          <div className="space-y-4">
            <h1 className="text-4xl font-bold text-balance">API Reference</h1>
            <p className="text-xl text-muted-foreground text-pretty leading-relaxed">
              Complete reference documentation for Tea's standard library, including all modules, functions, and types.
            </p>
          </div>

          {/* Quick Search */}
          <Card className="p-6 bg-card border-border">
            <div className="flex items-center gap-3">
              <Book className="h-5 w-5 text-muted-foreground" />
              <input
                type="text"
                placeholder="Search API documentation..."
                className="flex-1 bg-transparent border-none outline-none text-foreground placeholder:text-muted-foreground"
              />
              <Button size="sm" variant="ghost">
                Search
              </Button>
            </div>
          </Card>

          {/* Categories Grid */}
          <div className="grid md:grid-cols-2 gap-6">
            {referenceCategories.map((category) => (
              <Link key={category.href} href={category.href}>
                <Card className="h-full p-6 bg-card border-border hover:bg-muted/50 transition-colors group cursor-pointer">
                  <div className="flex items-start gap-4 mb-4">
                    <div className="h-12 w-12 rounded-lg bg-accent/10 flex items-center justify-center shrink-0">
                      <category.icon className="h-6 w-6 text-accent" />
                    </div>
                    <div className="flex-1">
                      <h3 className="font-semibold text-lg mb-2 group-hover:text-accent transition-colors">
                        {category.title}
                      </h3>
                      <p className="text-sm text-muted-foreground leading-relaxed mb-3">{category.description}</p>
                      <div className="flex flex-wrap gap-2">
                        {category.items.map((item) => (
                          <span
                            key={item}
                            className="text-xs px-2 py-1 rounded-md bg-muted text-muted-foreground font-mono"
                          >
                            {item}
                          </span>
                        ))}
                      </div>
                    </div>
                  </div>
                </Card>
              </Link>
            ))}
          </div>

          {/* Quick Links */}
          <div className="grid md:grid-cols-3 gap-6">
            <Card className="p-6 bg-muted/30 border-border">
              <h3 className="font-semibold text-lg mb-3">Language Reference</h3>
              <ul className="space-y-2 text-sm">
                <li>
                  <Link href="/docs/syntax" className="text-muted-foreground hover:text-accent transition-colors">
                    Syntax Guide
                  </Link>
                </li>
                <li>
                  <Link href="/docs/types" className="text-muted-foreground hover:text-accent transition-colors">
                    Type System
                  </Link>
                </li>
                <li>
                  <Link href="/docs/generics" className="text-muted-foreground hover:text-accent transition-colors">
                    Generics
                  </Link>
                </li>
                <li>
                  <Link href="/docs/modules" className="text-muted-foreground hover:text-accent transition-colors">
                    Modules
                  </Link>
                </li>
              </ul>
            </Card>

            <Card className="p-6 bg-muted/30 border-border">
              <h3 className="font-semibold text-lg mb-3">Guides</h3>
              <ul className="space-y-2 text-sm">
                <li>
                  <Link
                    href="/docs/getting-started"
                    className="text-muted-foreground hover:text-accent transition-colors"
                  >
                    Getting Started
                  </Link>
                </li>
                <li>
                  <Link href="/docs/backends" className="text-muted-foreground hover:text-accent transition-colors">
                    Backend Options
                  </Link>
                </li>
                <li>
                  <Link
                    href="/docs/error-handling"
                    className="text-muted-foreground hover:text-accent transition-colors"
                  >
                    Error Handling
                  </Link>
                </li>
                <li>
                  <Link href="/docs/testing" className="text-muted-foreground hover:text-accent transition-colors">
                    Testing
                  </Link>
                </li>
              </ul>
            </Card>

            <Card className="p-6 bg-muted/30 border-border">
              <h3 className="font-semibold text-lg mb-3">Community</h3>
              <ul className="space-y-2 text-sm">
                <li>
                  <Link
                    href="https://github.com/special-tea/tea"
                    className="text-muted-foreground hover:text-accent transition-colors"
                  >
                    GitHub Repository
                  </Link>
                </li>
                <li>
                  <Link href="/community" className="text-muted-foreground hover:text-accent transition-colors">
                    Discord Community
                  </Link>
                </li>
                <li>
                  <Link href="/docs/contributing" className="text-muted-foreground hover:text-accent transition-colors">
                    Contributing Guide
                  </Link>
                </li>
                <li>
                  <Link href="/blog" className="text-muted-foreground hover:text-accent transition-colors">
                    Blog
                  </Link>
                </li>
              </ul>
            </Card>
          </div>
        </div>
      </main>

      {/* Footer */}
      <footer className="border-t border-border mt-20">
        <div className="container mx-auto px-4 py-12">
          <div className="text-center text-sm text-muted-foreground">
            © 2025 Tea Language. Open source under MIT License.
          </div>
        </div>
      </footer>
    </div>
  )
}
