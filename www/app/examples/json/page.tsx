import { Card } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import Link from "next/link"
import { ArrowLeft, Copy, Play } from "lucide-react"

export default function JsonExamplePage() {
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
        <div className="max-w-4xl mx-auto space-y-8">
          {/* Back Button */}
          <Button variant="ghost" size="sm" className="gap-2" asChild>
            <Link href="/examples">
              <ArrowLeft className="h-4 w-4" />
              Back to Examples
            </Link>
          </Button>

          {/* Header */}
          <div className="space-y-4">
            <div className="flex items-center gap-3">
              <h1 className="text-4xl font-bold">JSON Parsing</h1>
              <span className="inline-block text-xs px-2 py-1 rounded-full bg-green-500/10 text-green-500">
                Beginner
              </span>
            </div>
            <p className="text-xl text-muted-foreground text-pretty leading-relaxed">
              Learn how to parse, manipulate, and generate JSON data using Tea's built-in JSON module.
            </p>
          </div>

          {/* Example 1: Basic Parsing */}
          <div className="space-y-4">
            <h2 className="text-2xl font-bold">Basic JSON Parsing</h2>
            <Card className="p-6 bg-card border-border">
              <div className="flex items-center justify-between mb-4">
                <h3 className="font-semibold">parse_json.tea</h3>
                <div className="flex gap-2">
                  <Button size="sm" variant="ghost" className="h-7 w-7 p-0">
                    <Copy className="h-3.5 w-3.5" />
                  </Button>
                  <Button size="sm" className="h-7 gap-1.5 bg-accent text-accent-foreground hover:bg-accent/90">
                    <Play className="h-3.5 w-3.5" />
                    Run
                  </Button>
                </div>
              </div>
              <pre className="bg-muted p-4 rounded-md overflow-x-auto">
                <code className="font-mono text-sm">
                  <span className="text-purple-400">import</span> <span className="text-foreground">JSON</span>
                  {"\n\n"}
                  <span className="text-muted-foreground"># Parse a JSON string</span>
                  {"\n"}
                  <span className="text-purple-400">var</span> <span className="text-foreground">json_string = </span>
                  <span className="text-yellow-300">
                    '{"{"}"name": "Alice", "age": 30, "city": "NYC"{"}"}'
                  </span>
                  {"\n"}
                  <span className="text-purple-400">var</span> <span className="text-foreground">data = </span>
                  <span className="text-foreground">JSON</span>
                  <span className="text-foreground">.</span>
                  <span className="text-blue-400">parse</span>
                  <span className="text-foreground">(json_string)</span>
                  {"\n\n"}
                  <span className="text-muted-foreground"># Access values</span>
                  {"\n"}
                  <span className="text-blue-400">print</span>
                  <span className="text-foreground">(</span>
                  <span className="text-yellow-300">"Name: </span>
                  <span className="text-orange-400">{"${data['name']}"}</span>
                  <span className="text-yellow-300">"</span>
                  <span className="text-foreground">)</span>
                  {"\n"}
                  <span className="text-blue-400">print</span>
                  <span className="text-foreground">(</span>
                  <span className="text-yellow-300">"Age: </span>
                  <span className="text-orange-400">{"${data['age']}"}</span>
                  <span className="text-yellow-300">"</span>
                  <span className="text-foreground">)</span>
                  {"\n"}
                  <span className="text-blue-400">print</span>
                  <span className="text-foreground">(</span>
                  <span className="text-yellow-300">"City: </span>
                  <span className="text-orange-400">{"${data['city']}"}</span>
                  <span className="text-yellow-300">"</span>
                  <span className="text-foreground">)</span>
                </code>
              </pre>
              <div className="mt-4 pt-4 border-t border-border">
                <p className="text-sm font-semibold text-muted-foreground mb-2">Output:</p>
                <pre className="bg-muted p-3 rounded-md">
                  <code className="font-mono text-sm text-accent">
                    Name: Alice{"\n"}
                    Age: 30{"\n"}
                    City: NYC
                  </code>
                </pre>
              </div>
            </Card>
          </div>

          {/* Example 2: Reading from File */}
          <div className="space-y-4">
            <h2 className="text-2xl font-bold">Reading JSON from File</h2>
            <Card className="p-6 bg-card border-border">
              <div className="flex items-center justify-between mb-4">
                <h3 className="font-semibold">read_json_file.tea</h3>
                <div className="flex gap-2">
                  <Button size="sm" variant="ghost" className="h-7 w-7 p-0">
                    <Copy className="h-3.5 w-3.5" />
                  </Button>
                  <Button size="sm" className="h-7 gap-1.5 bg-accent text-accent-foreground hover:bg-accent/90">
                    <Play className="h-3.5 w-3.5" />
                    Run
                  </Button>
                </div>
              </div>
              <pre className="bg-muted p-4 rounded-md overflow-x-auto">
                <code className="font-mono text-sm">
                  <span className="text-purple-400">import</span> <span className="text-foreground">JSON</span>
                  {"\n"}
                  <span className="text-purple-400">import</span> <span className="text-foreground">File</span>
                  {"\n\n"}
                  <span className="text-muted-foreground"># Read JSON from a file</span>
                  {"\n"}
                  <span className="text-purple-400">var</span> <span className="text-foreground">content = </span>
                  <span className="text-foreground">File</span>
                  <span className="text-foreground">.</span>
                  <span className="text-blue-400">read</span>
                  <span className="text-foreground">(</span>
                  <span className="text-yellow-300">"users.json"</span>
                  <span className="text-foreground">)</span>
                  {"\n"}
                  <span className="text-purple-400">var</span> <span className="text-foreground">users = </span>
                  <span className="text-foreground">JSON</span>
                  <span className="text-foreground">.</span>
                  <span className="text-blue-400">parse</span>
                  <span className="text-foreground">(content)</span>
                  {"\n\n"}
                  <span className="text-muted-foreground"># Iterate over users</span>
                  {"\n"}
                  <span className="text-purple-400">for</span> <span className="text-orange-400">user</span>{" "}
                  <span className="text-purple-400">of</span> <span className="text-foreground">users</span>
                  {"\n  "}
                  <span className="text-blue-400">print</span>
                  <span className="text-foreground">(</span>
                  <span className="text-yellow-300">"User: </span>
                  <span className="text-orange-400">{"${user['name']}"}</span>
                  <span className="text-yellow-300">, Email: </span>
                  <span className="text-orange-400">{"${user['email']}"}</span>
                  <span className="text-yellow-300">"</span>
                  <span className="text-foreground">)</span>
                  {"\n"}
                  <span className="text-purple-400">end</span>
                </code>
              </pre>
            </Card>
          </div>

          {/* Example 3: Generating JSON */}
          <div className="space-y-4">
            <h2 className="text-2xl font-bold">Generating JSON</h2>
            <Card className="p-6 bg-card border-border">
              <div className="flex items-center justify-between mb-4">
                <h3 className="font-semibold">generate_json.tea</h3>
                <div className="flex gap-2">
                  <Button size="sm" variant="ghost" className="h-7 w-7 p-0">
                    <Copy className="h-3.5 w-3.5" />
                  </Button>
                  <Button size="sm" className="h-7 gap-1.5 bg-accent text-accent-foreground hover:bg-accent/90">
                    <Play className="h-3.5 w-3.5" />
                    Run
                  </Button>
                </div>
              </div>
              <pre className="bg-muted p-4 rounded-md overflow-x-auto">
                <code className="font-mono text-sm">
                  <span className="text-purple-400">import</span> <span className="text-foreground">JSON</span>
                  {"\n\n"}
                  <span className="text-muted-foreground"># Create a data structure</span>
                  {"\n"}
                  <span className="text-purple-400">var</span> <span className="text-foreground">person = {"{"}</span>
                  {"\n  "}
                  <span className="text-yellow-300">"name"</span>
                  <span className="text-foreground">: </span>
                  <span className="text-yellow-300">"Bob"</span>
                  <span className="text-foreground">,</span>
                  {"\n  "}
                  <span className="text-yellow-300">"age"</span>
                  <span className="text-foreground">: 25,</span>
                  {"\n  "}
                  <span className="text-yellow-300">"hobbies"</span>
                  <span className="text-foreground">: [</span>
                  <span className="text-yellow-300">"reading"</span>
                  <span className="text-foreground">, </span>
                  <span className="text-yellow-300">"coding"</span>
                  <span className="text-foreground">, </span>
                  <span className="text-yellow-300">"gaming"</span>
                  <span className="text-foreground">],</span>
                  {"\n  "}
                  <span className="text-yellow-300">"address"</span>
                  <span className="text-foreground">: {"{"}</span>
                  {"\n    "}
                  <span className="text-yellow-300">"street"</span>
                  <span className="text-foreground">: </span>
                  <span className="text-yellow-300">"123 Main St"</span>
                  <span className="text-foreground">,</span>
                  {"\n    "}
                  <span className="text-yellow-300">"city"</span>
                  <span className="text-foreground">: </span>
                  <span className="text-yellow-300">"Boston"</span>
                  {"\n  "}
                  <span className="text-foreground">{"}"}</span>
                  {"\n"}
                  <span className="text-foreground">{"}"}</span>
                  {"\n\n"}
                  <span className="text-muted-foreground"># Convert to JSON string</span>
                  {"\n"}
                  <span className="text-purple-400">var</span> <span className="text-foreground">json_output = </span>
                  <span className="text-foreground">JSON</span>
                  <span className="text-foreground">.</span>
                  <span className="text-blue-400">stringify</span>
                  <span className="text-foreground">(person, </span>
                  <span className="text-orange-400">indent</span>
                  <span className="text-foreground">: 2)</span>
                  {"\n"}
                  <span className="text-blue-400">print</span>
                  <span className="text-foreground">(json_output)</span>
                </code>
              </pre>
              <div className="mt-4 pt-4 border-t border-border">
                <p className="text-sm font-semibold text-muted-foreground mb-2">Output:</p>
                <pre className="bg-muted p-3 rounded-md overflow-x-auto">
                  <code className="font-mono text-xs text-accent">
                    {"{"}
                    {"\n  "}"name": "Bob",
                    {"\n  "}"age": 25,
                    {"\n  "}"hobbies": ["reading", "coding", "gaming"],
                    {"\n  "}"address": {"{"}
                    {"\n    "}"street": "123 Main St",
                    {"\n    "}"city": "Boston"
                    {"\n  "}
                    {"}"}
                    {"\n"}
                    {"}"}
                  </code>
                </pre>
              </div>
            </Card>
          </div>

          {/* Key Concepts */}
          <Card className="p-6 bg-muted/30 border-border">
            <h2 className="text-xl font-bold mb-4">Key Concepts</h2>
            <ul className="space-y-3">
              <li className="flex items-start gap-3">
                <div className="h-6 w-6 rounded-full bg-accent/10 flex items-center justify-center shrink-0 mt-0.5">
                  <span className="text-xs font-bold text-accent">1</span>
                </div>
                <div>
                  <strong className="text-foreground">JSON.parse()</strong>
                  <p className="text-sm text-muted-foreground">
                    Converts a JSON string into a Tea data structure (objects and arrays)
                  </p>
                </div>
              </li>
              <li className="flex items-start gap-3">
                <div className="h-6 w-6 rounded-full bg-accent/10 flex items-center justify-center shrink-0 mt-0.5">
                  <span className="text-xs font-bold text-accent">2</span>
                </div>
                <div>
                  <strong className="text-foreground">JSON.stringify()</strong>
                  <p className="text-sm text-muted-foreground">
                    Converts Tea data structures into JSON strings, with optional formatting
                  </p>
                </div>
              </li>
              <li className="flex items-start gap-3">
                <div className="h-6 w-6 rounded-full bg-accent/10 flex items-center justify-center shrink-0 mt-0.5">
                  <span className="text-xs font-bold text-accent">3</span>
                </div>
                <div>
                  <strong className="text-foreground">File Integration</strong>
                  <p className="text-sm text-muted-foreground">
                    Combine JSON parsing with file operations to read and write JSON files
                  </p>
                </div>
              </li>
            </ul>
          </Card>

          {/* Next Steps */}
          <div className="flex items-center justify-between pt-8 border-t border-border">
            <Button variant="outline" asChild>
              <Link href="/examples/cli">← Previous: CLI Applications</Link>
            </Button>
            <Button variant="outline" asChild>
              <Link href="/examples/filesystem">Next: File System Operations →</Link>
            </Button>
          </div>
        </div>
      </main>
    </div>
  )
}
