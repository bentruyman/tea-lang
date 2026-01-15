import { Card } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import Link from "next/link"
import { ArrowLeft, Book } from "lucide-react"

const modules = [
  {
    name: "Array",
    description: "Dynamic arrays with type-safe operations",
    methods: [
      { name: "append", signature: "(item: T) -> Void", description: "Add an item to the end of the array" },
      { name: "length", signature: "() -> Int", description: "Get the number of items in the array" },
      { name: "map", signature: "<U>(fn: (T) -> U) -> Array<U>", description: "Transform each element" },
      { name: "filter", signature: "(fn: (T) -> Bool) -> Array<T>", description: "Filter elements by predicate" },
      { name: "reduce", signature: "<U>(initial: U, fn: (U, T) -> U) -> U", description: "Reduce to single value" },
    ],
  },
  {
    name: "String",
    description: "Immutable string operations and utilities",
    methods: [
      { name: "length", signature: "() -> Int", description: "Get the length of the string" },
      { name: "split", signature: "(delimiter: String) -> Array<String>", description: "Split string by delimiter" },
      { name: "trim", signature: "() -> String", description: "Remove leading and trailing whitespace" },
      { name: "uppercase", signature: "() -> String", description: "Convert to uppercase" },
      { name: "lowercase", signature: "() -> String", description: "Convert to lowercase" },
    ],
  },
  {
    name: "Math",
    description: "Mathematical functions and constants",
    methods: [
      { name: "abs", signature: "(x: Number) -> Number", description: "Absolute value" },
      { name: "sqrt", signature: "(x: Number) -> Number", description: "Square root" },
      { name: "pow", signature: "(base: Number, exp: Number) -> Number", description: "Exponentiation" },
      { name: "floor", signature: "(x: Number) -> Int", description: "Round down to nearest integer" },
      { name: "ceil", signature: "(x: Number) -> Int", description: "Round up to nearest integer" },
    ],
  },
  {
    name: "IO",
    description: "Input/output operations for console and streams",
    methods: [
      { name: "print", signature: "(value: Any) -> Void", description: "Print value to stdout" },
      { name: "println", signature: "(value: Any) -> Void", description: "Print value with newline" },
      { name: "input", signature: "(prompt: String) -> String", description: "Read line from stdin" },
      { name: "error", signature: "(message: String) -> Void", description: "Print to stderr" },
    ],
  },
]

export default function StdlibReferencePage() {
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
            <h1 className="text-4xl font-bold text-balance">Standard Library</h1>
            <p className="text-xl text-muted-foreground text-pretty leading-relaxed">
              Core modules and functions that are available in every Tea program without explicit imports.
            </p>
          </div>

          {/* Modules */}
          {modules.map((module) => (
            <div key={module.name} className="space-y-4">
              <div className="flex items-center gap-3">
                <Book className="h-6 w-6 text-accent" />
                <h2 className="text-3xl font-bold">{module.name}</h2>
              </div>
              <p className="text-muted-foreground">{module.description}</p>

              <Card className="p-6 bg-card border-border">
                <div className="space-y-6">
                  {module.methods.map((method) => (
                    <div key={method.name} className="space-y-2">
                      <div className="flex items-baseline gap-2">
                        <code className="font-mono text-lg font-semibold text-accent">{method.name}</code>
                        <code className="font-mono text-sm text-muted-foreground">{method.signature}</code>
                      </div>
                      <p className="text-sm text-muted-foreground pl-4">{method.description}</p>

                      {/* Example usage */}
                      {method.name === "map" && (
                        <div className="pl-4 mt-3">
                          <p className="text-xs font-semibold text-muted-foreground mb-2">Example:</p>
                          <pre className="bg-muted p-3 rounded-md overflow-x-auto">
                            <code className="font-mono text-xs">
                              <span className="text-purple-400">var</span>{" "}
                              <span className="text-foreground">numbers = [1, 2, 3]</span>
                              {"\n"}
                              <span className="text-purple-400">var</span>{" "}
                              <span className="text-foreground">doubled = numbers.</span>
                              <span className="text-blue-400">map</span>
                              <span className="text-foreground">(</span>
                              <span className="text-orange-400">n</span>
                              <span className="text-foreground"> =&gt; n * 2)</span>
                              {"\n"}
                              <span className="text-muted-foreground"># [2, 4, 6]</span>
                            </code>
                          </pre>
                        </div>
                      )}

                      {method.name === "split" && (
                        <div className="pl-4 mt-3">
                          <p className="text-xs font-semibold text-muted-foreground mb-2">Example:</p>
                          <pre className="bg-muted p-3 rounded-md overflow-x-auto">
                            <code className="font-mono text-xs">
                              <span className="text-purple-400">var</span>{" "}
                              <span className="text-foreground">text = </span>
                              <span className="text-yellow-300">"hello,world,tea"</span>
                              {"\n"}
                              <span className="text-purple-400">var</span>{" "}
                              <span className="text-foreground">parts = text.</span>
                              <span className="text-blue-400">split</span>
                              <span className="text-foreground">(</span>
                              <span className="text-yellow-300">","</span>
                              <span className="text-foreground">)</span>
                              {"\n"}
                              <span className="text-muted-foreground"># ["hello", "world", "tea"]</span>
                            </code>
                          </pre>
                        </div>
                      )}

                      {method.name === "sqrt" && (
                        <div className="pl-4 mt-3">
                          <p className="text-xs font-semibold text-muted-foreground mb-2">Example:</p>
                          <pre className="bg-muted p-3 rounded-md overflow-x-auto">
                            <code className="font-mono text-xs">
                              <span className="text-purple-400">import</span>{" "}
                              <span className="text-foreground">Math</span>
                              {"\n"}
                              <span className="text-blue-400">print</span>
                              <span className="text-foreground">(Math.</span>
                              <span className="text-blue-400">sqrt</span>
                              <span className="text-foreground">(16))</span>
                              {"\n"}
                              <span className="text-muted-foreground"># 4.0</span>
                            </code>
                          </pre>
                        </div>
                      )}

                      {method.name === "print" && (
                        <div className="pl-4 mt-3">
                          <p className="text-xs font-semibold text-muted-foreground mb-2">Example:</p>
                          <pre className="bg-muted p-3 rounded-md overflow-x-auto">
                            <code className="font-mono text-xs">
                              <span className="text-blue-400">print</span>
                              <span className="text-foreground">(</span>
                              <span className="text-yellow-300">"Hello, Tea!"</span>
                              <span className="text-foreground">)</span>
                              {"\n"}
                              <span className="text-blue-400">print</span>
                              <span className="text-foreground">(42)</span>
                              {"\n"}
                              <span className="text-blue-400">print</span>
                              <span className="text-foreground">([1, 2, 3])</span>
                            </code>
                          </pre>
                        </div>
                      )}
                    </div>
                  ))}
                </div>
              </Card>
            </div>
          ))}

          {/* Additional Info */}
          <Card className="p-6 bg-muted/30 border-border">
            <h3 className="text-lg font-semibold mb-3">Using the Standard Library</h3>
            <p className="text-sm text-muted-foreground mb-4">
              Most standard library functions are available globally without imports. For module-specific functions, use
              the import statement:
            </p>
            <pre className="bg-muted p-4 rounded-md overflow-x-auto">
              <code className="font-mono text-sm">
                <span className="text-purple-400">import</span> <span className="text-foreground">Math</span>
                {"\n"}
                <span className="text-purple-400">import</span> <span className="text-foreground">JSON</span>
                {"\n"}
                <span className="text-purple-400">import</span> <span className="text-foreground">File</span>
              </code>
            </pre>
          </Card>

          {/* Navigation */}
          <div className="flex items-center justify-between pt-8 border-t border-border">
            <Button variant="outline" asChild>
              <Link href="/reference">← Back to Reference</Link>
            </Button>
            <Button variant="outline" asChild>
              <Link href="/reference/collections">Next: Collections →</Link>
            </Button>
          </div>
        </div>
      </main>
    </div>
  )
}
