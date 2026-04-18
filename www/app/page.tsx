import Link from "next/link";

import {
  ArrowRight,
  BookOpenText,
  Boxes,
  Download,
  Sparkles,
  Terminal,
} from "lucide-react";

import { CodeHighlighter } from "@/components/mdx/code-highlighter";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { referenceItems } from "@/lib/reference";
import { docItems, exampleItems } from "@/lib/site";

const homeSample = `use string from "std.string"

struct User {
  name: String
  age: Int
}

var user = User(name: "Ada", age: 37)
var total = 0

for value in [1, 2, 3, 4]
  total = total + value
end

@println(string.to_upper(user.name))
@println(total)`;

const installCommand = "curl -fsSL https://tea-lang.dev/install | bash";

const destinations = [
  {
    title: "Get Started",
    summary:
      "Install Tea locally, verify the CLI, and follow the first-run path from script to binary.",
    href: "/docs/install",
    icon: BookOpenText,
    tone: "feature" as const,
    kicker: "Install locally",
  },
  {
    title: "Reference",
    summary:
      "Look up built-ins and stdlib modules such as `std.fs`, `std.path`, `std.regex`, and `std.process`.",
    href: "/reference",
    icon: Boxes,
    tone: "quiet" as const,
    kicker: "APIs and built-ins",
  },
  {
    title: "Playground",
    summary:
      "Edit and run browser-safe Tea in a WASM-backed playground embedded in the docs site.",
    href: "/playground",
    icon: Sparkles,
    tone: "feature" as const,
    kicker: "In-browser runner",
  },
  {
    title: "Examples",
    summary:
      "Study complete, runnable examples including `echo`, `grep`, `todo`, and `team_scoreboard`.",
    href: "/examples",
    icon: Terminal,
    tone: "quiet" as const,
    kicker: "Runnable source",
  },
];

export default function HomePage() {
  const pathItems = [
    docItems.find((item) => item.slug === "install"),
    docItems.find((item) => item.slug === "getting-started"),
    docItems.find((item) => item.slug === "cli"),
    referenceItems.find((item) => item.slug === "builtins"),
    referenceItems.find((item) => item.slug === "fs"),
    exampleItems.find((item) => item.slug === "echo"),
  ].filter(Boolean) as { href: string; title: string; summary: string }[];

  return (
    <div className="mx-auto flex max-w-7xl flex-col gap-16 px-4 py-8 md:px-6 md:py-12">
      <section className="section-band texture-grid surface-feature overflow-hidden px-6 py-8 md:px-10 md:py-10 lg:px-14 lg:py-12">
        <div className="relative z-10 space-y-6 lg:space-y-8">
          <div className="space-y-5">
            <p className="text-xs font-semibold uppercase tracking-[0.28em] text-primary">
              Tea Documentation
            </p>

            <div className="grid gap-10 lg:grid-cols-[minmax(0,0.92fr)_minmax(22rem,0.88fr)] lg:items-start">
              <div className="max-w-xl space-y-5">
                <h1 className="font-display text-4xl font-semibold tracking-tight text-balance md:text-5xl lg:text-6xl">
                  Tea makes scripting fun again.
                </h1>
                <p className="max-w-lg text-base leading-7 text-muted-foreground md:text-lg md:leading-8">
                  A strongly typed scripting language with familiar syntax and
                  native compilation. Install the CLI, run a real script
                  locally, then use the playground, reference docs, and runnable
                  examples when you need them.
                </p>
                <div className="flex flex-wrap gap-3">
                  <Button
                    size="lg"
                    className="rounded-full px-6 font-semibold shadow-sm"
                    asChild
                  >
                    <Link href="/docs/install">
                      Install Tea
                      <Download className="h-4 w-4" />
                    </Link>
                  </Button>
                  <Button
                    variant="outline"
                    size="lg"
                    className="surface-quiet rounded-full border-border/70 px-6 font-semibold shadow-none hover:border-primary/25 hover:bg-background/80"
                    asChild
                  >
                    <Link href="/playground">
                      Open playground
                      <ArrowRight className="h-4 w-4" />
                    </Link>
                  </Button>
                </div>
              </div>

              <div className="space-y-3 lg:pt-2">
                <CodeHighlighter code={homeSample} language="tea" />
                <div className="font-mono text-sm">
                  <span className="text-muted-foreground">Run it in </span>
                  <Link
                    href="/playground"
                    className="text-foreground underline decoration-primary/40 underline-offset-4"
                  >
                    /playground
                  </Link>
                  <span className="text-muted-foreground">
                    {" "}
                    in your browser
                  </span>
                </div>
              </div>
            </div>
          </div>

          <Card className="surface-quiet grid gap-5 rounded-[1.5rem] border-border/70 bg-background/70 p-5 shadow-none backdrop-blur-sm md:p-6 lg:grid-cols-[minmax(0,0.9fr)_minmax(0,1.1fr)] lg:items-center">
            <div className="space-y-4 lg:pr-8">
              <div className="flex items-start justify-between gap-4">
                <div className="space-y-2">
                  <p className="text-xs font-semibold uppercase tracking-[0.24em] text-primary">
                    Quick install
                  </p>
                  <h2 className="font-display text-3xl font-semibold tracking-tight text-foreground">
                    Install, then keep moving.
                  </h2>
                </div>
                <Link
                  href="/docs/install"
                  className="shrink-0 pt-1 text-sm font-semibold text-foreground underline decoration-primary/40 underline-offset-4"
                >
                  Full guide
                </Link>
              </div>
              <p className="max-w-xl text-sm leading-6 text-muted-foreground md:text-base">
                The recommended path downloads a prebuilt Tea release for x86_64
                Linux or Apple Silicon macOS with checksum verification and
                installs it to <code>~/.local/bin</code> by default.
              </p>
            </div>

            <div className="space-y-3">
              <pre className="command-panel overflow-x-auto rounded-[1.15rem] px-4 py-4">
                <code className="font-mono text-sm text-inherit md:text-[0.95rem]">
                  {installCommand}
                </code>
              </pre>
              <p className="text-sm leading-6 text-muted-foreground">
                Tea uses a local C toolchain to build executables: run{" "}
                <code>xcode-select --install</code> on Apple Silicon macOS, or
                install <code>clang</code> with your package manager on Linux.
                Intel Macs should build from source.
              </p>
            </div>
          </Card>
        </div>
      </section>

      <section className="grid gap-4 md:grid-cols-2 xl:grid-cols-4">
        {destinations.map((item) => {
          const Icon = item.icon;
          const cardClassName =
            item.tone === "feature"
              ? "surface-feature texture-hatch border-primary/15"
              : "surface-card";

          return (
            <Link key={item.title} href={item.href}>
              <Card
                className={`${cardClassName} h-full gap-5 rounded-[1.6rem] p-6 transition-all duration-200 hover:-translate-y-1 hover:border-primary/20`}
              >
                <div className="flex items-start justify-between gap-4">
                  <div>
                    <p className="text-xs font-semibold uppercase tracking-[0.24em] text-primary">
                      {item.kicker}
                    </p>
                    <h2 className="mt-3 font-display text-3xl font-semibold tracking-tight text-foreground">
                      {item.title}
                    </h2>
                  </div>
                  <span className="surface-quiet flex h-12 w-12 items-center justify-center rounded-2xl border border-border/70">
                    <Icon className="h-5 w-5 text-primary" />
                  </span>
                </div>
                <p className="text-base leading-7 text-muted-foreground">
                  {item.summary}
                </p>
                <div className="mt-auto flex items-center gap-2 text-sm font-semibold text-foreground">
                  Explore {item.title}
                  <ArrowRight className="h-4 w-4" />
                </div>
              </Card>
            </Link>
          );
        })}
      </section>

      <section className="space-y-6">
        <div className="flex items-end justify-between gap-8">
          <div className="space-y-2">
            <p className="text-xs font-semibold uppercase tracking-[0.24em] text-primary">
              Start here
            </p>
            <h2 className="font-display text-3xl font-semibold tracking-tight text-balance">
              A guided path from install to real Tea programs.
            </h2>
          </div>
        </div>

        <div className="grid gap-x-6 gap-y-3 md:grid-cols-2 lg:grid-cols-3">
          {pathItems.map((item, index) => (
            <Link
              key={item.href}
              href={item.href}
              className="group flex items-baseline gap-3 rounded-xl px-1 py-2 transition-colors hover:bg-background/60"
            >
              <span className="shrink-0 text-xs font-semibold tabular-nums text-primary">
                {String(index + 1).padStart(2, "0")}
              </span>
              <div className="min-w-0">
                <p className="text-sm font-semibold text-foreground group-hover:text-primary">
                  {item.title}
                </p>
                <p className="truncate text-xs leading-5 text-muted-foreground">
                  {item.summary}
                </p>
              </div>
            </Link>
          ))}
        </div>
      </section>
    </div>
  );
}
