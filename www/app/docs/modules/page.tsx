import { Card } from "@/components/ui/card"
import Link from "next/link"
import { ArrowRight } from "lucide-react"

export default function ModulesPage() {
  return (
    <div className="space-y-12">
      {/* Header */}
      <div className="space-y-4">
        <h1 className="text-4xl font-bold text-balance">Modules & Imports</h1>
        <p className="text-xl text-muted-foreground text-pretty leading-relaxed">
          Tea's module system helps you organize code into reusable units. Learn how to create modules,
          export public functions, and import from the standard library or your own code.
        </p>
      </div>

      {/* Importing Modules */}
      <div className="space-y-6">
        <h2 className="text-3xl font-bold">Importing Modules</h2>
        <p className="text-muted-foreground">
          Use the <code className="text-accent">use</code> keyword to import modules.
        </p>

        <Card className="p-6 bg-card border-border">
          <h3 className="font-semibold text-lg mb-3 text-accent">Standard Library Imports</h3>
          <pre className="bg-muted p-4 rounded-md overflow-x-auto">
            <code className="font-mono text-sm">
              {`use fs = "std.fs"
use json = "std.json"
use path = "std.path"
use env = "std.env"
use assert = "std.assert"
use string = "std.string"

# Use the imported module
var content = fs.read_file("config.json")
var data = json.decode(content)`}
            </code>
          </pre>
        </Card>

        <Card className="p-6 bg-card border-border">
          <h3 className="font-semibold text-lg mb-3 text-accent">Relative Imports</h3>
          <p className="text-sm text-muted-foreground mb-4">
            Import from local files using relative paths.
          </p>
          <pre className="bg-muted p-4 rounded-md overflow-x-auto">
            <code className="font-mono text-sm">
              {`# Import from same directory
use helpers = "./helpers"

# Import from parent directory
use utils = "../utils/mod"

# Import from subdirectory
use models = "./models/user"`}
            </code>
          </pre>
        </Card>
      </div>

      {/* Creating Modules */}
      <div className="space-y-6">
        <h2 className="text-3xl font-bold">Creating Modules</h2>
        <p className="text-muted-foreground">
          A module is simply a <code className="text-accent">.tea</code> file. Use{" "}
          <code className="text-accent">pub</code> to export functions and types.
        </p>

        <Card className="p-6 bg-card border-border">
          <h3 className="font-semibold text-lg mb-3 text-accent">Exporting Functions</h3>
          <pre className="bg-muted p-4 rounded-md overflow-x-auto">
            <code className="font-mono text-sm">
              {`# helpers.tea

# Private function (only accessible within this module)
def scale(value: Int, factor: Int) -> Int
  value * factor
end

# Public function (can be imported by other modules)
pub def triple(value: Int) -> Int
  scale(value, 3)
end

pub def double(value: Int) -> Int
  scale(value, 2)
end`}
            </code>
          </pre>
        </Card>

        <Card className="p-6 bg-card border-border">
          <h3 className="font-semibold text-lg mb-3 text-accent">Using the Module</h3>
          <pre className="bg-muted p-4 rounded-md overflow-x-auto">
            <code className="font-mono text-sm">
              {`# main.tea
use helpers = "./helpers"

@println(helpers.triple(5))   # 15
@println(helpers.double(10))  # 20

# helpers.scale is not accessible (private)`}
            </code>
          </pre>
        </Card>
      </div>

      {/* Module Entry Points */}
      <div className="space-y-6">
        <h2 className="text-3xl font-bold">Module Entry Points</h2>
        <p className="text-muted-foreground">
          For directories with multiple files, use <code className="text-accent">mod.tea</code> as the entry point.
        </p>

        <Card className="p-6 bg-card border-border">
          <h3 className="font-semibold text-lg mb-3 text-accent">Directory Structure</h3>
          <pre className="bg-muted p-4 rounded-md overflow-x-auto font-mono text-sm">
            {`src/
├── main.tea
└── utils/
    ├── mod.tea      # Entry point for the utils module
    ├── strings.tea  # Internal helper
    └── numbers.tea  # Internal helper`}
          </pre>
        </Card>

        <Card className="p-6 bg-card border-border">
          <h3 className="font-semibold text-lg mb-3 text-accent">mod.tea</h3>
          <pre className="bg-muted p-4 rounded-md overflow-x-auto">
            <code className="font-mono text-sm">
              {`# utils/mod.tea
# Re-export public functions from internal modules

pub def format_name(name: String) -> String
  \`Name: \${name}\`
end

pub def add(a: Int, b: Int) -> Int
  a + b
end`}
            </code>
          </pre>
        </Card>

        <Card className="p-6 bg-card border-border">
          <h3 className="font-semibold text-lg mb-3 text-accent">Importing</h3>
          <pre className="bg-muted p-4 rounded-md overflow-x-auto">
            <code className="font-mono text-sm">
              {`# main.tea
use utils = "./utils/mod"

@println(utils.format_name("Tea"))  # "Name: Tea"
@println(utils.add(1, 2))           # 3`}
            </code>
          </pre>
        </Card>
      </div>

      {/* Visibility */}
      <div className="space-y-6">
        <h2 className="text-3xl font-bold">Visibility Rules</h2>

        <Card className="p-6 bg-card border-border">
          <div className="space-y-4">
            <div>
              <h3 className="font-semibold text-accent mb-2">Private (default)</h3>
              <p className="text-sm text-muted-foreground">
                Functions and types without <code className="text-accent">pub</code> are private to their module.
              </p>
            </div>

            <div>
              <h3 className="font-semibold text-accent mb-2">Public</h3>
              <p className="text-sm text-muted-foreground">
                Functions and types with <code className="text-accent">pub</code> can be imported by other modules.
              </p>
            </div>
          </div>

          <pre className="bg-muted p-4 rounded-md overflow-x-auto mt-4">
            <code className="font-mono text-sm">
              {`# Private - only accessible in this module
def internal_helper() -> Int
  42
end

# Private struct
struct InternalData {
  value: Int
}

# Public - can be imported
pub def api_function() -> Int
  internal_helper()
end

# Public struct
pub struct Config {
  debug: Bool
}`}
            </code>
          </pre>
        </Card>
      </div>

      {/* Standard Library */}
      <div className="space-y-6">
        <h2 className="text-3xl font-bold">Standard Library Modules</h2>
        <p className="text-muted-foreground">
          Tea includes several built-in modules for common tasks.
        </p>

        <Card className="p-6 bg-card border-border">
          <div className="overflow-x-auto">
            <table className="w-full text-sm">
              <thead>
                <tr className="border-b border-border">
                  <th className="text-left py-2 pr-4 text-accent">Module</th>
                  <th className="text-left py-2">Description</th>
                </tr>
              </thead>
              <tbody>
                <tr className="border-b border-border/50">
                  <td className="py-2 pr-4 font-mono">std.fs</td>
                  <td className="py-2 text-muted-foreground">File system operations</td>
                </tr>
                <tr className="border-b border-border/50">
                  <td className="py-2 pr-4 font-mono">std.json</td>
                  <td className="py-2 text-muted-foreground">JSON encoding and decoding</td>
                </tr>
                <tr className="border-b border-border/50">
                  <td className="py-2 pr-4 font-mono">std.path</td>
                  <td className="py-2 text-muted-foreground">Path manipulation utilities</td>
                </tr>
                <tr className="border-b border-border/50">
                  <td className="py-2 pr-4 font-mono">std.env</td>
                  <td className="py-2 text-muted-foreground">Environment variables</td>
                </tr>
                <tr className="border-b border-border/50">
                  <td className="py-2 pr-4 font-mono">std.string</td>
                  <td className="py-2 text-muted-foreground">String manipulation</td>
                </tr>
                <tr>
                  <td className="py-2 pr-4 font-mono">std.assert</td>
                  <td className="py-2 text-muted-foreground">Assertions for testing</td>
                </tr>
              </tbody>
            </table>
          </div>
        </Card>
      </div>

      {/* Next Steps */}
      <div className="space-y-6">
        <h2 className="text-3xl font-bold">Next Steps</h2>

        <div className="flex flex-col gap-4">
          <Link
            href="/reference/stdlib"
            className="flex items-center justify-between p-4 rounded-lg border border-border bg-card hover:bg-muted/50 transition-colors group panel-inset"
          >
            <div>
              <h3 className="font-semibold mb-1 group-hover:text-accent transition-colors">Standard Library Reference</h3>
              <p className="text-sm text-muted-foreground">Complete API reference for built-in modules</p>
            </div>
            <ArrowRight className="h-5 w-5 text-muted-foreground group-hover:text-accent transition-colors" />
          </Link>

          <Link
            href="/docs/project-structure"
            className="flex items-center justify-between p-4 rounded-lg border border-border bg-card hover:bg-muted/50 transition-colors group panel-inset"
          >
            <div>
              <h3 className="font-semibold mb-1 group-hover:text-accent transition-colors">Project Structure</h3>
              <p className="text-sm text-muted-foreground">Organize larger projects with modules</p>
            </div>
            <ArrowRight className="h-5 w-5 text-muted-foreground group-hover:text-accent transition-colors" />
          </Link>
        </div>
      </div>
    </div>
  )
}
