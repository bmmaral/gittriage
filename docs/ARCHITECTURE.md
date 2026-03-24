# Architecture

## Core idea

Nexus v2 is a **local-first repository fleet intelligence engine**.

The system is intentionally split into layers:

1. **Inventory**  
   Discover local repos and remote repos.

2. **Identity Resolution**  
   Determine which clones/remotes are likely the same logical project.

3. **Assessment**  
   Score canonicality, usability, OSS readiness, and risk.

4. **Planning**  
   Produce a deterministic action plan without mutating anything.

5. **Explanation / API (optional)**  
   Render or expose the results.

## Workspace crates

### `nexus-core`
Pure domain types and shared enums.

### `nexus-config`
Config file loading and defaults.

### `nexus-db`
SQLite connection and persistence boundary.

### `nexus-scan`
Filesystem scanning and project metadata extraction.

### `nexus-git`
Git metadata collection via system `git` in v1.

### `nexus-github`
Remote repository ingest via `gh` CLI in v1.

### `nexus-plan`
Clustering, scoring, and action generation.

### `nexus-report`
Markdown / JSON rendering.

### `nexus-adapters`
Optional CLI integrations (jscpd, semgrep, gitleaks, syft) for plan evidence.

### `nexus-api`
Axum HTTP read-only API over SQLite-backed inventory / plan.

### `nexus-cli`
Thin orchestration layer.

## Data flow

```text
roots + gh owner
   ↓
nexus-scan + nexus-git + nexus-github
   ↓
raw inventory persisted in sqlite
   ↓
nexus-plan resolves clusters
   ↓
scores + evidence
   ↓
plan.json + report.md
```

## Why SQLite first?

Because Nexus is a local-first CLI and needs:

- easy local persistence
- reproducible runs
- simple export/import
- zero external service dependency

## Why no web UI in v1?

The core risk is not presentation.  
The core risk is deciding the wrong canonical repo and sending an agent after it.

Therefore correctness of inventory + clustering beats UI.

## Native build dependencies

The workspace depends on **rusqlite** with bundled SQLite, which compiles C code during the build. Use a normal platform toolchain (Xcode CLT on macOS, GCC/Clang on Linux). Linux **musl** release binaries are produced in CI with `musl-tools` (`x86_64-linux-musl-gcc`).

## Safety model

v1 never:
- deletes repos
- moves repos
- archives remotes
- commits code
- opens PRs

It only produces a plan.
