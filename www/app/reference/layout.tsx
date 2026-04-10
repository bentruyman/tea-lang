import type React from "react"

import { SectionLayout } from "@/components/site-shell"
import { referenceSections } from "@/lib/reference"

export default function ReferenceLayout({ children }: { children: React.ReactNode }) {
  return <SectionLayout sections={referenceSections}>{children}</SectionLayout>
}
