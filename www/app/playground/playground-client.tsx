"use client";

import { startTransition, useEffect, useRef, useState } from "react";
import {
  AlertTriangle,
  LoaderCircle,
  Play,
  RotateCcw,
  Sparkles,
} from "lucide-react";

import { TeaEditor } from "@/components/playground/tea-editor";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";

type PlaygroundDiagnostic = {
  message: string;
  level: string;
  span?: {
    line: number;
    column: number;
    endLine: number;
    endColumn: number;
  } | null;
};

type RunnerPayload = {
  diagnostics: PlaygroundDiagnostic[];
  stdout: string[];
  result?: string | null;
  runtimeError?: string | null;
  exitCode?: number | null;
};

type WorkerResponse =
  | { type: "result"; id: number; payload: RunnerPayload }
  | { type: "error"; id: number; error: string };

const presets = [
  {
    id: "structs",
    title: "Structs + Loops",
    summary:
      "A small browser-safe program with a struct constructor and list iteration.",
    source: `struct User {
  name: String
  age: Int
}

var user = User(name: "Ada", age: 37)
var total = 0

for value in [1, 2, 3, 4]
  total = total + value
end

@println(user.name)
@println(total)`,
  },
  {
    id: "strings",
    title: "String Tools",
    summary:
      "Pure Tea stdlib code expanded in-browser from the checked-in stdlib source.",
    source: `use string = "std.string"

var headline = "tea in the browser"

@println(string.to_upper(headline))
@println(string.replace(headline, "browser", "worker"))`,
  },
  {
    id: "json",
    title: "JSON Decode",
    summary:
      "Runs the browser-safe JSON intrinsic path without touching the filesystem.",
    source: `use json = "std.json"

var payload = json.decode("{\\"name\\":\\"Tea\\",\\"scores\\":[1,2,3]}")

@println(payload["name"])
@println(@len(payload["scores"]))`,
  },
];

export function PlaygroundClient() {
  const initialPreset = presets[0];
  const workerRef = useRef<Worker | null>(null);
  const requestIdRef = useRef(0);
  const activeRequestRef = useRef(0);

  const [selectedPreset, setSelectedPreset] = useState(initialPreset.id);
  const [source, setSource] = useState(initialPreset.source);
  const [stdout, setStdout] = useState<string[]>([]);
  const [diagnostics, setDiagnostics] = useState<PlaygroundDiagnostic[]>([]);
  const [runtimeError, setRuntimeError] = useState<string | null>(null);
  const [result, setResult] = useState<string | null>(null);
  const [status, setStatus] = useState<
    "loading" | "ready" | "running" | "error"
  >("loading");
  const [workerError, setWorkerError] = useState<string | null>(null);

  useEffect(() => {
    const worker = new Worker(
      new URL("./tea-runner.worker.ts", import.meta.url),
      {
        type: "module",
      },
    );
    workerRef.current = worker;
    worker.postMessage({
      type: "configure",
      assetBaseUrl: window.location.origin,
    });
    setStatus("ready");

    worker.onmessage = (event: MessageEvent<WorkerResponse>) => {
      const message = event.data;
      if (message.id !== activeRequestRef.current) {
        return;
      }

      if (message.type === "error") {
        startTransition(() => {
          setStatus("error");
          setWorkerError(message.error);
        });
        return;
      }

      startTransition(() => {
        setDiagnostics(message.payload.diagnostics);
        setStdout(message.payload.stdout);
        setRuntimeError(message.payload.runtimeError ?? null);
        setResult(message.payload.result ?? null);
        setWorkerError(null);
        setStatus("ready");
      });
    };

    worker.onerror = (event) => {
      setStatus("error");
      setWorkerError(event.message || "The Tea worker crashed.");
    };

    return () => {
      worker.terminate();
      workerRef.current = null;
    };
  }, []);

  function resetOutput() {
    setStdout([]);
    setDiagnostics([]);
    setRuntimeError(null);
    setResult(null);
    setWorkerError(null);
  }

  function selectPreset(presetId: string) {
    const preset = presets.find((item) => item.id === presetId);
    if (!preset) {
      return;
    }

    startTransition(() => {
      setSelectedPreset(preset.id);
      setSource(preset.source);
      resetOutput();
    });
  }

  function runSource() {
    if (!workerRef.current) {
      setStatus("error");
      setWorkerError("The Tea worker is not available.");
      return;
    }

    const id = requestIdRef.current + 1;
    requestIdRef.current = id;
    activeRequestRef.current = id;
    setStatus("running");
    setWorkerError(null);
    setRuntimeError(null);

    workerRef.current.postMessage({
      type: "run",
      id,
      payload: {
        entryPath: "/playground.tea",
        files: {
          "/playground.tea": source,
        },
        fuel: 25000,
      },
    });
  }

  return (
    <div className="mx-auto flex max-w-7xl flex-col gap-8 px-4 py-8 md:px-6 md:py-12">
      <section className="section-band surface-feature texture-grid overflow-hidden rounded-[2rem] px-6 py-8 md:px-10 md:py-10">
        <div className="grid gap-8 lg:grid-cols-[minmax(0,1.1fr)_minmax(18rem,0.9fr)]">
          <div className="space-y-5">
            <p className="text-xs font-semibold uppercase tracking-[0.28em] text-primary">
              WASM Playground
            </p>
            <h1 className="font-display text-4xl font-semibold tracking-tight text-balance md:text-5xl">
              Run browser-safe Tea without leaving the docs site.
            </h1>
            <p className="max-w-2xl text-base leading-7 text-muted-foreground md:text-lg md:leading-8">
              This runner compiles Tea&apos;s front-end to WebAssembly,
              evaluates the AST in a browser-safe interpreter, and rejects
              native-only modules like <code>std.fs</code>,
              <code>std.process</code>, and <code>std.env</code>.
            </p>
          </div>

          <div className="surface-card flex flex-col gap-4 rounded-[1.5rem] border border-border/70 p-5">
            <div className="flex items-center gap-3 text-sm font-semibold text-foreground">
              <Sparkles className="h-4 w-4 text-primary" />
              Browser target rules
            </div>
            <ul className="space-y-2 text-sm leading-6 text-muted-foreground">
              <li>
                Supports Tea core syntax, loops, structs, lists, dicts, strings,
                and JSON.
              </li>
              <li>
                Stdlib coverage is limited to browser-safe paths used by this
                runner.
              </li>
              <li>
                Execution is capped with an interpreter fuel limit to stop
                runaway loops.
              </li>
              <li>
                Build the wasm assets with <code>bun run build:tea-wasm</code>{" "}
                inside <code>www/</code>.
              </li>
            </ul>
          </div>
        </div>
      </section>

      <section className="grid gap-6 lg:grid-cols-[minmax(0,1.1fr)_minmax(20rem,0.9fr)]">
        <Card className="surface-card gap-5 rounded-[1.8rem] border border-border/70 p-4 md:p-5">
          <div className="flex flex-wrap items-center justify-between gap-3">
            <div>
              <p className="text-xs font-semibold uppercase tracking-[0.24em] text-primary">
                Editor
              </p>
              <h2 className="mt-2 font-display text-3xl font-semibold tracking-tight">
                Playground source
              </h2>
            </div>
            <div className="flex flex-wrap items-center gap-2">
              <Button
                size="lg"
                className="rounded-full px-6 font-semibold"
                onClick={runSource}
              >
                {status === "running" ? (
                  <>
                    <LoaderCircle className="h-4 w-4 animate-spin" />
                    Running
                  </>
                ) : (
                  <>
                    <Play className="h-4 w-4" />
                    Run Tea
                  </>
                )}
              </Button>
              <Button
                variant="outline"
                size="lg"
                className="rounded-full px-6 font-semibold"
                onClick={() => selectPreset(initialPreset.id)}
              >
                <RotateCcw className="h-4 w-4" />
                Reset
              </Button>
            </div>
          </div>

          <div className="grid gap-3 md:grid-cols-3">
            {presets.map((preset) => (
              <button
                key={preset.id}
                type="button"
                onClick={() => selectPreset(preset.id)}
                className={`rounded-[1.25rem] border p-4 text-left transition-colors ${
                  preset.id === selectedPreset
                    ? "border-primary/30 bg-primary/8"
                    : "border-border/70 bg-background/60 hover:border-primary/20"
                }`}
              >
                <p className="text-sm font-semibold text-foreground">
                  {preset.title}
                </p>
                <p className="mt-2 text-sm leading-6 text-muted-foreground">
                  {preset.summary}
                </p>
              </button>
            ))}
          </div>

          <TeaEditor value={source} onChange={setSource} />
        </Card>

        <div className="grid gap-6">
          <Card className="surface-card gap-4 rounded-[1.8rem] border border-border/70 p-5">
            <div className="flex items-center justify-between gap-4">
              <div>
                <p className="text-xs font-semibold uppercase tracking-[0.24em] text-primary">
                  Runtime
                </p>
                <h2 className="mt-2 font-display text-3xl font-semibold tracking-tight">
                  Output
                </h2>
              </div>
              <div className="rounded-full border border-border/70 px-3 py-1 text-xs font-semibold uppercase tracking-[0.22em] text-muted-foreground">
                {status}
              </div>
            </div>

            <div className="min-h-[14rem] rounded-[1.35rem] border border-border/70 bg-background/70 p-4 font-mono text-sm leading-6 text-foreground">
              {stdout.length > 0 ? (
                <pre className="whitespace-pre-wrap">{stdout.join("")}</pre>
              ) : (
                <p className="text-muted-foreground">
                  Run a preset or edit the source to see stdout here.
                </p>
              )}
              {result ? (
                <p className="mt-4 text-primary">Result: {result}</p>
              ) : null}
            </div>

            {runtimeError ? (
              <div className="warning-panel rounded-[1.25rem] border p-4 text-sm leading-6">
                <div className="flex items-center gap-2 font-semibold">
                  <AlertTriangle className="h-4 w-4" />
                  Runtime error
                </div>
                <p className="mt-2 font-mono text-[0.85rem]">{runtimeError}</p>
              </div>
            ) : null}

            {workerError ? (
              <div className="rounded-[1.25rem] border border-destructive/30 bg-destructive/10 p-4 text-sm leading-6 text-destructive">
                <p className="font-semibold">Worker error</p>
                <p className="mt-2 font-mono text-[0.85rem]">{workerError}</p>
              </div>
            ) : null}
          </Card>

          <Card className="surface-card gap-4 rounded-[1.8rem] border border-border/70 p-5">
            <div>
              <p className="text-xs font-semibold uppercase tracking-[0.24em] text-primary">
                Compiler
              </p>
              <h2 className="mt-2 font-display text-3xl font-semibold tracking-tight">
                Diagnostics
              </h2>
            </div>

            {diagnostics.length > 0 ? (
              <div className="space-y-3">
                {diagnostics.map((diagnostic, index) => (
                  <div
                    key={`${diagnostic.message}-${index}`}
                    className="rounded-[1.25rem] border border-border/70 bg-background/70 p-4"
                  >
                    <div className="flex items-center justify-between gap-3">
                      <p className="text-sm font-semibold uppercase tracking-[0.18em] text-primary">
                        {diagnostic.level}
                      </p>
                      {diagnostic.span ? (
                        <p className="font-mono text-xs text-muted-foreground">
                          {diagnostic.span.line}:{diagnostic.span.column}
                        </p>
                      ) : null}
                    </div>
                    <p className="mt-2 text-sm leading-6 text-foreground">
                      {diagnostic.message}
                    </p>
                  </div>
                ))}
              </div>
            ) : (
              <div className="rounded-[1.25rem] border border-dashed border-border/80 p-4 text-sm leading-6 text-muted-foreground">
                Browser-target diagnostics appear here. Native-only modules and
                unsupported control flow are rejected before execution.
              </div>
            )}
          </Card>
        </div>
      </section>
    </div>
  );
}
