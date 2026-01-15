import { Card } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import Link from "next/link"
import { FileCode, Database, Zap, Terminal, Boxes } from "lucide-react"
import Image from "next/image"

const examples = [
  {
    title: "Hello World",
    description: "Your first Tea program - the classic Hello World example",
    icon: Terminal,
    href: "/examples/hello-world",
    difficulty: "Beginner",
  },
  {
    title: "CLI Applications",
    description: "Build command-line tools with argument parsing and user input",
    icon: Terminal,
    href: "/examples/cli",
    difficulty: "Beginner",
  },
  {
    title: "JSON Parsing",
    description: "Parse and manipulate JSON data with Tea's built-in JSON module",
    icon: FileCode,
    href: "/examples/json",
    difficulty: "Beginner",
  },
  {
    title: "File System Operations",
    description: "Read, write, and manipulate files and directories",
    icon: Database,
    href: "/examples/filesystem",
    difficulty: "Intermediate",
  },
  {
    title: "Generic Functions",
    description: "Write reusable, type-safe code with generics",
    icon: Boxes,
    href: "/examples/generics",
    difficulty: "Intermediate",
  },
  {
    title: "Web Server",
    description: "Build a simple HTTP server with routing and middleware",
    icon: Zap,
    href: "/examples/web-server",
    difficulty: "Advanced",
  },
  {
    title: "Concurrent Processing",
    description: "Use Tea's concurrency features for parallel processing",
    icon: Zap,
    href: "/examples/concurrency",
    difficulty: "Advanced",
  },
  {
    title: "Data Structures",
    description: "Implement common data structures like stacks, queues, and trees",
    icon: Boxes,
    href: "/examples/data-structures",
    difficulty: "Intermediate",
  },
]

export default function ExamplesPage() {
  return (
    <div className="min-h-screen bg-background">
      {/* Header */}
      <header className="border-b border-border bg-background/95 backdrop-blur supports-[backdrop-filter]:bg-background/60 sticky top-0 z-50">
        <div className="container mx-auto px-4 h-16 flex items-center justify-between">
          <div className="flex items-center gap-8">
            <Link href="/" className="flex items-center gap-2">
              <Image src="/tea-logo.svg" alt="Tea" width={32} height={32} className="h-8 w-8" />
              <span className="font-semibold text-xl text-foreground">Tea</span>
            </Link>
            <nav className="hidden md:flex items-center gap-6">
              <Link href="/docs" className="text-sm text-muted-foreground hover:text-foreground transition-colors">
                Docs
              </Link>
              <Link href="/examples" className="text-sm text-foreground font-medium">
                Examples
              </Link>
              <Link href="/reference" className="text-sm text-muted-foreground hover:text-foreground transition-colors">
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
            <h1 className="text-4xl font-bold text-balance">Tea Examples</h1>
            <p className="text-xl text-muted-foreground text-pretty leading-relaxed">
              Learn Tea through practical examples. Each example includes complete, runnable code with explanations of
              key concepts.
            </p>
          </div>

          {/* Filter by Difficulty */}
          <div className="flex items-center gap-3">
            <span className="text-sm font-medium text-muted-foreground">Filter by difficulty:</span>
            <div className="flex gap-2">
              <Button variant="outline" size="sm" className="bg-transparent">
                All
              </Button>
              <Button variant="ghost" size="sm">
                Beginner
              </Button>
              <Button variant="ghost" size="sm">
                Intermediate
              </Button>
              <Button variant="ghost" size="sm">
                Advanced
              </Button>
            </div>
          </div>

          {/* Examples Grid */}
          <div className="grid md:grid-cols-2 lg:grid-cols-3 gap-6">
            {examples.map((example) => (
              <Link key={example.href} href={example.href}>
                <Card className="h-full p-6 bg-card border-border hover:bg-muted/50 transition-colors group cursor-pointer">
                  <div className="flex items-start gap-4 mb-4">
                    <div className="h-12 w-12 rounded-lg bg-accent/10 flex items-center justify-center shrink-0">
                      <example.icon className="h-6 w-6 text-accent" />
                    </div>
                    <div className="flex-1 min-w-0">
                      <h3 className="font-semibold text-lg mb-1 group-hover:text-accent transition-colors">
                        {example.title}
                      </h3>
                      <span
                        className={`inline-block text-xs px-2 py-0.5 rounded-full ${
                          example.difficulty === "Beginner"
                            ? "bg-green-500/10 text-green-500"
                            : example.difficulty === "Intermediate"
                              ? "bg-yellow-500/10 text-yellow-500"
                              : "bg-red-500/10 text-red-500"
                        }`}
                      >
                        {example.difficulty}
                      </span>
                    </div>
                  </div>
                  <p className="text-sm text-muted-foreground leading-relaxed">{example.description}</p>
                </Card>
              </Link>
            ))}
          </div>

          {/* Call to Action */}
          <Card className="p-8 bg-muted/30 border-border text-center">
            <h2 className="text-2xl font-bold mb-3">Want to contribute an example?</h2>
            <p className="text-muted-foreground mb-6 max-w-2xl mx-auto">
              We're always looking for more examples to help the community learn Tea. If you have an interesting use
              case or pattern, we'd love to see it!
            </p>
            <div className="flex items-center justify-center gap-3">
              <Button className="bg-accent text-accent-foreground hover:bg-accent/90" asChild>
                <Link href="/docs/contributing">Contributing Guide</Link>
              </Button>
              <Button variant="outline" asChild>
                <Link href="https://github.com/special-tea/tea" target="_blank">
                  View on GitHub
                </Link>
              </Button>
            </div>
          </Card>
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
