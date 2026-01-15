import { Card } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import Link from "next/link"
import { ArrowRight } from "lucide-react"

export default function SyntaxPage() {
  return (
    <div className="space-y-12">
      {/* Header */}
      <div className="space-y-4">
        <h1 className="text-4xl font-bold text-balance">Syntax Basics</h1>
        <p className="text-xl text-muted-foreground text-pretty leading-relaxed">
          Learn the fundamentals of Tea's syntax. Tea uses a clean, readable syntax with Python-like indentation
          awareness and Ruby-inspired block endings.
        </p>
      </div>

      {/* Variables */}
      <div className="space-y-6">
        <h2 className="text-3xl font-bold">Variables</h2>
        <p className="text-muted-foreground">
          Tea has two kinds of bindings: mutable variables with <code className="text-accent">var</code> and
          immutable constants with <code className="text-accent">const</code>.
        </p>

        <Card className="p-6 bg-card border-border">
          <h3 className="font-semibold text-lg mb-3 text-accent">Mutable Variables</h3>
          <p className="text-sm text-muted-foreground mb-4">
            Use <code className="text-accent">var</code> for values that can be reassigned.
          </p>
          <pre className="bg-muted p-4 rounded-md overflow-x-auto">
            <code className="font-mono text-sm">
              {`var count = 0
count = count + 1

var name = "Tea"
name = "Tea Language"

var numbers = [1, 2, 3]
numbers = [4, 5, 6]`}
            </code>
          </pre>
        </Card>

        <Card className="p-6 bg-card border-border">
          <h3 className="font-semibold text-lg mb-3 text-accent">Constants</h3>
          <p className="text-sm text-muted-foreground mb-4">
            Use <code className="text-accent">const</code> for values that should never change.
          </p>
          <pre className="bg-muted p-4 rounded-md overflow-x-auto">
            <code className="font-mono text-sm">
              {`const PI = 3.14159
const APP_NAME = "MyApp"
const MAX_RETRIES = 3

# Constants can also hold functions
const double = |x: Int| => x * 2`}
            </code>
          </pre>
        </Card>

        <Card className="p-6 bg-card border-border">
          <h3 className="font-semibold text-lg mb-3 text-accent">Type Annotations</h3>
          <p className="text-sm text-muted-foreground mb-4">
            Types are inferred automatically, but you can add explicit annotations.
          </p>
          <pre className="bg-muted p-4 rounded-md overflow-x-auto">
            <code className="font-mono text-sm">
              {`# Type inference
var count = 42          # Inferred as Int
var name = "Tea"        # Inferred as String

# Explicit types
var count: Int = 42
var name: String = "Tea"

# Optional types
var maybe_name: String? = nil`}
            </code>
          </pre>
        </Card>
      </div>

      {/* Basic Types */}
      <div className="space-y-6">
        <h2 className="text-3xl font-bold">Basic Types</h2>

        <Card className="p-6 bg-card border-border">
          <div className="grid md:grid-cols-2 gap-6">
            <div>
              <h3 className="font-semibold text-accent mb-3">Primitive Types</h3>
              <pre className="bg-muted p-3 rounded-md overflow-x-auto">
                <code className="font-mono text-sm">
                  {`var n: Int = 42
var f: Float = 3.14
var b: Bool = true
var s: String = "hello"`}
                </code>
              </pre>
            </div>
            <div>
              <h3 className="font-semibold text-accent mb-3">Collection Types</h3>
              <pre className="bg-muted p-3 rounded-md overflow-x-auto">
                <code className="font-mono text-sm">
                  {`var list: List[Int] = [1, 2, 3]
var dict = { x: 1, y: 2 }
var scores = { "a": 10, "b": 20 }`}
                </code>
              </pre>
            </div>
          </div>
        </Card>
      </div>

      {/* Comments */}
      <div className="space-y-6">
        <h2 className="text-3xl font-bold">Comments</h2>

        <Card className="p-6 bg-card border-border">
          <pre className="bg-muted p-4 rounded-md overflow-x-auto">
            <code className="font-mono text-sm">
              {`# Single line comment

## Doc comment for functions, structs, and fields
## These can be used for documentation generation

var x = 42  # Inline comment`}
            </code>
          </pre>
        </Card>
      </div>

      {/* Control Flow */}
      <div className="space-y-6">
        <h2 className="text-3xl font-bold">Control Flow</h2>

        <Card className="p-6 bg-card border-border">
          <h3 className="font-semibold text-lg mb-3 text-accent">If Statements</h3>
          <p className="text-sm text-muted-foreground mb-4">
            Conditionals use <code className="text-accent">if</code>, <code className="text-accent">else</code>, and <code className="text-accent">end</code>.
          </p>
          <pre className="bg-muted p-4 rounded-md overflow-x-auto">
            <code className="font-mono text-sm">
              {`var score = 85

if score >= 90
  @println("Grade: A")
else
  @println("Grade: B or lower")
end

# Nested conditions
if score >= 90
  @println("A")
else
  if score >= 80
    @println("B")
  else
    @println("C or lower")
  end
end`}
            </code>
          </pre>
        </Card>

        <Card className="p-6 bg-card border-border">
          <h3 className="font-semibold text-lg mb-3 text-accent">If Expressions</h3>
          <p className="text-sm text-muted-foreground mb-4">
            Use <code className="text-accent">if()</code> for ternary-style expressions that return values.
          </p>
          <pre className="bg-muted p-4 rounded-md overflow-x-auto">
            <code className="font-mono text-sm">
              {`const is_admin = true
const role = if(is_admin) "admin" else "user"

var x = 10
var y = if(x > 5) x * 2 else x

# With function calls
const result = if(true) double(5) else double(10)`}
            </code>
          </pre>
        </Card>

        <Card className="p-6 bg-card border-border">
          <h3 className="font-semibold text-lg mb-3 text-accent">While Loops</h3>
          <pre className="bg-muted p-4 rounded-md overflow-x-auto">
            <code className="font-mono text-sm">
              {`var count = 0

while count < 5
  @println(count)
  count = count + 1
end`}
            </code>
          </pre>
        </Card>
      </div>

      {/* Operators */}
      <div className="space-y-6">
        <h2 className="text-3xl font-bold">Operators</h2>

        <Card className="p-6 bg-card border-border">
          <div className="grid md:grid-cols-2 gap-6">
            <div>
              <h3 className="font-semibold text-accent mb-3">Arithmetic</h3>
              <pre className="bg-muted p-3 rounded-md overflow-x-auto">
                <code className="font-mono text-sm">
                  {`var sum = 1 + 2
var diff = 5 - 3
var product = 4 * 2
var quotient = 10 / 2
var remainder = 7 % 3`}
                </code>
              </pre>
            </div>
            <div>
              <h3 className="font-semibold text-accent mb-3">Comparison</h3>
              <pre className="bg-muted p-3 rounded-md overflow-x-auto">
                <code className="font-mono text-sm">
                  {`var eq = 1 == 1    # true
var ne = 1 != 2    # true
var lt = 1 < 2     # true
var gt = 2 > 1     # true
var le = 1 <= 1    # true
var ge = 2 >= 2    # true`}
                </code>
              </pre>
            </div>
            <div>
              <h3 className="font-semibold text-accent mb-3">Logical</h3>
              <pre className="bg-muted p-3 rounded-md overflow-x-auto">
                <code className="font-mono text-sm">
                  {`var and_result = true && false  # false
var or_result = true || false   # true
var not_result = !true          # false`}
                </code>
              </pre>
            </div>
            <div>
              <h3 className="font-semibold text-accent mb-3">Assignment</h3>
              <pre className="bg-muted p-3 rounded-md overflow-x-auto">
                <code className="font-mono text-sm">
                  {`var x = 10
x += 5   # x = 15
x -= 3   # x = 12
x *= 2   # x = 24`}
                </code>
              </pre>
            </div>
          </div>
        </Card>
      </div>

      {/* Strings */}
      <div className="space-y-6">
        <h2 className="text-3xl font-bold">Strings</h2>

        <Card className="p-6 bg-card border-border">
          <h3 className="font-semibold text-lg mb-3 text-accent">String Literals</h3>
          <pre className="bg-muted p-4 rounded-md overflow-x-auto">
            <code className="font-mono text-sm">
              {`var simple = "Hello, World!"
var with_escape = "Line 1\\nLine 2"
var with_quotes = "She said \\"Hello\\""`}
            </code>
          </pre>
        </Card>

        <Card className="p-6 bg-card border-border">
          <h3 className="font-semibold text-lg mb-3 text-accent">String Interpolation</h3>
          <p className="text-sm text-muted-foreground mb-4">
            Use backticks for interpolated strings with <code className="text-accent">{"${...}"}</code> expressions.
          </p>
          <pre className="bg-muted p-4 rounded-md overflow-x-auto">
            <code className="font-mono text-sm">
              {`var name = "Tea"
var greeting = \`Hello, \${name}!\`

var x = 5
var result = \`Next number: \${x + 1}\`

# Function calls in interpolation
def double(n: Int) -> Int
  n * 2
end

@println(\`Doubled: \${double(21)}\`)  # "Doubled: 42"`}
            </code>
          </pre>
        </Card>

        <Card className="p-6 bg-card border-border">
          <h3 className="font-semibold text-lg mb-3 text-accent">String Operations</h3>
          <pre className="bg-muted p-4 rounded-md overflow-x-auto">
            <code className="font-mono text-sm">
              {`var s = "Hello"

# Length
var len = @len(s)          # 5

# Indexing (0-based)
var first = s[0]           # "H"
var last = s[4]            # "o"

# Slicing
var sub = s[0..3]          # "Hel" (exclusive end)
var sub2 = s[1...4]        # "ell" (inclusive end)

# Concatenation
var full = s + " World"    # "Hello World"`}
            </code>
          </pre>
        </Card>
      </div>

      {/* Printing */}
      <div className="space-y-6">
        <h2 className="text-3xl font-bold">Printing Output</h2>

        <Card className="p-6 bg-card border-border">
          <p className="text-sm text-muted-foreground mb-4">
            Use the <code className="text-accent">@println</code> intrinsic to print values.
          </p>
          <pre className="bg-muted p-4 rounded-md overflow-x-auto">
            <code className="font-mono text-sm">
              {`@println("Hello, World!")
@println(42)
@println(true)

var name = "Tea"
@println(\`Welcome to \${name}!\`)`}
            </code>
          </pre>
        </Card>
      </div>

      {/* Next Steps */}
      <div className="space-y-6">
        <h2 className="text-3xl font-bold">Next Steps</h2>

        <div className="flex flex-col gap-4">
          <Link
            href="/docs/types"
            className="flex items-center justify-between p-4 rounded-lg border border-border bg-card hover:bg-muted/50 transition-colors group panel-inset"
          >
            <div>
              <h3 className="font-semibold mb-1 group-hover:text-accent transition-colors">Type System</h3>
              <p className="text-sm text-muted-foreground">Learn about Tea's static type system and inference</p>
            </div>
            <ArrowRight className="h-5 w-5 text-muted-foreground group-hover:text-accent transition-colors" />
          </Link>

          <Link
            href="/docs/functions"
            className="flex items-center justify-between p-4 rounded-lg border border-border bg-card hover:bg-muted/50 transition-colors group panel-inset"
          >
            <div>
              <h3 className="font-semibold mb-1 group-hover:text-accent transition-colors">Functions</h3>
              <p className="text-sm text-muted-foreground">Define and use functions in Tea</p>
            </div>
            <ArrowRight className="h-5 w-5 text-muted-foreground group-hover:text-accent transition-colors" />
          </Link>
        </div>
      </div>
    </div>
  )
}
