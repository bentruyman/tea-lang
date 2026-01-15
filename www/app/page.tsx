import Link from "next/link"
import { Button } from "@/components/ui/button"
import { Card } from "@/components/ui/card"
import { CodePlayground } from "@/components/code-playground"
import { Terminal, Zap, Package, ArrowRight, BookOpen, Code2, Github } from "lucide-react"
import Image from "next/image"

export default function HomePage() {
  return (
    <div className="min-h-screen bg-background texture-grid">
      {/* Header */}
      <header className="border-b border-border/50 bg-card/80 backdrop-blur-sm sticky top-0 z-50">
        <div className="container mx-auto px-4 h-16 flex items-center justify-between">
          <Link href="/" className="flex items-center gap-3">
            <Image src="/tea-logo.svg" alt="Tea" width={32} height={32} />
            <span className="font-bold text-xl">Tea</span>
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
            <Button variant="ghost" size="sm" asChild>
              <Link href="https://github.com/special-tea/tea" target="_blank" rel="noopener noreferrer">
                <Github className="h-4 w-4 mr-2" />
                GitHub
              </Link>
            </Button>
          </nav>
        </div>
      </header>

      {/* Hero Section - Added texture-noise for depth */}
      <section className="container mx-auto px-4 py-24 md:py-32 relative texture-noise">
        <div className="max-w-5xl mx-auto text-center space-y-8">
          <div className="flex justify-center mb-6">
            <div className="p-4 rounded-2xl glow-accent">
              <Image src="/tea-logo.svg" alt="Tea Logo" width={96} height={96} />
            </div>
          </div>

          <h1 className="text-5xl md:text-7xl font-black text-balance leading-tight">
            Build Things <span className="text-emerald-400">Fast</span>
          </h1>

          <p className="text-xl md:text-2xl text-muted-foreground text-balance max-w-3xl mx-auto leading-relaxed">
            Tea is a strongly-typed, compiled language designed for building tools and applications. Write familiar
            code, compile to native binaries, ship instantly.
          </p>

          <div className="flex flex-col sm:flex-row items-center justify-center gap-4 pt-4">
            <Button size="lg" className="bg-accent text-accent-foreground hover:bg-accent/90 glow-accent" asChild>
              <Link href="/docs/getting-started">
                Get Started
                <ArrowRight className="ml-2 h-4 w-4" />
              </Link>
            </Button>
            <Button size="lg" variant="outline" asChild>
              <Link href="/docs/install">Install Tea</Link>
            </Button>
          </div>

          {/* Quick Install - Added corner brackets and panel-inset */}
          <div className="pt-8">
            <Card className="inline-block bg-card border-accent/30 p-6 text-left corner-brackets panel-inset">
              <div className="flex items-center gap-2 mb-3">
                <Terminal className="h-4 w-4 text-accent" />
                <span className="text-xs font-semibold text-accent uppercase tracking-wide">Quick Install</span>
              </div>
              <pre className="font-mono text-sm text-foreground">
                <code>git clone https://github.com/special-tea/tea.git</code>
              </pre>
            </Card>
          </div>
        </div>

        <div className="divider-mechanical mt-24 max-w-4xl mx-auto" />
      </section>

      {/* Features Grid - Added texture-dots background */}
      <section className="container mx-auto px-4 py-16 md:py-24 texture-dots">
        <div className="max-w-6xl mx-auto">
          <h2 className="text-3xl md:text-4xl font-bold text-center mb-12">Why Tea?</h2>

          <div className="grid md:grid-cols-3 gap-6">
            <Card className="p-6 bg-card border-border hover:border-accent/50 transition-all panel-inset hover:glow-accent">
              <Terminal className="h-10 w-10 text-accent mb-4" />
              <h3 className="font-bold text-xl mb-3">CLI-First</h3>
              <p className="text-muted-foreground leading-relaxed">
                Built specifically for command-line tools with native binary compilation. Perfect for building fast,
                portable utilities.
              </p>
            </Card>

            <Card className="p-6 bg-card border-border hover:border-accent/50 transition-all panel-inset hover:glow-accent">
              <Zap className="h-10 w-10 text-accent mb-4" />
              <h3 className="font-bold text-xl mb-3">Fast</h3>
              <p className="text-muted-foreground leading-relaxed">
                Zero startup time, no JIT overhead, pure native performance. Your tools start instantly with minimal
                resource usage.
              </p>
            </Card>

            <Card className="p-6 bg-card border-border hover:border-accent/50 transition-all panel-inset hover:glow-accent">
              <Package className="h-10 w-10 text-accent mb-4" />
              <h3 className="font-bold text-xl mb-3">Type Safe</h3>
              <p className="text-muted-foreground leading-relaxed">
                Catch bugs at compile time with powerful static typing and inference. Write confident code with full
                type safety.
              </p>
            </Card>
          </div>
        </div>
      </section>

      {/* Code Playground Section - Added texture-brushed for industrial feel */}
      <section className="container mx-auto px-4 py-16 md:py-24 bg-muted/20 texture-brushed relative">
        <div className="divider-mechanical absolute top-0 left-1/2 -translate-x-1/2 w-full max-w-4xl" />

        <div className="max-w-5xl mx-auto">
          <div className="text-center mb-12">
            <h2 className="text-3xl md:text-4xl font-bold mb-4">See Tea in Action</h2>
            <p className="text-lg text-muted-foreground">Try the interactive code playground</p>
          </div>

          <CodePlayground />
        </div>

        <div className="divider-mechanical absolute bottom-0 left-1/2 -translate-x-1/2 w-full max-w-4xl" />
      </section>

      {/* Quick Links - Added texture-hatch pattern */}
      <section className="container mx-auto px-4 py-16 md:py-24 texture-hatch">
        <div className="max-w-4xl mx-auto grid md:grid-cols-3 gap-6">
          <Link href="/docs/getting-started" className="group">
            <Card className="p-6 bg-card border-border hover:border-accent/50 transition-all h-full panel-inset hover:glow-accent">
              <div className="h-10 w-10 rounded-lg bg-accent/10 flex items-center justify-center mb-4">
                <BookOpen className="h-5 w-5 text-accent" />
              </div>
              <h3 className="font-semibold text-lg mb-2 group-hover:text-accent transition-colors">Documentation</h3>
              <p className="text-sm text-muted-foreground leading-relaxed">
                Learn Tea from basics to advanced features
              </p>
            </Card>
          </Link>

          <Link href="/examples" className="group">
            <Card className="p-6 bg-card border-border hover:border-accent/50 transition-all h-full panel-inset hover:glow-accent">
              <div className="h-10 w-10 rounded-lg bg-accent/10 flex items-center justify-center mb-4">
                <Code2 className="h-5 w-5 text-accent" />
              </div>
              <h3 className="font-semibold text-lg mb-2 group-hover:text-accent transition-colors">Examples</h3>
              <p className="text-sm text-muted-foreground leading-relaxed">
                Explore practical code examples and patterns
              </p>
            </Card>
          </Link>

          <Link href="/reference" className="group">
            <Card className="p-6 bg-card border-border hover:border-accent/50 transition-all h-full panel-inset hover:glow-accent">
              <div className="h-10 w-10 rounded-lg bg-accent/10 flex items-center justify-center mb-4">
                <Package className="h-5 w-5 text-accent" />
              </div>
              <h3 className="font-semibold text-lg mb-2 group-hover:text-accent transition-colors">API Reference</h3>
              <p className="text-sm text-muted-foreground leading-relaxed">Complete standard library documentation</p>
            </Card>
          </Link>
        </div>
      </section>

      {/* Footer - Added subtle texture-grid-fine */}
      <footer className="border-t border-border/50 bg-card/50 texture-grid-fine">
        <div className="container mx-auto px-4 py-12">
          <div className="flex flex-col md:flex-row items-center justify-between gap-4">
            <div className="flex items-center gap-3">
              <Image src="/tea-logo.svg" alt="Tea" width={24} height={24} />
              <span className="font-bold">Tea Language</span>
            </div>
            <div className="flex items-center gap-6">
              <Link href="/docs" className="text-sm text-muted-foreground hover:text-foreground transition-colors">
                Documentation
              </Link>
              <Link href="/examples" className="text-sm text-muted-foreground hover:text-foreground transition-colors">
                Examples
              </Link>
              <Link
                href="https://github.com/special-tea/tea"
                target="_blank"
                rel="noopener noreferrer"
                className="text-sm text-muted-foreground hover:text-foreground transition-colors"
              >
                GitHub
              </Link>
            </div>
          </div>
        </div>
      </footer>
    </div>
  )
}
