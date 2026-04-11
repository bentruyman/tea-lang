import { PageIntro, GroupedSectionCardGrid } from "@/components/site-shell";
import { referenceSections } from "@/lib/reference";

export default function ReferencePage() {
  return (
    <div className="space-y-10">
      <PageIntro
        eyebrow="Reference"
        title="Reference"
        description="Built-ins and stdlib modules generated from the compiler and checked-in Tea stdlib sources."
      />
      <GroupedSectionCardGrid sections={referenceSections} />
    </div>
  );
}
