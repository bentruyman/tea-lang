import { readFile } from "node:fs/promises";
import path from "node:path";

export const dynamic = "force-static";

export async function GET() {
  const scriptPath = path.resolve(process.cwd(), "..", "scripts", "install.sh");
  const contents = await readFile(scriptPath, "utf8");

  return new Response(contents, {
    headers: {
      "content-type": "application/x-sh; charset=utf-8",
      "cache-control": "public, max-age=300, s-maxage=300",
    },
  });
}
