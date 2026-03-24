# Nexus TODO

Nexus is a **Rust-first, local-first repo fleet triage CLI**. The product should stay lightweight, deterministic, and useful without AI.

This TODO tracks the work needed to harden the CLI, formalize the scoring model, add an optional TUI, and ship broad binary distribution without drifting into a web-platform product.

## Product decisions

- [x] **Main branch is Rust-only.**
  - The Rust workspace under `crates/` is the product.
  - Python/TypeScript code is not maintained on `main`.

- [x] **Legacy v1 is archived, not co-developed.**
  - Tag `legacy-py-mvp` exists for the legacy era.
  - Archival branch: `legacy/v1-python-ts`.
  - `docs/LEGACY_V1.md` documents the migration.

- [x] **Nexus remains local-first and read-only by default.**
  - Stable direction: `scan`, `score`, `plan`, `report`, `doctor`, `tools`.
  - No destructive mutation in stable releases until explicitly designed.

- [x] **No web dashboard.**
  - The secondary interface, if needed, is a **minimal TUI**, not a browser app.

- [x] **AI is optional.**
  - Nexus must run correctly and usefully without AI.
  - AI may explain and suggest; it must not own the core scoring logic.

---

## P0 — Product boundary and command surface

### Freeze the v1 product shape
- [x] Confirm the stable core command set:
  - [x] `scan`
  - [x] `score`
  - [x] `plan`
  - [x] `report`
  - [x] `doctor`
  - [x] `tools`
- [x] Decide whether `apply --dry-run` remains as a preview command or is folded into `plan`/`report` (**stays** as the v1 preview; documented in `docs/CLI.md`)
- [x] Decide whether `serve` remains experimental only, moves behind a feature flag, or is removed entirely (**remains experimental**; may add a feature flag later—documented in `docs/CLI.md`)
- [x] Remove/avoid any docs language that makes Nexus sound like a web platform or long-running service (README, ARCHITECTURE, CLI, FAQ)

### Align docs with the chosen strategy
- [x] Add `docs/PRODUCT_STRATEGY.md` (present in repo)
- [x] Update `README.md` to reflect:
  - [x] repo fleet triage positioning
  - [x] no dashboard
  - [x] optional TUI
  - [x] optional AI
  - [x] default scoring model and optional profiles
- [x] Update `docs/CLI.md` to reflect the stable core and planned next-layer commands
- [x] Update `docs/ARCHITECTURE.md` to ensure any API/server components are clearly secondary or experimental
- [x] Replace “OSS compatibility/readiness” wording where appropriate with **Publish Readiness** and **Open Source Readiness** profiles (`docs/SCORING.md`)

---

## P1 — Scoring system hardening

### Finalize default scores
- [x] Lock the default score set (JSON: `canonical`, `usability`, `recoverability`, `oss_readiness` / publish readiness, `risk`):
  - [x] Canonical Confidence
  - [x] Repo Health
  - [x] Recoverability
  - [x] Maintenance Risk
- [x] Document each default score with:
  - [x] exact intent
  - [x] evidence signals
  - [x] weight rationale (see `crates/nexus-plan/src/scoring.rs` constants / comments + `docs/SCORING.md`)
  - [ ] failure modes / blind spots (expand over time)

### Finalize optional profiles
- [ ] Define **Publish Readiness** profile
- [ ] Define **Open Source Readiness** profile
- [ ] Define **Security / Supply-Chain Posture** profile
- [ ] Define **AI Handoff Readiness** profile
- [ ] Ensure optional profiles do not distort the default headline experience

### Evidence quality
- [x] Add merge-base evidence as a first-class signal for canonical confidence (contributes to canonical score + evidence; see `merge_base` kind)
- [x] Add stronger duplicate-detection heuristics beyond name matching (shared `fingerprint` across clusters; same display name split across clusters)
- [x] Add explicit evidence for why a clone was **not** selected as canonical (`not_canonical_clone`)
- [x] Add protection against false canonical selection in ambiguous clusters (`ambiguous_cluster` evidence + report **Warnings**)
- [x] Make score explanations mandatory in report output (`### Score explanations` in markdown)
- [x] Version scoring rules separately from app version (`PlanDocument.scoring_rules_version`, `nexus_plan::SCORING_RULES_VERSION`)

---

## P2 — Planning engine and action quality

### Deterministic planning
- [x] Ensure every plan action includes:
  - [x] reason
  - [x] evidence summary (optional JSON field `evidence_summary`, populated for key actions)
  - [x] confidence (optional `confidence` on `PlanAction`)
  - [x] risk/trade-off note (optional `risk_note` on `PlanAction`)
- [ ] Add explicit handling for:
  - [x] remote-only repos (`CloneLocalWorkspace` action + `remote_only_cluster` evidence)
  - [x] local-only repos (`CreateRemoteRepo` + `no_remote_linked` / `local_only_cluster` evidence)
  - [x] pivoted repos (heuristic: `duplicate_name_split_clusters` when same display name maps to different clusters / remotes)
  - [x] stale-but-important repos (`stale_but_artifacted` when old last commit but manifest+README present)
  - [x] ambiguous duplicate clusters (`name_bucket_duplicate_cluster` for multi-clone name-only buckets; cross-cluster hints above)
- [ ] Decide whether plan priorities are global, profile-based, or both

### Overrides and user intent
- [ ] Add manual cluster pinning / canonical overrides in config
- [ ] Add ignore/archive hints without performing destructive actions
- [ ] Decide how user overrides affect score computation and evidence display

---

## P3 — CLI and UX

### Core CLI polish
- [x] Ensure `scan -> score -> plan -> report` is the documented golden path (`README.md`, `docs/CLI.md`)
- [x] Add consistent machine-readable output options where needed (`doctor --format json`, `apply --dry-run --format json`; `score` / `report` / `export` already JSON-capable)
- [x] Improve `doctor` output with actionable remediation
- [x] Improve `report` readability for large inventories (score labels, explanations, action sub-bullets)
- [x] Add clear warning language for ambiguous plans and low-confidence scores (report **Warnings** section)

### New commands
- [x] Add `nexus explain` (`cluster` | `clone` | `remote`; text/json)
  - [x] deterministic explanation without AI
  - [ ] optional AI-enhanced explanation when configured
- [x] Add `nexus export`
- [x] Add `nexus import` for saved inventory state / cross-machine comparison
- [ ] Decide whether `nexus suggest` ships in v1.x or later

### TUI
- [ ] Design a minimal TUI over the same engine
- [ ] Scope the first TUI release to:
  - [ ] cluster browsing
  - [ ] score sorting/filtering
  - [ ] canonical evidence inspection
  - [ ] manual pinning/override
  - [ ] plan preview/export
- [ ] Ensure the TUI does not become a pseudo-dashboard

---

## P4 — Adapter ecosystem

### External tools
- [x] Add timeouts and graceful degradation for external tools
- [ ] Cache adapter outputs per run
- [ ] Normalize adapter evidence into a shared schema
- [ ] Decide which adapters are officially supported vs best effort

### Profile integration
- [ ] Wire adapters cleanly into optional profiles, not the default happy path
- [ ] Ensure missing adapters never break `scan`, `score`, `plan`, or `report`
- [ ] Document adapter installation and cost/benefit clearly

---

## P5 — AI integration

### Optional AI support
- [ ] Add config for OpenAI-compatible endpoints
- [ ] Support user-supplied API key and base URL
- [ ] Define the grounding contract: AI consumes structured Nexus output, not arbitrary repo state by default
- [ ] Add safeguards so AI cannot silently change scores or canonical decisions

### AI-assisted features
- [ ] Ship AI-assisted explanation after deterministic `explain` exists
- [ ] Evaluate AI-assisted `suggest` only after planning and scoring are stable
- [ ] Add clear UX language indicating when output is deterministic vs model-generated

---

## P6 — Testing and QA

### Rust tests
- [ ] Add or expand planner rule tests for:
  - [ ] canonical clone selection
  - [ ] remote-only projects
  - [ ] local-only projects
  - [ ] ambiguous duplicate clusters
  - [ ] stale-but-important repos
  - [ ] override/pinning behavior
- [ ] Add snapshot tests for JSON plan/report stability
- [x] Add regression tests for scoring explanations and evidence rendering (markdown snapshot; `not_canonical_clone` planner test)
- [ ] Add tests for adapter absence/failure cases across all optional profiles

### CI/CD
- [ ] Keep CI focused on the Rust product and supported packaging paths
- [ ] Verify:
  - [ ] `cargo fmt --check`
  - [ ] `cargo clippy -- -D warnings`
  - [ ] `cargo test --workspace`
  - [ ] docs/examples compile where applicable
- [ ] Decide on Windows and macOS release support in addition to Linux
- [ ] Add package-manager validation where practical for release workflows

---

## P7 — Packaging and distribution

### Distribution strategy
- [ ] Publish the first tagged release with release notes
- [ ] Provide install instructions for:
  - [ ] `cargo`
  - [ ] prebuilt binaries
- [ ] Prioritize package channels in this order:
  - [ ] Homebrew
  - [ ] Chocolatey
  - [ ] npm / npx / bunx wrapper
  - [ ] Scoop
  - [ ] AUR
  - [ ] Nix
- [ ] Keep npm/bun distribution as a thin binary wrapper, not a JS reimplementation

### CLI ergonomics
- [ ] Add shell completions
- [ ] Add man page / `--help` polish
- [ ] Add terminal demos or asciinema-style examples in docs

---

## P8 — Documentation and positioning

### Public-facing docs
- [x] Write a sharper 30-second pitch around repo fleet triage (`README.md`, `docs/PRODUCT_STRATEGY.md`)
- [x] Add a clear “Who this is for / not for” section to README
- [x] Add a “Why not a dashboard?” rationale briefly in docs/FAQ
- [x] Add examples for:
  - [x] duplicate resolution
  - [x] recoverability scoring
  - [x] publish-readiness profile
  - [x] AI optional flow
  (see `docs/EXAMPLES.md`)

### Internal clarity
- [x] Keep architecture docs honest about what is experimental (`docs/ARCHITECTURE.md`, `docs/CLI.md`)
- [x] Keep scoring docs aligned with real implementation (`docs/SCORING.md` ↔ `ScoreBundle` JSON fields)
- [ ] Keep package/distribution docs aligned with actual support policy

---

## Explicit non-goals for now

These are policies, not pending tasks:

- No web dashboard
- No automatic deletion/move/archive of repos
- No automatic commits or PR creation
- No revival of the legacy Python/TS app on `main`
- No AI dependency for core usefulness
- No opaque single score that cannot explain itself
- No enterprise portal ambitions before the CLI/TUI product is unquestionably strong
