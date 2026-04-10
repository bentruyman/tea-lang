import Link from "next/link"

import { ArrowRight, GitBranch, CircleDot, MessageSquare } from "lucide-react"

import { Card } from "@/components/ui/card"
import { PageIntro } from "@/components/site-shell"
import { repo } from "@/lib/site"

const channels = [
  {
    title: "Repository",
    kicker: "Source code",
    description: "Browse the compiler, stdlib, examples, and docs directly in the main repo.",
    href: repo.url,
    icon: GitBranch,
    tone: "feature" as const,
  },
  {
    title: "Issues",
    kicker: "Bugs and features",
    description: "Report bugs, request improvements, and track documentation or compiler work.",
    href: `${repo.url}/issues`,
    icon: CircleDot,
    tone: "card" as const,
  },
  {
    title: "Discussions",
    kicker: "Conversation",
    description: "Use discussions for design questions, examples, and general Tea workflow talk.",
    href: `${repo.url}/discussions`,
    icon: MessageSquare,
    tone: "quiet" as const,
  },
]

const steps = [
  {
    title: "Read the contributing guide",
    description: "Understand the build system, test flow, and code style before jumping in.",
    href: "/docs/contributing",
    external: false,
  },
  {
    title: "Browse open issues",
    description: "Find bugs, feature requests, and documentation tasks that need attention.",
    href: `${repo.url}/issues`,
    external: true,
  },
  {
    title: "Submit a pull request",
    description: "Fork, branch, and open a PR against the main repo.",
    href: `${repo.url}/pulls`,
    external: true,
  },
]

export default function CommunityPage() {
  return (
    <div className="container mx-auto space-y-12 px-4 py-10">
      <div className="section-band texture-grid-fine surface-quiet p-6 md:p-10">
        <div className="relative z-10 space-y-6">
          <PageIntro
            eyebrow="Community"
            title="Contribute in the repository"
            description="Tea is currently organized around its GitHub repository: issues, pull requests, and discussions are the real collaboration surface."
          />
          <div className="site-divider" />
        </div>
      </div>

      <div className="grid gap-4 md:grid-cols-3">
        {channels.map((item) => {
          const Icon = item.icon
          const cardClass =
            item.tone === "feature"
              ? "surface-feature texture-hatch border-primary/15"
              : item.tone === "card"
                ? "surface-card"
                : "surface-quiet"

          return (
            <Link key={item.title} href={item.href} target="_blank" rel="noreferrer">
              <Card
                className={`${cardClass} h-full gap-4 rounded-[1.6rem] p-6 transition-all duration-200 hover:-translate-y-1 hover:border-primary/20`}
              >
                <div className="flex items-start justify-between gap-4">
                  <p className="text-xs font-semibold uppercase tracking-[0.24em] text-primary">{item.kicker}</p>
                  <span className="surface-quiet flex h-10 w-10 items-center justify-center rounded-xl border border-border/70">
                    <Icon className="h-4 w-4 text-primary" />
                  </span>
                </div>
                <h2 className="font-display text-2xl font-semibold tracking-tight text-foreground">{item.title}</h2>
                <p className="text-sm leading-6 text-muted-foreground">{item.description}</p>
                <div className="mt-auto flex items-center gap-2 text-sm font-semibold text-foreground">
                  Open
                  <ArrowRight className="h-4 w-4" />
                </div>
              </Card>
            </Link>
          )
        })}
      </div>

      <div className="divider-section" />

      <section className="grid gap-10 lg:grid-cols-[minmax(0,0.9fr)_minmax(0,1.1fr)] lg:items-start">
        <div className="space-y-4">
          <p className="text-xs font-semibold uppercase tracking-[0.24em] text-primary">Get involved</p>
          <h2 className="font-display text-4xl font-semibold tracking-tight text-balance">
            Start contributing to Tea.
          </h2>
          <p className="max-w-xl text-lg leading-8 text-muted-foreground">
            Whether you're fixing a bug, adding a stdlib module, or improving the docs, the path is the same: fork,
            build, test, PR.
          </p>
          <div className="site-divider max-w-sm" />
        </div>

        <div className="space-y-4">
          {steps.map((step, index) => (
            <Link
              key={step.href}
              href={step.href}
              className="group block"
              {...(step.external ? { target: "_blank", rel: "noreferrer" } : {})}
            >
              <div className="surface-card grid gap-4 rounded-[1.5rem] border border-border/70 p-5 transition-all duration-200 group-hover:-translate-y-0.5 group-hover:border-primary/20 md:grid-cols-[auto_minmax(0,1fr)_auto] md:items-center">
                <div className="surface-quiet flex h-12 w-12 items-center justify-center rounded-2xl border border-border/70 text-sm font-semibold text-primary">
                  {String(index + 1).padStart(2, "0")}
                </div>
                <div className="space-y-1">
                  <h3 className="font-display text-2xl font-semibold tracking-tight text-foreground">{step.title}</h3>
                  <p className="text-sm leading-6 text-muted-foreground">{step.description}</p>
                </div>
                <ArrowRight className="h-5 w-5 text-muted-foreground transition-transform duration-200 group-hover:translate-x-1 group-hover:text-foreground" />
              </div>
            </Link>
          ))}
        </div>
      </section>
    </div>
  )
}
