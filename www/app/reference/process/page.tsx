import { Card } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import Link from "next/link"
import { ArrowLeft, ArrowRight } from "lucide-react"

export default function ProcessPage() {
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
              <Link
                href="/playground"
                className="text-sm text-muted-foreground hover:text-foreground transition-colors"
              >
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
            <h1 className="text-4xl font-bold text-balance">Process Management</h1>
            <p className="text-xl text-muted-foreground text-pretty leading-relaxed">
              The <code className="text-accent">std.env</code> module provides functions for working with environment
              variables and the current working directory.
            </p>
          </div>

          {/* Import */}
          <div className="space-y-6">
            <h2 className="text-3xl font-bold">Import</h2>
            <Card className="p-6 bg-card border-border">
              <pre className="bg-muted p-4 rounded-md overflow-x-auto">
                <code className="font-mono text-sm">use env = "std.env"</code>
              </pre>
            </Card>
          </div>

          {/* Environment Variables */}
          <div className="space-y-6">
            <h2 className="text-3xl font-bold">Environment Variables</h2>

            <Card className="p-6 bg-card border-border">
              <h3 className="font-semibold text-lg mb-3 text-accent">env.get(name: String) -&gt; String</h3>
              <p className="text-sm text-muted-foreground mb-4">
                Get the value of an environment variable.
              </p>
              <pre className="bg-muted p-4 rounded-md overflow-x-auto">
                <code className="font-mono text-sm">
                  {`use env = "std.env"

var home = env.get("HOME")
var path = env.get("PATH")
var user = env.get("USER")

@println(\`Home directory: \${home}\`)
@println(\`Current user: \${user}\`)`}
                </code>
              </pre>
            </Card>

            <Card className="p-6 bg-card border-border">
              <h3 className="font-semibold text-lg mb-3 text-accent">env.set(name: String, value: String) -&gt; Void</h3>
              <p className="text-sm text-muted-foreground mb-4">
                Set an environment variable for the current process.
              </p>
              <pre className="bg-muted p-4 rounded-md overflow-x-auto">
                <code className="font-mono text-sm">
                  {`use env = "std.env"

env.set("MY_VAR", "my_value")
env.set("DEBUG", "true")
env.set("APP_ENV", "production")

# Verify it was set
@println(env.get("MY_VAR"))  # "my_value"`}
                </code>
              </pre>
            </Card>

            <Card className="p-6 bg-card border-border">
              <h3 className="font-semibold text-lg mb-3 text-accent">env.vars() -&gt; Dict[String, String]</h3>
              <p className="text-sm text-muted-foreground mb-4">
                Get all environment variables as a dictionary.
              </p>
              <pre className="bg-muted p-4 rounded-md overflow-x-auto">
                <code className="font-mono text-sm">
                  {`use env = "std.env"

var all_vars = env.vars()

@println(\`HOME: \${all_vars.HOME}\`)
@println(\`USER: \${all_vars.USER}\`)`}
                </code>
              </pre>
            </Card>
          </div>

          {/* Working Directory */}
          <div className="space-y-6">
            <h2 className="text-3xl font-bold">Working Directory</h2>

            <Card className="p-6 bg-card border-border">
              <h3 className="font-semibold text-lg mb-3 text-accent">env.cwd() -&gt; String</h3>
              <p className="text-sm text-muted-foreground mb-4">
                Get the current working directory.
              </p>
              <pre className="bg-muted p-4 rounded-md overflow-x-auto">
                <code className="font-mono text-sm">
                  {`use env = "std.env"

var cwd = env.cwd()
@println(\`Current directory: \${cwd}\`)`}
                </code>
              </pre>
            </Card>
          </div>

          {/* Common Patterns */}
          <div className="space-y-6">
            <h2 className="text-3xl font-bold">Common Patterns</h2>

            <Card className="p-6 bg-card border-border">
              <h3 className="font-semibold text-lg mb-3 text-accent">Configuration from Environment</h3>
              <pre className="bg-muted p-4 rounded-md overflow-x-auto">
                <code className="font-mono text-sm">
                  {`use env = "std.env"

# Read configuration from environment
var db_host = env.get("DB_HOST")
var db_port = env.get("DB_PORT")
var db_name = env.get("DB_NAME")

@println(\`Connecting to \${db_host}:\${db_port}/\${db_name}\`)`}
                </code>
              </pre>
            </Card>

            <Card className="p-6 bg-card border-border">
              <h3 className="font-semibold text-lg mb-3 text-accent">Debug Mode Detection</h3>
              <pre className="bg-muted p-4 rounded-md overflow-x-auto">
                <code className="font-mono text-sm">
                  {`use env = "std.env"

var debug = env.get("DEBUG")

if debug == "true"
  @println("Debug mode enabled")
  @println(\`Working directory: \${env.cwd()}\`)
end`}
                </code>
              </pre>
            </Card>

            <Card className="p-6 bg-card border-border">
              <h3 className="font-semibold text-lg mb-3 text-accent">Path Construction</h3>
              <pre className="bg-muted p-4 rounded-md overflow-x-auto">
                <code className="font-mono text-sm">
                  {`use env = "std.env"
use path = "std.path"
use fs = "std.fs"

# Build paths from environment
var home = env.get("HOME")
var config_dir = path.join([home, ".config", "myapp"])

# Read config file
var config_file = path.join([config_dir, "settings.json"])
var content = fs.read_file(config_file)`}
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
                  {`use env = "std.env"
use fs = "std.fs"
use path = "std.path"
use json = "std.json"

# Application initialization
def init_app() -> Dict[String, String]
  # Get configuration from environment
  var app_env = env.get("APP_ENV")
  var debug = env.get("DEBUG")

  @println(\`Starting in \${app_env} mode\`)

  if debug == "true"
    @println(\`CWD: \${env.cwd()}\`)
  end

  # Load config file based on environment
  var home = env.get("HOME")
  var config_path = path.join([home, ".config", "myapp", \`\${app_env}.json\`])

  var content = fs.read_file(config_path)
  var config = json.decode(content)

  @println(\`Loaded config for: \${config.name}\`)
  config
end

var config = init_app()
@println(\`Server will run on port \${config.port}\`)`}
                </code>
              </pre>
            </Card>
          </div>

          {/* Navigation */}
          <div className="flex items-center justify-between pt-8 border-t border-border">
            <Button variant="outline" asChild>
              <Link href="/reference/json-yaml">← JSON & YAML</Link>
            </Button>
            <Button variant="outline" asChild>
              <Link href="/reference">Back to Reference</Link>
            </Button>
          </div>
        </div>
      </main>
    </div>
  )
}
