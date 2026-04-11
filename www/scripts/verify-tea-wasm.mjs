import { existsSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const root = join(here, "..");
const optional = process.argv.includes("--optional");

const requiredFiles = [
  "public/tea-web/pkg/tea_web.js",
  "public/tea-web/pkg/tea_web_bg.wasm",
  "public/tea-web/pkg/tea_web.d.ts",
];

const missingFiles = requiredFiles.filter(
  (path) => !existsSync(join(root, path)),
);

if (missingFiles.length === 0) {
  process.exit(0);
}

const lines = [
  "Tea WASM assets are missing.",
  "Build and commit them before deploying: `bun run build:tea-wasm`.",
  ...missingFiles.map((path) => `Missing: ${path}`),
];

if (optional) {
  for (const line of lines) {
    console.warn(line);
  }
  process.exit(0);
}

for (const line of lines) {
  console.error(line);
}
process.exit(1);
