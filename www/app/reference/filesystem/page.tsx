import { Card } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import Link from "next/link"
import { ArrowLeft } from "lucide-react"

export default function FilesystemPage() {
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
            <h1 className="text-4xl font-bold text-balance">File System</h1>
            <p className="text-xl text-muted-foreground text-pretty leading-relaxed">
              The <code className="text-accent">std.fs</code> module provides functions for reading, writing, and managing
              files and directories.
            </p>
          </div>

          {/* Import */}
          <div className="space-y-6">
            <h2 className="text-3xl font-bold">Import</h2>
            <Card className="p-6 bg-card border-border">
              <pre className="bg-muted p-4 rounded-md overflow-x-auto">
                <code className="font-mono text-sm">use fs = "std.fs"</code>
              </pre>
            </Card>
          </div>

          {/* Reading Files */}
          <div className="space-y-6">
            <h2 className="text-3xl font-bold">Reading Files</h2>

            <Card className="p-6 bg-card border-border">
              <h3 className="font-semibold text-lg mb-3 text-accent">fs.read_file(path: String) -&gt; String</h3>
              <p className="text-sm text-muted-foreground mb-4">
                Read the entire contents of a text file.
              </p>
              <pre className="bg-muted p-4 rounded-md overflow-x-auto">
                <code className="font-mono text-sm">
                  {`use fs = "std.fs"

var content = fs.read_file("config.json")
@println(content)

# Read and process line by line
var readme = fs.read_file("README.md")
@println(\`File length: \${@len(readme)}\`)`}
                </code>
              </pre>
            </Card>
          </div>

          {/* Writing Files */}
          <div className="space-y-6">
            <h2 className="text-3xl font-bold">Writing Files</h2>

            <Card className="p-6 bg-card border-border">
              <h3 className="font-semibold text-lg mb-3 text-accent">fs.write_file(path: String, content: String) -&gt; Void</h3>
              <p className="text-sm text-muted-foreground mb-4">
                Write text content to a file. Creates the file if it doesn't exist, overwrites if it does.
              </p>
              <pre className="bg-muted p-4 rounded-md overflow-x-auto">
                <code className="font-mono text-sm">
                  {`use fs = "std.fs"

fs.write_file("output.txt", "Hello, World!")

# Write with string interpolation
var name = "Tea"
var version = "1.0"
fs.write_file("info.txt", \`\${name} version \${version}\`)`}
                </code>
              </pre>
            </Card>
          </div>

          {/* Directory Operations */}
          <div className="space-y-6">
            <h2 className="text-3xl font-bold">Directory Operations</h2>

            <Card className="p-6 bg-card border-border">
              <h3 className="font-semibold text-lg mb-3 text-accent">fs.create_dir(path: String) -&gt; Void</h3>
              <p className="text-sm text-muted-foreground mb-4">
                Create a new directory.
              </p>
              <pre className="bg-muted p-4 rounded-md overflow-x-auto">
                <code className="font-mono text-sm">
                  {`use fs = "std.fs"

fs.create_dir("output")
fs.create_dir("data/cache")`}
                </code>
              </pre>
            </Card>

            <Card className="p-6 bg-card border-border">
              <h3 className="font-semibold text-lg mb-3 text-accent">fs.read_dir(path: String) -&gt; List[String]</h3>
              <p className="text-sm text-muted-foreground mb-4">
                List all entries in a directory.
              </p>
              <pre className="bg-muted p-4 rounded-md overflow-x-auto">
                <code className="font-mono text-sm">
                  {`use fs = "std.fs"

var entries = fs.read_dir(".")

var i = 0
while i < @len(entries)
  @println(entries[i])
  i = i + 1
end`}
                </code>
              </pre>
            </Card>

            <Card className="p-6 bg-card border-border">
              <h3 className="font-semibold text-lg mb-3 text-accent">fs.remove(path: String) -&gt; Void</h3>
              <p className="text-sm text-muted-foreground mb-4">
                Remove a file or directory.
              </p>
              <pre className="bg-muted p-4 rounded-md overflow-x-auto">
                <code className="font-mono text-sm">
                  {`use fs = "std.fs"

fs.remove("temp.txt")
fs.remove("cache_dir")`}
                </code>
              </pre>
            </Card>
          </div>

          {/* Path Module */}
          <div className="space-y-6">
            <h2 className="text-3xl font-bold">Path Utilities</h2>
            <p className="text-muted-foreground">
              Use <code className="text-accent">std.path</code> for path manipulation.
            </p>

            <Card className="p-6 bg-card border-border">
              <pre className="bg-muted p-4 rounded-md overflow-x-auto">
                <code className="font-mono text-sm">
                  {`use path = "std.path"

# Join path components
var full = path.join(["usr", "local", "bin"])  # "usr/local/bin"

# Get directory name
var dir = path.dirname("/home/user/file.txt")  # "/home/user"

# Get file name
var name = path.basename("/home/user/file.txt")  # "file.txt"

# Get extension
var ext = path.extension("script.tea")  # "tea"

# Split into components
var parts = path.split("/usr/local/bin")  # ["usr", "local", "bin"]`}
                </code>
              </pre>
            </Card>
          </div>

          {/* Navigation */}
          <div className="flex items-center justify-between pt-8 border-t border-border">
            <Button variant="outline" asChild>
              <Link href="/reference/collections">← Collections</Link>
            </Button>
            <Button variant="outline" asChild>
              <Link href="/reference/json-yaml">JSON & YAML →</Link>
            </Button>
          </div>
        </div>
      </main>
    </div>
  )
}
