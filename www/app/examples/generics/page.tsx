import { Card } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import Link from "next/link"
import { ArrowLeft, Copy, Play } from "lucide-react"

export default function GenericsExamplePage() {
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
              <h1 className="text-4xl font-bold">Generic Functions</h1>
              <span className="inline-block text-xs px-2 py-1 rounded-full bg-yellow-500/10 text-yellow-500">
                Intermediate
              </span>
            </div>
            <p className="text-xl text-muted-foreground text-pretty leading-relaxed">
              Learn how to write reusable, type-safe code using Tea's powerful generic system with compile-time
              specialization.
            </p>
          </div>

          {/* Example 1: Basic Generic Function */}
          <div className="space-y-4">
            <h2 className="text-2xl font-bold">Basic Generic Function</h2>
            <Card className="p-6 bg-card border-border">
              <div className="flex items-center justify-between mb-4">
                <h3 className="font-semibold">generic_first.tea</h3>
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
                  <span className="text-muted-foreground"># Generic function to get first element</span>
                  {"\n"}
                  <span className="text-purple-400">def</span> <span className="text-blue-400">first</span>
                  <span className="text-foreground">&lt;</span>
                  <span className="text-green-400">T</span>
                  <span className="text-foreground">&gt;(</span>
                  <span className="text-orange-400">arr</span>
                  <span className="text-foreground">: </span>
                  <span className="text-green-400">Array</span>
                  <span className="text-foreground">&lt;</span>
                  <span className="text-green-400">T</span>
                  <span className="text-foreground">&gt;) -&gt; </span>
                  <span className="text-green-400">T</span>
                  {"\n  "}
                  <span className="text-foreground">arr[0]</span>
                  {"\n"}
                  <span className="text-purple-400">end</span>
                  {"\n\n"}
                  <span className="text-muted-foreground"># Works with any type</span>
                  {"\n"}
                  <span className="text-purple-400">var</span>{" "}
                  <span className="text-foreground">numbers = [1, 2, 3]</span>
                  {"\n"}
                  <span className="text-purple-400">var</span> <span className="text-foreground">names = [</span>
                  <span className="text-yellow-300">"Alice"</span>
                  <span className="text-foreground">, </span>
                  <span className="text-yellow-300">"Bob"</span>
                  <span className="text-foreground">]</span>
                  {"\n\n"}
                  <span className="text-blue-400">print</span>
                  <span className="text-foreground">(</span>
                  <span className="text-blue-400">first</span>
                  <span className="text-foreground">(numbers))</span>
                  <span className="text-muted-foreground"> # 1</span>
                  {"\n"}
                  <span className="text-blue-400">print</span>
                  <span className="text-foreground">(</span>
                  <span className="text-blue-400">first</span>
                  <span className="text-foreground">(names))</span>
                  <span className="text-muted-foreground"> # "Alice"</span>
                </code>
              </pre>
            </Card>
          </div>

          {/* Example 2: Generic with Constraints */}
          <div className="space-y-4">
            <h2 className="text-2xl font-bold">Generic with Type Constraints</h2>
            <Card className="p-6 bg-card border-border">
              <div className="flex items-center justify-between mb-4">
                <h3 className="font-semibold">generic_comparable.tea</h3>
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
                  <span className="text-muted-foreground"># Generic function with constraint</span>
                  {"\n"}
                  <span className="text-purple-400">def</span> <span className="text-blue-400">max</span>
                  <span className="text-foreground">&lt;</span>
                  <span className="text-green-400">T</span>
                  <span className="text-foreground">: </span>
                  <span className="text-green-400">Comparable</span>
                  <span className="text-foreground">&gt;(</span>
                  <span className="text-orange-400">a</span>
                  <span className="text-foreground">: </span>
                  <span className="text-green-400">T</span>
                  <span className="text-foreground">, </span>
                  <span className="text-orange-400">b</span>
                  <span className="text-foreground">: </span>
                  <span className="text-green-400">T</span>
                  <span className="text-foreground">) -&gt; </span>
                  <span className="text-green-400">T</span>
                  {"\n  "}
                  <span className="text-purple-400">if</span> <span className="text-foreground">a &gt; b</span>
                  {"\n    "}
                  <span className="text-foreground">a</span>
                  {"\n  "}
                  <span className="text-purple-400">else</span>
                  {"\n    "}
                  <span className="text-foreground">b</span>
                  {"\n  "}
                  <span className="text-purple-400">end</span>
                  {"\n"}
                  <span className="text-purple-400">end</span>
                  {"\n\n"}
                  <span className="text-blue-400">print</span>
                  <span className="text-foreground">(</span>
                  <span className="text-blue-400">max</span>
                  <span className="text-foreground">(10, 20))</span>
                  <span className="text-muted-foreground"> # 20</span>
                  {"\n"}
                  <span className="text-blue-400">print</span>
                  <span className="text-foreground">(</span>
                  <span className="text-blue-400">max</span>
                  <span className="text-foreground">(</span>
                  <span className="text-yellow-300">"apple"</span>
                  <span className="text-foreground">, </span>
                  <span className="text-yellow-300">"banana"</span>
                  <span className="text-foreground">))</span>
                  <span className="text-muted-foreground"> # "banana"</span>
                </code>
              </pre>
            </Card>
          </div>

          {/* Example 3: Generic Class */}
          <div className="space-y-4">
            <h2 className="text-2xl font-bold">Generic Class</h2>
            <Card className="p-6 bg-card border-border">
              <div className="flex items-center justify-between mb-4">
                <h3 className="font-semibold">generic_stack.tea</h3>
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
                  <span className="text-muted-foreground"># Generic Stack implementation</span>
                  {"\n"}
                  <span className="text-purple-400">class</span> <span className="text-green-400">Stack</span>
                  <span className="text-foreground">&lt;</span>
                  <span className="text-green-400">T</span>
                  <span className="text-foreground">&gt;</span>
                  {"\n  "}
                  <span className="text-purple-400">var</span> <span className="text-orange-400">items</span>
                  <span className="text-foreground">: </span>
                  <span className="text-green-400">Array</span>
                  <span className="text-foreground">&lt;</span>
                  <span className="text-green-400">T</span>
                  <span className="text-foreground">&gt;</span>
                  {"\n\n  "}
                  <span className="text-purple-400">def</span> <span className="text-blue-400">init</span>
                  <span className="text-foreground">()</span>
                  {"\n    "}
                  <span className="text-orange-400">@items</span> <span className="text-foreground">= []</span>
                  {"\n  "}
                  <span className="text-purple-400">end</span>
                  {"\n\n  "}
                  <span className="text-purple-400">def</span> <span className="text-blue-400">push</span>
                  <span className="text-foreground">(</span>
                  <span className="text-orange-400">item</span>
                  <span className="text-foreground">: </span>
                  <span className="text-green-400">T</span>
                  <span className="text-foreground">)</span>
                  {"\n    "}
                  <span className="text-orange-400">@items</span>
                  <span className="text-foreground">.</span>
                  <span className="text-blue-400">append</span>
                  <span className="text-foreground">(item)</span>
                  {"\n  "}
                  <span className="text-purple-400">end</span>
                  {"\n\n  "}
                  <span className="text-purple-400">def</span> <span className="text-blue-400">pop</span>
                  <span className="text-foreground">() -&gt; </span>
                  <span className="text-green-400">T</span>
                  {"\n    "}
                  <span className="text-orange-400">@items</span>
                  <span className="text-foreground">.</span>
                  <span className="text-blue-400">pop</span>
                  <span className="text-foreground">()</span>
                  {"\n  "}
                  <span className="text-purple-400">end</span>
                  {"\n\n  "}
                  <span className="text-purple-400">def</span> <span className="text-blue-400">size</span>
                  <span className="text-foreground">() -&gt; </span>
                  <span className="text-green-400">Int</span>
                  {"\n    "}
                  <span className="text-orange-400">@items</span>
                  <span className="text-foreground">.</span>
                  <span className="text-blue-400">length</span>
                  {"\n  "}
                  <span className="text-purple-400">end</span>
                  {"\n"}
                  <span className="text-purple-400">end</span>
                  {"\n\n"}
                  <span className="text-muted-foreground"># Use with integers</span>
                  {"\n"}
                  <span className="text-purple-400">var</span> <span className="text-foreground">int_stack = </span>
                  <span className="text-green-400">Stack</span>
                  <span className="text-foreground">&lt;</span>
                  <span className="text-green-400">Int</span>
                  <span className="text-foreground">&gt;.new()</span>
                  {"\n"}
                  <span className="text-foreground">int_stack.</span>
                  <span className="text-blue-400">push</span>
                  <span className="text-foreground">(1)</span>
                  {"\n"}
                  <span className="text-foreground">int_stack.</span>
                  <span className="text-blue-400">push</span>
                  <span className="text-foreground">(2)</span>
                  {"\n"}
                  <span className="text-blue-400">print</span>
                  <span className="text-foreground">(int_stack.</span>
                  <span className="text-blue-400">pop</span>
                  <span className="text-foreground">())</span>
                  <span className="text-muted-foreground"> # 2</span>
                  {"\n\n"}
                  <span className="text-muted-foreground"># Use with strings</span>
                  {"\n"}
                  <span className="text-purple-400">var</span> <span className="text-foreground">str_stack = </span>
                  <span className="text-green-400">Stack</span>
                  <span className="text-foreground">&lt;</span>
                  <span className="text-green-400">String</span>
                  <span className="text-foreground">&gt;.new()</span>
                  {"\n"}
                  <span className="text-foreground">str_stack.</span>
                  <span className="text-blue-400">push</span>
                  <span className="text-foreground">(</span>
                  <span className="text-yellow-300">"hello"</span>
                  <span className="text-foreground">)</span>
                  {"\n"}
                  <span className="text-foreground">str_stack.</span>
                  <span className="text-blue-400">push</span>
                  <span className="text-foreground">(</span>
                  <span className="text-yellow-300">"world"</span>
                  <span className="text-foreground">)</span>
                  {"\n"}
                  <span className="text-blue-400">print</span>
                  <span className="text-foreground">(str_stack.</span>
                  <span className="text-blue-400">pop</span>
                  <span className="text-foreground">())</span>
                  <span className="text-muted-foreground"> # "world"</span>
                </code>
              </pre>
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
                  <strong className="text-foreground">Type Parameters</strong>
                  <p className="text-sm text-muted-foreground">
                    Use <code className="text-accent">&lt;T&gt;</code> to define generic type parameters that can be
                    replaced with any type
                  </p>
                </div>
              </li>
              <li className="flex items-start gap-3">
                <div className="h-6 w-6 rounded-full bg-accent/10 flex items-center justify-center shrink-0 mt-0.5">
                  <span className="text-xs font-bold text-accent">2</span>
                </div>
                <div>
                  <strong className="text-foreground">Type Constraints</strong>
                  <p className="text-sm text-muted-foreground">
                    Restrict generic types with constraints like <code className="text-accent">T: Comparable</code> to
                    ensure required operations
                  </p>
                </div>
              </li>
              <li className="flex items-start gap-3">
                <div className="h-6 w-6 rounded-full bg-accent/10 flex items-center justify-center shrink-0 mt-0.5">
                  <span className="text-xs font-bold text-accent">3</span>
                </div>
                <div>
                  <strong className="text-foreground">Compile-Time Specialization</strong>
                  <p className="text-sm text-muted-foreground">
                    Tea generates specialized versions of generic code for each type used, ensuring optimal performance
                  </p>
                </div>
              </li>
              <li className="flex items-start gap-3">
                <div className="h-6 w-6 rounded-full bg-accent/10 flex items-center justify-center shrink-0 mt-0.5">
                  <span className="text-xs font-bold text-accent">4</span>
                </div>
                <div>
                  <strong className="text-foreground">Type Safety</strong>
                  <p className="text-sm text-muted-foreground">
                    Generics provide compile-time type checking, preventing type errors at runtime
                  </p>
                </div>
              </li>
            </ul>
          </Card>

          {/* Next Steps */}
          <div className="flex items-center justify-between pt-8 border-t border-border">
            <Button variant="outline" asChild>
              <Link href="/examples/filesystem">← Previous: File System Operations</Link>
            </Button>
            <Button variant="outline" asChild>
              <Link href="/docs/generics">Learn More About Generics →</Link>
            </Button>
          </div>
        </div>
      </main>
    </div>
  )
}
