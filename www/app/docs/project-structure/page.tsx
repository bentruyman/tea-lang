import { Card } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import Link from "next/link"
import { Folder, FileCode, Settings, TestTube, BookOpen, ArrowRight } from "lucide-react"

export default function ProjectStructurePage() {
  return (
    <div className="space-y-12">
      {/* Header */}
      <div className="space-y-4">
        <h1 className="text-4xl font-bold text-balance">Project Structure</h1>
        <p className="text-xl text-muted-foreground text-pretty leading-relaxed">
          Learn how to organize your Tea projects for maintainability and scalability. This guide covers recommended
          directory layouts and file naming conventions.
        </p>
      </div>

      {/* Basic Structure */}
      <div className="space-y-6">
        <h2 className="text-3xl font-bold">Basic Project Layout</h2>
        <p className="text-muted-foreground">
          A typical Tea project follows this structure. Tea is flexible about organization, but this layout works well
          for most projects.
        </p>

        <Card className="p-6 bg-card border-border">
          <pre className="font-mono text-sm overflow-x-auto">
            <code>
              {`my-project/
├── src/
│   ├── main.tea          # Entry point
│   ├── config.tea        # Configuration
│   └── utils/
│       ├── mod.tea       # Module definition
│       └── helpers.tea   # Helper functions
├── tests/
│   └── utils_test.tea    # Test files
├── examples/
│   └── demo.tea          # Example programs
└── README.md             # Documentation`}
            </code>
          </pre>
        </Card>
      </div>

      {/* Key Directories */}
      <div className="space-y-6">
        <h2 className="text-3xl font-bold">Key Directories</h2>

        <div className="grid gap-4">
          <Card className="p-6 bg-card border-border">
            <div className="flex items-start gap-4">
              <div className="h-10 w-10 rounded-lg bg-accent/10 flex items-center justify-center shrink-0">
                <Folder className="h-5 w-5 text-accent" />
              </div>
              <div>
                <h3 className="font-semibold text-lg mb-2">src/</h3>
                <p className="text-sm text-muted-foreground mb-3">
                  Contains your main source code. The entry point is typically <code className="text-accent">main.tea</code> or named after your project.
                </p>
                <pre className="bg-muted p-3 rounded-md overflow-x-auto">
                  <code className="font-mono text-xs text-foreground">
                    {`# src/main.tea
use utils = "./utils/mod"

def main() -> Void
  @println(utils.greet("World"))
end

main()`}
                  </code>
                </pre>
              </div>
            </div>
          </Card>

          <Card className="p-6 bg-card border-border">
            <div className="flex items-start gap-4">
              <div className="h-10 w-10 rounded-lg bg-accent/10 flex items-center justify-center shrink-0">
                <FileCode className="h-5 w-5 text-accent" />
              </div>
              <div>
                <h3 className="font-semibold text-lg mb-2">Module Files (mod.tea)</h3>
                <p className="text-sm text-muted-foreground mb-3">
                  Each directory that acts as a module should have a <code className="text-accent">mod.tea</code> file that exports public functions.
                </p>
                <pre className="bg-muted p-3 rounded-md overflow-x-auto">
                  <code className="font-mono text-xs text-foreground">
                    {`# src/utils/mod.tea
pub def greet(name: String) -> String
  \`Hello, \${name}!\`
end

pub def add(a: Int, b: Int) -> Int
  a + b
end`}
                  </code>
                </pre>
              </div>
            </div>
          </Card>

          <Card className="p-6 bg-card border-border">
            <div className="flex items-start gap-4">
              <div className="h-10 w-10 rounded-lg bg-accent/10 flex items-center justify-center shrink-0">
                <TestTube className="h-5 w-5 text-accent" />
              </div>
              <div>
                <h3 className="font-semibold text-lg mb-2">tests/</h3>
                <p className="text-sm text-muted-foreground mb-3">
                  Test files are typically placed in a <code className="text-accent">tests/</code> directory. Use the <code className="text-accent">std.assert</code> module for assertions.
                </p>
                <pre className="bg-muted p-3 rounded-md overflow-x-auto">
                  <code className="font-mono text-xs text-foreground">
                    {`# tests/utils_test.tea
use assert = "std.assert"
use utils = "../src/utils/mod"

assert.eq(utils.add(1, 2), 3)
assert.eq(utils.greet("Tea"), "Hello, Tea!")`}
                  </code>
                </pre>
              </div>
            </div>
          </Card>
        </div>
      </div>

      {/* File Naming */}
      <div className="space-y-6">
        <h2 className="text-3xl font-bold">File Naming Conventions</h2>

        <Card className="p-6 bg-card border-border">
          <div className="space-y-4">
            <div>
              <h3 className="font-semibold text-accent mb-2">Source Files</h3>
              <p className="text-sm text-muted-foreground">
                Use <code className="text-accent">snake_case</code> for file names: <code className="text-accent">my_module.tea</code>, <code className="text-accent">string_utils.tea</code>
              </p>
            </div>

            <div>
              <h3 className="font-semibold text-accent mb-2">Module Entry Points</h3>
              <p className="text-sm text-muted-foreground">
                Name module entry points <code className="text-accent">mod.tea</code> to clearly indicate they define a module's public interface.
              </p>
            </div>

            <div>
              <h3 className="font-semibold text-accent mb-2">Test Files</h3>
              <p className="text-sm text-muted-foreground">
                Suffix test files with <code className="text-accent">_test.tea</code>: <code className="text-accent">utils_test.tea</code>, <code className="text-accent">parser_test.tea</code>
              </p>
            </div>
          </div>
        </Card>
      </div>

      {/* Module Imports */}
      <div className="space-y-6">
        <h2 className="text-3xl font-bold">Module Imports</h2>
        <p className="text-muted-foreground">
          Tea supports both standard library imports and relative path imports.
        </p>

        <Card className="p-6 bg-card border-border">
          <div className="space-y-4">
            <div>
              <h3 className="font-semibold text-accent mb-2">Standard Library</h3>
              <pre className="bg-muted p-3 rounded-md overflow-x-auto">
                <code className="font-mono text-xs text-foreground">
                  {`use fs = "std.fs"
use json = "std.json"
use path = "std.path"`}
                </code>
              </pre>
            </div>

            <div>
              <h3 className="font-semibold text-accent mb-2">Relative Imports</h3>
              <pre className="bg-muted p-3 rounded-md overflow-x-auto">
                <code className="font-mono text-xs text-foreground">
                  {`use helpers = "./helpers"
use utils = "../utils/mod"`}
                </code>
              </pre>
            </div>
          </div>
        </Card>
      </div>

      {/* Larger Project */}
      <div className="space-y-6">
        <h2 className="text-3xl font-bold">Larger Project Example</h2>
        <p className="text-muted-foreground">
          For larger projects, you might organize by feature or domain.
        </p>

        <Card className="p-6 bg-card border-border">
          <pre className="font-mono text-sm overflow-x-auto">
            <code>
              {`my-app/
├── src/
│   ├── main.tea
│   ├── config/
│   │   └── mod.tea
│   ├── models/
│   │   ├── mod.tea
│   │   ├── user.tea
│   │   └── team.tea
│   ├── services/
│   │   ├── mod.tea
│   │   ├── auth.tea
│   │   └── data.tea
│   └── utils/
│       ├── mod.tea
│       └── helpers.tea
├── tests/
│   ├── models_test.tea
│   └── services_test.tea
└── examples/
    └── demo.tea`}
            </code>
          </pre>
        </Card>
      </div>

      {/* Next Steps */}
      <div className="space-y-6">
        <h2 className="text-3xl font-bold">Next Steps</h2>

        <div className="flex flex-col gap-4">
          <Link
            href="/docs/modules"
            className="flex items-center justify-between p-4 rounded-lg border border-border bg-card hover:bg-muted/50 transition-colors group panel-inset"
          >
            <div>
              <h3 className="font-semibold mb-1 group-hover:text-accent transition-colors">Modules & Imports</h3>
              <p className="text-sm text-muted-foreground">Learn more about Tea's module system</p>
            </div>
            <ArrowRight className="h-5 w-5 text-muted-foreground group-hover:text-accent transition-colors" />
          </Link>

          <Link
            href="/docs/syntax"
            className="flex items-center justify-between p-4 rounded-lg border border-border bg-card hover:bg-muted/50 transition-colors group panel-inset"
          >
            <div>
              <h3 className="font-semibold mb-1 group-hover:text-accent transition-colors">Syntax Basics</h3>
              <p className="text-sm text-muted-foreground">Start learning Tea's syntax</p>
            </div>
            <ArrowRight className="h-5 w-5 text-muted-foreground group-hover:text-accent transition-colors" />
          </Link>
        </div>
      </div>
    </div>
  )
}
