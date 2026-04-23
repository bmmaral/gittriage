# Architecture

The `main` branch ships **only** the Rust workspace under `crates/`. Older Python/TypeScript prototypes are archived on branch `legacy/v1-python-ts` (see `docs/LEGACY_V1.md`).

## Core idea

GitTriage is a **local-first repo fleet triage** CLI: inventory, identity resolution, scoring, planning, and reports—without a web dashboard (see `docs/PRODUCT_STRATEGY.md`, `docs/FAQ.md`).

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
   Markdown/JSON reports from the CLI. **`gittriage serve`** provides a small **experimental** read-only JSON API over SQLite for local scripting; it is secondary to the CLI, not a stable platform surface.

## Workspace crates

### `gittriage-core`
Pure domain types and shared enums.

### `gittriage-config`
Config file loading and defaults.

### `gittriage-db`
SQLite connection and persistence boundary. Uses WAL mode, `busy_timeout`, and schema versioning (`gittriage_meta` table). Full inventory replace (`replace_inventory_snapshot`) backs **`gittriage import`** and clears persisted plan rows.

### `gittriage-scan`
Filesystem scanning and project metadata extraction. Supports `git_only` (default) and `project_roots` scan modes, `max_depth`, `.gittriageignore` patterns, and automatic stop-at-`.git` to prevent monorepo sub-package noise. Detects SPDX license identifiers, lockfiles, CI configs, and test directories.

### `gittriage-git`
Git metadata collection via system `git` in v1, including upstream tracking metadata (configured upstream branch, ahead/behind counts, and no-upstream state) for drift-aware analysis.

### `gittriage-github`
Remote repository ingest via `gh` CLI in v1. Supports up to 5000 repos per owner with truncation warnings.

### `gittriage-plan`
Clustering, scoring, and action generation.

### `gittriage-report`
Markdown / JSON rendering.

### `gittriage-tui`
Ratatui-based **read-only** cluster browser (`gittriage tui`): sort/filter, evidence list, `gittriage.toml` pin snippet, plan JSON export. Same `PlanDocument` as `plan`/`report`; not a dashboard.

### `gittriage-adapters`
Optional CLI integrations (jscpd, semgrep, gitleaks, syft) for plan evidence.

### `gittriage-ai`
Optional AI-assisted explanations using OpenAI-compatible endpoints. Consumes structured plan output only (grounding contract); never modifies scores, canonical selections, or actions. Requires `ai.enabled = true` in config and `GITTRIAGE_AI_API_KEY` or `OPENAI_API_KEY`. See `docs/CLI.md` for `gittriage explain --ai` and `gittriage ai-summary`.

### `gittriage-api`
Axum HTTP **read-only** API over SQLite (powers **`serve`** only). Binds to `127.0.0.1` by default; loads config once at startup. Experimental and secondary to the CLI; not a dashboard backend.

### `gittriage`
Thin orchestration layer.

## Data flow

```text
roots + gh owner
   ↓
gittriage-scan + gittriage-git + gittriage-github
   ↓
raw inventory persisted in sqlite
   ↓
gittriage-plan resolves clusters
   ↓
scores + evidence (`gittriage score` or inside `gittriage plan`)
   ↓
plan.json + persisted plan row (`plan`) / report.md (`report`) / interactive browse (`tui`)
```

## Why SQLite first?

Because GitTriage is a local-first CLI and needs:

- easy local persistence
- reproducible runs
- simple export/import
- zero external service dependency

## Why no web UI?

The core risk is not presentation—it is **choosing the wrong canonical repo**. The product prioritizes correct inventory + clustering over a browser UI. A dashboard would also drag the project toward hosted state and platform scope; that is an explicit non-goal (`docs/PRODUCT_STRATEGY.md`). The **TUI** (`gittriage tui`) provides interactive inspection; it is not a replacement for the CLI engine.

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
