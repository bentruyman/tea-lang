import { Card } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import Link from "next/link"
import { ArrowLeft } from "lucide-react"

export default function CollectionsPage() {
  return (
    <div className="min-h-screen bg-background">
      {/* Header */}
      <header className="border-b border-border bg-background/95 backdrop-blur supports-[backdrop-filter]:bg-background/60 sticky top-0 z-50">
        <div className="container mx-auto px-4 h-16 flex items-center justify-between">
          <div className="flex items-center gap-8">
            <Link href="/" className="flex items-center gap-2">
              <div className="h-8 w-8 rounded-md bg-accent flex items-center justify-center">
                <span className="font-bold text-accent-foreground">T</span>
              </div>
              <span className="font-semibold text-xl text-foreground">Tea</span>
            </Link>
            <nav className="hidden md:flex items-center gap-6">
              <Link href="/docs" className="text-sm text-muted-foreground hover:text-foreground transition-colors">
                Docs
              </Link>
              <Link href="/examples" className="text-sm text-muted-foreground hover:text-foreground transition-colors">
                Examples
              </Link>
              <Link href="/reference" className="text-sm text-foreground font-medium">
                Reference
              </Link>
              <Link href="/playground" className="text-sm text-muted-foreground hover:text-foreground transition-colors">
                Playground
              </Link>
            </nav>
          </div>
          <Button variant="ghost" size="sm" asChild>
            <Link href="https://github.com/special-tea/tea" target="_blank">
              GitHub
            </Link>
          </Button>
        </div>
      </header>

      {/* Main Content */}
      <main className="container mx-auto px-4 py-12">
        <div className="max-w-5xl mx-auto space-y-12">
          {/* Back Button */}
          <Button variant="ghost" size="sm" className="gap-2" asChild>
            <Link href="/reference">
              <ArrowLeft className="h-4 w-4" />
              Back to Reference
            </Link>
          </Button>

          {/* Header */}
          <div className="space-y-4">
            <h1 className="text-4xl font-bold text-balance">Collections</h1>
            <p className="text-xl text-muted-foreground text-pretty leading-relaxed">
              Tea provides built-in collection types for organizing and manipulating groups of values.
              Learn about lists and dictionaries.
            </p>
          </div>

          {/* Lists */}
          <div className="space-y-6">
            <h2 className="text-3xl font-bold">Lists</h2>
            <p className="text-muted-foreground">
              Lists are ordered, indexable collections of elements with the same type.
            </p>

            <Card className="p-6 bg-card border-border">
              <h3 className="font-semibold text-lg mb-3 text-accent">Creating Lists</h3>
              <pre className="bg-muted p-4 rounded-md overflow-x-auto">
                <code className="font-mono text-sm">
                  {`# List literals
var numbers = [1, 2, 3, 4, 5]      # List[Int]
var words = ["hello", "world"]     # List[String]
var empty: List[Int] = []          # Empty list with type annotation

# Nested lists
var matrix = [
  [1, 2, 3],
  [4, 5, 6],
  [7, 8, 9]
]`}
                </code>
              </pre>
            </Card>

            <Card className="p-6 bg-card border-border">
              <h3 className="font-semibold text-lg mb-3 text-accent">Accessing Elements</h3>
              <pre className="bg-muted p-4 rounded-md overflow-x-auto">
                <code className="font-mono text-sm">
                  {`var numbers = [10, 20, 30, 40, 50]

# Index access (0-based)
var first = numbers[0]    # 10
var third = numbers[2]    # 30
var last = numbers[4]     # 50

# Nested access
var matrix = [[1, 2], [3, 4]]
var value = matrix[1][0]  # 3`}
                </code>
              </pre>
            </Card>

            <Card className="p-6 bg-card border-border">
              <h3 className="font-semibold text-lg mb-3 text-accent">List Slicing</h3>
              <pre className="bg-muted p-4 rounded-md overflow-x-auto">
                <code className="font-mono text-sm">
                  {`var numbers = [1, 2, 3, 4, 5]

# Exclusive end (0..3 gives indices 0, 1, 2)
var first_three = numbers[0..3]    # [1, 2, 3]

# Inclusive end (0...3 gives indices 0, 1, 2, 3)
var first_four = numbers[0...3]    # [1, 2, 3, 4]

# From middle
var middle = numbers[1..4]         # [2, 3, 4]`}
                </code>
              </pre>
            </Card>

            <Card className="p-6 bg-card border-border">
              <h3 className="font-semibold text-lg mb-3 text-accent">List Length</h3>
              <pre className="bg-muted p-4 rounded-md overflow-x-auto">
                <code className="font-mono text-sm">
                  {`var numbers = [1, 2, 3, 4, 5]
var len = @len(numbers)  # 5

# Iterating with length
var i = 0
while i < @len(numbers)
  @println(numbers[i])
  i = i + 1
end`}
                </code>
              </pre>
            </Card>
          </div>

          {/* Dictionaries */}
          <div className="space-y-6">
            <h2 className="text-3xl font-bold">Dictionaries</h2>
            <p className="text-muted-foreground">
              Dictionaries are key-value mappings. Tea supports both object-style and string-keyed dictionaries.
            </p>

            <Card className="p-6 bg-card border-border">
              <h3 className="font-semibold text-lg mb-3 text-accent">Object-Style Dictionaries</h3>
              <pre className="bg-muted p-4 rounded-md overflow-x-auto">
                <code className="font-mono text-sm">
                  {`# Object-style with identifier keys
var point = { x: 10, y: 20 }

# Access with dot notation
@println(point.x)  # 10
@println(point.y)  # 20

# Nested objects
var config = {
  server: { host: "localhost", port: 8080 },
  debug: true
}
@println(config.server.host)  # "localhost"`}
                </code>
              </pre>
            </Card>

            <Card className="p-6 bg-card border-border">
              <h3 className="font-semibold text-lg mb-3 text-accent">String-Keyed Dictionaries</h3>
              <pre className="bg-muted p-4 rounded-md overflow-x-auto">
                <code className="font-mono text-sm">
                  {`# String keys for dynamic keys
var scores = { "alice": 100, "bob": 85, "charlie": 92 }

# Access with dot notation
@println(scores.alice)    # 100
@println(scores.bob)      # 85`}
                </code>
              </pre>
            </Card>
          </div>

          {/* Common Patterns */}
          <div className="space-y-6">
            <h2 className="text-3xl font-bold">Common Patterns</h2>

            <Card className="p-6 bg-card border-border">
              <h3 className="font-semibold text-lg mb-3 text-accent">Iterating Over Lists</h3>
              <pre className="bg-muted p-4 rounded-md overflow-x-auto">
                <code className="font-mono text-sm">
                  {`var items = ["apple", "banana", "cherry"]

var i = 0
while i < @len(items)
  @println(\`Item \${i}: \${items[i]}\`)
  i = i + 1
end`}
                </code>
              </pre>
            </Card>

            <Card className="p-6 bg-card border-border">
              <h3 className="font-semibold text-lg mb-3 text-accent">Lists of Structs</h3>
              <pre className="bg-muted p-4 rounded-md overflow-x-auto">
                <code className="font-mono text-sm">
                  {`struct User {
  name: String
  age: Int
}

var users = [
  User(name: "Alice", age: 30),
  User(name: "Bob", age: 25),
  User(name: "Charlie", age: 35)
]

# Access struct fields
@println(users[0].name)  # "Alice"
@println(users[1].age)   # 25`}
                </code>
              </pre>
            </Card>
          </div>

          {/* Navigation */}
          <div className="flex items-center justify-between pt-8 border-t border-border">
            <Button variant="outline" asChild>
              <Link href="/reference/stdlib">← Standard Library</Link>
            </Button>
            <Button variant="outline" asChild>
              <Link href="/reference/filesystem">File System →</Link>
            </Button>
          </div>
        </div>
      </main>
    </div>
  )
}
