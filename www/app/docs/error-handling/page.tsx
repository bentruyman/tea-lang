import { Card } from "@/components/ui/card"
import Link from "next/link"
import { ArrowRight, AlertTriangle } from "lucide-react"

export default function ErrorHandlingPage() {
  return (
    <div className="space-y-12">
      {/* Header */}
      <div className="space-y-4">
        <h1 className="text-4xl font-bold text-balance">Error Handling</h1>
        <p className="text-xl text-muted-foreground text-pretty leading-relaxed">
          Tea provides a robust error handling system with typed errors, explicit error declarations,
          and pattern matching for handling different error cases.
        </p>
      </div>

      {/* Defining Errors */}
      <div className="space-y-6">
        <h2 className="text-3xl font-bold">Defining Error Types</h2>
        <p className="text-muted-foreground">
          Use the <code className="text-accent">error</code> keyword to define custom error types with variants.
        </p>

        <Card className="p-6 bg-card border-border">
          <pre className="bg-muted p-4 rounded-md overflow-x-auto">
            <code className="font-mono text-sm">
              {`error TeamError {
  NotFound(name: String)
  InvalidScore(score: Int)
}

error FileError {
  NotFound(path: String)
  PermissionDenied(path: String, user: String)
  IOError(message: String)
}`}
            </code>
          </pre>
        </Card>
      </div>

      {/* Throwing Errors */}
      <div className="space-y-6">
        <h2 className="text-3xl font-bold">Throwing Errors</h2>
        <p className="text-muted-foreground">
          Functions that can fail must declare their error types in their signature using{" "}
          <code className="text-accent">!</code>. Use <code className="text-accent">throw</code> to raise an error.
        </p>

        <Card className="p-6 bg-card border-border">
          <h3 className="font-semibold text-lg mb-3 text-accent">Function Signatures</h3>
          <pre className="bg-muted p-4 rounded-md overflow-x-auto">
            <code className="font-mono text-sm">
              {`# Declare that this function can throw TeamError.NotFound
def find_team(name: String) -> Team ! TeamError.NotFound
  var idx = 0
  while idx < @len(teams)
    var candidate = teams[idx]
    if candidate.name == name
      return candidate
    end
    idx = idx + 1
  end

  # Throw the error when team isn't found
  throw TeamError.NotFound(name)
end`}
            </code>
          </pre>
        </Card>

        <Card className="p-6 bg-card border-border">
          <h3 className="font-semibold text-lg mb-3 text-accent">Multiple Error Types</h3>
          <pre className="bg-muted p-4 rounded-md overflow-x-auto">
            <code className="font-mono text-sm">
              {`# A function can throw multiple error types
def process_file(path: String) -> String ! FileError.NotFound, FileError.IOError
  if !file_exists(path)
    throw FileError.NotFound(path)
  end

  var content = read_file(path)
  if content == ""
    throw FileError.IOError("File is empty")
  end

  content
end`}
            </code>
          </pre>
        </Card>
      </div>

      {/* Catching Errors */}
      <div className="space-y-6">
        <h2 className="text-3xl font-bold">Catching Errors</h2>
        <p className="text-muted-foreground">
          Use <code className="text-accent">catch</code> to handle errors with pattern matching.
        </p>

        <Card className="p-6 bg-card border-border">
          <h3 className="font-semibold text-lg mb-3 text-accent">Basic Catch</h3>
          <pre className="bg-muted p-4 rounded-md overflow-x-auto">
            <code className="font-mono text-sm">
              {`def announce_favorite(name: String) -> String
  var team = find_team(name) catch err
    case is TeamError.NotFound
      return \`Sorry, \${err.name} is not on the schedule\`
    case _
      return "Unexpected error occurred"
  end

  \`Fan favorite: \${team.name}!\`
end`}
            </code>
          </pre>
        </Card>

        <Card className="p-6 bg-card border-border">
          <h3 className="font-semibold text-lg mb-3 text-accent">Multiple Cases</h3>
          <pre className="bg-muted p-4 rounded-md overflow-x-auto">
            <code className="font-mono text-sm">
              {`def load_config(path: String) -> String
  var content = read_config(path) catch err
    case is FileError.NotFound
      return \`Config not found: \${err.path}\`
    case is FileError.PermissionDenied
      return \`Access denied to \${err.path}\`
    case is FileError.IOError
      return \`Read error: \${err.message}\`
    case _
      return "Unknown error"
  end

  content
end`}
            </code>
          </pre>
        </Card>
      </div>

      {/* Error Data */}
      <div className="space-y-6">
        <h2 className="text-3xl font-bold">Accessing Error Data</h2>
        <p className="text-muted-foreground">
          When catching an error, access its fields through the captured variable.
        </p>

        <Card className="p-6 bg-card border-border">
          <pre className="bg-muted p-4 rounded-md overflow-x-auto">
            <code className="font-mono text-sm">
              {`error ValidationError {
  InvalidEmail(email: String, reason: String)
  InvalidAge(age: Int, min: Int, max: Int)
}

def validate_user(email: String, age: Int) -> Bool ! ValidationError
  if !contains(email, "@")
    throw ValidationError.InvalidEmail(email, "missing @")
  end

  if age < 18 || age > 120
    throw ValidationError.InvalidAge(age, 18, 120)
  end

  true
end

def try_validate(email: String, age: Int) -> String
  var valid = validate_user(email, age) catch err
    case is ValidationError.InvalidEmail
      return \`Bad email "\${err.email}": \${err.reason}\`
    case is ValidationError.InvalidAge
      return \`Age \${err.age} must be between \${err.min}-\${err.max}\`
    case _
      return "Validation failed"
  end

  "User is valid"
end`}
            </code>
          </pre>
        </Card>
      </div>

      {/* Panic */}
      <div className="space-y-6">
        <h2 className="text-3xl font-bold">Panic</h2>
        <p className="text-muted-foreground">
          For unrecoverable errors, use <code className="text-accent">@panic</code> to immediately halt execution.
        </p>

        <Card className="p-6 bg-card border-border">
          <div className="flex items-start gap-3 mb-4 p-3 bg-muted/50 rounded-md">
            <AlertTriangle className="h-5 w-5 text-accent shrink-0 mt-0.5" />
            <p className="text-sm text-muted-foreground">
              Panic should only be used for programming errors or impossible states, not for expected failures.
            </p>
          </div>
          <pre className="bg-muted p-4 rounded-md overflow-x-auto">
            <code className="font-mono text-sm">
              {`def divide(a: Int, b: Int) -> Int
  if b == 0
    @panic("division by zero")
  end
  a / b
end

# Using panic for assertions
use assert = "std.assert"

def test_add() -> Void
  assert.eq(1 + 1, 2)  # Panics if not equal
  assert.ok(true)      # Panics if false
end`}
            </code>
          </pre>
        </Card>
      </div>

      {/* Complete Example */}
      <div className="space-y-6">
        <h2 className="text-3xl font-bold">Complete Example</h2>

        <Card className="p-6 bg-card border-border">
          <pre className="bg-muted p-4 rounded-md overflow-x-auto">
            <code className="font-mono text-sm">
              {`struct Team {
  name: String
  wins: Int
  losses: Int
}

error TeamError {
  NotFound(name: String)
}

const format_team = |team: Team| => \`\${team.name}: \${team.wins} wins / \${team.losses} losses\`

const teams = [
  Team(name: "Ada", wins: 7, losses: 3),
  Team(name: "Grace", wins: 9, losses: 1),
  Team(name: "Linus", wins: 5, losses: 5)
]

def find_team(name: String) -> Team ! TeamError.NotFound
  var idx = 0
  while idx < @len(teams)
    var candidate = teams[idx]
    if candidate.name == name
      return candidate
    end
    idx = idx + 1
  end
  throw TeamError.NotFound(name)
end

## Announce the favorite team
def announce_favorite(name: String) -> String
  var team = find_team(name) catch err
    case is TeamError.NotFound
      return \`Sorry, \${err.name} is not on the schedule\`
    case _
      return "Unexpected lookup issue"
  end

  \`Fan favorite: \${format_team(team)}\`
end

@println(announce_favorite("Grace"))    # Found!
@println(announce_favorite("Unknown"))  # Not found`}
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
              <p className="text-sm text-muted-foreground">Learn more about case expressions</p>
            </div>
            <ArrowRight className="h-5 w-5 text-muted-foreground group-hover:text-accent transition-colors" />
          </Link>

          <Link
            href="/docs/modules"
            className="flex items-center justify-between p-4 rounded-lg border border-border bg-card hover:bg-muted/50 transition-colors group panel-inset"
          >
            <div>
              <h3 className="font-semibold mb-1 group-hover:text-accent transition-colors">Modules & Imports</h3>
              <p className="text-sm text-muted-foreground">Organize code and share error types</p>
            </div>
            <ArrowRight className="h-5 w-5 text-muted-foreground group-hover:text-accent transition-colors" />
          </Link>
        </div>
      </div>
    </div>
  )
}
