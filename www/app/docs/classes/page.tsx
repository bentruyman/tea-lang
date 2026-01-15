import { Card } from "@/components/ui/card"
import Link from "next/link"
import { ArrowRight } from "lucide-react"

export default function ClassesPage() {
  return (
    <div className="space-y-12">
      {/* Header */}
      <div className="space-y-4">
        <h1 className="text-4xl font-bold text-balance">Classes & Objects</h1>
        <p className="text-xl text-muted-foreground text-pretty leading-relaxed">
          Tea uses structs to define custom data types. Structs provide a way to group related data together
          with named fields and strong typing.
        </p>
      </div>

      {/* Defining Structs */}
      <div className="space-y-6">
        <h2 className="text-3xl font-bold">Defining Structs</h2>
        <p className="text-muted-foreground">
          Use the <code className="text-accent">struct</code> keyword to define a new type with named fields.
        </p>

        <Card className="p-6 bg-card border-border">
          <h3 className="font-semibold text-lg mb-3 text-accent">Basic Struct</h3>
          <pre className="bg-muted p-4 rounded-md overflow-x-auto">
            <code className="font-mono text-sm">
              {`struct Point {
  x: Int
  y: Int
}

var origin = Point(x: 0, y: 0)
@println(origin.x)  # 0
@println(origin.y)  # 0`}
            </code>
          </pre>
        </Card>

        <Card className="p-6 bg-card border-border">
          <h3 className="font-semibold text-lg mb-3 text-accent">With Documentation</h3>
          <p className="text-sm text-muted-foreground mb-4">
            Use <code className="text-accent">##</code> comments to document structs and fields.
          </p>
          <pre className="bg-muted p-4 rounded-md overflow-x-auto">
            <code className="font-mono text-sm">
              {`## A user account with profile information
struct User {
  ## The user's display name
  name: String
  ## How many years old the user is
  age: Int
}

var alice = User(name: "Alice", age: 30)`}
            </code>
          </pre>
        </Card>
      </div>

      {/* Creating Instances */}
      <div className="space-y-6">
        <h2 className="text-3xl font-bold">Creating Instances</h2>
        <p className="text-muted-foreground">
          Create struct instances by calling the struct name like a function with named arguments.
        </p>

        <Card className="p-6 bg-card border-border">
          <pre className="bg-muted p-4 rounded-md overflow-x-auto">
            <code className="font-mono text-sm">
              {`struct Team {
  name: String
  wins: Int
  losses: Int
}

# Create with named arguments
var team = Team(name: "Tigers", wins: 10, losses: 5)

# Access fields
@println(team.name)    # "Tigers"
@println(team.wins)    # 10
@println(team.losses)  # 5`}
            </code>
          </pre>
        </Card>
      </div>

      {/* Nested Structs */}
      <div className="space-y-6">
        <h2 className="text-3xl font-bold">Nested Structs</h2>
        <p className="text-muted-foreground">
          Struct fields can be other structs, enabling complex data modeling.
        </p>

        <Card className="p-6 bg-card border-border">
          <pre className="bg-muted p-4 rounded-md overflow-x-auto">
            <code className="font-mono text-sm">
              {`struct Address {
  street: String
  city: String
}

struct Person {
  name: String
  address: Address
}

var alice = Person(
  name: "Alice",
  address: Address(
    street: "123 Main St",
    city: "Springfield"
  )
)

@println(alice.address.city)  # "Springfield"`}
            </code>
          </pre>
        </Card>
      </div>

      {/* Functions with Structs */}
      <div className="space-y-6">
        <h2 className="text-3xl font-bold">Functions with Structs</h2>
        <p className="text-muted-foreground">
          Pass structs to functions and return them as values.
        </p>

        <Card className="p-6 bg-card border-border">
          <pre className="bg-muted p-4 rounded-md overflow-x-auto">
            <code className="font-mono text-sm">
              {`struct Team {
  name: String
  wins: Int
  losses: Int
}

def format_team(team: Team) -> String
  \`\${team.name}: \${team.wins} wins / \${team.losses} losses\`
end

def win_rate(team: Team) -> Float
  var total = team.wins + team.losses
  if total == 0
    return 0.0
  end
  team.wins / total
end

var tigers = Team(name: "Tigers", wins: 8, losses: 2)
@println(format_team(tigers))  # "Tigers: 8 wins / 2 losses"`}
            </code>
          </pre>
        </Card>
      </div>

      {/* Collections of Structs */}
      <div className="space-y-6">
        <h2 className="text-3xl font-bold">Collections of Structs</h2>
        <p className="text-muted-foreground">
          Store structs in lists and iterate over them.
        </p>

        <Card className="p-6 bg-card border-border">
          <pre className="bg-muted p-4 rounded-md overflow-x-auto">
            <code className="font-mono text-sm">
              {`struct Team {
  name: String
  wins: Int
  losses: Int
}

const teams = [
  Team(name: "Ada", wins: 7, losses: 3),
  Team(name: "Grace", wins: 9, losses: 1),
  Team(name: "Linus", wins: 5, losses: 5)
]

# Find the best team
var champion = teams[0]
var i = 1

while i < @len(teams)
  var contender = teams[i]
  if contender.wins > champion.wins
    champion = contender
  end
  i = i + 1
end

@println(\`Top team: \${champion.name}\`)  # "Top team: Grace"`}
            </code>
          </pre>
        </Card>
      </div>

      {/* Generic Structs */}
      <div className="space-y-6">
        <h2 className="text-3xl font-bold">Generic Structs</h2>
        <p className="text-muted-foreground">
          Structs can be generic, allowing them to work with any type.
        </p>

        <Card className="p-6 bg-card border-border">
          <pre className="bg-muted p-4 rounded-md overflow-x-auto">
            <code className="font-mono text-sm">
              {`struct Box[T] {
  value: T
}

var int_box = Box[Int](value: 42)
var str_box = Box[String](value: "hello")

@println(int_box.value)  # 42
@println(str_box.value)  # "hello"`}
            </code>
          </pre>
        </Card>
      </div>

      {/* Visibility */}
      <div className="space-y-6">
        <h2 className="text-3xl font-bold">Visibility</h2>
        <p className="text-muted-foreground">
          By default, structs are private to their module. Use <code className="text-accent">pub</code> to export them.
        </p>

        <Card className="p-6 bg-card border-border">
          <pre className="bg-muted p-4 rounded-md overflow-x-auto">
            <code className="font-mono text-sm">
              {`# Private struct (only accessible in this module)
struct InternalData {
  value: Int
}

# Public struct (can be imported by other modules)
pub struct Config {
  debug: Bool
  max_retries: Int
}`}
            </code>
          </pre>
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
              <p className="text-sm text-muted-foreground">Write generic structs and functions</p>
            </div>
            <ArrowRight className="h-5 w-5 text-muted-foreground group-hover:text-accent transition-colors" />
          </Link>

          <Link
            href="/docs/pattern-matching"
            className="flex items-center justify-between p-4 rounded-lg border border-border bg-card hover:bg-muted/50 transition-colors group panel-inset"
          >
            <div>
              <h3 className="font-semibold mb-1 group-hover:text-accent transition-colors">Pattern Matching</h3>
              <p className="text-sm text-muted-foreground">Match and destructure data</p>
            </div>
            <ArrowRight className="h-5 w-5 text-muted-foreground group-hover:text-accent transition-colors" />
          </Link>

          <Link
            href="/docs/error-handling"
            className="flex items-center justify-between p-4 rounded-lg border border-border bg-card hover:bg-muted/50 transition-colors group panel-inset"
          >
            <div>
              <h3 className="font-semibold mb-1 group-hover:text-accent transition-colors">Error Handling</h3>
              <p className="text-sm text-muted-foreground">Define error types with structs</p>
            </div>
            <ArrowRight className="h-5 w-5 text-muted-foreground group-hover:text-accent transition-colors" />
          </Link>
        </div>
      </div>
    </div>
  )
}
