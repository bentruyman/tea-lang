import type React from "react";

import { SectionLayout } from "@/components/site-shell";
import { docsSections } from "@/lib/site";

export default function DocsLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return <SectionLayout sections={docsSections}>{children}</SectionLayout>;
}
