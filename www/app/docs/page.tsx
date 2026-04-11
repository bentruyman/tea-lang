import { PageIntro, GroupedSectionCardGrid } from "@/components/site-shell";
import { docsSections } from "@/lib/site";

export default function DocsPage() {
  return (
    <div className="space-y-10">
      <PageIntro
        eyebrow="Docs"
        title="Documentation"
        description="Language and workflow guides based on the current compiler, examples, and CLI implementation."
      />
      <GroupedSectionCardGrid sections={docsSections} />
    </div>
  );
}
