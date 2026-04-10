import type React from "react"

import { SectionLayout } from "@/components/site-shell"
import { exampleSections } from "@/lib/site"

export default function ExamplesLayout({ children }: { children: React.ReactNode }) {
  return <SectionLayout sections={exampleSections}>{children}</SectionLayout>
}
