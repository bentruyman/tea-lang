import { Card } from "@/components/ui/card"
import Link from "next/link"
import { ArrowRight, AlertCircle } from "lucide-react"

export default function ConcurrencyPage() {
  return (
    <div className="space-y-12">
      {/* Header */}
      <div className="space-y-4">
        <h1 className="text-4xl font-bold text-balance">Concurrency</h1>
        <p className="text-xl text-muted-foreground text-pretty leading-relaxed">
          Tea is designed with concurrency in mind, though the full concurrency model is still evolving.
          This page covers the current state and future directions.
        </p>
      </div>

      {/* Current State */}
      <div className="space-y-6">
        <h2 className="text-3xl font-bold">Current State</h2>

        <Card className="p-6 bg-card border-border">
          <div className="flex items-start gap-3 mb-4 p-3 bg-muted/50 rounded-md">
            <AlertCircle className="h-5 w-5 text-accent shrink-0 mt-0.5" />
            <p className="text-sm text-muted-foreground">
              Concurrency features are under active development. The current version of Tea is primarily
              single-threaded, but designed with future concurrency support in mind.
            </p>
          </div>

          <p className="text-muted-foreground">
            Tea programs currently execute sequentially. However, Tea's design choices—like immutable values,
            explicit state management, and typed errors—lay the groundwork for safe concurrent programming.
          </p>
        </Card>
      </div>

      {/* Sequential Execution */}
      <div className="space-y-6">
        <h2 className="text-3xl font-bold">Sequential Execution</h2>
        <p className="text-muted-foreground">
          Currently, Tea programs execute one statement at a time, in order.
        </p>

        <Card className="p-6 bg-card border-border">
          <pre className="bg-muted p-4 rounded-md overflow-x-auto">
            <code className="font-mono text-sm">
              {`use fs = "std.fs"

# These operations happen sequentially
var config = fs.read_file("config.json")
var data = fs.read_file("data.json")
var template = fs.read_file("template.html")

# Process after all reads complete
@println("All files loaded")`}
            </code>
          </pre>
        </Card>
      </div>

      {/* Design Principles */}
      <div className="space-y-6">
        <h2 className="text-3xl font-bold">Design Principles for Future Concurrency</h2>
        <p className="text-muted-foreground">
          Tea's language design incorporates principles that will enable safe concurrency:
        </p>

        <div className="grid gap-4">
          <Card className="p-6 bg-card border-border">
            <h3 className="font-semibold text-lg mb-2 text-accent">Immutable by Default</h3>
            <p className="text-sm text-muted-foreground mb-3">
              Constants (<code className="text-accent">const</code>) cannot be modified after creation, making them
              safe to share between concurrent tasks.
            </p>
            <pre className="bg-muted p-3 rounded-md overflow-x-auto">
              <code className="font-mono text-sm">
                {`const CONFIG = { max_connections: 100 }
# Safe to read from multiple tasks`}
              </code>
            </pre>
          </Card>

          <Card className="p-6 bg-card border-border">
            <h3 className="font-semibold text-lg mb-2 text-accent">Explicit Mutability</h3>
            <p className="text-sm text-muted-foreground mb-3">
              Mutable state is clearly marked with <code className="text-accent">var</code>, making it
              easy to identify shared state that needs synchronization.
            </p>
            <pre className="bg-muted p-3 rounded-md overflow-x-auto">
              <code className="font-mono text-sm">
                {`var counter = 0  # Mutable - needs care in concurrent contexts
counter = counter + 1`}
              </code>
            </pre>
          </Card>

          <Card className="p-6 bg-card border-border">
            <h3 className="font-semibold text-lg mb-2 text-accent">Typed Errors</h3>
            <p className="text-sm text-muted-foreground mb-3">
              Explicit error types make it clear what can fail, which is essential for handling
              failures in concurrent operations.
            </p>
            <pre className="bg-muted p-3 rounded-md overflow-x-auto">
              <code className="font-mono text-sm">
                {`def fetch_data(url: String) -> String ! NetworkError
  # Error type is part of the signature
end`}
              </code>
            </pre>
          </Card>

          <Card className="p-6 bg-card border-border">
            <h3 className="font-semibold text-lg mb-2 text-accent">Value Semantics</h3>
            <p className="text-sm text-muted-foreground mb-3">
              Structs are passed by value, avoiding shared mutable state problems common in
              reference-based languages.
            </p>
            <pre className="bg-muted p-3 rounded-md overflow-x-auto">
              <code className="font-mono text-sm">
                {`struct Task {
  id: Int
  name: String
}

# Each task is a separate copy
var task1 = Task(id: 1, name: "First")
var task2 = task1  # Independent copy`}
              </code>
            </pre>
          </Card>
        </div>
      </div>

      {/* Future Directions */}
      <div className="space-y-6">
        <h2 className="text-3xl font-bold">Future Directions</h2>
        <p className="text-muted-foreground">
          Planned concurrency features for Tea include:
        </p>

        <Card className="p-6 bg-card border-border">
          <ul className="space-y-4 text-sm">
            <li className="flex items-start gap-3">
              <span className="text-accent font-bold">1.</span>
              <div>
                <strong className="text-foreground">Async/Await</strong>
                <span className="text-muted-foreground"> - Non-blocking I/O for file operations, network requests</span>
              </div>
            </li>
            <li className="flex items-start gap-3">
              <span className="text-accent font-bold">2.</span>
              <div>
                <strong className="text-foreground">Lightweight Tasks</strong>
                <span className="text-muted-foreground"> - Green threads or coroutines for concurrent execution</span>
              </div>
            </li>
            <li className="flex items-start gap-3">
              <span className="text-accent font-bold">3.</span>
              <div>
                <strong className="text-foreground">Channels</strong>
                <span className="text-muted-foreground"> - Safe communication between concurrent tasks</span>
              </div>
            </li>
            <li className="flex items-start gap-3">
              <span className="text-accent font-bold">4.</span>
              <div>
                <strong className="text-foreground">Structured Concurrency</strong>
                <span className="text-muted-foreground"> - Ensuring child tasks complete before parents</span>
              </div>
            </li>
          </ul>
        </Card>
      </div>

      {/* Best Practices */}
      <div className="space-y-6">
        <h2 className="text-3xl font-bold">Best Practices for Now</h2>
        <p className="text-muted-foreground">
          Write code today that will work well with future concurrency features.
        </p>

        <Card className="p-6 bg-card border-border">
          <ul className="space-y-3 text-sm">
            <li className="flex items-start gap-3">
              <span className="text-accent">•</span>
              <span className="text-muted-foreground">
                Prefer <code className="text-accent">const</code> over <code className="text-accent">var</code> when possible
              </span>
            </li>
            <li className="flex items-start gap-3">
              <span className="text-accent">•</span>
              <span className="text-muted-foreground">
                Keep mutable state localized within functions
              </span>
            </li>
            <li className="flex items-start gap-3">
              <span className="text-accent">•</span>
              <span className="text-muted-foreground">
                Write pure functions that don't depend on global state
              </span>
            </li>
            <li className="flex items-start gap-3">
              <span className="text-accent">•</span>
              <span className="text-muted-foreground">
                Use explicit error types for operations that can fail
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
            href="/docs/memory"
            className="flex items-center justify-between p-4 rounded-lg border border-border bg-card hover:bg-muted/50 transition-colors group panel-inset"
          >
            <div>
              <h3 className="font-semibold mb-1 group-hover:text-accent transition-colors">Memory Management</h3>
              <p className="text-sm text-muted-foreground">Learn how Tea manages memory</p>
            </div>
            <ArrowRight className="h-5 w-5 text-muted-foreground group-hover:text-accent transition-colors" />
          </Link>

          <Link
            href="/docs/contributing"
            className="flex items-center justify-between p-4 rounded-lg border border-border bg-card hover:bg-muted/50 transition-colors group panel-inset"
          >
            <div>
              <h3 className="font-semibold mb-1 group-hover:text-accent transition-colors">Contributing</h3>
              <p className="text-sm text-muted-foreground">Help shape Tea's concurrency model</p>
            </div>
            <ArrowRight className="h-5 w-5 text-muted-foreground group-hover:text-accent transition-colors" />
          </Link>
        </div>
      </div>
    </div>
  )
}
