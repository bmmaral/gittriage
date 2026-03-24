# Nexus v2 TODO

## Phase 0 — reset and framing
- [x] Tag old repo as `legacy-py-mvp` (helper: `scripts/tag-legacy-python.sh`; run locally)
- [x] Replace root README with new product definition
- [ ] Delete stale runtime artifacts from repo (manual / policy decision — legacy Python tree still present for reference)
- [x] Freeze v1 scope and publish architecture docs (`docs/`)

## Phase 1 — workspace bootstrap
- [x] Create Rust workspace
- [x] Add shared deps in root `Cargo.toml`
- [x] Add `rust-toolchain.toml`
- [x] Add `justfile`
- [x] Add `.editorconfig`, `.gitignore`, `.pre-commit-config.yaml`

## Phase 2 — config and DB
- [x] Implement config loading precedence
- [x] Implement SQLite DB open/create
- [x] Apply migration `0001_init.sql`
- [x] Add DB smoke tests
- [x] Add `nexus doctor` basic diagnostics

## Phase 3 — local scan
- [x] Scan multiple root paths
- [x] Detect Git repos
- [x] Detect non-git project folders via manifests
- [x] Extract README title
- [x] Extract license presence
- [x] Compute lightweight fingerprint
- [x] Persist scan results

## Phase 4 — git metadata
- [x] Read HEAD OID
- [x] Read current branch
- [x] Read remotes
- [x] Read dirty state
- [x] Read last commit time
- [x] Normalize remote URLs

## Phase 5 — GitHub ingest
- [x] Check `gh` availability
- [x] Ingest repos for owner
- [x] Persist remote repo metadata
- [x] Match local clones to remotes (`clone_remote_links`, GitHub URL match)

## Phase 6 — identity resolution
- [x] Group by normalized remote URL
- [x] Fallback by repo/display name
- [ ] Add merge-base evidence later
- [x] Mark ambiguous clusters
- [x] Persist clusters + evidence

## Phase 7 — scoring
- [x] Canonical score
- [x] Usability score
- [x] OSS readiness score
- [x] Risk score
- [x] Evidence rendering

## Phase 8 — planning
- [x] Emit `plan.json`
- [x] Emit markdown report
- [x] Prioritize actions
- [x] Never mutate repos in v1
- [x] Add dry-run `apply` placeholder only

## Phase 9 — hardening
- [x] Unit tests for scoring (clustering / identity tests in `nexus-plan`)
- [x] Snapshot tests for reports (`nexus-report` + `insta`)
- [x] Golden fixture datasets (`fixtures/golden/plan-v1.json` + `nexus-core` roundtrip test)
- [x] CI workflow (`.github/workflows/rust-ci.yml`, includes **linux-musl** cross-compile)
- [x] Security workflow (`.github/workflows/security.yml`)
- [x] Release workflow (`.github/workflows/release.yml` — musl binary on `v*` tags)

## Phase 10 — optional integrations
- [x] jscpd adapter (`nexus-adapters`, `nexus plan --external`)
- [x] semgrep adapter
- [x] gitleaks adapter
- [x] syft adapter
- [x] axum JSON API (`nexus serve` / `crates/nexus-api`)
