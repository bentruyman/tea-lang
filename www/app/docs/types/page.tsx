import { Card } from "@/components/ui/card"
import Link from "next/link"
import { ArrowRight } from "lucide-react"

export default function TypesPage() {
  return (
    <div className="space-y-12">
      {/* Header */}
      <div className="space-y-4">
        <h1 className="text-4xl font-bold text-balance">Type System</h1>
        <p className="text-xl text-muted-foreground text-pretty leading-relaxed">
          Tea features a powerful static type system with full type inference. Catch errors at compile time while
          writing concise, readable code.
        </p>
      </div>

      {/* Primitive Types */}
      <div className="space-y-6">
        <h2 className="text-3xl font-bold">Primitive Types</h2>

        <Card className="p-6 bg-card border-border">
          <div className="overflow-x-auto">
            <table className="w-full text-sm">
              <thead>
                <tr className="border-b border-border">
                  <th className="text-left py-2 pr-4 text-accent">Type</th>
                  <th className="text-left py-2 pr-4">Description</th>
                  <th className="text-left py-2">Example</th>
                </tr>
              </thead>
              <tbody className="font-mono">
                <tr className="border-b border-border/50">
                  <td className="py-2 pr-4 text-accent">Int</td>
                  <td className="py-2 pr-4 font-sans text-muted-foreground">64-bit signed integer</td>
                  <td className="py-2"><code>42</code>, <code>-10</code></td>
                </tr>
                <tr className="border-b border-border/50">
                  <td className="py-2 pr-4 text-accent">Float</td>
                  <td className="py-2 pr-4 font-sans text-muted-foreground">64-bit floating point</td>
                  <td className="py-2"><code>3.14</code>, <code>-0.5</code></td>
                </tr>
                <tr className="border-b border-border/50">
                  <td className="py-2 pr-4 text-accent">Bool</td>
                  <td className="py-2 pr-4 font-sans text-muted-foreground">Boolean value</td>
                  <td className="py-2"><code>true</code>, <code>false</code></td>
                </tr>
                <tr className="border-b border-border/50">
                  <td className="py-2 pr-4 text-accent">String</td>
                  <td className="py-2 pr-4 font-sans text-muted-foreground">UTF-8 string</td>
                  <td className="py-2"><code>"hello"</code></td>
                </tr>
                <tr>
                  <td className="py-2 pr-4 text-accent">Void</td>
                  <td className="py-2 pr-4 font-sans text-muted-foreground">No return value</td>
                  <td className="py-2">Function returns nothing</td>
                </tr>
              </tbody>
            </table>
          </div>
        </Card>
      </div>

      {/* Type Inference */}
      <div className="space-y-6">
        <h2 className="text-3xl font-bold">Type Inference</h2>
        <p className="text-muted-foreground">
          Tea automatically infers types from values, so you rarely need explicit annotations for local variables.
        </p>

        <Card className="p-6 bg-card border-border">
          <pre className="bg-muted p-4 rounded-md overflow-x-auto">
            <code className="font-mono text-sm">
              {`# Types are inferred automatically
var count = 42            # Int
var price = 19.99         # Float
var active = true         # Bool
var name = "Tea"          # String
var numbers = [1, 2, 3]   # List[Int]
var point = { x: 0, y: 0 }  # Dict

# Explicit annotations are optional
var count: Int = 42
var name: String = "Tea"`}
            </code>
          </pre>
        </Card>
      </div>

      {/* Collection Types */}
      <div className="space-y-6">
        <h2 className="text-3xl font-bold">Collection Types</h2>

        <Card className="p-6 bg-card border-border">
          <h3 className="font-semibold text-lg mb-3 text-accent">Lists</h3>
          <p className="text-sm text-muted-foreground mb-4">
            Ordered collections of elements with the same type.
          </p>
          <pre className="bg-muted p-4 rounded-md overflow-x-auto">
            <code className="font-mono text-sm">
              {`var numbers = [1, 2, 3]           # List[Int]
var words = ["hello", "world"]    # List[String]
var nested = [[1, 2], [3, 4]]     # List[List[Int]]

# Access by index
var first = numbers[0]            # 1
var second = words[1]             # "world"

# Nested access
var value = nested[0][1]          # 2

# Length
var len = @len(numbers)           # 3

# Slicing
var slice = numbers[0..2]         # [1, 2]`}
            </code>
          </pre>
        </Card>

        <Card className="p-6 bg-card border-border">
          <h3 className="font-semibold text-lg mb-3 text-accent">Dictionaries</h3>
          <p className="text-sm text-muted-foreground mb-4">
            Key-value mappings with flexible access patterns.
          </p>
          <pre className="bg-muted p-4 rounded-md overflow-x-auto">
            <code className="font-mono text-sm">
              {`# Object-style dict
var point = { x: 10, y: 20 }
@println(point.x)                 # 10
@println(point.y)                 # 20

# String-keyed dict
var scores = { "alice": 100, "bob": 85 }
@println(scores.alice)            # 100`}
            </code>
          </pre>
        </Card>
      </div>

      {/* Optional Types */}
      <div className="space-y-6">
        <h2 className="text-3xl font-bold">Optional Types</h2>
        <p className="text-muted-foreground">
          Optional types represent values that may or may not exist. Use <code className="text-accent">?</code> after
          a type to make it optional.
        </p>

        <Card className="p-6 bg-card border-border">
          <h3 className="font-semibold text-lg mb-3 text-accent">Declaring Optionals</h3>
          <pre className="bg-muted p-4 rounded-md overflow-x-auto">
            <code className="font-mono text-sm">
              {`var maybe_name: String? = nil
var maybe_count: Int? = nil

# Optionals can hold values
maybe_name = "Tea"
maybe_count = 42

# Or be nil
maybe_name = nil`}
            </code>
          </pre>
        </Card>

        <Card className="p-6 bg-card border-border">
          <h3 className="font-semibold text-lg mb-3 text-accent">Nil Coalescing</h3>
          <p className="text-sm text-muted-foreground mb-4">
            Use <code className="text-accent">??</code> to provide a default value when an optional is nil.
          </p>
          <pre className="bg-muted p-4 rounded-md overflow-x-auto">
            <code className="font-mono text-sm">
              {`var maybe_name: String? = nil
var name = maybe_name ?? "Anonymous"  # "Anonymous"

var maybe_count: Int? = 42
var count = maybe_count ?? 0          # 42`}
            </code>
          </pre>
        </Card>

        <Card className="p-6 bg-card border-border">
          <h3 className="font-semibold text-lg mb-3 text-accent">Force Unwrap</h3>
          <p className="text-sm text-muted-foreground mb-4">
            Use <code className="text-accent">!</code> to unwrap an optional when you know it has a value.
            This will panic if the value is nil.
          </p>
          <pre className="bg-muted p-4 rounded-md overflow-x-auto">
            <code className="font-mono text-sm">
              {`var maybe_name: String? = nil

if maybe_name == nil
  maybe_name = "Tea"
end

# Safe to unwrap after nil check
@println(maybe_name!)  # "Tea"`}
            </code>
          </pre>
        </Card>
      </div>

      {/* Struct Types */}
      <div className="space-y-6">
        <h2 className="text-3xl font-bold">Struct Types</h2>
        <p className="text-muted-foreground">
          Define custom types with named fields using structs.
        </p>

        <Card className="p-6 bg-card border-border">
          <pre className="bg-muted p-4 rounded-md overflow-x-auto">
            <code className="font-mono text-sm">
              {`## A user account
struct User {
  ## The user's display name
  name: String
  ## How many years old the user is
  age: Int
}

# Create instances with named arguments
var alice = User(name: "Alice", age: 30)

# Access fields with dot notation
@println(alice.name)   # "Alice"
@println(alice.age)    # 30`}
            </code>
          </pre>
        </Card>
      </div>

      {/* Function Types */}
      <div className="space-y-6">
        <h2 className="text-3xl font-bold">Function Types</h2>
        <p className="text-muted-foreground">
          Functions are first-class values in Tea with their own types.
        </p>

        <Card className="p-6 bg-card border-border">
          <pre className="bg-muted p-4 rounded-md overflow-x-auto">
            <code className="font-mono text-sm">
              {`# Function type: Func(parameters) -> return_type
def make_adder(base: Int) -> Func(Int) -> Int
  |value: Int| => base + value
end

var add_ten = make_adder(10)
@println(add_ten(5))   # 15

# Lambdas as values
const double = |x: Int| => x * 2
@println(double(21))   # 42`}
            </code>
          </pre>
        </Card>
      </div>

      {/* Generics Preview */}
      <div className="space-y-6">
        <h2 className="text-3xl font-bold">Generic Types</h2>
        <p className="text-muted-foreground">
          Tea supports generics for writing reusable, type-safe code. See the{" "}
          <Link href="/docs/generics" className="text-accent hover:underline">Generics</Link> page for details.
        </p>

        <Card className="p-6 bg-card border-border">
          <pre className="bg-muted p-4 rounded-md overflow-x-auto">
            <code className="font-mono text-sm">
              {`struct Box[T] {
  value: T
}

def identity[T](value: T) -> T
  value
end

var int_box = Box[Int](value: 42)
var str_box = Box[String](value: "hello")`}
            </code>
          </pre>
        </Card>
      </div>

      {/* Next Steps */}
      <div className="space-y-6">
        <h2 className="text-3xl font-bold">Next Steps</h2>

        <div className="flex flex-col gap-4">
          <Link
            href="/docs/functions"
            className="flex items-center justify-between p-4 rounded-lg border border-border bg-card hover:bg-muted/50 transition-colors group panel-inset"
          >
            <div>
              <h3 className="font-semibold mb-1 group-hover:text-accent transition-colors">Functions</h3>
              <p className="text-sm text-muted-foreground">Define functions with type annotations</p>
            </div>
            <ArrowRight className="h-5 w-5 text-muted-foreground group-hover:text-accent transition-colors" />
          </Link>

          <Link
            href="/docs/classes"
            className="flex items-center justify-between p-4 rounded-lg border border-border bg-card hover:bg-muted/50 transition-colors group panel-inset"
          >
            <div>
              <h3 className="font-semibold mb-1 group-hover:text-accent transition-colors">Classes & Objects</h3>
              <p className="text-sm text-muted-foreground">Learn about structs and custom types</p>
            </div>
            <ArrowRight className="h-5 w-5 text-muted-foreground group-hover:text-accent transition-colors" />
          </Link>

          <Link
            href="/docs/generics"
            className="flex items-center justify-between p-4 rounded-lg border border-border bg-card hover:bg-muted/50 transition-colors group panel-inset"
          >
            <div>
              <h3 className="font-semibold mb-1 group-hover:text-accent transition-colors">Generics</h3>
              <p className="text-sm text-muted-foreground">Write reusable, type-safe code</p>
            </div>
            <ArrowRight className="h-5 w-5 text-muted-foreground group-hover:text-accent transition-colors" />
          </Link>
        </div>
      </div>
    </div>
  )
}
