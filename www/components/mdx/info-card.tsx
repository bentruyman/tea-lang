import { ReactNode } from "react";

import { cn } from "@/lib/utils";

interface KeyConceptCardProps {
  title: string;
  children: ReactNode;
  className?: string;
}

export function KeyConceptCard({
  title,
  children,
  className,
}: KeyConceptCardProps) {
  return (
    <div className={cn("space-y-3", className)}>
      <h2 className="font-display text-[1.9rem] font-semibold tracking-tight text-foreground">
        {title}
      </h2>
      <div className="text-sm leading-7 text-muted-foreground">{children}</div>
    </div>
  );
}

interface NoteCardProps {
  title: string;
  children: ReactNode;
  className?: string;
}

export function NoteCard({ title, children, className }: NoteCardProps) {
  return (
    <div className={cn("space-y-3", className)}>
      <h2 className="font-display text-[1.9rem] font-semibold tracking-tight text-foreground">
        {title}
      </h2>
      <div className="text-sm leading-6 text-muted-foreground">{children}</div>
    </div>
  );
}

interface FeaturePillProps {
  kicker: string;
  title: string;
  children: ReactNode;
  className?: string;
}

export function FeaturePill({
  kicker,
  title,
  children,
  className,
}: FeaturePillProps) {
  return (
    <div
      className={cn(
        "surface-quiet rounded-2xl border border-border/70 p-4",
        className,
      )}
    >
      <p className="text-xs font-semibold uppercase tracking-[0.2em] text-primary">
        {kicker}
      </p>
      <h2 className="mt-2 font-display text-xl font-semibold tracking-tight text-foreground">
        {title}
      </h2>
      <div className="mt-2 text-sm leading-6 text-muted-foreground">
        {children}
      </div>
    </div>
  );
}
