import siteMap from "@/lib/site-map.json";

type NavItem = {
  title: string;
  href: string;
};

type SectionItem = NavItem & {
  slug: string;
  summary: string;
  sourcePath?: string;
  readmePath?: string;
  module?: string;
};

type ReferenceNavItem = NavItem & {
  slug: string;
};

type SectionGroup = {
  title: string;
  items: SectionItem[];
};

type ReferenceSectionGroup = {
  title: string;
  items: ReferenceNavItem[];
};

type SiteMap = {
  repo: {
    name: string;
    url: string;
    clone: string;
  };
  topNav: NavItem[];
  docsSections: SectionGroup[];
  referenceSections: ReferenceSectionGroup[];
  exampleSections: SectionGroup[];
};

const map = siteMap as SiteMap;

function flatten<T>(sections: { items: T[] }[]) {
  return sections.flatMap((section) => section.items);
}

export const repo = map.repo;
export const topNav = map.topNav;
export const docsSections = map.docsSections;
export const referenceNavSections = map.referenceSections;
export const exampleSections = map.exampleSections;

export const docItems = flatten(docsSections);
export const referenceNavItems = flatten(referenceNavSections);
export const exampleItems = flatten(exampleSections);

export function findDoc(slug: string) {
  return docItems.find((item) => item.slug === slug);
}

export function findReferenceNav(slug: string) {
  return referenceNavItems.find((item) => item.slug === slug);
}

export function findExample(slug: string) {
  return exampleItems.find((item) => item.slug === slug);
}

export function githubBlobUrl(relativePath: string) {
  return `${repo.url}/blob/main/${relativePath}`;
}

export function findSectionTitle(
  sections: SectionGroup[],
  slug: string,
): string | undefined {
  for (const section of sections) {
    if (section.items.some((item) => item.slug === slug)) {
      return section.title;
    }
  }
  return undefined;
}

export function isTopNavActive(pathname: string, href: string) {
  if (href === "/") {
    return pathname === "/";
  }

  return pathname === href || pathname.startsWith(`${href}/`);
}
