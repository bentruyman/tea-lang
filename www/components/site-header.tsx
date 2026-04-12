"use client";

import Image from "next/image";
import Link from "next/link";
import { usePathname } from "next/navigation";

import { ThemeToggle } from "@/components/theme-toggle";
import { Button } from "@/components/ui/button";
import { isTopNavActive, repo, topNav } from "@/lib/site";
import { cn } from "@/lib/utils";

export function SiteHeader() {
  const pathname = usePathname();

  return (
    <header className="sticky top-0 z-50 border-b border-border/60 bg-background/86 backdrop-blur-xl">
      <div className="container mx-auto flex h-[4.5rem] items-center justify-between gap-6 px-4 py-3">
        <Link href="/" className="group flex items-center gap-2.5">
          <Image
            src="/tea-logo.svg"
            alt=""
            width={28}
            height={28}
            className="transition-transform duration-200 group-hover:scale-105"
          />
          <span className="font-display text-4xl font-semibold leading-none tracking-tight text-foreground">
            Tea
          </span>
        </Link>
        <nav className="surface-quiet hidden items-center gap-1 rounded-full border border-border/70 p-1 md:flex">
          {topNav.map((item) => {
            const isActive = isTopNavActive(pathname, item.href);
            return (
              <Link
                key={item.href}
                href={item.href}
                className={cn(
                  "inline-flex h-9 items-center rounded-full px-4 text-sm leading-none transition-all",
                  isActive
                    ? "bg-foreground text-background shadow-sm"
                    : "text-muted-foreground hover:bg-background/80 hover:text-foreground",
                )}
              >
                {item.title}
              </Link>
            );
          })}
        </nav>
        <div className="flex items-center gap-2.5">
          <ThemeToggle />
          <Button
            variant="outline"
            size="sm"
            className="surface-quiet rounded-full border-border/70 px-4 font-semibold shadow-none hover:border-primary/25 hover:bg-accent/40 hover:text-foreground"
            asChild
          >
            <Link href={repo.url} target="_blank" rel="noreferrer">
              GitHub
            </Link>
          </Button>
        </div>
      </div>
    </header>
  );
}
