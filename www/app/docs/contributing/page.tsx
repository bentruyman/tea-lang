import { Card } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import Link from "next/link"
import { ArrowRight, GitBranch, Bug, Lightbulb, Code2 } from "lucide-react"

export default function ContributingPage() {
  return (
    <div className="space-y-12">
      {/* Header */}
      <div className="space-y-4">
        <h1 className="text-4xl font-bold text-balance">Contributing Guide</h1>
        <p className="text-xl text-muted-foreground text-pretty leading-relaxed">
          Thank you for your interest in contributing to Tea! This guide will help you get started with contributing
          to the project, whether you're fixing bugs, adding features, or improving documentation.
        </p>
      </div>

      {/* Ways to Contribute */}
      <div className="space-y-6">
        <h2 className="text-3xl font-bold">Ways to Contribute</h2>

        <div className="grid md:grid-cols-2 gap-4">
          <Card className="p-6 bg-card border-border">
            <div className="flex items-start gap-4">
              <div className="h-10 w-10 rounded-lg bg-accent/10 flex items-center justify-center shrink-0">
                <Bug className="h-5 w-5 text-accent" />
              </div>
              <div>
                <h3 className="font-semibold text-lg mb-2">Report Bugs</h3>
                <p className="text-sm text-muted-foreground">
                  Found a bug? Open an issue with reproduction steps and expected behavior.
                </p>
              </div>
            </div>
          </Card>

          <Card className="p-6 bg-card border-border">
            <div className="flex items-start gap-4">
              <div className="h-10 w-10 rounded-lg bg-accent/10 flex items-center justify-center shrink-0">
                <Lightbulb className="h-5 w-5 text-accent" />
              </div>
              <div>
                <h3 className="font-semibold text-lg mb-2">Suggest Features</h3>
                <p className="text-sm text-muted-foreground">
                  Have an idea? Open a feature request to discuss new functionality.
                </p>
              </div>
            </div>
          </Card>

          <Card className="p-6 bg-card border-border">
            <div className="flex items-start gap-4">
              <div className="h-10 w-10 rounded-lg bg-accent/10 flex items-center justify-center shrink-0">
                <Code2 className="h-5 w-5 text-accent" />
              </div>
              <div>
                <h3 className="font-semibold text-lg mb-2">Submit Code</h3>
                <p className="text-sm text-muted-foreground">
                  Fix bugs, add features, or improve performance with pull requests.
                </p>
              </div>
            </div>
          </Card>

          <Card className="p-6 bg-card border-border">
            <div className="flex items-start gap-4">
              <div className="h-10 w-10 rounded-lg bg-accent/10 flex items-center justify-center shrink-0">
                <GitBranch className="h-5 w-5 text-accent" />
              </div>
              <div>
                <h3 className="font-semibold text-lg mb-2">Improve Docs</h3>
                <p className="text-sm text-muted-foreground">
                  Help improve documentation, examples, and tutorials.
                </p>
              </div>
            </div>
          </Card>
        </div>
      </div>

      {/* Getting Started */}
      <div className="space-y-6">
        <h2 className="text-3xl font-bold">Getting Started</h2>

        <Card className="p-6 bg-card border-border">
          <h3 className="font-semibold text-lg mb-3 text-accent">1. Fork and Clone</h3>
          <pre className="bg-muted p-4 rounded-md overflow-x-auto">
            <code className="font-mono text-sm">
              {`# Fork the repository on GitHub, then:
git clone https://github.com/YOUR_USERNAME/tea.git
cd tea
git remote add upstream https://github.com/special-tea/tea.git`}
            </code>
          </pre>
        </Card>

        <Card className="p-6 bg-card border-border">
          <h3 className="font-semibold text-lg mb-3 text-accent">2. Set Up Development Environment</h3>
          <pre className="bg-muted p-4 rounded-md overflow-x-auto">
            <code className="font-mono text-sm">
              {`# Install dependencies and build
make setup

# Verify everything works
make test`}
            </code>
          </pre>
        </Card>

        <Card className="p-6 bg-card border-border">
          <h3 className="font-semibold text-lg mb-3 text-accent">3. Create a Branch</h3>
          <pre className="bg-muted p-4 rounded-md overflow-x-auto">
            <code className="font-mono text-sm">
              {`# Sync with upstream
git fetch upstream
git checkout main
git merge upstream/main

# Create feature branch
git checkout -b feature/your-feature-name`}
            </code>
          </pre>
        </Card>
      </div>

      {/* Development Workflow */}
      <div className="space-y-6">
        <h2 className="text-3xl font-bold">Development Workflow</h2>

        <Card className="p-6 bg-card border-border">
          <h3 className="font-semibold text-lg mb-3 text-accent">Build Commands</h3>
          <pre className="bg-muted p-4 rounded-md overflow-x-auto">
            <code className="font-mono text-sm">
              {`# Build the project
cargo build

# Build in release mode
cargo build --release

# Run Tea without installing
cargo run -p tea-cli -- script.tea`}
            </code>
          </pre>
        </Card>

        <Card className="p-6 bg-card border-border">
          <h3 className="font-semibold text-lg mb-3 text-accent">Testing</h3>
          <pre className="bg-muted p-4 rounded-md overflow-x-auto">
            <code className="font-mono text-sm">
              {`# Run all tests
make test

# Run specific test
cargo test -p tea-compiler test_name

# Run E2E tests
./scripts/e2e.sh`}
            </code>
          </pre>
        </Card>

        <Card className="p-6 bg-card border-border">
          <h3 className="font-semibold text-lg mb-3 text-accent">Formatting</h3>
          <pre className="bg-muted p-4 rounded-md overflow-x-auto">
            <code className="font-mono text-sm">
              {`# Format all code
make fmt

# Format Rust only
cargo fmt --all

# Format Tea code
cargo run -p tea-cli -- fmt .`}
            </code>
          </pre>
        </Card>
      </div>

      {/* Pull Request Guidelines */}
      <div className="space-y-6">
        <h2 className="text-3xl font-bold">Pull Request Guidelines</h2>

        <Card className="p-6 bg-card border-border">
          <ul className="space-y-3 text-sm">
            <li className="flex items-start gap-3">
              <span className="text-accent font-bold">1.</span>
              <span className="text-muted-foreground">
                <strong className="text-foreground">Keep PRs focused</strong> - One feature or fix per PR
              </span>
            </li>
            <li className="flex items-start gap-3">
              <span className="text-accent font-bold">2.</span>
              <span className="text-muted-foreground">
                <strong className="text-foreground">Write tests</strong> - Include tests for new functionality
              </span>
            </li>
            <li className="flex items-start gap-3">
              <span className="text-accent font-bold">3.</span>
              <span className="text-muted-foreground">
                <strong className="text-foreground">Update documentation</strong> - Document new features or changes
              </span>
            </li>
            <li className="flex items-start gap-3">
              <span className="text-accent font-bold">4.</span>
              <span className="text-muted-foreground">
                <strong className="text-foreground">Follow code style</strong> - Run <code className="text-accent">make fmt</code> before committing
              </span>
            </li>
            <li className="flex items-start gap-3">
              <span className="text-accent font-bold">5.</span>
              <span className="text-muted-foreground">
                <strong className="text-foreground">Write clear commit messages</strong> - Describe what and why
              </span>
            </li>
          </ul>
        </Card>
      </div>

      {/* Project Structure */}
      <div className="space-y-6">
        <h2 className="text-3xl font-bold">Project Structure</h2>

        <Card className="p-6 bg-card border-border">
          <pre className="font-mono text-sm overflow-x-auto">
            {`tea/
├── tea-cli/          # CLI binary
├── tea-compiler/     # Core compilation pipeline
├── tea-runtime/      # C runtime library
├── tea-intrinsics/   # Rust intrinsics implementations
├── tea-lsp/          # Language server
├── stdlib/           # Standard library (Tea code)
├── examples/         # Example programs
├── spec/             # Grammar specifications
└── tree-sitter-tea/  # Syntax highlighting`}
          </pre>
        </Card>
      </div>

      {/* Getting Help */}
      <div className="space-y-6">
        <h2 className="text-3xl font-bold">Getting Help</h2>

        <Card className="p-6 bg-card border-border">
          <p className="text-muted-foreground mb-4">
            If you have questions or need help:
          </p>
          <div className="flex flex-wrap gap-3">
            <Button variant="outline" size="sm" asChild>
              <Link href="https://github.com/special-tea/tea/discussions" target="_blank">
                GitHub Discussions
              </Link>
            </Button>
            <Button variant="outline" size="sm" asChild>
              <Link href="https://github.com/special-tea/tea/issues" target="_blank">
                Open an Issue
              </Link>
            </Button>
            <Button variant="outline" size="sm" asChild>
              <Link href="/community">
                Join Community
              </Link>
            </Button>
          </div>
        </Card>
      </div>

      {/* Next Steps */}
      <div className="space-y-6">
        <h2 className="text-3xl font-bold">Next Steps</h2>

        <div className="flex flex-col gap-4">
          <Link
            href="/docs/code-style"
            className="flex items-center justify-between p-4 rounded-lg border border-border bg-card hover:bg-muted/50 transition-colors group panel-inset"
          >
            <div>
              <h3 className="font-semibold mb-1 group-hover:text-accent transition-colors">Code Style</h3>
              <p className="text-sm text-muted-foreground">Learn about Tea's coding conventions</p>
            </div>
            <ArrowRight className="h-5 w-5 text-muted-foreground group-hover:text-accent transition-colors" />
          </Link>

          <Link
            href="/docs/testing"
            className="flex items-center justify-between p-4 rounded-lg border border-border bg-card hover:bg-muted/50 transition-colors group panel-inset"
          >
            <div>
              <h3 className="font-semibold mb-1 group-hover:text-accent transition-colors">Testing</h3>
              <p className="text-sm text-muted-foreground">Write and run tests</p>
            </div>
            <ArrowRight className="h-5 w-5 text-muted-foreground group-hover:text-accent transition-colors" />
          </Link>
        </div>
      </div>
    </div>
  )
}
