import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";
import os from "node:os";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const appRoot = path.resolve(scriptDir, "..");
const repoRoot = path.resolve(appRoot, "..");
const outputPath = path.join(appRoot, "generated", "reference-manifest.json");
const generatedAt = "1970-01-01T00:00:00Z";

fs.mkdirSync(path.dirname(outputPath), { recursive: true });

function isExecutable(candidate) {
  if (!candidate) {
    return false;
  }

  try {
    fs.accessSync(candidate, fs.constants.X_OK);
    return true;
  } catch {
    return false;
  }
}

function findOnPath(binary) {
  const pathEntries = (process.env.PATH ?? "").split(path.delimiter).filter(Boolean);
  for (const dir of pathEntries) {
    const candidate = path.join(dir, binary);
    if (isExecutable(candidate)) {
      return candidate;
    }
  }
  return null;
}

function resolveRustToolchain() {
  const envCargo = process.env.CARGO;
  if (isExecutable(envCargo)) {
    const rustcCandidate = path.join(path.dirname(envCargo), "rustc");
    if (isExecutable(rustcCandidate)) {
      return { cargo: envCargo, rustc: rustcCandidate };
    }
  }

  const cargoOnPath = findOnPath("cargo");
  const rustcOnPath = findOnPath("rustc");
  if (cargoOnPath && rustcOnPath) {
    return { cargo: cargoOnPath, rustc: rustcOnPath };
  }

  const protoResult = spawnSync("proto", ["bin", "rust"], {
    encoding: "utf8",
    stdio: ["ignore", "pipe", "ignore"],
  });
  if (protoResult.status === 0) {
    const candidates = protoResult.stdout
      .split(/\r?\n/)
      .map((line) => line.trim())
      .filter(Boolean);
    for (const cargo of candidates) {
      if (!isExecutable(cargo)) {
        continue;
      }
      const rustc = path.join(path.dirname(cargo), "rustc");
      if (isExecutable(rustc)) {
        return { cargo, rustc };
      }
    }
  }

  const toolchainsDir = path.join(os.homedir(), ".rustup", "toolchains");
  if (fs.existsSync(toolchainsDir)) {
    for (const entry of fs.readdirSync(toolchainsDir)) {
      const cargo = path.join(toolchainsDir, entry, "bin", "cargo");
      const rustc = path.join(toolchainsDir, entry, "bin", "rustc");
      if (isExecutable(cargo) && isExecutable(rustc)) {
        return { cargo, rustc };
      }
    }
  }

  throw new Error("Unable to locate cargo and rustc for docs manifest generation");
}

const { cargo, rustc } = resolveRustToolchain();

const result = spawnSync(
  cargo,
  [
    "run",
    "-p",
    "tea-cli",
    "--",
    "docs-manifest",
    "--out",
    outputPath,
    "--generated-at",
    generatedAt,
  ],
  {
    cwd: repoRoot,
    env: {
      ...process.env,
      PATH: `${path.dirname(cargo)}${path.delimiter}${process.env.PATH ?? ""}`,
      RUSTC: rustc,
    },
    stdio: "inherit",
  },
);

if (result.status !== 0) {
  process.exit(result.status ?? 1);
}
