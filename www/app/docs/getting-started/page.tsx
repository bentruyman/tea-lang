import { Button } from "@/components/ui/button"
import { Card } from "@/components/ui/card"
import Link from "next/link"
import { ArrowRight, CheckCircle2, Terminal, FileCode, Zap } from "lucide-react"

export default function GettingStartedPage() {
  return (
    <div className="space-y-12">
      {/* Header */}
      <div className="space-y-4">
        <h1 className="text-4xl font-bold text-balance">Getting Started with Tea</h1>
        <p className="text-xl text-muted-foreground text-pretty leading-relaxed">
          This guide will help you install Tea, write your first program, and understand the basics of the language.
          You'll be up and running in less than 10 minutes.
        </p>
      </div>

      {/* Prerequisites */}
      <div className="space-y-4">
        <h2 className="text-3xl font-bold">Prerequisites</h2>
        <Card className="p-6 bg-card border-border">
          <p className="text-muted-foreground mb-4">Before installing Tea, make sure you have:</p>
          <ul className="space-y-2">
            <li className="flex items-start gap-3">
              <CheckCircle2 className="h-5 w-5 text-accent mt-0.5 shrink-0" />
              <span>
                <strong className="text-foreground">Git</strong> - Version control system (
                <a href="https://git-scm.com/" className="text-accent hover:underline" target="_blank" rel="noreferrer">
                  download here
                </a>
                )
              </span>
            </li>
            <li className="flex items-start gap-3">
              <CheckCircle2 className="h-5 w-5 text-accent mt-0.5 shrink-0" />
              <span>
                <strong className="text-foreground">C++ Compiler</strong> - GCC 9+ or Clang 10+ for building from source
              </span>
            </li>
            <li className="flex items-start gap-3">
              <CheckCircle2 className="h-5 w-5 text-accent mt-0.5 shrink-0" />
              <span>
                <strong className="text-foreground">LLVM 14+</strong> - Required for native compilation
              </span>
            </li>
            <li className="flex items-start gap-3">
              <CheckCircle2 className="h-5 w-5 text-accent mt-0.5 shrink-0" />
              <span>
                <strong className="text-foreground">Make</strong> - Build automation tool
              </span>
            </li>
          </ul>
        </Card>
      </div>

      {/* Installation Steps */}
      <div className="space-y-6">
        <h2 className="text-3xl font-bold">Installation</h2>

        <div className="space-y-6">
          {/* Step 1 */}
          <Card className="p-6 bg-card border-border">
            <div className="flex items-center gap-3 mb-4">
              <div className="h-8 w-8 rounded-full bg-accent text-accent-foreground flex items-center justify-center font-bold">
                1
              </div>
              <h3 className="text-xl font-semibold">Clone the Repository</h3>
            </div>
            <p className="text-muted-foreground mb-4">
              First, clone the Tea repository from GitHub to your local machine:
            </p>
            <pre className="bg-muted p-4 rounded-md overflow-x-auto">
              <code className="font-mono text-sm text-foreground">
                git clone https://github.com/special-tea/tea.git{"\n"}
                cd tea
              </code>
            </pre>
          </Card>

          {/* Step 2 */}
          <Card className="p-6 bg-card border-border">
            <div className="flex items-center gap-3 mb-4">
              <div className="h-8 w-8 rounded-full bg-accent text-accent-foreground flex items-center justify-center font-bold">
                2
              </div>
              <h3 className="text-xl font-semibold">Build and Install</h3>
            </div>
            <p className="text-muted-foreground mb-4">
              Run the setup script to build Tea and install it to your system:
            </p>
            <pre className="bg-muted p-4 rounded-md overflow-x-auto mb-4">
              <code className="font-mono text-sm text-foreground">
                make setup{"\n"}
                make install
              </code>
            </pre>
            <p className="text-sm text-muted-foreground">
              This will compile the Tea compiler and install it to <code className="text-accent">/usr/local/bin</code>{" "}
              by default. You may need to use <code className="text-accent">sudo</code> for installation.
            </p>
          </Card>

          {/* Step 3 */}
          <Card className="p-6 bg-card border-border">
            <div className="flex items-center gap-3 mb-4">
              <div className="h-8 w-8 rounded-full bg-accent text-accent-foreground flex items-center justify-center font-bold">
                3
              </div>
              <h3 className="text-xl font-semibold">Verify Installation</h3>
            </div>
            <p className="text-muted-foreground mb-4">Check that Tea is installed correctly:</p>
            <pre className="bg-muted p-4 rounded-md overflow-x-auto mb-4">
              <code className="font-mono text-sm text-foreground">tea --version</code>
            </pre>
            <p className="text-sm text-muted-foreground mb-4">You should see output like:</p>
            <pre className="bg-muted p-4 rounded-md overflow-x-auto">
              <code className="font-mono text-sm text-accent">Tea version 0.1.0</code>
            </pre>
          </Card>
        </div>
      </div>

      {/* Your First Program */}
      <div className="space-y-6">
        <h2 className="text-3xl font-bold">Your First Tea Program</h2>
        <p className="text-muted-foreground">Let's write a simple "Hello, World!" program to get started.</p>

        <Card className="p-6 bg-card border-border">
          <div className="flex items-center gap-3 mb-4">
            <FileCode className="h-6 w-6 text-accent" />
            <h3 className="text-xl font-semibold">Create hello.tea</h3>
          </div>
          <p className="text-muted-foreground mb-4">Create a new file called hello.tea with the following content:</p>
          <pre className="bg-muted p-4 rounded-md overflow-x-auto">
            <code className="font-mono text-sm">
              <span className="text-muted-foreground"># hello.tea</span>
              {"\n"}
              <span className="text-blue-400">print</span>
              <span className="text-foreground">(</span>
              <span className="text-yellow-300">"Hello, Tea!"</span>
              <span className="text-foreground">)</span>
            </code>
          </pre>
        </Card>

        <Card className="p-6 bg-card border-border">
          <div className="flex items-center gap-3 mb-4">
            <Terminal className="h-6 w-6 text-accent" />
            <h3 className="text-xl font-semibold">Run your program</h3>
          </div>
          <p className="text-muted-foreground mb-4">Execute your program using Tea:</p>
          <pre className="bg-muted p-4 rounded-md overflow-x-auto mb-4">
            <code className="font-mono text-sm text-foreground">tea run hello.tea</code>
          </pre>
          <p className="text-sm text-muted-foreground mb-4">Output:</p>
          <pre className="bg-muted p-4 rounded-md overflow-x-auto">
            <code className="font-mono text-sm text-accent">Hello, Tea!</code>
          </pre>
        </Card>

        <Card className="p-6 bg-card border-border">
          <div className="flex items-center gap-3 mb-4">
            <Zap className="h-6 w-6 text-accent" />
            <h3 className="text-xl font-semibold">Compile to native binary</h3>
          </div>
          <p className="text-muted-foreground mb-4">Compile your program to a native binary for optimal performance:</p>
          <pre className="bg-muted p-4 rounded-md overflow-x-auto mb-4">
            <code className="font-mono text-sm text-foreground">
              tea compile hello.tea -o hello{"\n"}
              ./hello
            </code>
          </pre>
          <p className="text-sm text-muted-foreground">
            The compiled binary runs directly on your system with no runtime dependencies.
          </p>
        </Card>
      </div>

      {/* A More Complex Example */}
      <div className="space-y-6">
        <h2 className="text-3xl font-bold">A More Complex Example</h2>
        <p className="text-muted-foreground">
          Let's explore Tea's features with a program that demonstrates functions, types, and control flow.
        </p>

        <Card className="p-6 bg-card border-border">
          <pre className="bg-muted p-4 rounded-md overflow-x-auto">
            <code className="font-mono text-sm">
              <span className="text-muted-foreground"># greet.tea</span>
              {"\n\n"}
              <span className="text-muted-foreground"># Define a function with type annotations</span>
              {"\n"}
              <span className="text-purple-400">def</span> <span className="text-blue-400">greet</span>
              <span className="text-foreground">(</span>
              <span className="text-orange-400">name</span>
              <span className="text-foreground">: </span>
              <span className="text-green-400">String</span>
              <span className="text-foreground">, </span>
              <span className="text-orange-400">age</span>
              <span className="text-foreground">: </span>
              <span className="text-green-400">Int</span>
              <span className="text-foreground">) -&gt; </span>
              <span className="text-green-400">String</span>
              {"\n  "}
              <span className="text-purple-400">if</span> <span className="text-foreground">age &lt; 18</span>
              {"\n    "}
              <span className="text-yellow-300">"Hello, young </span>
              <span className="text-orange-400">{"${name}"}</span>
              <span className="text-yellow-300">!"</span>
              {"\n  "}
              <span className="text-purple-400">else</span>
              {"\n    "}
              <span className="text-yellow-300">"Hello, </span>
              <span className="text-orange-400">{"${name}"}</span>
              <span className="text-yellow-300">!"</span>
              {"\n  "}
              <span className="text-purple-400">end</span>
              {"\n"}
              <span className="text-purple-400">end</span>
              {"\n\n"}
              <span className="text-muted-foreground"># Create an array of names</span>
              {"\n"}
              <span className="text-purple-400">var</span> <span className="text-foreground">people = [</span>
              {"\n  "}
              <span className="text-foreground">{"{"}</span>
              <span className="text-yellow-300">"name"</span>
              <span className="text-foreground">: </span>
              <span className="text-yellow-300">"Alice"</span>
              <span className="text-foreground">, </span>
              <span className="text-yellow-300">"age"</span>
              <span className="text-foreground">: 25{"}"}</span>
              <span className="text-foreground">,</span>
              {"\n  "}
              <span className="text-foreground">{"{"}</span>
              <span className="text-yellow-300">"name"</span>
              <span className="text-foreground">: </span>
              <span className="text-yellow-300">"Bob"</span>
              <span className="text-foreground">, </span>
              <span className="text-yellow-300">"age"</span>
              <span className="text-foreground">: 16{"}"}</span>
              <span className="text-foreground">,</span>
              {"\n  "}
              <span className="text-foreground">{"{"}</span>
              <span className="text-yellow-300">"name"</span>
              <span className="text-foreground">: </span>
              <span className="text-yellow-300">"Charlie"</span>
              <span className="text-foreground">, </span>
              <span className="text-yellow-300">"age"</span>
              <span className="text-foreground">: 30{"}"}</span>
              {"\n"}
              <span className="text-foreground">]</span>
              {"\n\n"}
              <span className="text-muted-foreground"># Iterate and greet each person</span>
              {"\n"}
              <span className="text-purple-400">for</span> <span className="text-orange-400">person</span>{" "}
              <span className="text-purple-400">of</span> <span className="text-foreground">people</span>
              {"\n  "}
              <span className="text-purple-400">var</span> <span className="text-foreground">message = </span>
              <span className="text-blue-400">greet</span>
              <span className="text-foreground">(person[</span>
              <span className="text-yellow-300">"name"</span>
              <span className="text-foreground">], person[</span>
              <span className="text-yellow-300">"age"</span>
              <span className="text-foreground">])</span>
              {"\n  "}
              <span className="text-blue-400">print</span>
              <span className="text-foreground">(message)</span>
              {"\n"}
              <span className="text-purple-400">end</span>
            </code>
          </pre>

          <div className="mt-4 pt-4 border-t border-border">
            <p className="text-sm font-semibold text-muted-foreground mb-2">Output:</p>
            <pre className="bg-muted p-4 rounded-md overflow-x-auto">
              <code className="font-mono text-sm text-accent">
                Hello, Alice!{"\n"}
                Hello, young Bob!{"\n"}
                Hello, Charlie!
              </code>
            </pre>
          </div>
        </Card>

        <Card className="p-6 bg-card border-border">
          <h3 className="text-lg font-semibold mb-3">Key Concepts Demonstrated</h3>
          <ul className="space-y-3">
            <li className="flex items-start gap-3">
              <CheckCircle2 className="h-5 w-5 text-accent mt-0.5 shrink-0" />
              <div>
                <strong className="text-foreground">Function Definitions:</strong>
                <span className="text-muted-foreground">
                  {" "}
                  Use <code className="text-accent">def</code> with type annotations for parameters and return types
                </span>
              </div>
            </li>
            <li className="flex items-start gap-3">
              <CheckCircle2 className="h-5 w-5 text-accent mt-0.5 shrink-0" />
              <div>
                <strong className="text-foreground">String Interpolation:</strong>
                <span className="text-muted-foreground">
                  {" "}
                  Embed variables in strings using <code className="text-accent">{"${variable}"}</code>
                </span>
              </div>
            </li>
            <li className="flex items-start gap-3">
              <CheckCircle2 className="h-5 w-5 text-accent mt-0.5 shrink-0" />
              <div>
                <strong className="text-foreground">Control Flow:</strong>
                <span className="text-muted-foreground">
                  {" "}
                  Use <code className="text-accent">if/else</code> for conditional logic
                </span>
              </div>
            </li>
            <li className="flex items-start gap-3">
              <CheckCircle2 className="h-5 w-5 text-accent mt-0.5 shrink-0" />
              <div>
                <strong className="text-foreground">Loops:</strong>
                <span className="text-muted-foreground">
                  {" "}
                  Iterate over collections with <code className="text-accent">for...of</code>
                </span>
              </div>
            </li>
            <li className="flex items-start gap-3">
              <CheckCircle2 className="h-5 w-5 text-accent mt-0.5 shrink-0" />
              <div>
                <strong className="text-foreground">Type Inference:</strong>
                <span className="text-muted-foreground">
                  {" "}
                  Variables like <code className="text-accent">message</code> have their types inferred automatically
                </span>
              </div>
            </li>
          </ul>
        </Card>
      </div>

      {/* Next Steps */}
      <div className="space-y-6">
        <h2 className="text-3xl font-bold">Next Steps</h2>
        <p className="text-muted-foreground">
          Now that you've written your first Tea programs, here's what to explore next:
        </p>

        <div className="grid md:grid-cols-2 gap-4">
          <Link
            href="/docs/syntax"
            className="flex items-start gap-4 p-4 rounded-lg border border-border bg-card hover:bg-muted/50 transition-colors group"
          >
            <div className="h-10 w-10 rounded-lg bg-accent/10 flex items-center justify-center shrink-0">
              <FileCode className="h-5 w-5 text-accent" />
            </div>
            <div className="flex-1">
              <h3 className="font-semibold mb-1 group-hover:text-accent transition-colors">Learn the Syntax</h3>
              <p className="text-sm text-muted-foreground">Deep dive into Tea's syntax and language features</p>
            </div>
            <ArrowRight className="h-5 w-5 text-muted-foreground group-hover:text-accent transition-colors shrink-0 mt-2" />
          </Link>

          <Link
            href="/docs/types"
            className="flex items-start gap-4 p-4 rounded-lg border border-border bg-card hover:bg-muted/50 transition-colors group"
          >
            <div className="h-10 w-10 rounded-lg bg-accent/10 flex items-center justify-center shrink-0">
              <Zap className="h-5 w-5 text-accent" />
            </div>
            <div className="flex-1">
              <h3 className="font-semibold mb-1 group-hover:text-accent transition-colors">Understand Types</h3>
              <p className="text-sm text-muted-foreground">Master Tea's static type system and inference</p>
            </div>
            <ArrowRight className="h-5 w-5 text-muted-foreground group-hover:text-accent transition-colors shrink-0 mt-2" />
          </Link>

          <Link
            href="/examples"
            className="flex items-start gap-4 p-4 rounded-lg border border-border bg-card hover:bg-muted/50 transition-colors group"
          >
            <div className="h-10 w-10 rounded-lg bg-accent/10 flex items-center justify-center shrink-0">
              <FileCode className="h-5 w-5 text-accent" />
            </div>
            <div className="flex-1">
              <h3 className="font-semibold mb-1 group-hover:text-accent transition-colors">Browse Examples</h3>
              <p className="text-sm text-muted-foreground">See real-world Tea programs and patterns</p>
            </div>
            <ArrowRight className="h-5 w-5 text-muted-foreground group-hover:text-accent transition-colors shrink-0 mt-2" />
          </Link>

          <Link
            href="/reference/stdlib"
            className="flex items-start gap-4 p-4 rounded-lg border border-border bg-card hover:bg-muted/50 transition-colors group"
          >
            <div className="h-10 w-10 rounded-lg bg-accent/10 flex items-center justify-center shrink-0">
              <Terminal className="h-5 w-5 text-accent" />
            </div>
            <div className="flex-1">
              <h3 className="font-semibold mb-1 group-hover:text-accent transition-colors">
                Explore the Standard Library
              </h3>
              <p className="text-sm text-muted-foreground">Discover built-in functions and modules</p>
            </div>
            <ArrowRight className="h-5 w-5 text-muted-foreground group-hover:text-accent transition-colors shrink-0 mt-2" />
          </Link>
        </div>
      </div>

      {/* Help Section */}
      <Card className="p-6 bg-muted/30 border-border">
        <h3 className="text-lg font-semibold mb-3">Need Help?</h3>
        <p className="text-muted-foreground mb-4">
          If you run into any issues or have questions, here are some resources:
        </p>
        <div className="flex flex-wrap gap-3">
          <Button variant="outline" size="sm" asChild>
            <Link href="https://github.com/special-tea/tea/issues" target="_blank">
              Report an Issue
            </Link>
          </Button>
          <Button variant="outline" size="sm" asChild>
            <Link href="/community">Join Discord</Link>
          </Button>
          <Button variant="outline" size="sm" asChild>
            <Link href="/docs">Read the Docs</Link>
          </Button>
        </div>
      </Card>
    </div>
  )
}
