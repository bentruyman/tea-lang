import { Card } from "@/components/ui/card"
import Link from "next/link"
import { ArrowRight } from "lucide-react"

export default function FunctionsPage() {
  return (
    <div className="space-y-12">
      {/* Header */}
      <div className="space-y-4">
        <h1 className="text-4xl font-bold text-balance">Functions</h1>
        <p className="text-xl text-muted-foreground text-pretty leading-relaxed">
          Functions are the building blocks of Tea programs. Learn how to define, call, and compose functions
          with type-safe parameters and return values.
        </p>
      </div>

      {/* Defining Functions */}
      <div className="space-y-6">
        <h2 className="text-3xl font-bold">Defining Functions</h2>
        <p className="text-muted-foreground">
          Use the <code className="text-accent">def</code> keyword to define functions. Function parameters and
          return types are annotated.
        </p>

        <Card className="p-6 bg-card border-border">
          <h3 className="font-semibold text-lg mb-3 text-accent">Basic Syntax</h3>
          <pre className="bg-muted p-4 rounded-md overflow-x-auto">
            <code className="font-mono text-sm">
              {`def greet(name: String) -> String
  \`Hello, \${name}!\`
end

# Call the function
@println(greet("Tea"))  # "Hello, Tea!"`}
            </code>
          </pre>
        </Card>

        <Card className="p-6 bg-card border-border">
          <h3 className="font-semibold text-lg mb-3 text-accent">Multiple Parameters</h3>
          <pre className="bg-muted p-4 rounded-md overflow-x-auto">
            <code className="font-mono text-sm">
              {`def add(a: Int, b: Int) -> Int
  a + b
end

def describe(name: String, age: Int) -> String
  \`\${name} is \${age} years old\`
end

@println(add(2, 3))              # 5
@println(describe("Alice", 30))  # "Alice is 30 years old"`}
            </code>
          </pre>
        </Card>

        <Card className="p-6 bg-card border-border">
          <h3 className="font-semibold text-lg mb-3 text-accent">Void Return Type</h3>
          <p className="text-sm text-muted-foreground mb-4">
            Use <code className="text-accent">Void</code> for functions that don't return a value.
          </p>
          <pre className="bg-muted p-4 rounded-md overflow-x-auto">
            <code className="font-mono text-sm">
              {`def print_greeting(name: String) -> Void
  @println(\`Hello, \${name}!\`)
end

print_greeting("World")`}
            </code>
          </pre>
        </Card>
      </div>

      {/* Return Values */}
      <div className="space-y-6">
        <h2 className="text-3xl font-bold">Return Values</h2>
        <p className="text-muted-foreground">
          The last expression in a function body is automatically returned. You can also use explicit{" "}
          <code className="text-accent">return</code> statements.
        </p>

        <Card className="p-6 bg-card border-border">
          <h3 className="font-semibold text-lg mb-3 text-accent">Implicit Return</h3>
          <pre className="bg-muted p-4 rounded-md overflow-x-auto">
            <code className="font-mono text-sm">
              {`def square(x: Int) -> Int
  x * x  # Last expression is returned
end

def max(a: Int, b: Int) -> Int
  if(a > b) a else b  # If-expression is returned
end`}
            </code>
          </pre>
        </Card>

        <Card className="p-6 bg-card border-border">
          <h3 className="font-semibold text-lg mb-3 text-accent">Explicit Return</h3>
          <p className="text-sm text-muted-foreground mb-4">
            Use <code className="text-accent">return</code> for early exits or clarity.
          </p>
          <pre className="bg-muted p-4 rounded-md overflow-x-auto">
            <code className="font-mono text-sm">
              {`def absolute(n: Int) -> Int
  if n < 0
    return -n
  end
  n
end

def find_index(items: List[String], target: String) -> Int
  var i = 0
  while i < @len(items)
    if items[i] == target
      return i
    end
    i = i + 1
  end
  -1  # Not found
end`}
            </code>
          </pre>
        </Card>
      </div>

      {/* Lambdas */}
      <div className="space-y-6">
        <h2 className="text-3xl font-bold">Lambda Functions</h2>
        <p className="text-muted-foreground">
          Anonymous functions (lambdas) use the arrow syntax. They're useful for callbacks and functional programming.
        </p>

        <Card className="p-6 bg-card border-border">
          <h3 className="font-semibold text-lg mb-3 text-accent">Basic Lambdas</h3>
          <pre className="bg-muted p-4 rounded-md overflow-x-auto">
            <code className="font-mono text-sm">
              {`# Lambda syntax: |params| => expression
const double = |x: Int| => x * 2
const add = |a: Int, b: Int| => a + b

@println(double(21))  # 42
@println(add(2, 3))   # 5`}
            </code>
          </pre>
        </Card>

        <Card className="p-6 bg-card border-border">
          <h3 className="font-semibold text-lg mb-3 text-accent">Closures</h3>
          <p className="text-sm text-muted-foreground mb-4">
            Lambdas can capture variables from their surrounding scope.
          </p>
          <pre className="bg-muted p-4 rounded-md overflow-x-auto">
            <code className="font-mono text-sm">
              {`def make_adder(base: Int) -> Func(Int) -> Int
  |value: Int| => base + value  # Captures 'base'
end

var add_ten = make_adder(10)
var add_five = make_adder(5)

@println(add_ten(32))   # 42
@println(add_five(32))  # 37`}
            </code>
          </pre>
        </Card>

        <Card className="p-6 bg-card border-border">
          <h3 className="font-semibold text-lg mb-3 text-accent">Function Types</h3>
          <p className="text-sm text-muted-foreground mb-4">
            Functions have types written as <code className="text-accent">Func(params) -&gt; return</code>.
          </p>
          <pre className="bg-muted p-4 rounded-md overflow-x-auto">
            <code className="font-mono text-sm">
              {`# A function that takes an Int and returns an Int
const transform: Func(Int) -> Int = |x: Int| => x * 2

# A function that takes two Ints and returns a Bool
const compare: Func(Int, Int) -> Bool = |a: Int, b: Int| => a > b

# Higher-order function
def apply_twice(f: Func(Int) -> Int, x: Int) -> Int
  f(f(x))
end

@println(apply_twice(transform, 5))  # 20`}
            </code>
          </pre>
        </Card>
      </div>

      {/* Visibility */}
      <div className="space-y-6">
        <h2 className="text-3xl font-bold">Visibility</h2>
        <p className="text-muted-foreground">
          By default, functions are private to their module. Use <code className="text-accent">pub</code> to export them.
        </p>

        <Card className="p-6 bg-card border-border">
          <pre className="bg-muted p-4 rounded-md overflow-x-auto">
            <code className="font-mono text-sm">
              {`# Private function (only accessible in this module)
def helper(x: Int) -> Int
  x * 2
end

# Public function (can be imported by other modules)
pub def double(x: Int) -> Int
  helper(x)
end`}
            </code>
          </pre>
        </Card>
      </div>

      {/* Doc Comments */}
      <div className="space-y-6">
        <h2 className="text-3xl font-bold">Documentation Comments</h2>
        <p className="text-muted-foreground">
          Use <code className="text-accent">##</code> for documentation comments that describe functions.
        </p>

        <Card className="p-6 bg-card border-border">
          <pre className="bg-muted p-4 rounded-md overflow-x-auto">
            <code className="font-mono text-sm">
              {`## Calculate the factorial of a non-negative integer.
##
## Examples:
##   factorial(5)  # => 120
##   factorial(0)  # => 1
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

      {/* Recursion */}
      <div className="space-y-6">
        <h2 className="text-3xl font-bold">Recursion</h2>
        <p className="text-muted-foreground">
          Tea fully supports recursive functions.
        </p>

        <Card className="p-6 bg-card border-border">
          <pre className="bg-muted p-4 rounded-md overflow-x-auto">
            <code className="font-mono text-sm">
              {`def fibonacci(n: Int) -> Int
  if n <= 1
    return n
  end
  fibonacci(n - 1) + fibonacci(n - 2)
end

@println(fibonacci(10))  # 55`}
            </code>
          </pre>
        </Card>
      </div>

      {/* Next Steps */}
      <div className="space-y-6">
        <h2 className="text-3xl font-bold">Next Steps</h2>

        <div className="flex flex-col gap-4">
          <Link
            href="/docs/classes"
            className="flex items-center justify-between p-4 rounded-lg border border-border bg-card hover:bg-muted/50 transition-colors group panel-inset"
          >
            <div>
              <h3 className="font-semibold mb-1 group-hover:text-accent transition-colors">Classes & Objects</h3>
              <p className="text-sm text-muted-foreground">Define custom types with structs</p>
            </div>
            <ArrowRight className="h-5 w-5 text-muted-foreground group-hover:text-accent transition-colors" />
          </Link>

          <Link
            href="/docs/generics"
            className="flex items-center justify-between p-4 rounded-lg border border-border bg-card hover:bg-muted/50 transition-colors group panel-inset"
          >
            <div>
              <h3 className="font-semibold mb-1 group-hover:text-accent transition-colors">Generics</h3>
              <p className="text-sm text-muted-foreground">Write generic functions with type parameters</p>
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
        </div>
      </div>
    </div>
  )
}
