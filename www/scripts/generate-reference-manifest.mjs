import fs from "node:fs"
import path from "node:path"
import { fileURLToPath } from "node:url"
import { spawnSync } from "node:child_process"

const scriptDir = path.dirname(fileURLToPath(import.meta.url))
const appRoot = path.resolve(scriptDir, "..")
const repoRoot = path.resolve(appRoot, "..")
const outputPath = path.join(appRoot, "generated", "reference-manifest.json")
const generatedAt = "1970-01-01T00:00:00Z"

fs.mkdirSync(path.dirname(outputPath), { recursive: true })

const result = spawnSync(
  "cargo",
  ["run", "-p", "tea-cli", "--", "docs-manifest", "--out", outputPath, "--generated-at", generatedAt],
  {
    cwd: repoRoot,
    stdio: "inherit",
  },
)

if (result.status !== 0) {
  process.exit(result.status ?? 1)
}
