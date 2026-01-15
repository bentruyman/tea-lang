import { Card } from "@/components/ui/card"
import Link from "next/link"
import { ArrowRight } from "lucide-react"

export default function GenericsPage() {
  return (
    <div className="space-y-12">
      {/* Header */}
      <div className="space-y-4">
        <h1 className="text-4xl font-bold text-balance">Generics</h1>
        <p className="text-xl text-muted-foreground text-pretty leading-relaxed">
          Generics allow you to write reusable, type-safe code that works with any type. Tea uses monomorphization
          to generate specialized copies at compile time for optimal performance.
        </p>
      </div>

      {/* Generic Functions */}
      <div className="space-y-6">
        <h2 className="text-3xl font-bold">Generic Functions</h2>
        <p className="text-muted-foreground">
          Define type parameters using square brackets after the function name.
        </p>

        <Card className="p-6 bg-card border-border">
          <h3 className="font-semibold text-lg mb-3 text-accent">Identity Function</h3>
          <pre className="bg-muted p-4 rounded-md overflow-x-auto">
            <code className="font-mono text-sm">
              {`def identity[T](value: T) -> T
  value
end

# Call with explicit type
var n = identity[Int](42)
var s = identity[String]("hello")

@println(n)  # 42
@println(s)  # "hello"`}
            </code>
          </pre>
        </Card>

        <Card className="p-6 bg-card border-border">
          <h3 className="font-semibold text-lg mb-3 text-accent">Multiple Type Parameters</h3>
          <pre className="bg-muted p-4 rounded-md overflow-x-auto">
            <code className="font-mono text-sm">
              {`def pair[A, B](first: A, second: B) -> List[A]
  # Return a list containing just the first element
  [first]
end

def swap[A, B](a: A, b: B) -> B
  b
end`}
            </code>
          </pre>
        </Card>
      </div>

      {/* Generic Structs */}
      <div className="space-y-6">
        <h2 className="text-3xl font-bold">Generic Structs</h2>
        <p className="text-muted-foreground">
          Structs can also be parameterized by types.
        </p>

        <Card className="p-6 bg-card border-border">
          <h3 className="font-semibold text-lg mb-3 text-accent">Box Type</h3>
          <pre className="bg-muted p-4 rounded-md overflow-x-auto">
            <code className="font-mono text-sm">
              {`struct Box[T] {
  value: T
}

# Create instances with specific types
var int_box = Box[Int](value: 42)
var str_box = Box[String](value: "hello")

@println(int_box.value)  # 42
@println(str_box.value)  # "hello"`}
            </code>
          </pre>
        </Card>

        <Card className="p-6 bg-card border-border">
          <h3 className="font-semibold text-lg mb-3 text-accent">Multiple Type Parameters</h3>
          <pre className="bg-muted p-4 rounded-md overflow-x-auto">
            <code className="font-mono text-sm">
              {`struct Pair[A, B] {
  first: A
  second: B
}

var pair = Pair[String, Int](first: "age", second: 30)
@println(pair.first)   # "age"
@println(pair.second)  # 30`}
            </code>
          </pre>
        </Card>
      </div>

      {/* Factory Functions */}
      <div className="space-y-6">
        <h2 className="text-3xl font-bold">Factory Functions</h2>
        <p className="text-muted-foreground">
          Combine generic functions with generic structs to create factory patterns.
        </p>

        <Card className="p-6 bg-card border-border">
          <pre className="bg-muted p-4 rounded-md overflow-x-auto">
            <code className="font-mono text-sm">
              {`struct Box[T] {
  value: T
}

def make_box[T](value: T) -> Box[T]
  Box[T](value: value)
end

# Use the factory function
var int_box = make_box[Int](42)
var str_box = make_box[String]("tea")

@println(int_box.value)  # 42
@println(str_box.value)  # "tea"`}
            </code>
          </pre>
        </Card>
      </div>

      {/* Composing Generics */}
      <div className="space-y-6">
        <h2 className="text-3xl font-bold">Composing Generics</h2>
        <p className="text-muted-foreground">
          Generic functions can call other generic functions.
        </p>

        <Card className="p-6 bg-card border-border">
          <pre className="bg-muted p-4 rounded-md overflow-x-auto">
            <code className="font-mono text-sm">
              {`use assert = "std.assert"

struct Box[T] {
  value: T
}

def identity[T](value: T) -> T
  value
end

def make_box[T](value: T) -> Box[T]
  Box[T](value: value)
end

# Compose: identity wrapped in make_box
var int_box = make_box[Int](identity[Int](42))
assert.eq(int_box.value, 42)

var string_box = make_box[String]("tea")
assert.eq(string_box.value, "tea")`}
            </code>
          </pre>
        </Card>
      </div>

      {/* Monomorphization */}
      <div className="space-y-6">
        <h2 className="text-3xl font-bold">How It Works: Monomorphization</h2>
        <p className="text-muted-foreground">
          Tea uses monomorphization to implement generics. This means the compiler generates specialized
          versions of generic code for each concrete type used. This approach provides:
        </p>

        <Card className="p-6 bg-card border-border">
          <ul className="space-y-3 text-sm">
            <li className="flex items-start gap-3">
              <span className="text-accent font-bold">1.</span>
              <div>
                <strong className="text-foreground">Zero runtime overhead</strong>
                <span className="text-muted-foreground"> - No boxing or vtables needed</span>
              </div>
            </li>
            <li className="flex items-start gap-3">
              <span className="text-accent font-bold">2.</span>
              <div>
                <strong className="text-foreground">Full type safety</strong>
                <span className="text-muted-foreground"> - Errors caught at compile time</span>
              </div>
            </li>
            <li className="flex items-start gap-3">
              <span className="text-accent font-bold">3.</span>
              <div>
                <strong className="text-foreground">Optimal performance</strong>
                <span className="text-muted-foreground"> - Each specialization is optimized for its type</span>
              </div>
            </li>
          </ul>
        </Card>
      </div>

      {/* Practical Example */}
      <div className="space-y-6">
        <h2 className="text-3xl font-bold">Practical Example</h2>
        <p className="text-muted-foreground">
          Here's a more complete example showing generics in action.
        </p>

        <Card className="p-6 bg-card border-border">
          <pre className="bg-muted p-4 rounded-md overflow-x-auto">
            <code className="font-mono text-sm">
              {`struct Container[T] {
  items: List[T]
  name: String
}

def create_container[T](name: String, items: List[T]) -> Container[T]
  Container[T](items: items, name: name)
end

def container_size[T](c: Container[T]) -> Int
  @len(c.items)
end

# Usage
var numbers = create_container[Int]("Numbers", [1, 2, 3, 4, 5])
var words = create_container[String]("Words", ["hello", "world"])

@println(\`\${numbers.name}: \${container_size[Int](numbers)} items\`)
@println(\`\${words.name}: \${container_size[String](words)} items\`)`}
            </code>
          </pre>
        </Card>
      </div>

      {/* Next Steps */}
      <div className="space-y-6">
        <h2 className="text-3xl font-bold">Next Steps</h2>

        <div className="flex flex-col gap-4">
          <Link
            href="/docs/pattern-matching"
            className="flex items-center justify-between p-4 rounded-lg border border-border bg-card hover:bg-muted/50 transition-colors group panel-inset"
          >
            <div>
              <h3 className="font-semibold mb-1 group-hover:text-accent transition-colors">Pattern Matching</h3>
              <p className="text-sm text-muted-foreground">Match and destructure complex data</p>
            </div>
            <ArrowRight className="h-5 w-5 text-muted-foreground group-hover:text-accent transition-colors" />
          </Link>

          <Link
            href="/docs/error-handling"
            className="flex items-center justify-between p-4 rounded-lg border border-border bg-card hover:bg-muted/50 transition-colors group panel-inset"
          >
            <div>
              <h3 className="font-semibold mb-1 group-hover:text-accent transition-colors">Error Handling</h3>
              <p className="text-sm text-muted-foreground">Handle errors with typed exceptions</p>
            </div>
            <ArrowRight className="h-5 w-5 text-muted-foreground group-hover:text-accent transition-colors" />
          </Link>

          <Link
            href="/examples"
            className="flex items-center justify-between p-4 rounded-lg border border-border bg-card hover:bg-muted/50 transition-colors group panel-inset"
          >
            <div>
              <h3 className="font-semibold mb-1 group-hover:text-accent transition-colors">Examples</h3>
              <p className="text-sm text-muted-foreground">See generics in real-world code</p>
            </div>
            <ArrowRight className="h-5 w-5 text-muted-foreground group-hover:text-accent transition-colors" />
          </Link>
        </div>
      </div>
    </div>
  )
}
