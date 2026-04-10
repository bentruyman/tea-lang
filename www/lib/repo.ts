import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const moduleDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(moduleDir, "../..");

export type StdlibFunctionDoc = {
  name: string;
  signature: string;
  description: string;
};

function absoluteRepoPath(relativePath: string) {
  return path.join(repoRoot, relativePath);
}

export function repoFileExists(relativePath: string) {
  return fs.existsSync(absoluteRepoPath(relativePath));
}

export function readRepoText(relativePath: string) {
  return fs.readFileSync(absoluteRepoPath(relativePath), "utf8");
}

export function readRepoExcerpt(
  relativePath: string,
  startLine = 1,
  endLine?: number,
) {
  const lines = readRepoText(relativePath).split("\n");
  const start = Math.max(startLine - 1, 0);
  const end = endLine ? Math.min(endLine, lines.length) : lines.length;
  return lines.slice(start, end).join("\n").trim();
}

export function parseStdlibFunctions(
  relativePath: string,
): StdlibFunctionDoc[] {
  const source = readRepoText(relativePath);
  const lines = source.split("\n");
  const functions: StdlibFunctionDoc[] = [];

  let commentBuffer: string[] = [];

  for (const line of lines) {
    const trimmed = line.trim();

    if (trimmed.startsWith("#")) {
      commentBuffer.push(trimmed.replace(/^#\s?/, ""));
      continue;
    }

    if (trimmed.startsWith("pub def ")) {
      const signature = trimmed.replace(/^pub /, "");
      const nameMatch = trimmed.match(/^pub def ([a-zA-Z_][a-zA-Z0-9_]*)/);
      const docs = commentBuffer
        .join("\n")
        .split("\n")
        .map((entry) => entry.trim())
        .filter(Boolean);

      const descriptionLines: string[] = [];
      for (const docLine of docs) {
        if (docLine === "Examples:" || docLine.startsWith("Examples:")) {
          break;
        }
        descriptionLines.push(docLine);
      }

      functions.push({
        name: nameMatch?.[1] ?? signature,
        signature,
        description: descriptionLines.join(" "),
      });

      commentBuffer = [];
      continue;
    }

    if (trimmed !== "") {
      commentBuffer = [];
    }
  }

  return functions;
}
