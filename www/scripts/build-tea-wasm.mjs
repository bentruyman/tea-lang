import { mkdirSync, readFileSync, rmSync, existsSync } from "node:fs"
import { dirname, join } from "node:path"
import { fileURLToPath } from "node:url"
import { spawnSync } from "node:child_process"

const here = dirname(fileURLToPath(import.meta.url))
const root = join(here, "..")
const crateDir = join(root, "..", "tea-web")
const outDir = join(root, "public", "tea-web", "pkg")
const optional = process.argv.includes("--optional")
const wasmPack = process.env.WASM_PACK ?? "wasm-pack"
const wasmBindgen = process.env.WASM_BINDGEN ?? "wasm-bindgen"

function runCommand(command, args) {
  return spawnSync(command, args, {
    cwd: root,
    encoding: "utf8",
  })
}

function resolveRustEnv() {
  const activeToolchain = runCommand("rustup", ["show", "active-toolchain"])
  const cargo = runCommand("rustup", ["which", "cargo"])
  const rustc = runCommand("rustup", ["which", "rustc"])

  if (activeToolchain.status !== 0 || cargo.status !== 0 || rustc.status !== 0) {
    return process.env
  }

  const toolchain = activeToolchain.stdout.trim().split(/\s+/)[0]
  const cargoPath = cargo.stdout.trim()
  const rustcPath = rustc.stdout.trim()
  const cargoBinDir = dirname(cargoPath)
  const cargoHomeBinDir = join(process.env.HOME ?? "", ".cargo", "bin")

  return {
    ...process.env,
    PATH: `${cargoHomeBinDir}:${cargoBinDir}:${process.env.PATH ?? ""}`,
    CARGO: cargoPath,
    RUSTC: rustcPath,
    RUSTUP_TOOLCHAIN: toolchain,
  }
}

function commandExists(command, env = process.env) {
  const result = spawnSync(command, ["--version"], {
    cwd: root,
    env,
    stdio: "ignore",
  })

  return !result.error && result.status === 0
}

function commandVersion(command, env = process.env) {
  const result = spawnSync(command, ["--version"], {
    cwd: root,
    env,
    encoding: "utf8",
  })

  if (result.error || result.status !== 0) {
    return null
  }

  return result.stdout.trim()
}

function expectedWasmBindgenVersion() {
  try {
    const lockfile = readFileSync(join(crateDir, "Cargo.lock"), "utf8")
    const match = lockfile.match(
      /\[\[package\]\]\s+name = "wasm-bindgen"\s+version = "([^"]+)"/m,
    )
    return match?.[1] ?? null
  } catch {
    return null
  }
}

function exitForMissingTool(lines) {
  if (optional) {
    for (const line of lines) {
      console.warn(line)
    }
    console.warn("Continuing without Tea WASM assets because this run is optional.")
    process.exit(0)
  }

  for (const line of lines) {
    console.error(line)
  }
  process.exit(1)
}

rmSync(outDir, { recursive: true, force: true })
mkdirSync(outDir, { recursive: true })

const rustEnv = resolveRustEnv()

if (!commandExists(wasmPack, rustEnv)) {
  exitForMissingTool([
    "wasm-pack is required to build the Tea playground bundle.",
    "Install it from https://rustwasm.github.io/wasm-pack/installer/",
  ])
}

if (!commandExists(wasmBindgen, rustEnv)) {
  exitForMissingTool([
    "wasm-bindgen-cli is required to package the Tea playground bundle.",
    "Install it with `cargo install wasm-bindgen-cli`.",
  ])
}

const installedWasmBindgenVersion = commandVersion(wasmBindgen, rustEnv)?.match(
  /(\d+\.\d+\.\d+)/,
)?.[1]
const requiredWasmBindgenVersion = expectedWasmBindgenVersion()

if (
  installedWasmBindgenVersion &&
  requiredWasmBindgenVersion &&
  installedWasmBindgenVersion !== requiredWasmBindgenVersion
) {
  exitForMissingTool([
    `wasm-bindgen-cli version mismatch: found ${installedWasmBindgenVersion}, but tea-web is locked to ${requiredWasmBindgenVersion}.`,
    `Install a matching CLI with \`cargo install wasm-bindgen-cli --version ${requiredWasmBindgenVersion}\`.`,
  ])
}

const result = spawnSync(
  wasmPack,
  [
    "build",
    crateDir,
    "--target",
    "web",
    "--release",
    "--out-dir",
    outDir,
    "--mode",
    "no-install",
  ],
  {
    cwd: root,
    env: rustEnv,
    stdio: "inherit",
  },
)

if (result.error && result.error.code === "ENOENT") {
  exitForMissingTool([
    "wasm-pack is required to build the Tea playground bundle.",
    "Install it from https://rustwasm.github.io/wasm-pack/installer/",
  ])
}

if (optional && result.status && result.status !== 0) {
  console.warn("Tea WASM bundle build failed during optional setup. Continuing anyway.")
  process.exit(0)
}

const generatedGitignore = join(outDir, ".gitignore")
if (existsSync(generatedGitignore)) {
  rmSync(generatedGitignore, { force: true })
}

process.exit(result.status ?? 1)
