#!/usr/bin/env node

import { spawnSync } from "node:child_process";
import { readFileSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import process from "node:process";

const DEFAULT_ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const ROOT = resolve(process.env.TEA_RELEASE_ROOT ?? DEFAULT_ROOT);
const REMOTE = process.env.TEA_RELEASE_REMOTE ?? "origin";
const TAG_PREFIX = process.env.TEA_RELEASE_TAG_PREFIX ?? "v";
const SEMVER_RE =
  /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?(?:\+[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?$/;

const WORKSPACE_PACKAGES = [
  "tea-cli",
  "tea-compiler",
  "tea-eval",
  "tea-intrinsics",
  "tea-lsp",
  "tea-runtime",
  "tea-support",
];

const TEA_WEB_PACKAGES = ["tea-compiler", "tea-eval", "tea-support", "tea-web"];

function usage(exitCode = 0) {
  console.error(
    [
      "Usage:",
      "  bun scripts/release.mjs prepare [--dry-run] <version>",
      "  bun scripts/release.mjs tag [--dry-run] <version>",
      "  bun scripts/release.mjs push-tag [--dry-run] <version>",
    ].join("\n"),
  );
  process.exit(exitCode);
}

function parseArgs(argv) {
  const args = [...argv];
  const mode = args.shift();
  if (mode !== "prepare" && mode !== "tag" && mode !== "push-tag") {
    usage(1);
  }

  let dryRun = false;
  let version = "";

  for (const arg of args) {
    if (arg === "--dry-run") {
      dryRun = true;
      continue;
    }
    if (!version) {
      version = arg;
      continue;
    }
    usage(1);
  }

  if (!SEMVER_RE.test(version)) {
    console.error(`error: expected a semver version, got "${version}"`);
    process.exit(1);
  }

  return { mode, dryRun, version };
}

function repoPath(path) {
  return resolve(ROOT, path);
}

function tagNameFor(version) {
  return `${TAG_PREFIX}${version}`;
}

function read(path) {
  return readFileSync(repoPath(path), "utf8");
}

function write(path, contents) {
  writeFileSync(repoPath(path), contents);
}

function detectNewline(text) {
  return text.includes("\r\n") ? "\r\n" : "\n";
}

function setTomlVersionInSection(text, sectionName, version) {
  const newline = detectNewline(text);
  const lines = text.split(/\r?\n/);
  const sectionIndex = lines.findIndex((line) => line.trim() === sectionName);

  if (sectionIndex === -1) {
    throw new Error(`missing TOML section ${sectionName}`);
  }

  let sectionEnd = lines.length;
  for (let i = sectionIndex + 1; i < lines.length; i += 1) {
    if (lines[i].trim().startsWith("[") && lines[i].trim().endsWith("]")) {
      sectionEnd = i;
      break;
    }
  }

  const nextLine = `version = "${version}"`;
  for (let i = sectionIndex + 1; i < sectionEnd; i += 1) {
    if (lines[i].trim().startsWith("version")) {
      lines[i] = nextLine;
      return lines.join(newline);
    }
  }

  lines.splice(sectionIndex + 1, 0, nextLine);
  return lines.join(newline);
}

function replaceVersionBlocks(text, packageNames, version) {
  let next = text;

  for (const packageName of packageNames) {
    const escaped = packageName.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
    const pattern = new RegExp(
      `(name = "${escaped}"\\nversion = ")([^"]+)(")`,
      "g",
    );

    let matched = false;
    next = next.replace(pattern, (_, prefix, _oldVersion, suffix) => {
      matched = true;
      return `${prefix}${version}${suffix}`;
    });

    if (!matched) {
      throw new Error(`missing lockfile entry for ${packageName}`);
    }
  }

  return next;
}

function replaceRootBunWorkspaceVersion(text, version) {
  const pattern =
    /("tree-sitter-tea": \{\s*\n\s*"name": "tree-sitter-tea",\s*\n\s*"version": ")([^"]+)(")/m;

  if (!pattern.test(text)) {
    throw new Error("missing bun.lock workspace entry for tree-sitter-tea");
  }

  return text.replace(pattern, `$1${version}$3`);
}

function setJsonVersion(path, updater) {
  const current = read(path);
  const parsed = JSON.parse(current);
  updater(parsed);
  return `${JSON.stringify(parsed, null, 2)}\n`;
}

function updateTextFile(path, updater, changedFiles, dryRun) {
  const current = read(path);
  const next = updater(current);

  if (next === current) {
    return;
  }

  changedFiles.push(path);
  if (!dryRun) {
    write(path, next);
  }
}

function updateJsonFile(path, updater, changedFiles, dryRun) {
  updateTextFile(
    path,
    () => setJsonVersion(path, updater),
    changedFiles,
    dryRun,
  );
}

function currentManifestVersions() {
  const workspaceCargo = read("Cargo.toml");
  const teaWebCargo = read("tea-web/Cargo.toml");
  const treeSitterPackage = JSON.parse(read("tree-sitter-tea/package.json"));
  const treeSitterMeta = JSON.parse(read("tree-sitter-tea/tree-sitter.json"));
  const docsPackage = JSON.parse(read("www/package.json"));
  const wasmPackage = JSON.parse(read("www/public/tea-web/pkg/package.json"));

  return [
    {
      file: "Cargo.toml",
      version: extractTomlVersion(workspaceCargo, "[workspace.package]"),
    },
    {
      file: "tea-web/Cargo.toml",
      version: extractTomlVersion(teaWebCargo, "[package]"),
    },
    {
      file: "tree-sitter-tea/package.json",
      version: treeSitterPackage.version,
    },
    {
      file: "tree-sitter-tea/tree-sitter.json",
      version: treeSitterMeta.metadata?.version,
    },
    {
      file: "www/package.json",
      version: docsPackage.version,
    },
    {
      file: "www/public/tea-web/pkg/package.json",
      version: wasmPackage.version,
    },
  ];
}

function extractTomlVersion(text, sectionName) {
  const sectionStart = text.indexOf(sectionName);
  if (sectionStart === -1) {
    throw new Error(`missing TOML section ${sectionName}`);
  }

  const sectionText = text.slice(sectionStart);
  const match = sectionText.match(/^version = "([^"]+)"/m);
  if (!match) {
    throw new Error(`missing version line in ${sectionName}`);
  }

  return match[1];
}

function prepareRelease(version, dryRun) {
  const changedFiles = [];

  updateTextFile(
    "Cargo.toml",
    (text) => setTomlVersionInSection(text, "[workspace.package]", version),
    changedFiles,
    dryRun,
  );
  updateTextFile(
    "tea-web/Cargo.toml",
    (text) => setTomlVersionInSection(text, "[package]", version),
    changedFiles,
    dryRun,
  );
  updateTextFile(
    "Cargo.lock",
    (text) => replaceVersionBlocks(text, WORKSPACE_PACKAGES, version),
    changedFiles,
    dryRun,
  );
  updateTextFile(
    "tea-web/Cargo.lock",
    (text) => replaceVersionBlocks(text, TEA_WEB_PACKAGES, version),
    changedFiles,
    dryRun,
  );
  updateJsonFile(
    "tree-sitter-tea/package.json",
    (json) => {
      json.version = version;
    },
    changedFiles,
    dryRun,
  );
  updateJsonFile(
    "tree-sitter-tea/tree-sitter.json",
    (json) => {
      json.metadata = json.metadata ?? {};
      json.metadata.version = version;
    },
    changedFiles,
    dryRun,
  );
  updateTextFile(
    "bun.lock",
    (text) => replaceRootBunWorkspaceVersion(text, version),
    changedFiles,
    dryRun,
  );
  updateJsonFile(
    "www/package.json",
    (json) => {
      json.version = version;
    },
    changedFiles,
    dryRun,
  );
  updateJsonFile(
    "www/public/tea-web/pkg/package.json",
    (json) => {
      json.version = version;
    },
    changedFiles,
    dryRun,
  );

  const label = dryRun ? "Would update" : "Updated";
  if (changedFiles.length === 0) {
    console.log(`Release ${version} is already prepared.`);
  } else {
    console.log(
      `${label} ${changedFiles.length} file(s) for release ${version}:`,
    );
    for (const file of changedFiles) {
      console.log(`- ${file}`);
    }
  }

  const tagName = tagNameFor(version);
  console.log("");
  console.log("Next steps:");
  console.log("- Commit the release prep changes if you are using the local/manual flow.");
  console.log(
    `- Preferred: run the GitHub Release workflow with version ${tagName} to commit, tag, build, and publish remotely.`,
  );
  console.log(`- Manual fallback: make release-tag ${version}`);
  console.log(`- Manual fallback: make release-push-tag ${version}`);
}

function git(args, options = {}) {
  const result = spawnSync("git", args, {
    cwd: ROOT,
    encoding: "utf8",
    ...options,
  });

  if (result.status !== 0) {
    const stderr = result.stderr?.trim();
    const stdout = result.stdout?.trim();
    const detail = stderr || stdout || `git ${args.join(" ")} failed`;
    throw new Error(detail);
  }

  return result.stdout.trim();
}

function ensureVersionConsistency(version) {
  const mismatches = currentManifestVersions().filter(
    (entry) => entry.version !== version,
  );

  if (mismatches.length > 0) {
    const lines = mismatches.map(
      (entry) => `- ${entry.file} is ${entry.version ?? "(missing)"}`,
    );
    throw new Error(
      [
        `release metadata is not ready for ${version}:`,
        ...lines,
        `run "make release ${version}" first`,
      ].join("\n"),
    );
  }
}

function createTag(version, dryRun) {
  ensureVersionConsistency(version);

  const status = git(["status", "--short"]);
  if (status) {
    throw new Error(
      "git working tree is not clean; commit the release prep changes before tagging",
    );
  }

  const tagName = tagNameFor(version);
  const existing = spawnSync(
    "git",
    ["rev-parse", "--verify", "--quiet", `refs/tags/${tagName}`],
    {
      cwd: ROOT,
      encoding: "utf8",
    },
  );
  if (existing.status === 0) {
    throw new Error(`tag ${tagName} already exists`);
  }

  const head = git(["rev-parse", "--short", "HEAD"]);

  if (dryRun) {
    console.log(`Would create annotated tag ${tagName} on ${head}.`);
    console.log(`Next step: make release-push-tag ${version}`);
    return;
  }

  git(["tag", "-a", tagName, "-m", `Release ${version}`]);
  console.log(`Created annotated tag ${tagName} on ${head}.`);
  console.log(`Next step: make release-push-tag ${version}`);
}

function ensureTagExists(tagName) {
  const existing = spawnSync(
    "git",
    ["rev-parse", "--verify", "--quiet", `refs/tags/${tagName}`],
    {
      cwd: ROOT,
      encoding: "utf8",
    },
  );

  if (existing.status !== 0) {
    throw new Error(
      `tag ${tagName} does not exist locally; run "make release-tag ${tagName.slice(TAG_PREFIX.length)}" first`,
    );
  }
}

function remoteUrl(remote) {
  const result = spawnSync("git", ["remote", "get-url", remote], {
    cwd: ROOT,
    encoding: "utf8",
  });

  if (result.status !== 0) {
    throw new Error(`git remote "${remote}" is not configured`);
  }

  return result.stdout.trim();
}

function pushTag(version, dryRun) {
  const tagName = tagNameFor(version);
  ensureTagExists(tagName);
  const url = remoteUrl(REMOTE);

  if (dryRun) {
    console.log(`Would push tag ${tagName} to ${REMOTE} (${url}).`);
    console.log(
      `Once pushed, smoke-test the installer with: TEA_REF=${tagName} ./scripts/install.sh`,
    );
    return;
  }

  git(["push", REMOTE, `refs/tags/${tagName}`]);
  console.log(`Pushed tag ${tagName} to ${REMOTE} (${url}).`);
  console.log(
    `Smoke-test the installer with: TEA_REF=${tagName} ./scripts/install.sh`,
  );
}

function main() {
  const { mode, dryRun, version } = parseArgs(process.argv.slice(2));

  try {
    if (mode === "prepare") {
      prepareRelease(version, dryRun);
      return;
    }
    if (mode === "tag") {
      createTag(version, dryRun);
      return;
    }

    pushTag(version, dryRun);
  } catch (error) {
    console.error(`error: ${error.message}`);
    process.exit(1);
  }
}

main();
