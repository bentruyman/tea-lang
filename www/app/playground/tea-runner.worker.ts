type RunTeaRequest = {
  entryPath: string;
  files: Record<string, string>;
  fuel?: number;
};

type RunTeaResponse = {
  diagnostics: {
    message: string;
    level: string;
    span?: {
      line: number;
      column: number;
      endLine: number;
      endColumn: number;
    } | null;
  }[];
  stdout: string[];
  result?: string | null;
  runtimeError?: string | null;
  exitCode?: number | null;
};

type WorkerRequest =
  | {
      type: "configure";
      assetBaseUrl: string;
    }
  | {
      type: "run";
      id: number;
      payload: RunTeaRequest;
    };

type WorkerResponse =
  | { type: "result"; id: number; payload: RunTeaResponse }
  | { type: "error"; id: number; error: string };

type TeaWebModule = {
  default(input?: string): Promise<void>;
  run_tea(input: RunTeaRequest): RunTeaResponse;
};

let modulePromise: Promise<TeaWebModule> | null = null;
let assetBaseUrl = "";

function resolveAssetUrl(path: string) {
  const normalizedBaseUrl =
    assetBaseUrl ||
    (typeof self.location?.origin === "string" ? self.location.origin : "");

  if (!normalizedBaseUrl || normalizedBaseUrl === "null") {
    throw new Error("Tea WASM asset base URL is not configured.");
  }

  return new URL(path, normalizedBaseUrl).toString();
}

async function loadModule() {
  if (!modulePromise) {
    modulePromise = (async () => {
      const importPath = resolveAssetUrl("/tea-web/pkg/tea_web.js");
      const wasmPath = resolveAssetUrl("/tea-web/pkg/tea_web_bg.wasm");
      try {
        const module = (await import(
          /* webpackIgnore: true */ importPath
        )) as TeaWebModule;
        await module.default(wasmPath);
        return module;
      } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        throw new Error(
          `Tea WASM assets are missing or failed to load. Run \`bun run build:tea-wasm\` in \`www/\` and restart the docs app. Original error: ${message}`,
        );
      }
    })();
  }

  return modulePromise;
}

self.onmessage = async (event: MessageEvent<WorkerRequest>) => {
  if (event.data.type === "configure") {
    assetBaseUrl = event.data.assetBaseUrl;
    return;
  }

  if (event.data.type !== "run") {
    return;
  }

  try {
    const module = await loadModule();
    const payload = module.run_tea(event.data.payload);
    self.postMessage({
      type: "result",
      id: event.data.id,
      payload,
    } satisfies WorkerResponse);
  } catch (error) {
    self.postMessage({
      type: "error",
      id: event.data.id,
      error: error instanceof Error ? error.message : String(error),
    } satisfies WorkerResponse);
  }
};

export {};
