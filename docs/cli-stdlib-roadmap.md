# CLI Standard Library Roadmap

This roadmap captures how Tea's standard library will evolve to serve CLI-first
workflows. It builds on the existing runtime-backed modules (`std.debug`,
`std.assert`, `std.util`, `std.fs`, `std.io`, `std.json`, `std.yaml`,
`support.cli`) and identifies the missing capabilities that CLI authors need
before they can rely on Tea for production tooling.

## Design Goals

- **Fail-fast tooling** – keep diagnostics rich and surface errors before
  executing user code.
- **Batteries-included core** – ship the most common filesystem, process, IO,
  and configuration helpers directly in `tea-runtime` so CLIs do not require
  vendor-specific shims.
- **Composable primitives** – expose streaming APIs that interoperate with the
  pipeline-and-snapshot testing story adopted in `tea test`.
- **Cross-platform guardrails** – document platform quirks up front and provide
  predictable behaviour when a feature is not supported.
- **Extensible surface** – reserve space for Tea-level packages to layered
  higher-level abstractions (templating, HTTP clients) without bloating the core
  runtime.

## Current Foundation

| Module          | Status | Notes |
| --------------- | ------ | ----- |
| `std.debug`     | ✅     | thin wrapper around runtime print helpers |
| `std.assert`    | ✅     | includes equality, failure, and snapshot assertions |
| `std.util`      | ✅     | predicates and conversions (type checks, `to_string`, `clamp_int`) |
| `std.fs`        | ✅ (narrow) | text/byte read & write, metadata basics |
| `std.path`      | ✅     | join/split, normalization, absolute/relative helpers |
| `std.env`       | ✅     | env var access, cwd changes, temp/home/config directory helpers |
| `std.io`        | ✅     | buffered stdin read, stdout/stderr write & flush |
| `std.json`      | ✅     | encode/decode with type-checker-aware literals |
| `std.yaml`      | ✅     | encode/decode parity with JSON |
| `support.cli`   | ✅     | argument access & parsing, snapshot capture (VM-only) |
| `std.process`   | ✅ (initial) | synchronous `run`, `spawn` + `wait`, streaming stdout/stderr helpers |

Gaps remain around richer filesystem traversal, process spawning, environment
management, piping, configuration, and network access.

## Roadmap Milestones

### Milestone A – Core CLI Runtime Essentials (tea-runtime)

Objective: close the must-have gaps for typical command-line tools.

- **`std.fs` expansion** – globbing, recursive directory iteration, atomic write
  helpers, permissions, symlink support. Initial surface adds
  `glob(pattern)`, `list_dir(path)`, `walk(path)`, `metadata(path)`,
  `is_symlink(path)`, `permissions(path)`, `write_text_atomic(path, contents)`,
  `write_bytes_atomic(path, bytes)`, `ensure_dir(path)`, and
  `ensure_parent(path)`. (tea-runtime)
- **`std.path` module** – join/split, normalization, relative/absolute handling
  with platform awareness. (tea-runtime)
- **`std.env` module** – get/set env vars, current dir, home detection, temp
  directories. (tea-runtime)
- **`std.process` module** – spawn, wait, exit codes, streamed stdin/stdout
  handles with lazy readers/writers. (tea-runtime, integrates with `support.cli`)
- **Diagnostics** – ensure new APIs surface Tea-friendly errors (spans,
  `Result`-like values).

Deliverables:
- Runtime implementations with VM & LLVM parity.
- Examples under `examples/stdlib/cli/` (e.g., dir walker, process runner).
- Tests covering success/failure paths, including snapshot-driven CLI checks.

### Milestone B – Streaming & Pipelining Primitives (tea-runtime)

Objective: embrace CLI composability and pipelines.

- **`std.process` streaming** – attach asynchronous-ish readers/writers,
  non-blocking reads, timeouts.
- **`std.io.stream` utilities** – chunked iterators, `each_line`, split/pipe
  helpers, binary stream adapters.
- **`support.cli.capture` LLVM parity** – allow compiled binaries to reuse the
  capture harness.
- **Sandboxed execution helpers** – wrappers that execute subcommands with
  working directory/env overrides.

Deliverables:
- End-to-end pipeline example (`examples/stdlib/cli/pipeline.tea`) built via VM & LLVM.
- Snapshot tests validating stdout/stderr streaming.

### Milestone C – Configuration & Secrets (tea-runtime + packages)

Objective: make it simple to load configuration safely.

- **`std.config`** – read `.env`, TOML, YAML, JSON with override precedence.
  (tea-runtime for parser glue; complex merges can live in a Tea package.)
- **`std.secret`** – typed wrappers for sensitive values with redaction-aware
  printing. (tea-runtime)
- **Credential providers** – optional Tea package for platform integrations
  (macOS/iOS keychain, Windows credential store, etc.).

Deliverables:
- Documentation and samples for layering runtime helpers with user-land
  packages.
- Tests ensuring secrets do not leak via default debug/print paths.

### Milestone D – Networking & Remote Calls (staged)

Objective: enable CLIs that call remote services.

- **`std.net` (tea-runtime)** – TCP/UDP clients, DNS resolution, timeouts,
  minimal async-friendly design (blocking for now).
- **`std.http` (tea-runtime)** – simple GET/POST with headers, streaming bodies,
  JSON convenience bridging to `std.json`.
- Advanced features (HTTP/2, TLS customization, retry policies) graduate to
  Tea-level packages (e.g., `support.http`).

Deliverables:
- Integration tests hitting local echo servers.
- Docs highlighting security defaults, TLS requirements, and how to replace the
  runtime transport.

### Milestone E – Higher-Level Tooling (Tea packages)

Objective: grow the ecosystem without bloating the core runtime.

- **Templating/rendering** – `support.template` Tea package built on runtime IO.
- **Release tooling** – package bundling, install scripts (building atop
  `tea build` improvements).
- **Testing utilities** – CLI golden-file harnesses, structured log assertions,
  watchers (likely Tea packages using existing runtime APIs).

## Cross-Cutting Concerns

- Every new module must land with VM + LLVM coverage, documentation, examples,
  and snapshot-friendly tests.
- Shared diagnostics live under `tea-support`; use the helpers so `std.fs`,
  `std.io`, and `support.cli` surface consistent error messages across VM and
  LLVM backends.
- Platform-specific behaviour must be documented in-module (`docs/`, README)
  with fallbacks or clear unsupported errors.
- Keep `support.cli` aligned with new modules (e.g., allow `std.process` to feed
  into capture utilities).
- Track future enhancements via dedicated issues (e.g., pointer deprecation
  cleanup, richer typing for decoded data).

## Next Steps

1. Prioritise Milestone A issues (`std.fs` expansion) and assign owners.
2. Use Milestone B to wire the new process/streaming APIs into the snapshot test
   harness.
3. Prepare design spikes for configuration and networking to validate runtime
  feasibility before committing to implementation timelines.
