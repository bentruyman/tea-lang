import "server-only"

import manifestData from "../generated/reference-manifest.json"

import { referenceNavItems, referenceNavSections } from "@/lib/site"

export type ReferenceFunction = {
  name: string
  signature_display: string
  summary: string
}

export type ReferenceEntry = {
  slug: string
  kind: "builtins" | "module"
  title: string
  eyebrow: string
  summary: string
  module_path: string | null
  source_path: string
  functions: ReferenceFunction[]
}

type ReferenceManifest = {
  manifest_version: number
  generated_at: string
  entries: ReferenceEntry[]
}

export type ReferenceItem = {
  slug: string
  href: string
  title: string
  summary: string
  eyebrow: string
  kind: "builtins" | "module"
  module_path: string | null
  source_path: string
  functions: ReferenceFunction[]
}

const referenceManifest = manifestData as ReferenceManifest
const manifestEntries = new Map(referenceManifest.entries.map((entry) => [entry.slug, entry]))

function resolveReferenceItem(item: { slug: string; href: string }): ReferenceItem {
  const entry = manifestEntries.get(item.slug)
  if (!entry) {
    throw new Error(`Missing reference manifest entry for slug "${item.slug}"`)
  }

  return {
    slug: item.slug,
    href: item.href,
    title: entry.title,
    summary: entry.summary,
    eyebrow: entry.eyebrow,
    kind: entry.kind,
    module_path: entry.module_path,
    source_path: entry.source_path,
    functions: entry.functions,
  }
}

export const referenceItems = referenceNavItems.map(resolveReferenceItem)

const referenceItemMap = new Map(referenceItems.map((item) => [item.slug, item]))

export const referenceSections = referenceNavSections.map((section) => ({
  title: section.title,
  items: section.items.map(resolveReferenceItem),
}))

export function findReference(slug: string) {
  return referenceItemMap.get(slug)
}

export function getReferenceManifest() {
  return referenceManifest
}
