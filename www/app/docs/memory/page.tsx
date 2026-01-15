import { Card } from "@/components/ui/card"
import Link from "next/link"
import { ArrowRight } from "lucide-react"

export default function MemoryPage() {
  return (
    <div className="space-y-12">
      {/* Header */}
      <div className="space-y-4">
        <h1 className="text-4xl font-bold text-balance">Memory Management</h1>
        <p className="text-xl text-muted-foreground text-pretty leading-relaxed">
          Tea handles memory management automatically, letting you focus on writing code without
          worrying about allocations and deallocations.
        </p>
      </div>

      {/* Automatic Memory Management */}
      <div className="space-y-6">
        <h2 className="text-3xl font-bold">Automatic Memory Management</h2>
        <p className="text-muted-foreground">
          Tea automatically manages memory for all values. You don't need to manually allocate or
          free memory—the runtime handles it for you.
        </p>

        <Card className="p-6 bg-card border-border">
          <pre className="bg-muted p-4 rounded-md overflow-x-auto">
            <code className="font-mono text-sm">
              {`# Memory is automatically allocated
var users = [
  User(name: "Alice", age: 30),
  User(name: "Bob", age: 25)
]

# Memory is automatically freed when no longer needed
def process_data() -> String
  var temp = "temporary string"
  # temp is automatically cleaned up when function returns
  return "result"
end`}
            </code>
          </pre>
        </Card>
      </div>

      {/* Value Semantics */}
      <div className="space-y-6">
        <h2 className="text-3xl font-bold">Value Semantics</h2>
        <p className="text-muted-foreground">
          Tea uses value semantics for most types. When you assign a value to a new variable or pass
          it to a function, a copy is made.
        </p>

        <Card className="p-6 bg-card border-border">
          <h3 className="font-semibold text-lg mb-3 text-accent">Copying Values</h3>
          <pre className="bg-muted p-4 rounded-md overflow-x-auto">
            <code className="font-mono text-sm">
              {`struct Point {
  x: Int
  y: Int
}

var p1 = Point(x: 10, y: 20)
var p2 = p1  # p2 is a copy of p1

# Modifying p1 doesn't affect p2
# (assuming we could modify structs)`}
            </code>
          </pre>
        </Card>

        <Card className="p-6 bg-card border-border">
          <h3 className="font-semibold text-lg mb-3 text-accent">Benefits of Value Semantics</h3>
          <ul className="space-y-3 text-sm">
            <li className="flex items-start gap-3">
              <span className="text-accent">•</span>
              <span className="text-muted-foreground">
                <strong className="text-foreground">Predictable behavior</strong> - No unexpected mutations through shared references
              </span>
            </li>
            <li className="flex items-start gap-3">
              <span className="text-accent">•</span>
              <span className="text-muted-foreground">
                <strong className="text-foreground">Thread safety</strong> - Copies don't share state
              </span>
            </li>
            <li className="flex items-start gap-3">
              <span className="text-accent">•</span>
              <span className="text-muted-foreground">
                <strong className="text-foreground">Easier reasoning</strong> - Function arguments can't be modified by the function
              </span>
            </li>
          </ul>
        </Card>
      </div>

      {/* Stack vs Heap */}
      <div className="space-y-6">
        <h2 className="text-3xl font-bold">Stack and Heap Allocation</h2>
        <p className="text-muted-foreground">
          Tea's compiler automatically determines the most efficient allocation strategy.
        </p>

        <Card className="p-6 bg-card border-border">
          <div className="space-y-4">
            <div>
              <h3 className="font-semibold text-accent mb-2">Stack Allocation</h3>
              <p className="text-sm text-muted-foreground">
                Small, fixed-size values (integers, booleans, small structs) are typically allocated
                on the stack for fast access and automatic cleanup.
              </p>
            </div>

            <div>
              <h3 className="font-semibold text-accent mb-2">Heap Allocation</h3>
              <p className="text-sm text-muted-foreground">
                Dynamic-size values (strings, lists, larger structs) are allocated on the heap.
                The runtime manages their lifecycle.
              </p>
            </div>
          </div>

          <pre className="bg-muted p-4 rounded-md overflow-x-auto mt-4">
            <code className="font-mono text-sm">
              {`# Stack allocated (typically)
var count = 42
var flag = true

# Heap allocated (typically)
var name = "Hello, World!"
var numbers = [1, 2, 3, 4, 5]`}
            </code>
          </pre>
        </Card>
      </div>

      {/* String Handling */}
      <div className="space-y-6">
        <h2 className="text-3xl font-bold">String Handling</h2>
        <p className="text-muted-foreground">
          Tea strings are UTF-8 encoded and immutable. String operations create new strings
          rather than modifying existing ones.
        </p>

        <Card className="p-6 bg-card border-border">
          <pre className="bg-muted p-4 rounded-md overflow-x-auto">
            <code className="font-mono text-sm">
              {`var greeting = "Hello"
var message = greeting + ", World!"
# greeting is unchanged, message is a new string

# String interpolation creates new strings
var name = "Tea"
var welcome = \`Welcome to \${name}!\`
# A new string is allocated for welcome`}
            </code>
          </pre>
        </Card>
      </div>

      {/* Collections */}
      <div className="space-y-6">
        <h2 className="text-3xl font-bold">Collection Memory</h2>
        <p className="text-muted-foreground">
          Lists and dictionaries grow dynamically as needed. The runtime handles resizing.
        </p>

        <Card className="p-6 bg-card border-border">
          <pre className="bg-muted p-4 rounded-md overflow-x-auto">
            <code className="font-mono text-sm">
              {`# Lists grow as needed
var items = [1, 2, 3]
# Memory is managed automatically as the list grows

# Nested collections
var matrix = [
  [1, 2, 3],
  [4, 5, 6],
  [7, 8, 9]
]
# Each inner list is separately managed`}
            </code>
          </pre>
        </Card>
      </div>

      {/* LLVM Optimization */}
      <div className="space-y-6">
        <h2 className="text-3xl font-bold">LLVM Optimizations</h2>
        <p className="text-muted-foreground">
          Tea compiles to native code via LLVM, which applies various memory optimizations.
        </p>

        <Card className="p-6 bg-card border-border">
          <ul className="space-y-3 text-sm">
            <li className="flex items-start gap-3">
              <span className="text-accent">•</span>
              <span className="text-muted-foreground">
                <strong className="text-foreground">Escape analysis</strong> - Values that don't escape can stay on the stack
              </span>
            </li>
            <li className="flex items-start gap-3">
              <span className="text-accent">•</span>
              <span className="text-muted-foreground">
                <strong className="text-foreground">Copy elision</strong> - Unnecessary copies are eliminated
              </span>
            </li>
            <li className="flex items-start gap-3">
              <span className="text-accent">•</span>
              <span className="text-muted-foreground">
                <strong className="text-foreground">Inlining</strong> - Small functions are inlined to reduce overhead
              </span>
            </li>
            <li className="flex items-start gap-3">
              <span className="text-accent">•</span>
              <span className="text-muted-foreground">
                <strong className="text-foreground">Dead code elimination</strong> - Unused allocations are removed
              </span>
            </li>
          </ul>
        </Card>
      </div>

      {/* Best Practices */}
      <div className="space-y-6">
        <h2 className="text-3xl font-bold">Best Practices</h2>

        <Card className="p-6 bg-card border-border">
          <ul className="space-y-3 text-sm">
            <li className="flex items-start gap-3">
              <span className="text-accent">1.</span>
              <span className="text-muted-foreground">
                <strong className="text-foreground">Prefer const</strong> - Immutable values are easier to optimize
              </span>
            </li>
            <li className="flex items-start gap-3">
              <span className="text-accent">2.</span>
              <span className="text-muted-foreground">
                <strong className="text-foreground">Avoid excessive string concatenation</strong> - Build strings efficiently with interpolation
              </span>
            </li>
            <li className="flex items-start gap-3">
              <span className="text-accent">3.</span>
              <span className="text-muted-foreground">
                <strong className="text-foreground">Keep scopes small</strong> - Variables are cleaned up when they go out of scope
              </span>
            </li>
            <li className="flex items-start gap-3">
              <span className="text-accent">4.</span>
              <span className="text-muted-foreground">
                <strong className="text-foreground">Use native compilation for performance</strong> - <code className="text-accent">tea build</code> produces optimized binaries
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
            href="/docs/metaprogramming"
            className="flex items-center justify-between p-4 rounded-lg border border-border bg-card hover:bg-muted/50 transition-colors group panel-inset"
          >
            <div>
              <h3 className="font-semibold mb-1 group-hover:text-accent transition-colors">Metaprogramming</h3>
              <p className="text-sm text-muted-foreground">Compile-time code generation</p>
            </div>
            <ArrowRight className="h-5 w-5 text-muted-foreground group-hover:text-accent transition-colors" />
          </Link>

          <Link
            href="/docs/concurrency"
            className="flex items-center justify-between p-4 rounded-lg border border-border bg-card hover:bg-muted/50 transition-colors group panel-inset"
          >
            <div>
              <h3 className="font-semibold mb-1 group-hover:text-accent transition-colors">Concurrency</h3>
              <p className="text-sm text-muted-foreground">Memory safety in concurrent code</p>
            </div>
            <ArrowRight className="h-5 w-5 text-muted-foreground group-hover:text-accent transition-colors" />
          </Link>
        </div>
      </div>
    </div>
  )
}
