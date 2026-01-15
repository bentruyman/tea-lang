import { Card } from "@/components/ui/card"
import Link from "next/link"
import { ArrowRight } from "lucide-react"

export default function CodeStylePage() {
  return (
    <div className="space-y-12">
      {/* Header */}
      <div className="space-y-4">
        <h1 className="text-4xl font-bold text-balance">Code Style</h1>
        <p className="text-xl text-muted-foreground text-pretty leading-relaxed">
          This guide covers coding conventions for both Tea code and Rust code in the Tea compiler.
          Following consistent style makes the codebase easier to read and maintain.
        </p>
      </div>

      {/* Tea Code Style */}
      <div className="space-y-6">
        <h2 className="text-3xl font-bold">Tea Code Style</h2>

        <Card className="p-6 bg-card border-border">
          <h3 className="font-semibold text-lg mb-3 text-accent">Indentation</h3>
          <p className="text-sm text-muted-foreground mb-4">
            Use 2 spaces for indentation. Never use tabs.
          </p>
          <pre className="bg-muted p-4 rounded-md overflow-x-auto">
            <code className="font-mono text-sm">
              {`def calculate(x: Int) -> Int
  if x > 0
    return x * 2
  end
  0
end`}
            </code>
          </pre>
        </Card>

        <Card className="p-6 bg-card border-border">
          <h3 className="font-semibold text-lg mb-3 text-accent">Naming Conventions</h3>
          <div className="space-y-4">
            <div>
              <h4 className="font-semibold mb-2">Variables and Functions: snake_case</h4>
              <pre className="bg-muted p-3 rounded-md overflow-x-auto">
                <code className="font-mono text-sm">
                  {`var user_count = 0
var is_valid = true

def calculate_total(items: List[Int]) -> Int
  # ...
end`}
                </code>
              </pre>
            </div>

            <div>
              <h4 className="font-semibold mb-2">Types and Structs: PascalCase</h4>
              <pre className="bg-muted p-3 rounded-md overflow-x-auto">
                <code className="font-mono text-sm">
                  {`struct UserAccount {
  name: String
  email: String
}

error ValidationError {
  InvalidEmail(message: String)
}`}
                </code>
              </pre>
            </div>

            <div>
              <h4 className="font-semibold mb-2">Constants: SCREAMING_SNAKE_CASE or snake_case</h4>
              <pre className="bg-muted p-3 rounded-md overflow-x-auto">
                <code className="font-mono text-sm">
                  {`const MAX_RETRIES = 3
const PI = 3.14159

# Or lowercase for function-like constants
const double = |x: Int| => x * 2`}
                </code>
              </pre>
            </div>
          </div>
        </Card>

        <Card className="p-6 bg-card border-border">
          <h3 className="font-semibold text-lg mb-3 text-accent">Block Endings</h3>
          <p className="text-sm text-muted-foreground mb-4">
            Always use <code className="text-accent">end</code> to close blocks.
          </p>
          <pre className="bg-muted p-4 rounded-md overflow-x-auto">
            <code className="font-mono text-sm">
              {`def process(data: String) -> Bool
  if @len(data) > 0
    @println(data)
    return true
  end
  false
end

while count < 10
  count = count + 1
end`}
            </code>
          </pre>
        </Card>

        <Card className="p-6 bg-card border-border">
          <h3 className="font-semibold text-lg mb-3 text-accent">String Interpolation</h3>
          <p className="text-sm text-muted-foreground mb-4">
            Use backticks for interpolated strings.
          </p>
          <pre className="bg-muted p-4 rounded-md overflow-x-auto">
            <code className="font-mono text-sm">
              {`# Good
var message = \`Hello, \${name}!\`

# Also good for simple cases
var greeting = "Hello, World!"`}
            </code>
          </pre>
        </Card>

        <Card className="p-6 bg-card border-border">
          <h3 className="font-semibold text-lg mb-3 text-accent">Documentation Comments</h3>
          <p className="text-sm text-muted-foreground mb-4">
            Use <code className="text-accent">##</code> for documentation comments.
          </p>
          <pre className="bg-muted p-4 rounded-md overflow-x-auto">
            <code className="font-mono text-sm">
              {`## Calculate the factorial of n.
##
## Examples:
##   factorial(5)  # => 120
pub def factorial(n: Int) -> Int
  if n <= 1
    return 1
  end
  n * factorial(n - 1)
end`}
            </code>
          </pre>
        </Card>
      </div>

      {/* Rust Code Style */}
      <div className="space-y-6">
        <h2 className="text-3xl font-bold">Rust Code Style (Compiler)</h2>

        <Card className="p-6 bg-card border-border">
          <h3 className="font-semibold text-lg mb-3 text-accent">Formatting</h3>
          <p className="text-sm text-muted-foreground mb-4">
            Use <code className="text-accent">cargo fmt</code> to format all Rust code.
          </p>
          <pre className="bg-muted p-4 rounded-md overflow-x-auto">
            <code className="font-mono text-sm">cargo fmt --all</code>
          </pre>
        </Card>

        <Card className="p-6 bg-card border-border">
          <h3 className="font-semibold text-lg mb-3 text-accent">Import Ordering</h3>
          <p className="text-sm text-muted-foreground mb-4">
            Group imports with blank lines between sections.
          </p>
          <pre className="bg-muted p-4 rounded-md overflow-x-auto">
            <code className="font-mono text-sm">
              {`// Standard library
use std::collections::HashMap;
use std::fs;

// External crates
use anyhow::Result;
use serde::Deserialize;

// Internal modules
use crate::ast::Node;
use crate::parser::Parser;`}
            </code>
          </pre>
        </Card>

        <Card className="p-6 bg-card border-border">
          <h3 className="font-semibold text-lg mb-3 text-accent">Error Handling</h3>
          <p className="text-sm text-muted-foreground mb-4">
            Use <code className="text-accent">anyhow::Result</code> for fallible functions and{" "}
            <code className="text-accent">thiserror</code> for custom errors.
          </p>
          <pre className="bg-muted p-4 rounded-md overflow-x-auto">
            <code className="font-mono text-sm">
              {`use anyhow::{Result, bail};

fn parse_file(path: &str) -> Result<Ast> {
    let content = fs::read_to_string(path)?;

    if content.is_empty() {
        bail!("File is empty");
    }

    // ...
}`}
            </code>
          </pre>
        </Card>
      </div>

      {/* Formatting Tools */}
      <div className="space-y-6">
        <h2 className="text-3xl font-bold">Formatting Tools</h2>

        <Card className="p-6 bg-card border-border">
          <pre className="bg-muted p-4 rounded-md overflow-x-auto">
            <code className="font-mono text-sm">
              {`# Format everything
make fmt

# Format Rust code only
cargo fmt --all

# Format Tea code only
cargo run -p tea-cli -- fmt .

# Check formatting without changing files
cargo fmt --all -- --check`}
            </code>
          </pre>
        </Card>
      </div>

      {/* Best Practices */}
      <div className="space-y-6">
        <h2 className="text-3xl font-bold">Best Practices</h2>

        <Card className="p-6 bg-card border-border">
          <ul className="space-y-3 text-sm">
            <li className="flex items-start gap-3">
              <span className="text-accent">•</span>
              <span className="text-muted-foreground">
                Keep functions short and focused on a single task
              </span>
            </li>
            <li className="flex items-start gap-3">
              <span className="text-accent">•</span>
              <span className="text-muted-foreground">
                Use meaningful variable names that describe their purpose
              </span>
            </li>
            <li className="flex items-start gap-3">
              <span className="text-accent">•</span>
              <span className="text-muted-foreground">
                Add comments for complex logic, but let clear code speak for itself
              </span>
            </li>
            <li className="flex items-start gap-3">
              <span className="text-accent">•</span>
              <span className="text-muted-foreground">
                Prefer <code className="text-accent">const</code> over <code className="text-accent">var</code> when possible
              </span>
            </li>
            <li className="flex items-start gap-3">
              <span className="text-accent">•</span>
              <span className="text-muted-foreground">
                Run <code className="text-accent">make fmt</code> before committing
              </span>
            </li>
          </ul>
        </Card>
      </div>

      {/* Next Steps */}
      <div className="space-y-6">
        <h2 className="text-3xl font-bold">Next Steps</h2>

        <div className="flex flex-col gap-4">
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

          <Link
            href="/docs/contributing"
            className="flex items-center justify-between p-4 rounded-lg border border-border bg-card hover:bg-muted/50 transition-colors group panel-inset"
          >
            <div>
              <h3 className="font-semibold mb-1 group-hover:text-accent transition-colors">Contributing Guide</h3>
              <p className="text-sm text-muted-foreground">Submit your first pull request</p>
            </div>
            <ArrowRight className="h-5 w-5 text-muted-foreground group-hover:text-accent transition-colors" />
          </Link>
        </div>
      </div>
    </div>
  )
}
