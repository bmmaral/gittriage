# Changelog

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
