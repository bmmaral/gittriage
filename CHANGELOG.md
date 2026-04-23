# Changelog

## Unreleased

### Agent / coding-agent surface

- **`gittriage-agent` crate:** deterministic `resolve`, `verdict`, `preflight`, `check-path`, and `summary --agent` helpers with provenance (`generated_at`, `inventory_run_id`, `scope`, `freshness`, `data_sources`) and explicit **`unsafe_for_automation`** on verdict-shaped output.
- **CLI:** `preflight`, `resolve`, `verdict`, `check-path`, and `summary --agent` (shared `--format text|json` and plan flags).
- **`gittriage serve`:** `GET /v2/agent/*` routes mirroring the CLI (contract-tested in `gittriage-api`).
- **TUI:** automation verdict column, canonical path in cluster detail, `v` to cycle all / safe / unsafe / duplicates / local-only views.
- **Report:** unscoped markdown leads with agent-preflight sections (unsafe, duplicates, canonical paths, dirty canon, nested) before per-cluster detail; per-cluster **`### Scores (summary)`** replaces long score narratives (full axis copy via `explain` / `score`). Scoped `--scope` reports keep the classic score + explanations blocks.

### CI

- **rust-ci:** `cargo-deny` runs via `taiki-e/install-action` and `cargo deny check` (avoids Docker-based `cargo-deny-action` runner issues). `deny.toml`: drop unused `OpenSSL` license allow entry (clears `license-not-encountered` noise).
- **rust-ci:** the adapter test harness now uses platform-aware `PATH` joining, so Windows fake `.cmd` shims resolve correctly and the adapter absence suite stays green.
- **regression-dashboard:** a post-`rust-ci` workflow now builds `plan.json`, `score --format json`, and `summary --agent --format json` artifacts plus a combined drift dashboard JSON for CI review.
- **truth benchmark:** a corpus-driven canonical-correctness harness now covers freshness, git metadata, dirty state, path semantics, local-only, and remote-only cases and emits a JSON pass/fail summary for CI artifacts.

- **release:** Optional jobs publish to a Homebrew tap (`HOMEBREW_TAP_TOKEN`), Scoop bucket (`SCOOP_BUCKET_TOKEN`), and Chocolatey (`CHOCOLATEY_API_KEY`) using `packaging/scripts/bump_release_packaging.py`.

### Git metadata

- `gittriage-git` now records upstream tracking metadata (`upstream_branch`, ahead/behind counts, and no-upstream state) in `CloneRecord.upstream_tracking` so repo analysis can reason about branch drift.

## v0.1.1 — 2026-03-26

### Packaging & distribution

- **Breaking (packaging):** Cargo package `gittriage-cli` renamed to **`gittriage`** (same binary name `gittriage`). Chocolatey id is now **`gittriage`**. npm wrapper is published as **`@bmmaral/gittriage`** on **GitHub Packages** (see `docs/DISTRIBUTION.md`).
- **CI / README:** GitHub Packages `npm publish` hardened (auth line in `.npmrc`, `NPM_CONFIG_PROVENANCE=false`). README registry badges use honest labels for crates.io / Chocolatey until those registries list the package.

### Fixes

- macOS x86_64 release artifacts built via cross-compile on `macos-latest` (no flaky `macos-13` runner).
- Homebrew formula source URL aligned with tagged releases; **v0.1.1** tarball SHA-256, AUR `sha256sums`, Scoop `hash`, and Chocolatey `checksum64` match the published GitHub Release assets.

## v0.1.0 — 2026-03-25

Initial public release of **GitTriage** (formerly Nexus).

### Highlights

- **Full rename** from `nexus` to `gittriage` across all crates, binary, docs, and packaging.
- **13-crate Rust workspace**: gittriage-core, gittriage-config, gittriage-db, gittriage-scan, gittriage-git, gittriage-github, gittriage-plan, gittriage-report, gittriage-adapters, gittriage-tui, gittriage-ai, gittriage-api, gittriage (CLI crate / binary).
- **Stable core commands**: `scan`, `score`, `plan`, `report`, `doctor`, `tools`, `export`, `import`, `explain`.
- **Secondary**: `tui` — interactive terminal browser with sort, filter, evidence, pin, export.
- **Experimental**: `ai-summary`, `apply --dry-run`, `serve`.

### Scanner

- `git_only` scan mode (default) prevents monorepo sub-package noise.
- `.gittriageignore` / `.nexusignore` glob patterns for exclusions.
- `max_depth` traversal limit.
- Fast SPDX license sniffing (MIT, Apache-2.0, GPL, BSD, ISC, MPL, Unlicense, etc.).
- Project cue detection: lockfiles, CI configs, test directories.

### Scoring (v5)

- Five-axis deterministic scoring: canonical confidence, repo health, recoverability, publish readiness, maintenance risk.
- Graduated risk scaling for duplicate clones.
- Negative evidence for missing hygiene signals.
- `--profile` flag: `default`, `publish`, `open_source`, `security`, `ai_handoff`.

### Infrastructure

- SQLite with WAL mode, busy_timeout, schema versioning.
- `serve` binds to `127.0.0.1` by default; `--listen` flag for explicit network access.
- Config `db_path` resolves relative to config file location (not cwd).
- Tilde expansion in `db_path`.
- GitHub ingest supports up to 5000 repos with truncation warnings.

### CI/CD

- GitHub Actions: Linux (ubuntu + musl), macOS, Windows, cargo-deny.
- Release workflow builds Linux musl, macOS (arm64 + x86_64), Windows with `.sha256` checksums.
- Security workflow: gitleaks + semgrep.

### Packaging

- Homebrew formula, Scoop manifest, Chocolatey package, npm thin wrapper, AUR PKGBUILD, Nix flake.

### Optional AI

- `gittriage explain --ai` and `gittriage ai-summary` for narrative explanations.
- OpenAI-compatible endpoints; never modifies deterministic scores.
