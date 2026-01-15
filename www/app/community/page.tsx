import { Card } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import Link from "next/link"
import { Github, MessageCircle, BookOpen, Users, Heart, Code2 } from "lucide-react"

export default function CommunityPage() {
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
              <Link href="/playground" className="text-sm text-muted-foreground hover:text-foreground transition-colors">
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
        <div className="max-w-5xl mx-auto space-y-12">
          {/* Header */}
          <div className="space-y-4 text-center">
            <h1 className="text-4xl font-bold text-balance">Join the Tea Community</h1>
            <p className="text-xl text-muted-foreground text-pretty leading-relaxed max-w-2xl mx-auto">
              Connect with other Tea developers, get help, share your projects, and help shape the future of the language.
            </p>
          </div>

          {/* Main Links */}
          <div className="grid md:grid-cols-2 gap-6">
            <Card className="p-6 bg-card border-border hover:bg-muted/50 transition-colors">
              <div className="flex items-start gap-4">
                <div className="h-12 w-12 rounded-lg bg-accent/10 flex items-center justify-center shrink-0">
                  <Github className="h-6 w-6 text-accent" />
                </div>
                <div className="flex-1">
                  <h2 className="text-xl font-semibold mb-2">GitHub</h2>
                  <p className="text-sm text-muted-foreground mb-4">
                    Star the repo, report issues, contribute code, and follow development.
                  </p>
                  <Button className="bg-accent text-accent-foreground hover:bg-accent/90" asChild>
                    <Link href="https://github.com/special-tea/tea" target="_blank">
                      View on GitHub
                    </Link>
                  </Button>
                </div>
              </div>
            </Card>

            <Card className="p-6 bg-card border-border hover:bg-muted/50 transition-colors">
              <div className="flex items-start gap-4">
                <div className="h-12 w-12 rounded-lg bg-accent/10 flex items-center justify-center shrink-0">
                  <MessageCircle className="h-6 w-6 text-accent" />
                </div>
                <div className="flex-1">
                  <h2 className="text-xl font-semibold mb-2">Discussions</h2>
                  <p className="text-sm text-muted-foreground mb-4">
                    Ask questions, share ideas, and discuss Tea with the community.
                  </p>
                  <Button variant="outline" asChild>
                    <Link href="https://github.com/special-tea/tea/discussions" target="_blank">
                      Join Discussions
                    </Link>
                  </Button>
                </div>
              </div>
            </Card>
          </div>

          {/* Ways to Participate */}
          <div className="space-y-6">
            <h2 className="text-2xl font-bold text-center">Ways to Participate</h2>

            <div className="grid md:grid-cols-3 gap-4">
              <Card className="p-5 bg-card border-border">
                <div className="h-10 w-10 rounded-lg bg-accent/10 flex items-center justify-center mb-3">
                  <Code2 className="h-5 w-5 text-accent" />
                </div>
                <h3 className="font-semibold mb-2">Contribute Code</h3>
                <p className="text-sm text-muted-foreground mb-3">
                  Fix bugs, add features, or improve the compiler and standard library.
                </p>
                <Button variant="ghost" size="sm" className="text-accent" asChild>
                  <Link href="/docs/contributing">Contributing Guide</Link>
                </Button>
              </Card>

              <Card className="p-5 bg-card border-border">
                <div className="h-10 w-10 rounded-lg bg-accent/10 flex items-center justify-center mb-3">
                  <BookOpen className="h-5 w-5 text-accent" />
                </div>
                <h3 className="font-semibold mb-2">Improve Docs</h3>
                <p className="text-sm text-muted-foreground mb-3">
                  Help make the documentation clearer, more complete, and beginner-friendly.
                </p>
                <Button variant="ghost" size="sm" className="text-accent" asChild>
                  <Link href="/docs">Browse Docs</Link>
                </Button>
              </Card>

              <Card className="p-5 bg-card border-border">
                <div className="h-10 w-10 rounded-lg bg-accent/10 flex items-center justify-center mb-3">
                  <Users className="h-5 w-5 text-accent" />
                </div>
                <h3 className="font-semibold mb-2">Help Others</h3>
                <p className="text-sm text-muted-foreground mb-3">
                  Answer questions, review pull requests, and welcome new contributors.
                </p>
                <Button variant="ghost" size="sm" className="text-accent" asChild>
                  <Link href="https://github.com/special-tea/tea/issues" target="_blank">
                    View Issues
                  </Link>
                </Button>
              </Card>
            </div>
          </div>

          {/* Share Your Work */}
          <div className="space-y-6">
            <h2 className="text-2xl font-bold text-center">Share Your Work</h2>

            <Card className="p-6 bg-card border-border">
              <div className="space-y-4">
                <p className="text-muted-foreground">
                  Built something cool with Tea? We'd love to see it! Here's how you can share:
                </p>

                <ul className="space-y-3 text-sm">
                  <li className="flex items-start gap-3">
                    <span className="text-accent">•</span>
                    <span className="text-muted-foreground">
                      <strong className="text-foreground">GitHub Discussions</strong> - Share your projects in the "Show and Tell" category
                    </span>
                  </li>
                  <li className="flex items-start gap-3">
                    <span className="text-accent">•</span>
                    <span className="text-muted-foreground">
                      <strong className="text-foreground">Examples</strong> - Contribute example code to the official examples directory
                    </span>
                  </li>
                  <li className="flex items-start gap-3">
                    <span className="text-accent">•</span>
                    <span className="text-muted-foreground">
                      <strong className="text-foreground">Blog Posts</strong> - Write about your experience using Tea
                    </span>
                  </li>
                </ul>
              </div>
            </Card>
          </div>

          {/* Code of Conduct */}
          <div className="space-y-6">
            <h2 className="text-2xl font-bold text-center">Community Guidelines</h2>

            <Card className="p-6 bg-card border-border">
              <div className="flex items-start gap-4">
                <div className="h-10 w-10 rounded-lg bg-accent/10 flex items-center justify-center shrink-0">
                  <Heart className="h-5 w-5 text-accent" />
                </div>
                <div>
                  <p className="text-muted-foreground mb-4">
                    The Tea community is committed to being welcoming, inclusive, and respectful.
                    We expect all community members to:
                  </p>

                  <ul className="space-y-2 text-sm text-muted-foreground">
                    <li className="flex items-start gap-2">
                      <span className="text-accent">•</span>
                      Be respectful and considerate in all interactions
                    </li>
                    <li className="flex items-start gap-2">
                      <span className="text-accent">•</span>
                      Welcome newcomers and help them get started
                    </li>
                    <li className="flex items-start gap-2">
                      <span className="text-accent">•</span>
                      Focus on constructive feedback and collaboration
                    </li>
                    <li className="flex items-start gap-2">
                      <span className="text-accent">•</span>
                      Respect differing viewpoints and experiences
                    </li>
                  </ul>
                </div>
              </div>
            </Card>
          </div>

          {/* Get Started */}
          <Card className="p-6 bg-muted/30 border-border text-center">
            <h2 className="text-xl font-semibold mb-3">Ready to Get Involved?</h2>
            <p className="text-muted-foreground mb-6 max-w-xl mx-auto">
              Whether you're new to Tea or an experienced contributor, there's a place for you in our community.
            </p>
            <div className="flex flex-wrap justify-center gap-3">
              <Button className="bg-accent text-accent-foreground hover:bg-accent/90" asChild>
                <Link href="/docs/getting-started">Get Started with Tea</Link>
              </Button>
              <Button variant="outline" asChild>
                <Link href="/docs/contributing">Start Contributing</Link>
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
