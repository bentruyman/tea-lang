import { PageIntro, GroupedSectionCardGrid } from "@/components/site-shell"
import { referenceSections } from "@/lib/site"

export default function ReferencePage() {
  return (
    <div className="space-y-10">
      <PageIntro
        eyebrow="Reference"
        title="Reference"
        description="Built-ins and stdlib modules that exist in the repo today, with exports read directly from checked-in Tea source."
      />
      <GroupedSectionCardGrid sections={referenceSections} />
    </div>
  )
}
