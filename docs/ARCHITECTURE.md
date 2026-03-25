# Architecture

The `main` branch ships **only** the Rust workspace under `crates/`. Older Python/TypeScript prototypes are archived on branch `legacy/v1-python-ts` (see `docs/LEGACY_V1.md`).

## Core idea

Nexus is a **local-first repo fleet triage** CLI: inventory, identity resolution, scoring, planning, and reports—without a web dashboard (see `docs/PRODUCT_STRATEGY.md`, `docs/FAQ.md`).

The system is intentionally split into layers:

1. **Inventory**  
   Discover local repos and remote repos.

2. **Identity Resolution**  
   Determine which clones/remotes are likely the same logical project.

3. **Assessment**  
   Score clusters along several dimensions (see `docs/SCORING.md` for product names vs JSON field names).

4. **Planning**  
   Produce a deterministic action plan without mutating anything.

5. **Presentation (CLI) and optional hooks**  
   Markdown/JSON reports from the CLI. **`nexus serve`** provides a small **experimental** read-only JSON API over SQLite for local scripting; it is secondary to the CLI, not a stable platform surface.

## Workspace crates

### `nexus-core`
Pure domain types and shared enums.

### `nexus-config`
Config file loading and defaults.

### `nexus-db`
SQLite connection and persistence boundary. Full inventory replace (`replace_inventory_snapshot`) backs **`nexus import`** and clears persisted plan rows.

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

### `nexus-tui`
Ratatui-based **read-only** cluster browser (`nexus tui`): sort/filter, evidence list, `nexus.toml` pin snippet, plan JSON export. Same `PlanDocument` as `plan`/`report`; not a dashboard.

### `nexus-adapters`
Optional CLI integrations (jscpd, semgrep, gitleaks, syft) for plan evidence.

### `nexus-ai`
Optional AI-assisted explanations using OpenAI-compatible endpoints. Consumes structured plan output only (grounding contract); never modifies scores, canonical selections, or actions. Requires `ai.enabled = true` in config and `NEXUS_AI_API_KEY` or `OPENAI_API_KEY`. See `docs/CLI.md` for `nexus explain --ai` and `nexus ai-summary`.

### `nexus-api`
Axum HTTP **read-only** API over SQLite (powers **`serve`** only). Experimental and secondary to the CLI; not a dashboard backend.

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
scores + evidence (`nexus score` or inside `nexus plan`)
   ↓
plan.json + persisted plan row (`plan`) / report.md (`report`) / interactive browse (`tui`)
```

## Why SQLite first?

Because Nexus is a local-first CLI and needs:

- easy local persistence
- reproducible runs
- simple export/import
- zero external service dependency

## Why no web UI?

The core risk is not presentation—it is **choosing the wrong canonical repo**. The product prioritizes correct inventory + clustering over a browser UI. A dashboard would also drag the project toward hosted state and platform scope; that is an explicit non-goal (`docs/PRODUCT_STRATEGY.md`). The **TUI** (`nexus tui`) provides interactive inspection; it is not a replacement for the CLI engine.

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
