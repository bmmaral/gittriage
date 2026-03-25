# Scoring model

Nexus uses a **small, explainable** scoring model (see `docs/PRODUCT_STRATEGY.md`). The engine is **deterministic**; optional profiles layer on via `planner.scoring_profile` and evidence (see `docs/SCORING_PROFILES.md`) without changing the default five-axis `ScoreBundle` fields.

## JSON fields vs product language (v0)

`plan.json` and `nexus score --format json` expose a `ScoreBundle` with **stable Rust/JSON field names** today. They map to the product strategy as follows:

| JSON field (`ScoreBundle`) | Product concept (strategy) | Notes |
| --- | --- | --- |
| `canonical` | **Canonical confidence** | How sure we are about the canonical working copy |
| `usability` | **Repo health** | Manifest, README, license-onboarding cues from scan |
| `recoverability` | **Recoverability** | Git metadata, remote linkage, recency, clean worktree—can you resync or restore confidently? |
| `oss_readiness` | **Publish readiness** signals | License/docs/publish cues—not "OSS compatibility" as a headline for all users |
| `risk` | **Maintenance risk** | Higher = more caution / time sink |

`PlanDocument` also carries **`scoring_rules_version`** (integer): the version of the deterministic rule set in `nexus-plan` (`crates/nexus-plan/src/scoring.rs`). It can change without bumping the CLI semver.

Do **not** treat `oss_readiness` as "this project is OSS-ready" for every user; many users only want triage. Optional profiles (Publish Readiness, Open Source Readiness, Security, AI Handoff) are documented in `docs/SCORING_PROFILES.md`.

## Cluster confidence model (v5)

The planner computes a continuous **confidence** value (0.0–1.0) for each cluster, which determines `ClusterStatus`:

- `confidence >= ambiguous_cluster_threshold` (default 0.60) → **Resolved**
- otherwise → **Ambiguous**

Confidence factors (cumulative, clamped to [0, 1]):

| Factor | Contribution |
| --- | --- |
| Baseline (any members) | 0.50 |
| Mixed local + remote | +0.20 |
| Multiple local clones | +0.05 |
| Git metadata present (`.git` + `head_oid`) | +0.08 |
| Consistent fingerprint across ≥2 clones | +0.10 |
| Recent activity (commit or push ≤90 days) | +0.05 |

Single-member clusters start at 0.50; a full-featured cluster with local+remote+git+matching fingerprints+recent activity can reach 0.98.

## Canonical score — `scores.canonical` (0–100)

**Product name:** canonical confidence.
A higher score means "this cluster's chosen canonical member is likely the source of truth."

### Evidence inputs (v5)

| Kind | Delta | Trigger |
| --- | --- | --- |
| `canonical_clone_pick` | +14 | Selected as canonical (freshness, git, clean tree) |
| `git_repo` | +10 | `.git` metadata present |
| `commit_head_present` | +6 | HEAD oid recorded |
| `default_branch_known` | +5 | Default branch recorded |
| `active_branch_known` | +4 | Active branch recorded |
| `recent_activity` | +12 | Last commit within 14 days |
| `fresh_commits` | +8 | Last commit within 90 days |
| `stale_but_tracked` | +4 | Last commit within 12 months |
| `dirty_worktree` | −4 | Uncommitted changes |
| `upstream_remote` | +18 | Canonical remote candidate (push recency) |
| `remote_default_branch` | +5 | Upstream default branch known |
| `merge_base` | +8 | Git merge-base shared ancestor found |
| `user_pinned_canonical` | +4 | Manual `canonical_pins` override |

### Canonical selection heuristic

The planner picks the clone with the most recent `last_commit_at`, breaking ties with `is_git` (prefer true) then `!is_dirty` (prefer clean). User `canonical_pins` override this selection entirely.

## Usability score — `scores.usability` (0–100)

**Product name:** repo health.
A higher score means "easier to build, reason about, and continue."

### Evidence inputs (v5)

| Kind | Delta | Trigger |
| --- | --- | --- |
| `manifest_present` | +14 | Project manifest detected |
| `no_manifest` | **−6** | No recognizable project manifest |
| `readme_present` | +12 | README / title detected |
| `no_readme` | **−8** | No README detected |
| `license_signal_usability` | +6 | License metadata present |
| `no_license` | **−4** | No license file detected |
| `content_fingerprint` | +4 | Scan fingerprint present |

The scanner also detects **project cues** that feed into evidence and future scoring refinements:

- `has_lockfile` — Cargo.lock, package-lock.json, yarn.lock, poetry.lock, etc.
- `has_ci` — `.github/workflows`, `.gitlab-ci.yml`, `.circleci`, Jenkinsfile, `.travis.yml`
- `has_tests_dir` — `tests/`, `test/`, `spec/`, `__tests__/`, `test_suite/`

These are currently scan-time boolean signals available in `CloneRecord`. Adapter-driven signals (secret findings, SBOM, static analysis) come from `--external`.

## Recoverability — `scores.recoverability` (0–100)

**Product name:** recoverability.
A higher score means "you can likely resync, restore, or reason about lineage without heroics."

### Evidence inputs (v5)

| Kind | Delta | Trigger |
| --- | --- | --- |
| `git_object_db` | +18 | Full git history available locally |
| `resolved_head` | +10 | HEAD resolved for checkout |
| `default_branch_recover` | +10 | Default branch aids clone/sync |
| `active_branch_recover` | +6 | Active branch indicates working state |
| `clean_worktree_recover` | +8 | Clean tree easier to sync |
| `recent_sync_signal` | +12 | Recent commit supports recovery confidence (≤90 days) |
| `remote_backup_path` | +16 | Cluster has linked remote(s) |

## Publish readiness (JSON: `scores.oss_readiness`) (0–100)

**Product name:** publish readiness (not "OSS readiness" as the default narrative).
A higher score means "signals that usually help handoff or publication" (license, docs, hygiene).

### Evidence inputs (v5)

| Kind | Delta | Trigger |
| --- | --- | --- |
| `license_present` | +18 | SPDX / license file signal |
| `readme_publish_signal` | +8 | README present supports publish readiness |
| `manifest_publish_signal` | +6 | Project manifest supports packaging |
| `remote_active` | +12 | Remote not archived |
| `remote_archived` | −8 | Archived upstream reduces publish readiness |
| `not_fork_signal` | +6 | Upstream appears primary (not a fork) |
| `fork_signal` | −4 | Fork flag on remote metadata |
| `remote_recent_push` | +6 | Push activity within 90 days |

**Open Source Readiness** (stricter profile: CONTRIBUTING, SECURITY, CoC, etc.) is available as an optional scoring profile—see `docs/SCORING_PROFILES.md`.

## Risk score — `scores.risk` (0–100)

**Product name:** maintenance risk.
A higher score means "touch this carefully."

### Evidence inputs (v5)

| Kind | Delta | Trigger |
| --- | --- | --- |
| `very_stale_canonical` | **+6** | Canonical clone has no commits in over a year |
| `no_commit_timestamp` | **+4** | No last commit timestamp on canonical clone |
| `multiple_clones` (2) | +24 | More than one local clone in cluster |
| `multiple_clones` (3–4) | +30 | Moderate duplication, review for consolidation |
| `multiple_clones` (5+) | +36 | High duplication risk, consolidation strongly recommended |
| `dirty_non_canonical_clones` | +4 per clone | Non-canonical clones with uncommitted changes |
| `archived_remote_risk` | **+8** | Upstream is archived |
| `remote_stale_push` | **+4** | No push activity in over a year on canonical remote |

**v5 graduated risk:** The `multiple_clones` delta scales with clone count rather than being a flat penalty. This better captures the escalating cost of managing many copies.

## Evidence discipline

Every important score movement should be tied to evidence items:

```json
[
  {"kind": "canonical_clone_pick", "delta": 14, "detail": "selected as canonical local candidate"},
  {"kind": "recent_activity", "delta": 12, "detail": "last commit within 14 days"},
  {"kind": "manifest_present", "delta": 14, "detail": "project manifest detected"}
]
```

Scores without supporting evidence are a bug in the engine or report layer.

## Failure modes and blind spots

The model is intentionally **shallow** so it stays explainable. Treat low scores and `Ambiguous` status as "investigate," not ground truth.

- **Canonical confidence** can be wrong when remotes are missing, forks share names, or clones are grouped by display name only (`name:` buckets). Merge-base evidence helps only when git object databases overlap locally.
- **Repo health** is scan-heuristic (manifest, README, license metadata)—not a build or test result. **v5 negative evidence** (e.g. `no_manifest`, `no_readme`) now explicitly penalizes missing hygiene signals rather than relying solely on the absence of positive bumps.
- **Recoverability** assumes recorded git metadata and links match reality; shallow clones and sparse checkouts may look worse than they are.
- **Publish readiness** (`oss_readiness`) is not legal or compliance advice; it is a small set of file/metadata signals.
- **Maintenance risk** aggregates ambiguity and gaps; it will false-positive on intentional offline or experimental trees. **v5 graduated risk** may over-penalize intentional multi-checkout workflows (e.g. separate build/test clones).
- **User config** (`canonical_pins`, `ignored_cluster_keys`) overrides planner *recommendations* for actions or canonical selection but does not erase underlying scan facts—read evidence alongside overrides.

## Triage hints (zero-delta evidence)

**Rule set v4+** adds zero-delta **triage hints** (not score drivers) for inventory shape and user intent:

| Kind | Meaning |
| --- | --- |
| `name_bucket_duplicate_cluster` | Several clones share a **name-only** cluster bucket |
| `fingerprint_split_clusters` | Same scan **fingerprint** appears in more than one cluster |
| `duplicate_name_split_clusters` | Same **display name** ended up in multiple clusters (fork/pivot/weak signal) |
| `stale_but_artifacted` | Canonical clone's last commit is very old but manifest + README exist |
| `user_pinned_canonical` | Clone id from `planner.canonical_pins` forced as canonical (+ small canonical bump) |
| `user_ignored_cluster` | `cluster_key` in `planner.ignored_cluster_keys` — actions cleared; scores unchanged |
| `user_archive_hint` | `cluster_key` in `planner.archive_hint_cluster_keys` — reminder only |
| `scoring_profile_active` | Non-default `planner.scoring_profile` (see `docs/SCORING_PROFILES.md`) |
