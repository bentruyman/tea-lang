import { Card } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import Link from "next/link"
import { ArrowLeft } from "lucide-react"

export default function JsonYamlPage() {
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
            <h1 className="text-4xl font-bold text-balance">JSON & YAML</h1>
            <p className="text-xl text-muted-foreground text-pretty leading-relaxed">
              The <code className="text-accent">std.json</code> module provides functions for encoding and decoding JSON data.
            </p>
          </div>

          {/* Import */}
          <div className="space-y-6">
            <h2 className="text-3xl font-bold">Import</h2>
            <Card className="p-6 bg-card border-border">
              <pre className="bg-muted p-4 rounded-md overflow-x-auto">
                <code className="font-mono text-sm">use json = "std.json"</code>
              </pre>
            </Card>
          </div>

          {/* Encoding */}
          <div className="space-y-6">
            <h2 className="text-3xl font-bold">Encoding JSON</h2>

            <Card className="p-6 bg-card border-border">
              <h3 className="font-semibold text-lg mb-3 text-accent">json.encode(value: Dict[String, String]) -&gt; String</h3>
              <p className="text-sm text-muted-foreground mb-4">
                Convert a dictionary to a JSON string.
              </p>
              <pre className="bg-muted p-4 rounded-md overflow-x-auto">
                <code className="font-mono text-sm">
                  {`use json = "std.json"

var data = { "name": "tea", "version": "1.0" }
var json_str = json.encode(data)

@println(json_str)  # {"name":"tea","version":"1.0"}`}
                </code>
              </pre>
            </Card>
          </div>

          {/* Decoding */}
          <div className="space-y-6">
            <h2 className="text-3xl font-bold">Decoding JSON</h2>

            <Card className="p-6 bg-card border-border">
              <h3 className="font-semibold text-lg mb-3 text-accent">json.decode(json_str: String) -&gt; Dict[String, String]</h3>
              <p className="text-sm text-muted-foreground mb-4">
                Parse a JSON string into a dictionary.
              </p>
              <pre className="bg-muted p-4 rounded-md overflow-x-auto">
                <code className="font-mono text-sm">
                  {`use json = "std.json"

var json_str = "{\\"name\\":\\"tea\\",\\"version\\":\\"1.0\\"}"
var data = json.decode(json_str)

@println(data.name)     # "tea"
@println(data.version)  # "1.0"`}
                </code>
              </pre>
            </Card>
          </div>

          {/* Working with Files */}
          <div className="space-y-6">
            <h2 className="text-3xl font-bold">Working with JSON Files</h2>

            <Card className="p-6 bg-card border-border">
              <h3 className="font-semibold text-lg mb-3 text-accent">Reading JSON Files</h3>
              <pre className="bg-muted p-4 rounded-md overflow-x-auto">
                <code className="font-mono text-sm">
                  {`use fs = "std.fs"
use json = "std.json"

# Read and parse a JSON file
var content = fs.read_file("config.json")
var config = json.decode(content)

@println(\`App: \${config.name}\`)
@println(\`Debug: \${config.debug}\`)`}
                </code>
              </pre>
            </Card>

            <Card className="p-6 bg-card border-border">
              <h3 className="font-semibold text-lg mb-3 text-accent">Writing JSON Files</h3>
              <pre className="bg-muted p-4 rounded-md overflow-x-auto">
                <code className="font-mono text-sm">
                  {`use fs = "std.fs"
use json = "std.json"

# Create data and write to file
var settings = {
  "theme": "dark",
  "language": "en",
  "notifications": "true"
}

var json_str = json.encode(settings)
fs.write_file("settings.json", json_str)`}
                </code>
              </pre>
            </Card>
          </div>

          {/* Common Patterns */}
          <div className="space-y-6">
            <h2 className="text-3xl font-bold">Common Patterns</h2>

            <Card className="p-6 bg-card border-border">
              <h3 className="font-semibold text-lg mb-3 text-accent">Configuration Loading</h3>
              <pre className="bg-muted p-4 rounded-md overflow-x-auto">
                <code className="font-mono text-sm">
                  {`use fs = "std.fs"
use json = "std.json"
use path = "std.path"

def load_config(config_path: String) -> Dict[String, String]
  var content = fs.read_file(config_path)
  json.decode(content)
end

# Load from default location
var config = load_config("config/app.json")
@println(\`Server: \${config.host}:\${config.port}\`)`}
                </code>
              </pre>
            </Card>
          </div>

          {/* Best Practices */}
          <div className="space-y-6">
            <h2 className="text-3xl font-bold">Best Practices</h2>

            <Card className="p-6 bg-card border-border">
              <ul className="space-y-3 text-sm">
                <li className="flex items-start gap-3">
                  <span className="text-accent">•</span>
                  <span className="text-muted-foreground">
                    Always validate JSON data before accessing fields
                  </span>
                </li>
                <li className="flex items-start gap-3">
                  <span className="text-accent">•</span>
                  <span className="text-muted-foreground">
                    Use meaningful file names with <code className="text-accent">.json</code> extension
                  </span>
                </li>
                <li className="flex items-start gap-3">
                  <span className="text-accent">•</span>
                  <span className="text-muted-foreground">
                    Keep configuration files in a dedicated directory (e.g., <code className="text-accent">config/</code>)
                  </span>
                </li>
              </ul>
            </Card>
          </div>

          {/* Navigation */}
          <div className="flex items-center justify-between pt-8 border-t border-border">
            <Button variant="outline" asChild>
              <Link href="/reference/filesystem">← File System</Link>
            </Button>
            <Button variant="outline" asChild>
              <Link href="/reference/process">Process Management →</Link>
            </Button>
          </div>
        </div>
      </main>
    </div>
  )
}
