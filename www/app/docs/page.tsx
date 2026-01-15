import { Button } from "@/components/ui/button"
import { Card } from "@/components/ui/card"
import Link from "next/link"
import { ArrowRight, BookOpen, Code2, Download, Zap, Rocket } from "lucide-react"

export default function DocsPage() {
  return (
    <div className="space-y-12">
      {/* Hero Section */}
      <div className="space-y-4">
        <h1 className="text-4xl font-bold text-balance">Welcome to Tea Documentation</h1>
        <p className="text-xl text-muted-foreground text-pretty leading-relaxed">
          Learn how to build fast, type-safe applications with Tea. This guide covers everything from basic syntax to
          advanced features like generics and native compilation.
        </p>
      </div>

      {/* Quick Links - Added panel-inset and hover:glow-accent */}
      <div className="grid md:grid-cols-3 gap-6">
        <Card className="p-6 bg-card border-border hover:bg-muted/50 transition-colors panel-inset hover:glow-accent">
          <div className="flex items-center gap-3 mb-3">
            <div className="h-10 w-10 rounded-lg bg-accent/10 flex items-center justify-center">
              <Rocket className="h-5 w-5 text-accent" />
            </div>
            <h3 className="font-semibold text-lg">Quick Start</h3>
          </div>
          <p className="text-sm text-muted-foreground mb-4 leading-relaxed">
            Get up and running with Tea in minutes. Install the compiler and write your first program.
          </p>
          <Button variant="ghost" size="sm" className="gap-2 text-accent hover:text-accent" asChild>
            <Link href="/docs/getting-started">
              Get Started
              <ArrowRight className="h-4 w-4" />
            </Link>
          </Button>
        </Card>

        <Card className="p-6 bg-card border-border hover:bg-muted/50 transition-colors panel-inset hover:glow-accent">
          <div className="flex items-center gap-3 mb-3">
            <div className="h-10 w-10 rounded-lg bg-accent/10 flex items-center justify-center">
              <BookOpen className="h-5 w-5 text-accent" />
            </div>
            <h3 className="font-semibold text-lg">Language Guide</h3>
          </div>
          <p className="text-sm text-muted-foreground mb-4 leading-relaxed">
            Deep dive into Tea's syntax, type system, and language features with comprehensive examples.
          </p>
          <Button variant="ghost" size="sm" className="gap-2 text-accent hover:text-accent" asChild>
            <Link href="/docs/syntax">
              Learn the Language
              <ArrowRight className="h-4 w-4" />
            </Link>
          </Button>
        </Card>

        <Card className="p-6 bg-card border-border hover:bg-muted/50 transition-colors panel-inset hover:glow-accent">
          <div className="flex items-center gap-3 mb-3">
            <div className="h-10 w-10 rounded-lg bg-accent/10 flex items-center justify-center">
              <Code2 className="h-5 w-5 text-accent" />
            </div>
            <h3 className="font-semibold text-lg">API Reference</h3>
          </div>
          <p className="text-sm text-muted-foreground mb-4 leading-relaxed">
            Complete reference for Tea's standard library, including collections, I/O, and more.
          </p>
          <Button variant="ghost" size="sm" className="gap-2 text-accent hover:text-accent" asChild>
            <Link href="/reference/stdlib">
              Browse API
              <ArrowRight className="h-4 w-4" />
            </Link>
          </Button>
        </Card>
      </div>

      <div className="divider-mechanical" />

      {/* Installation Section */}
      <div className="space-y-6">
        <div>
          <h2 className="text-3xl font-bold mb-2">Installation</h2>
          <p className="text-muted-foreground">Get Tea installed on your system in just a few steps.</p>
        </div>

        <Card className="p-6 bg-card border-border corner-brackets panel-inset">
          <div className="space-y-4">
            <div>
              <h3 className="font-semibold text-accent mb-2 flex items-center gap-2">
                <Download className="h-4 w-4" />
                Clone the Repository
              </h3>
              <pre className="bg-muted p-4 rounded-md overflow-x-auto texture-grid-fine">
                <code className="font-mono text-sm text-foreground">
                  git clone https://github.com/special-tea/tea.git{"\n"}
                  cd tea
                </code>
              </pre>
            </div>

            <div>
              <h3 className="font-semibold text-accent mb-2 flex items-center gap-2">
                <Zap className="h-4 w-4" />
                Build and Install
              </h3>
              <pre className="bg-muted p-4 rounded-md overflow-x-auto texture-grid-fine">
                <code className="font-mono text-sm text-foreground">
                  make setup{"\n"}
                  make install
                </code>
              </pre>
            </div>

            <div>
              <h3 className="font-semibold text-accent mb-2">Verify Installation</h3>
              <pre className="bg-muted p-4 rounded-md overflow-x-auto texture-grid-fine">
                <code className="font-mono text-sm text-foreground">
                  tea --version{"\n"}
                  tea --help
                </code>
              </pre>
            </div>
          </div>

          <div className="mt-6">
            <Button className="bg-accent text-accent-foreground hover:bg-accent/90 glow-accent" asChild>
              <Link href="/docs/install">Full Installation Guide</Link>
            </Button>
          </div>
        </Card>
      </div>

      <div className="divider-mechanical" />

      {/* Key Features */}
      <div className="space-y-6">
        <div>
          <h2 className="text-3xl font-bold mb-2">Key Features</h2>
          <p className="text-muted-foreground">What makes Tea special</p>
        </div>

        <div className="grid md:grid-cols-2 gap-6">
          <Card className="p-6 bg-card border-border panel-inset">
            <h3 className="font-semibold text-lg mb-3 text-accent">Static Typing with Inference</h3>
            <p className="text-sm text-muted-foreground leading-relaxed mb-4">
              Tea combines the safety of static typing with the convenience of type inference. Write concise code while
              catching errors at compile time.
            </p>
            <pre className="bg-muted p-3 rounded-md overflow-x-auto texture-grid-fine">
              <code className="font-mono text-xs">
                <span className="text-purple-400">var</span>{" "}
                <span className="text-foreground">numbers = [1, 2, 3]</span>
                {"\n"}
                <span className="text-muted-foreground">// Type inferred as Array&lt;Int&gt;</span>
              </code>
            </pre>
          </Card>

          <Card className="p-6 bg-card border-border panel-inset">
            <h3 className="font-semibold text-lg mb-3 text-accent">Powerful Generics</h3>
            <p className="text-sm text-muted-foreground leading-relaxed mb-4">
              Write reusable, type-safe code with Tea's generic system. Generics are specialized at compile time for
              optimal performance.
            </p>
            <pre className="bg-muted p-3 rounded-md overflow-x-auto texture-grid-fine">
              <code className="font-mono text-xs">
                <span className="text-purple-400">def</span> <span className="text-blue-400">first</span>
                <span className="text-foreground">&lt;T&gt;(arr: Array&lt;T&gt;)</span>
                {"\n  "}
                <span className="text-foreground">arr[0]</span>
                {"\n"}
                <span className="text-purple-400">end</span>
              </code>
            </pre>
          </Card>

          <Card className="p-6 bg-card border-border panel-inset">
            <h3 className="font-semibold text-lg mb-3 text-accent">Compiles to Native Binaries</h3>
            <p className="text-sm text-muted-foreground leading-relaxed mb-4">
              Compile your Tea code to fast native executables. Deploy standalone tools with no runtime
              dependencies—perfect for command-line utilities and system tools.
            </p>
            <pre className="bg-muted p-3 rounded-md overflow-x-auto texture-grid-fine">
              <code className="font-mono text-xs">
                <span className="text-muted-foreground"># Compile to native binary</span>
                {"\n"}
                <span className="text-foreground">tea compile app.tea -o app</span>
                {"\n\n"}
                <span className="text-muted-foreground"># Run the compiled binary</span>
                {"\n"}
                <span className="text-foreground">./app</span>
              </code>
            </pre>
          </Card>

          <Card className="p-6 bg-card border-border panel-inset">
            <h3 className="font-semibold text-lg mb-3 text-accent">Rich Standard Library</h3>
            <p className="text-sm text-muted-foreground leading-relaxed mb-4">
              Comprehensive standard library with filesystem operations, JSON/YAML parsing, process management, and
              more.
            </p>
            <pre className="bg-muted p-3 rounded-md overflow-x-auto texture-grid-fine">
              <code className="font-mono text-xs">
                <span className="text-purple-400">import</span> <span className="text-foreground">JSON</span>
                {"\n\n"}
                <span className="text-purple-400">var</span>{" "}
                <span className="text-foreground">data = JSON.parse(file)</span>
                {"\n"}
                <span className="text-blue-400">print</span>
                <span className="text-foreground">(data["name"])</span>
              </code>
            </pre>
          </Card>
        </div>
      </div>

      <div className="divider-mechanical" />

      {/* Next Steps */}
      <div className="space-y-6">
        <div>
          <h2 className="text-3xl font-bold mb-2">Next Steps</h2>
          <p className="text-muted-foreground">Continue your Tea journey</p>
        </div>

        <div className="flex flex-col gap-4">
          <Link
            href="/docs/getting-started"
            className="flex items-center justify-between p-4 rounded-lg border border-border bg-card hover:bg-muted/50 transition-colors group panel-inset"
          >
            <div>
              <h3 className="font-semibold mb-1 group-hover:text-accent transition-colors">Getting Started Guide</h3>
              <p className="text-sm text-muted-foreground">Write your first Tea program in 5 minutes</p>
            </div>
            <ArrowRight className="h-5 w-5 text-muted-foreground group-hover:text-accent transition-colors" />
          </Link>

          <Link
            href="/docs/types"
            className="flex items-center justify-between p-4 rounded-lg border border-border bg-card hover:bg-muted/50 transition-colors group panel-inset"
          >
            <div>
              <h3 className="font-semibold mb-1 group-hover:text-accent transition-colors">Type System Deep Dive</h3>
              <p className="text-sm text-muted-foreground">Learn about Tea's powerful type system</p>
            </div>
            <ArrowRight className="h-5 w-5 text-muted-foreground group-hover:text-accent transition-colors" />
          </Link>

          <Link
            href="/examples"
            className="flex items-center justify-between p-4 rounded-lg border border-border bg-card hover:bg-muted/50 transition-colors group panel-inset"
          >
            <div>
              <h3 className="font-semibold mb-1 group-hover:text-accent transition-colors">Browse Examples</h3>
              <p className="text-sm text-muted-foreground">See Tea in action with real-world examples</p>
            </div>
            <ArrowRight className="h-5 w-5 text-muted-foreground group-hover:text-accent transition-colors" />
          </Link>
        </div>
      </div>
    </div>
  )
}
