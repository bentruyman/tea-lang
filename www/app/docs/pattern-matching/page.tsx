import { Card } from "@/components/ui/card"
import Link from "next/link"
import { ArrowRight } from "lucide-react"

export default function PatternMatchingPage() {
  return (
    <div className="space-y-12">
      {/* Header */}
      <div className="space-y-4">
        <h1 className="text-4xl font-bold text-balance">Pattern Matching</h1>
        <p className="text-xl text-muted-foreground text-pretty leading-relaxed">
          Pattern matching in Tea allows you to handle different cases in error handling and conditional logic.
          Use the <code className="text-accent">case</code> keyword to match against different patterns.
        </p>
      </div>

      {/* Error Matching */}
      <div className="space-y-6">
        <h2 className="text-3xl font-bold">Error Pattern Matching</h2>
        <p className="text-muted-foreground">
          The primary use of pattern matching in Tea is with error handling. When you catch an error,
          you can match against specific error types.
        </p>

        <Card className="p-6 bg-card border-border">
          <h3 className="font-semibold text-lg mb-3 text-accent">Matching Error Types</h3>
          <pre className="bg-muted p-4 rounded-md overflow-x-auto">
            <code className="font-mono text-sm">
              {`error TeamError {
  NotFound(name: String)
  InvalidScore(score: Int)
}

def find_team(name: String) -> Team ! TeamError.NotFound
  # ... search logic ...
  throw TeamError.NotFound(name)
end

def handle_lookup(name: String) -> String
  var team = find_team(name) catch err
    case is TeamError.NotFound
      return \`Team "\${err.name}" not found\`
    case _
      return "Unknown error occurred"
  end

  \`Found team: \${team.name}\`
end`}
            </code>
          </pre>
        </Card>

        <Card className="p-6 bg-card border-border">
          <h3 className="font-semibold text-lg mb-3 text-accent">Case Syntax</h3>
          <p className="text-sm text-muted-foreground mb-4">
            Use <code className="text-accent">case is</code> to match specific error types, and{" "}
            <code className="text-accent">case _</code> as a catch-all for any other cases.
          </p>
          <pre className="bg-muted p-4 rounded-md overflow-x-auto">
            <code className="font-mono text-sm">
              {`var result = risky_operation() catch err
  case is NetworkError
    return "Network issue: check connection"
  case is ValidationError
    return \`Invalid input: \${err.message}\`
  case is TimeoutError
    return "Operation timed out, please retry"
  case _
    return "An unexpected error occurred"
end`}
            </code>
          </pre>
        </Card>
      </div>

      {/* Accessing Error Data */}
      <div className="space-y-6">
        <h2 className="text-3xl font-bold">Accessing Error Data</h2>
        <p className="text-muted-foreground">
          When matching an error, you can access the error's fields through the captured variable.
        </p>

        <Card className="p-6 bg-card border-border">
          <pre className="bg-muted p-4 rounded-md overflow-x-auto">
            <code className="font-mono text-sm">
              {`error FileError {
  NotFound(path: String)
  PermissionDenied(path: String, user: String)
  IOError(message: String)
}

def read_config(path: String) -> String ! FileError
  # ... file reading logic ...
  throw FileError.NotFound(path)
end

def load_config(path: String) -> String
  var content = read_config(path) catch err
    case is FileError.NotFound
      # Access the 'path' field from the error
      return \`Config file not found: \${err.path}\`
    case is FileError.PermissionDenied
      # Access multiple fields
      return \`User \${err.user} cannot read \${err.path}\`
    case is FileError.IOError
      return \`IO error: \${err.message}\`
    case _
      return "Unknown file error"
  end

  content
end`}
            </code>
          </pre>
        </Card>
      </div>

      {/* Conditional Logic */}
      <div className="space-y-6">
        <h2 className="text-3xl font-bold">Conditional Logic Patterns</h2>
        <p className="text-muted-foreground">
          For simple conditional logic, Tea uses if-expressions and if-statements rather than
          pattern matching syntax.
        </p>

        <Card className="p-6 bg-card border-border">
          <h3 className="font-semibold text-lg mb-3 text-accent">If Expressions</h3>
          <pre className="bg-muted p-4 rounded-md overflow-x-auto">
            <code className="font-mono text-sm">
              {`# Simple ternary-style conditional
const status = if(score >= 90) "A" else "B"

# Nested conditions
const grade = if(score >= 90) "A"
  else if(score >= 80) "B"
  else if(score >= 70) "C"
  else "F"`}
            </code>
          </pre>
        </Card>

        <Card className="p-6 bg-card border-border">
          <h3 className="font-semibold text-lg mb-3 text-accent">Matching Values</h3>
          <p className="text-sm text-muted-foreground mb-4">
            For matching against specific values, use chained if-else statements.
          </p>
          <pre className="bg-muted p-4 rounded-md overflow-x-auto">
            <code className="font-mono text-sm">
              {`def describe_day(day: Int) -> String
  if day == 1
    return "Monday"
  end
  if day == 2
    return "Tuesday"
  end
  if day == 3
    return "Wednesday"
  end
  # ... etc
  "Unknown day"
end`}
            </code>
          </pre>
        </Card>
      </div>

      {/* Complete Example */}
      <div className="space-y-6">
        <h2 className="text-3xl font-bold">Complete Example</h2>
        <p className="text-muted-foreground">
          Here's a full example showing pattern matching with error handling.
        </p>

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

const teams = [
  Team(name: "Ada", wins: 7, losses: 3),
  Team(name: "Grace", wins: 9, losses: 1)
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

  \`Fan favorite: \${team.name} with \${team.wins} wins!\`
end

@println(announce_favorite("Grace"))    # "Fan favorite: Grace with 9 wins!"
@println(announce_favorite("Unknown"))  # "Sorry, Unknown is not on the schedule"`}
            </code>
          </pre>
        </Card>
      </div>

      {/* Next Steps */}
      <div className="space-y-6">
        <h2 className="text-3xl font-bold">Next Steps</h2>

        <div className="flex flex-col gap-4">
          <Link
            href="/docs/error-handling"
            className="flex items-center justify-between p-4 rounded-lg border border-border bg-card hover:bg-muted/50 transition-colors group panel-inset"
          >
            <div>
              <h3 className="font-semibold mb-1 group-hover:text-accent transition-colors">Error Handling</h3>
              <p className="text-sm text-muted-foreground">Learn more about Tea's error handling system</p>
            </div>
            <ArrowRight className="h-5 w-5 text-muted-foreground group-hover:text-accent transition-colors" />
          </Link>

          <Link
            href="/docs/modules"
            className="flex items-center justify-between p-4 rounded-lg border border-border bg-card hover:bg-muted/50 transition-colors group panel-inset"
          >
            <div>
              <h3 className="font-semibold mb-1 group-hover:text-accent transition-colors">Modules & Imports</h3>
              <p className="text-sm text-muted-foreground">Organize code into modules</p>
            </div>
            <ArrowRight className="h-5 w-5 text-muted-foreground group-hover:text-accent transition-colors" />
          </Link>
        </div>
      </div>
    </div>
  )
}
