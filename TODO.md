# Nexus TODO

Nexus is a **Rust-first repository fleet intelligence CLI** (`crates/`). This file tracks **remaining** work toward a mature OSS project.

## Product decisions

- [x] **Main branch is Rust-only.**
  - The Rust workspace under `crates/` is the product.
  - Python/TypeScript code is not maintained on `main`.

- [x] **Legacy v1 is archived, not co-developed.**
  - Tag `legacy-py-mvp`: run `scripts/tag-legacy-python.sh` on the tip of `legacy/v1-python-ts`, then `git push origin legacy-py-mvp`.
  - Archival branch: `legacy/v1-python-ts` (created at the last commit that still contained the legacy tree).
  - `docs/LEGACY_V1.md` describes migration and links.

- [x] **Nexus v2 remains read-only by default.**
  - `scan`, `plan`, `report`, `doctor`, `tools`, `serve` (experimental).
  - No destructive mutation in stable releases until explicitly designed.

---

## P0 — Repository cleanup and product boundary

### Remove legacy runtime/code from `main`
- [x] Remove `cli/`
- [x] Remove `web/`
- [x] Remove `nexus.py`
- [x] Remove `server.py`
- [x] Remove `package.json`
- [x] Remove `package-lock.json`
- [x] Remove `requirements.txt`
- [x] Remove `demo.js`

### Remove committed runtime artifacts
- [x] Remove `.nexus/` from version control
- [x] Remove `__pycache__/` from version control
- [x] Remove `conversations/` from version control
- [x] Verify `.gitignore` prevents all local state / generated files from reappearing

### Remove legacy GitHub Actions
- [x] Delete `.github/workflows/analyze.yml`
- [x] Delete `.github/workflows/build.yml`
- [x] Delete `.github/workflows/nexus.yml`
- [x] Delete `.github/workflows/nightly-health.yml`
- [x] Delete `.github/workflows/reminder.yml`
- [x] Delete `.github/workflows/stale-branches.yml`
- [x] Delete `.github/workflows/weekly-summary.yml`

### Fix stale docs
- [x] Delete or fully rewrite `IMPLEMENTATION_STATUS.md` (removed)
- [x] Remove any references to the old “project memory / PRD sync” product where no longer relevant (`README`, `prd.md` note, docs aligned)
- [x] Ensure `README.md`, `docs/ARCHITECTURE.md`, `docs/CLI.md`, `docs/SCORING.md`, and `TODO.md` all describe the same product and command surface

---

## P1 — OSS correctness and release readiness

### Project metadata
- [x] Add a real `LICENSE` file at repo root
- [x] Add `CONTRIBUTING.md`
- [x] Add `CODE_OF_CONDUCT.md`
- [x] Add `CHANGELOG.md`
- [x] Fill in GitHub repo description
- [x] Add GitHub topics
- [x] Add homepage/docs link when ready

### Documentation quality
- [x] Expand `README.md` with:
  - [x] 30-second product pitch
  - [x] before/after problem statement
  - [x] install instructions
  - [x] example workflow (`scan -> plan -> report`)
  - [x] sample output
  - [x] limitations / non-goals
- [x] Expand `docs/SCORING.md` with concrete examples of evidence and weights
- [x] Add `docs/PLAN_SCHEMA.md` (aligned with `PlanDocument` / golden fixture)
- [x] Add `docs/CONFIG.md`
- [x] Add `docs/EXTERNAL_TOOLS.md` for `gh`, `jscpd`, `semgrep`, `gitleaks`, `syft`

### Command/API stability
- [x] Freeze v0 command names and flags (documented in `docs/CLI.md`)
- [x] Define JSON schema/versioning for `plan.json` (`schema_version`, `docs/PLAN_SCHEMA.md`)
- [x] Define markdown report sections and keep them stable (`docs/CLI.md`)
- [x] Decide whether `serve` is public/stable or experimental (**experimental**)
- [x] Add `--format json` / machine-readable outputs consistently where missing (`report --format json`, `plan --write` JSON; no additional commands required for v0)

---

## P2 — Core engine hardening

### Inventory and identity resolution
- [ ] Add stronger remote URL normalization rules
- [ ] Add merge-base evidence as a first-class scoring signal
- [ ] Add support for manual cluster pinning / overrides in config
- [ ] Add better duplicate-detection heuristics beyond name matching
- [ ] Add protection against false canonical selection in ambiguous clusters

### Scoring and planning
- [ ] Make every score explanation mandatory in report output
- [ ] Version scoring rules separately from app version if needed
- [ ] Add planner rule tests for:
  - [ ] canonical clone selection
  - [ ] remote-only projects
  - [ ] local-only projects
  - [ ] ambiguous duplicate clusters
  - [ ] stale-but-important repos
- [ ] Add explicit “why not selected as canonical” evidence for non-canonical clones

### External tool adapters
- [ ] Add timeouts and graceful degradation for external tools
- [ ] Cache adapter outputs per run
- [ ] Normalize adapter evidence into a shared schema
- [ ] Decide which adapters are “officially supported” vs “best effort”

---

## P3 — Testing and QA

### Rust tests
- [x] Add end-to-end integration test for:
  - [x] `scan`
  - [x] `plan`
  - [x] `report`
- [ ] Add fixture-based tests for local+remote cluster resolution
- [x] Add snapshot tests for markdown reports (`nexus-report` insta test)
- [ ] Add regression tests for path normalization and config precedence
- [x] Add tests for external adapter absence/failure cases (existing adapter tests)

### CI/CD
- [x] Keep `rust-ci.yml`, `security.yml`, and `release.yml` only (plus `cargo-deny` job in `rust-ci.yml`)
- [x] Extend CI to verify:
  - [x] `cargo fmt --check`
  - [x] `cargo clippy -- -D warnings`
  - [x] `cargo test --workspace`
  - [ ] docs/examples compile if applicable
- [x] Widen Semgrep scope after legacy code is removed
- [x] Add `cargo deny` or equivalent dependency/license checks
- [ ] Decide on Windows/macOS release support beyond current Linux-musl release

---

## P4 — Packaging and distribution

- [ ] Publish first tagged release with release notes
- [ ] Provide install instructions for:
  - [ ] cargo
  - [ ] prebuilt binary
- [ ] Decide on package channels:
  - [ ] Homebrew tap
  - [ ] Scoop
  - [ ] Arch/AUR
  - [ ] Nix
- [ ] Add shell completions
- [ ] Add man page / `--help` polish

---

## P5 — UX polish

- [ ] Improve `doctor` output with actionable remediation
- [ ] Improve `report` readability for large inventories
- [ ] Add `nexus explain <cluster|repo|clone>`
- [ ] Add clear warning language for ambiguous plans
- [ ] Add sample screenshots / terminal demos in docs

---

## P6 — Optional but valuable

- [x] Keep `serve` read-only and clearly marked if experimental
- [ ] Add a minimal TUI only if it helps inspect plans locally
- [ ] Add OpenClaw handoff/export format as a documented integration
- [ ] Add import/export of inventory state for cross-machine analysis

---

## Explicit non-goals for now

These are **policies**, not pending tasks:

- No automatic deletion/move/archive of repos
- No automatic commits or PR creation
- No revival of the legacy Python/TS app on `main`
- No heavy web UI before the scoring/planning engine is unquestionably reliable
