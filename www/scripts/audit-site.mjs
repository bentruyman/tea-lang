import fs from "node:fs"
import path from "node:path"
import { fileURLToPath } from "node:url"
import siteMap from "../lib/site-map.json" with { type: "json" }

const scriptDir = path.dirname(fileURLToPath(import.meta.url))
const appRoot = path.resolve(scriptDir, "..")
const repoRoot = path.resolve(appRoot, "..")
const referenceManifestPath = path.join(appRoot, "generated", "reference-manifest.json")

function walk(dir, acc = []) {
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    if (entry.name === "node_modules" || entry.name === ".next") {
      continue
    }

    const fullPath = path.join(dir, entry.name)
    if (entry.isDirectory()) {
      walk(fullPath, acc)
    } else {
      acc.push(fullPath)
    }
  }
  return acc
}

function appRouteFromFile(filePath) {
  const relative = path.relative(path.join(appRoot, "app"), filePath)
  const withoutPage = relative.replace(/(^|\/)page\.(tsx|mdx)$/, "")
  const route = `/${withoutPage}`.replace(/\\/g, "/").replace(/\/$/, "")
  return route === "" ? "/" : route
}

function gatherStaticRoutes() {
  const pageFiles = walk(path.join(appRoot, "app")).filter((file) => /page\.(tsx|mdx)$/.test(file))
  return pageFiles
    .map(appRouteFromFile)
    .filter((route) => !route.includes("["))
}

function collectDeclaredRoutes() {
  const routes = new Set(gatherStaticRoutes())

  for (const item of siteMap.topNav) {
    routes.add(item.href)
  }

  for (const section of siteMap.docsSections) {
    for (const item of section.items) {
      routes.add(item.href)
    }
  }

  for (const section of siteMap.referenceSections) {
    for (const item of section.items) {
      routes.add(item.href)
    }
  }

  for (const section of siteMap.exampleSections) {
    for (const item of section.items) {
      routes.add(item.href)
    }
  }

  return routes
}

function collectHrefs() {
  const files = walk(path.join(appRoot, "app")).concat(walk(path.join(appRoot, "components")), walk(path.join(appRoot, "lib")))
  const hrefs = new Set()
  const patterns = [
    /href\s*=\s*"([^"]+)"/g,
    /href:\s*"([^"]+)"/g,
    /"href"\s*:\s*"([^"]+)"/g,
  ]

  for (const file of files) {
    const text = fs.readFileSync(file, "utf8")
    for (const pattern of patterns) {
      for (const match of text.matchAll(pattern)) {
        if (match[1]?.startsWith("/")) {
          hrefs.add(match[1])
        }
      }
    }
  }

  return hrefs
}

function verifyContentFiles() {
  const sectionRoots = [
    ["docsSections", "docs"],
    ["exampleSections", "examples"],
  ]

  for (const [sectionKey, routeDir] of sectionRoots) {
    for (const section of siteMap[sectionKey]) {
      for (const item of section.items) {
        const mdxPath = path.join(appRoot, "app", routeDir, item.slug, "page.mdx")
        const tsxPath = path.join(appRoot, "app", routeDir, item.slug, "page.tsx")
        assert(
          fs.existsSync(mdxPath) || fs.existsSync(tsxPath),
          `Missing content page: app/${routeDir}/${item.slug}/page.(mdx|tsx)`,
        )
      }
    }
  }

  console.log("content file audit ok")
}

function loadReferenceManifest() {
  assert(fs.existsSync(referenceManifestPath), "Missing generated reference manifest: generated/reference-manifest.json")
  return JSON.parse(fs.readFileSync(referenceManifestPath, "utf8"))
}

function assert(condition, message) {
  if (!condition) {
    throw new Error(message)
  }
}

function verifyRoutes() {
  const routes = collectDeclaredRoutes()
  const hrefs = collectHrefs()
  const missing = [...hrefs].filter((href) => !routes.has(href)).sort()
  assert(missing.length === 0, `Broken internal hrefs:\n${missing.join("\n")}`)
  console.log(`route audit ok: ${hrefs.size} hrefs checked`)
}

function verifyReferenceSources() {
  const routeFile = path.join(appRoot, "app", "reference", "[slug]", "page.tsx")
  assert(fs.existsSync(routeFile), "Missing dynamic reference route: app/reference/[slug]/page.tsx")

  const manifest = loadReferenceManifest()
  const manifestEntries = new Map(manifest.entries.map((entry) => [entry.slug, entry]))
  const configuredSlugs = new Set(
    siteMap.referenceSections.flatMap((section) => section.items.map((item) => item.slug)),
  )

  for (const slug of configuredSlugs) {
    assert(manifestEntries.has(slug), `Missing reference manifest entry for slug: ${slug}`)
  }

  for (const slug of manifestEntries.keys()) {
    assert(configuredSlugs.has(slug), `Reference manifest entry is not present in site-map.json: ${slug}`)
  }

  for (const entry of manifest.entries) {
    assert(entry.title, `Reference manifest entry has no title: ${entry.slug}`)
    assert(entry.summary, `Reference manifest entry has no summary: ${entry.slug}`)
    assert(Array.isArray(entry.functions), `Reference manifest entry has invalid functions: ${entry.slug}`)
    assert(entry.functions.length > 0, `Reference manifest entry has no functions: ${entry.slug}`)
  }

  console.log("reference manifest audit ok")
}

function verifyExampleSources() {
  for (const section of siteMap.exampleSections) {
    for (const item of section.items) {
      const sourcePath = path.join(repoRoot, item.sourcePath)
      assert(fs.existsSync(sourcePath), `Missing example source: ${item.sourcePath}`)
      if (item.readmePath) {
        assert(fs.existsSync(path.join(repoRoot, item.readmePath)), `Missing example README: ${item.readmePath}`)
      }
    }
  }
  console.log("example source audit ok")
}

function verifyBannedPatterns() {
  const files = walk(path.join(appRoot, "app")).concat(walk(path.join(appRoot, "lib")))
  const banned = [
    { pattern: /\bimport JSON\b/, label: "import JSON" },
    { pattern: /\bclass\s+[A-Z]/, label: "class syntax" },
    { pattern: /\bArray</, label: "Array<T> syntax" },
    { pattern: /(?<!@)\bprint\(/, label: "plain print(...)" },
    { pattern: /\bJSON\.parse\b/, label: "JSON.parse" },
    { pattern: /\bJSON\.stringify\b/, label: "JSON.stringify" },
    { pattern: /\bfor\s+\w+\s+of\s+/, label: "for ... of" },
  ]

  const hits = []
  for (const file of files) {
    const text = fs.readFileSync(file, "utf8")
    for (const entry of banned) {
      if (entry.pattern.test(text)) {
        hits.push(`${entry.label}: ${path.relative(appRoot, file)}`)
      }
    }
  }

  assert(hits.length === 0, `Banned stale snippet patterns found:\n${hits.join("\n")}`)
  console.log("snippet audit ok")
}

verifyRoutes()
verifyContentFiles()
verifyReferenceSources()
verifyExampleSources()
verifyBannedPatterns()
