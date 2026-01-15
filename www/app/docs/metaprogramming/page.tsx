import { Card } from "@/components/ui/card"
import Link from "next/link"
import { ArrowRight, AlertCircle } from "lucide-react"

export default function MetaprogrammingPage() {
  return (
    <div className="space-y-12">
      {/* Header */}
      <div className="space-y-4">
        <h1 className="text-4xl font-bold text-balance">Metaprogramming</h1>
        <p className="text-xl text-muted-foreground text-pretty leading-relaxed">
          Metaprogramming in Tea allows you to write code that generates or manipulates code at compile time.
          This includes intrinsics and compile-time features.
        </p>
      </div>

      {/* Intrinsics */}
      <div className="space-y-6">
        <h2 className="text-3xl font-bold">Intrinsics</h2>
        <p className="text-muted-foreground">
          Intrinsics are built-in functions that start with <code className="text-accent">@</code>. They provide
          low-level capabilities that can't be implemented in pure Tea.
        </p>

        <Card className="p-6 bg-card border-border">
          <h3 className="font-semibold text-lg mb-3 text-accent">Common Intrinsics</h3>
          <div className="overflow-x-auto">
            <table className="w-full text-sm">
              <thead>
                <tr className="border-b border-border">
                  <th className="text-left py-2 pr-4 text-accent">Intrinsic</th>
                  <th className="text-left py-2">Description</th>
                </tr>
              </thead>
              <tbody className="font-mono">
                <tr className="border-b border-border/50">
                  <td className="py-2 pr-4 text-accent">@println(value)</td>
                  <td className="py-2 font-sans text-muted-foreground">Print a value with newline</td>
                </tr>
                <tr className="border-b border-border/50">
                  <td className="py-2 pr-4 text-accent">@len(collection)</td>
                  <td className="py-2 font-sans text-muted-foreground">Get length of string, list, or dict</td>
                </tr>
                <tr className="border-b border-border/50">
                  <td className="py-2 pr-4 text-accent">@panic(message)</td>
                  <td className="py-2 font-sans text-muted-foreground">Halt execution with error message</td>
                </tr>
                <tr>
                  <td className="py-2 pr-4 text-accent">@type_of(value)</td>
                  <td className="py-2 font-sans text-muted-foreground">Get the type of a value as a string</td>
                </tr>
              </tbody>
            </table>
          </div>
        </Card>

        <Card className="p-6 bg-card border-border">
          <h3 className="font-semibold text-lg mb-3 text-accent">Using Intrinsics</h3>
          <pre className="bg-muted p-4 rounded-md overflow-x-auto">
            <code className="font-mono text-sm">
              {`# Printing output
@println("Hello, World!")
@println(42)

# Getting length
var name = "Tea"
var len = @len(name)  # 3

var numbers = [1, 2, 3, 4, 5]
@println(@len(numbers))  # 5

# Panic for unrecoverable errors
def divide(a: Int, b: Int) -> Int
  if b == 0
    @panic("division by zero")
  end
  a / b
end`}
            </code>
          </pre>
        </Card>
      </div>

      {/* Generics as Metaprogramming */}
      <div className="space-y-6">
        <h2 className="text-3xl font-bold">Generics as Metaprogramming</h2>
        <p className="text-muted-foreground">
          Tea's generics use monomorphization, which is a form of compile-time code generation.
          The compiler generates specialized code for each concrete type.
        </p>

        <Card className="p-6 bg-card border-border">
          <pre className="bg-muted p-4 rounded-md overflow-x-auto">
            <code className="font-mono text-sm">
              {`def identity[T](value: T) -> T
  value
end

# At compile time, this generates:
# - identity_Int for identity[Int](42)
# - identity_String for identity[String]("hello")

var n = identity[Int](42)
var s = identity[String]("hello")`}
            </code>
          </pre>
        </Card>
      </div>

      {/* Doc Comments */}
      <div className="space-y-6">
        <h2 className="text-3xl font-bold">Documentation Comments</h2>
        <p className="text-muted-foreground">
          Use <code className="text-accent">##</code> comments to add documentation that can be processed by tools.
        </p>

        <Card className="p-6 bg-card border-border">
          <pre className="bg-muted p-4 rounded-md overflow-x-auto">
            <code className="font-mono text-sm">
              {`## A user account with profile information
##
## Examples:
##   var user = User(name: "Alice", age: 30)
##   @println(user.name)
struct User {
  ## The user's display name
  name: String
  ## How many years old the user is
  age: Int
}

## Calculate the factorial of a non-negative integer.
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

      {/* Compile-time Constants */}
      <div className="space-y-6">
        <h2 className="text-3xl font-bold">Compile-time Constants</h2>
        <p className="text-muted-foreground">
          Constants with <code className="text-accent">const</code> are evaluated at compile time when possible.
        </p>

        <Card className="p-6 bg-card border-border">
          <pre className="bg-muted p-4 rounded-md overflow-x-auto">
            <code className="font-mono text-sm">
              {`# These are compile-time constants
const PI = 3.14159
const MAX_SIZE = 1024
const APP_NAME = "MyApp"

# Constant expressions can include simple operations
const BUFFER_SIZE = MAX_SIZE * 2

# Constant lambdas
const double = |x: Int| => x * 2`}
            </code>
          </pre>
        </Card>
      </div>

      {/* Future Directions */}
      <div className="space-y-6">
        <h2 className="text-3xl font-bold">Future Directions</h2>

        <Card className="p-6 bg-card border-border">
          <div className="flex items-start gap-3 mb-4 p-3 bg-muted/50 rounded-md">
            <AlertCircle className="h-5 w-5 text-accent shrink-0 mt-0.5" />
            <p className="text-sm text-muted-foreground">
              Additional metaprogramming features are planned for future releases.
            </p>
          </div>

          <ul className="space-y-3 text-sm">
            <li className="flex items-start gap-3">
              <span className="text-accent">•</span>
              <span className="text-muted-foreground">
                <strong className="text-foreground">Compile-time function execution</strong> - Run functions during compilation
              </span>
            </li>
            <li className="flex items-start gap-3">
              <span className="text-accent">•</span>
              <span className="text-muted-foreground">
                <strong className="text-foreground">Macros</strong> - Code generation through syntax transformation
              </span>
            </li>
            <li className="flex items-start gap-3">
              <span className="text-accent">•</span>
              <span className="text-muted-foreground">
                <strong className="text-foreground">Reflection</strong> - Inspect types and values at runtime
              </span>
            </li>
            <li className="flex items-start gap-3">
              <span className="text-accent">•</span>
              <span className="text-muted-foreground">
                <strong className="text-foreground">Custom attributes</strong> - Annotate code with metadata
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
            href="/docs/generics"
            className="flex items-center justify-between p-4 rounded-lg border border-border bg-card hover:bg-muted/50 transition-colors group panel-inset"
          >
            <div>
              <h3 className="font-semibold mb-1 group-hover:text-accent transition-colors">Generics</h3>
              <p className="text-sm text-muted-foreground">Learn about monomorphization in depth</p>
            </div>
            <ArrowRight className="h-5 w-5 text-muted-foreground group-hover:text-accent transition-colors" />
          </Link>

          <Link
            href="/reference/stdlib"
            className="flex items-center justify-between p-4 rounded-lg border border-border bg-card hover:bg-muted/50 transition-colors group panel-inset"
          >
            <div>
              <h3 className="font-semibold mb-1 group-hover:text-accent transition-colors">Standard Library</h3>
              <p className="text-sm text-muted-foreground">See intrinsics in action</p>
            </div>
            <ArrowRight className="h-5 w-5 text-muted-foreground group-hover:text-accent transition-colors" />
          </Link>

          <Link
            href="/docs/contributing"
            className="flex items-center justify-between p-4 rounded-lg border border-border bg-card hover:bg-muted/50 transition-colors group panel-inset"
          >
            <div>
              <h3 className="font-semibold mb-1 group-hover:text-accent transition-colors">Contributing</h3>
              <p className="text-sm text-muted-foreground">Help shape future metaprogramming features</p>
            </div>
            <ArrowRight className="h-5 w-5 text-muted-foreground group-hover:text-accent transition-colors" />
          </Link>
        </div>
      </div>
    </div>
  )
}
