import { afterEach, describe, expect, test } from "bun:test";
import { mkdtempSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { spawnSync } from "node:child_process";

const SCRIPT = join(process.cwd(), "scripts", "release.mjs");
const tempDirs = [];

afterEach(() => {
  while (tempDirs.length > 0) {
    rmSync(tempDirs.pop(), { force: true, recursive: true });
  }
});

function makeTempDir(prefix) {
  const dir = mkdtempSync(join(tmpdir(), prefix));
  tempDirs.push(dir);
  return dir;
}

function writeFile(root, path, contents) {
  const fullPath = join(root, path);
  mkdirSync(dirname(fullPath), { recursive: true });
  writeFileSync(fullPath, contents);
}

function run(command, args, cwd, env = {}) {
  const result = spawnSync(command, args, {
    cwd,
    encoding: "utf8",
    env: {
      ...process.env,
      ...env,
    },
  });

  if (result.status !== 0) {
    throw new Error(
      `command failed: ${command} ${args.join(" ")}\nstdout:\n${result.stdout}\nstderr:\n${result.stderr}`,
    );
  }

  return result;
}

function initFixtureRepo() {
  const root = makeTempDir("tea-release-fixture-");

  writeFile(
    root,
    "Cargo.toml",
    `[workspace]
members = ["tea-cli"]

[workspace.package]
version = "0.1.0"
edition = "2021"
`,
  );
  writeFile(root, "tea-cli/Cargo.toml", `[package]\nname = "tea-cli"\nversion.workspace = true\n`);
  writeFile(root, "tea-compiler/Cargo.toml", `[package]\nname = "tea-compiler"\nversion.workspace = true\n`);
  writeFile(root, "tea-eval/Cargo.toml", `[package]\nname = "tea-eval"\nversion.workspace = true\n`);
  writeFile(root, "tea-intrinsics/Cargo.toml", `[package]\nname = "tea-intrinsics"\nversion.workspace = true\n`);
  writeFile(root, "tea-lsp/Cargo.toml", `[package]\nname = "tea-lsp"\nversion.workspace = true\n`);
  writeFile(root, "tea-runtime/Cargo.toml", `[package]\nname = "tea-runtime"\nversion.workspace = true\n`);
  writeFile(root, "tea-support/Cargo.toml", `[package]\nname = "tea-support"\nversion.workspace = true\n`);
  writeFile(
    root,
    "tea-web/Cargo.toml",
    `[package]
name = "tea-web"
version = "0.1.0"
edition = "2021"
`,
  );
  writeFile(
    root,
    "Cargo.lock",
    `[[package]]
name = "tea-cli"
version = "0.1.0"

[[package]]
name = "tea-compiler"
version = "0.1.0"

[[package]]
name = "tea-eval"
version = "0.1.0"

[[package]]
name = "tea-intrinsics"
version = "0.1.0"

[[package]]
name = "tea-lsp"
version = "0.1.0"

[[package]]
name = "tea-runtime"
version = "0.1.0"

[[package]]
name = "tea-support"
version = "0.1.0"
`,
  );
  writeFile(
    root,
    "tea-web/Cargo.lock",
    `[[package]]
name = "tea-compiler"
version = "0.1.0"

[[package]]
name = "tea-eval"
version = "0.1.0"

[[package]]
name = "tea-support"
version = "0.1.0"

[[package]]
name = "tea-web"
version = "0.1.0"
`,
  );
  writeFile(
    root,
    "tree-sitter-tea/package.json",
    `{
  "name": "tree-sitter-tea",
  "version": "0.1.0"
}
`,
  );
  writeFile(
    root,
    "tree-sitter-tea/tree-sitter.json",
    `{
  "name": "tree-sitter-tea",
  "metadata": {
    "version": "0.1.0"
  }
}
`,
  );
  writeFile(
    root,
    "bun.lock",
    `{
  "lockfileVersion": 1,
  "configVersion": 0,
  "workspaces": {
    "tree-sitter-tea": {
      "name": "tree-sitter-tea",
      "version": "0.1.0"
    }
  }
}
`,
  );
  writeFile(
    root,
    "www/package.json",
    `{
  "name": "tea-docs",
  "version": "0.1.0"
}
`,
  );
  writeFile(
    root,
    "www/public/tea-web/pkg/package.json",
    `{
  "name": "tea-web",
  "version": "0.1.0"
}
`,
  );

  run("git", ["init"], root);
  run("git", ["config", "user.name", "Tea Tests"], root);
  run("git", ["config", "user.email", "tea-tests@example.com"], root);
  run("git", ["config", "commit.gpgsign", "false"], root);
  run("git", ["config", "tag.gpgSign", "false"], root);
  run("git", ["add", "."], root);
  run("git", ["commit", "-m", "initial"], root);

  return root;
}

function runRelease(root, ...args) {
  return run(process.execPath, [SCRIPT, ...args], process.cwd(), {
    TEA_RELEASE_ROOT: root,
  });
}

describe("release automation", () => {
  test("prepare updates every tracked release manifest", () => {
    const root = initFixtureRepo();

    const result = runRelease(root, "prepare", "0.0.1-alpha.1");
    expect(result.stdout).toContain("Updated 9 file(s) for release 0.0.1-alpha.1");
    expect(readFileSync(join(root, "Cargo.toml"), "utf8")).toContain(
      'version = "0.0.1-alpha.1"',
    );
    expect(readFileSync(join(root, "tea-web/Cargo.lock"), "utf8")).toContain(
      'name = "tea-web"\nversion = "0.0.1-alpha.1"',
    );
    expect(readFileSync(join(root, "bun.lock"), "utf8")).toContain(
      '"version": "0.0.1-alpha.1"',
    );
    expect(readFileSync(join(root, "www/public/tea-web/pkg/package.json"), "utf8")).toContain(
      '"version": "0.0.1-alpha.1"',
    );
  });

  test("tag and push-tag publish a ref that can be cloned by tag", () => {
    const root = initFixtureRepo();
    const remotePath = makeTempDir("tea-release-remote-");
    const clonePath = makeTempDir("tea-release-clone-");
    const remoteUrl = `file://${remotePath}`;

    run("git", ["init", "--bare", remotePath], process.cwd());
    run("git", ["remote", "add", "origin", remoteUrl], root);
    runRelease(root, "prepare", "0.0.1-alpha.1");
    run("git", ["add", "."], root);
    run("git", ["commit", "-m", "release"], root);

    const tagResult = runRelease(root, "tag", "0.0.1-alpha.1");
    expect(tagResult.stdout).toContain("Created annotated tag v0.0.1-alpha.1");

    const pushResult = runRelease(root, "push-tag", "0.0.1-alpha.1");
    expect(pushResult.stdout).toContain(`Pushed tag v0.0.1-alpha.1 to origin (${remoteUrl}).`);

    run(
      "git",
      ["clone", "--depth", "1", "--branch", "v0.0.1-alpha.1", remoteUrl, clonePath],
      process.cwd(),
    );

    expect(readFileSync(join(clonePath, "Cargo.toml"), "utf8")).toContain(
      'version = "0.0.1-alpha.1"',
    );
  });
});
