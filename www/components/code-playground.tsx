"use client"

import { useState } from "react"
import { Button } from "@/components/ui/button"
import { Card } from "@/components/ui/card"
import { Copy, Play } from "lucide-react"

const SAMPLE_CODE = `def greet(name: String) -> String
  "Hello, \${name}!"
end

var names = ["Alice", "Bob", "Charlie"]
for person of names
  print(greet(person))
end`

const NATIVE_OUTPUT = `Hello, Alice!
Hello, Bob!
Hello, Charlie!`

export function CodePlayground() {
  const [output, setOutput] = useState("")
  const [isRunning, setIsRunning] = useState(false)

  const runCode = () => {
    setIsRunning(true)
    setOutput("")

    setTimeout(() => {
      setOutput(NATIVE_OUTPUT)
      setIsRunning(false)
    }, 800)
  }

  const copyCode = () => {
    navigator.clipboard.writeText(SAMPLE_CODE)
  }

  return (
    <Card className="bg-card border-border/50 overflow-hidden shadow-lg">
      <div className="flex items-center justify-between border-b border-border/50 bg-muted/20 px-5 py-3">
        <div className="flex items-center gap-3">
          <div className="flex gap-1.5">
            <div className="h-3 w-3 rounded-full bg-red-500/70" />
            <div className="h-3 w-3 rounded-full bg-yellow-500/70" />
            <div className="h-3 w-3 rounded-full bg-green-500/70" />
          </div>
          <span className="ml-2 font-mono text-sm text-muted-foreground">greet.tea</span>
        </div>
        <div className="flex items-center gap-2">
          <Button size="sm" variant="ghost" onClick={copyCode} className="h-8 w-8 p-0 hover:bg-muted">
            <Copy className="h-4 w-4" />
          </Button>
          <Button
            size="sm"
            onClick={runCode}
            disabled={isRunning}
            className="h-8 gap-1.5 bg-accent text-accent-foreground hover:bg-accent/90"
          >
            <Play className="h-4 w-4" />
            Run
          </Button>
        </div>
      </div>

      <div className="grid md:grid-cols-2">
        <div className="border-r border-border/50">
          <div className="px-5 py-3 border-b border-border/50 bg-muted/10">
            <span className="text-xs font-semibold text-muted-foreground uppercase tracking-wide">Code</span>
          </div>
          <pre className="p-6 overflow-x-auto">
            <code className="font-mono text-sm leading-loose">
              <span className="text-purple-400">def</span> <span className="text-blue-400">greet</span>
              <span className="text-foreground">(</span>
              <span className="text-orange-400">name</span>
              <span className="text-foreground">: </span>
              <span className="text-green-400">String</span>
              <span className="text-foreground">) -&gt; </span>
              <span className="text-green-400">String</span>
              {"\n  "}
              <span className="text-yellow-300">"Hello, </span>
              <span className="text-orange-400">{"${name}"}</span>
              <span className="text-yellow-300">!"</span>
              {"\n"}
              <span className="text-purple-400">end</span>
              {"\n\n"}
              <span className="text-purple-400">var</span> <span className="text-foreground">names = [</span>
              <span className="text-yellow-300">"Alice"</span>
              <span className="text-foreground">, </span>
              <span className="text-yellow-300">"Bob"</span>
              <span className="text-foreground">, </span>
              <span className="text-yellow-300">"Charlie"</span>
              <span className="text-foreground">]</span>
              {"\n"}
              <span className="text-purple-400">for</span> <span className="text-orange-400">person</span>{" "}
              <span className="text-purple-400">of</span> <span className="text-foreground">names</span>
              {"\n  "}
              <span className="text-blue-400">print</span>
              <span className="text-foreground">(</span>
              <span className="text-blue-400">greet</span>
              <span className="text-foreground">(</span>
              <span className="text-orange-400">person</span>
              <span className="text-foreground">))</span>
              {"\n"}
              <span className="text-purple-400">end</span>
            </code>
          </pre>
        </div>

        <div className="bg-muted/10">
          <div className="px-5 py-3 border-b border-border/50 bg-muted/10">
            <span className="text-xs font-semibold text-muted-foreground uppercase tracking-wide">Output</span>
          </div>
          <pre className="p-6 min-h-[250px] font-mono text-sm leading-loose">
            {isRunning ? (
              <span className="text-accent">Running...</span>
            ) : output ? (
              <span className="text-foreground">{output}</span>
            ) : (
              <span className="text-muted-foreground">Click Run to execute</span>
            )}
          </pre>
        </div>
      </div>
    </Card>
  )
}
