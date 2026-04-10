import { notFound } from "next/navigation"

import { FunctionPanel } from "@/components/mdx/function-card"
import { ContentPage } from "@/components/site-shell"
import { findReference, referenceItems } from "@/lib/reference"

export function generateStaticParams() {
  return referenceItems.map((item) => ({ slug: item.slug }))
}

export default async function ReferenceEntryPage({
  params,
}: {
  params: Promise<{ slug: string }>
}) {
  const { slug } = await params
  const entry = findReference(slug)

  if (!entry) {
    notFound()
  }

  return (
    <ContentPage eyebrow={entry.eyebrow} title={entry.title} description={entry.summary}>
      <FunctionPanel
        functions={entry.functions.map((fn) => ({
          signature: fn.signature_display,
          description: fn.summary,
        }))}
      />
    </ContentPage>
  )
}
