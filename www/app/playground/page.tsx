import { Card } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import Link from "next/link"
import { Play, Share2, BookOpen, AlertCircle } from "lucide-react"

export default function PlaygroundPage() {
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
              <Link href="/reference" className="text-sm text-muted-foreground hover:text-foreground transition-colors">
                Reference
              </Link>
              <Link href="/playground" className="text-sm text-foreground font-medium">
                Playground
              </Link>
            </nav>
          </div>
          <Button variant="ghost" size="sm" asChild>
            <Link href="https://github.com/special-tea/tea" target="_blank">
              GitHub
            </Link>
          </Button>
        </div>
      </header>

      {/* Main Content */}
      <main className="container mx-auto px-4 py-12">
        <div className="max-w-6xl mx-auto space-y-8">
          {/* Header */}
          <div className="space-y-4">
            <h1 className="text-4xl font-bold text-balance">Tea Playground</h1>
            <p className="text-xl text-muted-foreground text-pretty leading-relaxed">
              Try Tea directly in your browser. Write code, run it, and see the results instantly.
            </p>
          </div>

          {/* Coming Soon Notice */}
          <Card className="p-6 bg-card border-border">
            <div className="flex items-start gap-4">
              <div className="h-10 w-10 rounded-lg bg-accent/10 flex items-center justify-center shrink-0">
                <AlertCircle className="h-5 w-5 text-accent" />
              </div>
              <div>
                <h2 className="text-xl font-semibold mb-2">Coming Soon</h2>
                <p className="text-muted-foreground mb-4">
                  The web-based playground is currently under development. In the meantime, you can try Tea
                  locally by following the installation guide.
                </p>
                <div className="flex flex-wrap gap-3">
                  <Button className="bg-accent text-accent-foreground hover:bg-accent/90" asChild>
                    <Link href="/docs/install">Install Tea</Link>
                  </Button>
                  <Button variant="outline" asChild>
                    <Link href="/docs/getting-started">Getting Started</Link>
                  </Button>
                </div>
              </div>
            </div>
          </Card>

          {/* Placeholder Editor */}
          <div className="grid lg:grid-cols-2 gap-4">
            <Card className="p-4 bg-card border-border">
              <div className="flex items-center justify-between mb-3">
                <span className="text-sm font-medium text-muted-foreground">main.tea</span>
                <div className="flex gap-2">
                  <Button variant="ghost" size="sm" disabled>
                    <Share2 className="h-4 w-4 mr-2" />
                    Share
                  </Button>
                </div>
              </div>
              <div className="bg-muted rounded-md p-4 min-h-[300px] font-mono text-sm">
                <pre className="text-muted-foreground">
                  {`# Welcome to Tea!
# The playground will be available soon.

def greet(name: String) -> String
  \`Hello, \${name}!\`
end

@println(greet("World"))`}
                </pre>
              </div>
              <div className="flex justify-end mt-3">
                <Button className="bg-accent text-accent-foreground hover:bg-accent/90" disabled>
                  <Play className="h-4 w-4 mr-2" />
                  Run
                </Button>
              </div>
            </Card>

            <Card className="p-4 bg-card border-border">
              <div className="flex items-center justify-between mb-3">
                <span className="text-sm font-medium text-muted-foreground">Output</span>
              </div>
              <div className="bg-muted rounded-md p-4 min-h-[300px] font-mono text-sm">
                <pre className="text-muted-foreground/50">
                  {`# Output will appear here when the playground is ready`}
                </pre>
              </div>
            </Card>
          </div>

          {/* Planned Features */}
          <div className="space-y-6">
            <h2 className="text-2xl font-bold">Planned Features</h2>

            <div className="grid md:grid-cols-3 gap-4">
              <Card className="p-5 bg-card border-border">
                <div className="h-10 w-10 rounded-lg bg-accent/10 flex items-center justify-center mb-3">
                  <Play className="h-5 w-5 text-accent" />
                </div>
                <h3 className="font-semibold mb-2">Instant Execution</h3>
                <p className="text-sm text-muted-foreground">
                  Run Tea code directly in your browser with instant feedback.
                </p>
              </Card>

              <Card className="p-5 bg-card border-border">
                <div className="h-10 w-10 rounded-lg bg-accent/10 flex items-center justify-center mb-3">
                  <Share2 className="h-5 w-5 text-accent" />
                </div>
                <h3 className="font-semibold mb-2">Share Code</h3>
                <p className="text-sm text-muted-foreground">
                  Share your code snippets with others via URL.
                </p>
              </Card>

              <Card className="p-5 bg-card border-border">
                <div className="h-10 w-10 rounded-lg bg-accent/10 flex items-center justify-center mb-3">
                  <BookOpen className="h-5 w-5 text-accent" />
                </div>
                <h3 className="font-semibold mb-2">Example Library</h3>
                <p className="text-sm text-muted-foreground">
                  Browse and run examples from the documentation.
                </p>
              </Card>
            </div>
          </div>

          {/* Try Locally */}
          <Card className="p-6 bg-muted/30 border-border">
            <h2 className="text-xl font-semibold mb-4">Try Tea Locally</h2>
            <p className="text-muted-foreground mb-4">
              While the playground is being built, you can install Tea and try it on your machine:
            </p>
            <pre className="bg-muted p-4 rounded-md overflow-x-auto mb-4">
              <code className="font-mono text-sm">
                {`git clone https://github.com/special-tea/tea.git
cd tea
make setup && make install
tea --help`}
              </code>
            </pre>
            <Button variant="outline" asChild>
              <Link href="/docs/install">Full Installation Guide</Link>
            </Button>
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
