import { PageIntro, GroupedSectionCardGrid } from "@/components/site-shell";
import { exampleSections } from "@/lib/site";

export default function ExamplesPage() {
  return (
    <div className="space-y-10">
      <PageIntro
        eyebrow="Examples"
        title="Examples"
        description="Runnable Tea programs that are checked into this repository right now."
      />
      <GroupedSectionCardGrid sections={exampleSections} />
    </div>
  );
}
