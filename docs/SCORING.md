# Scoring model

Nexus uses a **small, explainable** scoring model (see `docs/PRODUCT_STRATEGY.md`). The engine is **deterministic**; optional profiles layer on via `planner.scoring_profile` and evidence (see `docs/SCORING_PROFILES.md`) without changing the default five-axis `ScoreBundle` fields.

## JSON fields vs product language (v0)

`plan.json` and `nexus score --format json` expose a `ScoreBundle` with **stable Rust/JSON field names** today. They map to the product strategy as follows:

| JSON field (`ScoreBundle`) | Product concept (strategy) | Notes |
| --- | --- | --- |
| `canonical` | **Canonical confidence** | How sure we are about the canonical working copy |
| `usability` | **Repo health** | Manifest, README, license-onboarding cues from scan |
| `recoverability` | **Recoverability** | Git metadata, remote linkage, recency, clean worktree—can you resync or restore confidently? |
| `oss_readiness` | **Publish readiness** signals | License/docs/publish cues—not “OSS compatibility” as a headline for all users |
| `risk` | **Maintenance risk** | Higher = more caution / time sink |

`PlanDocument` also carries **`scoring_rules_version`** (integer): the version of the deterministic rule set in `nexus-plan` (`crates/nexus-plan/src/scoring.rs`). It can change without bumping the CLI semver.

Do **not** treat `oss_readiness` as “this project is OSS-ready” for every user; many users only want triage. Optional **Open Source Readiness** and other profiles will be documented separately when implemented.

## Canonical score — `scores.canonical` (0–100)

**Product name:** canonical confidence.  
A higher score means “this cluster’s chosen canonical member is likely the source of truth.”

### Evidence inputs

- normalized remote URL match
- default branch / active branch presence
- latest commit recency
- dirty working tree
- README presence
- manifest presence
- test/CI signals
- remote-only vs local-only state
- duplicate overlap evidence
- merge-base evidence (when enabled)
- optional manual pin (`user_pinned_canonical` when `planner.canonical_pins` lists the clone id)

### Suggested weights

- remote URL certainty: 25
- freshest commit timeline: 15
- branch/head quality: 10
- manifest/readme coherence: 10
- tests + CI presence: 10
- low ambiguity cluster membership: 10
- active local worktree evidence: 10
- release/license/changelog signals: 5
- manual override: 5

### Worked examples (canonical)

Illustrative; exact `kind` strings and deltas come from the planner.

1. **Strong GitHub match**  
   Two local clones share `origin` normalized to the same host/path, and a `gh` ingest row matches. Evidence may include `remote_url_match` with a large positive delta.

2. **Freshness tie-break**  
   Same remote, two clones: one active, one stale. Evidence favors the active clone with commit-time detail.

3. **Ambiguous duplicates**  
   Similar names, no shared remote. Cluster `status` trends `Ambiguous` / `ManualReview`, risk rises, canonical confidence stays lower.

4. **Remote-only cluster**  
   GitHub row with no local clone: canonical remote set; actions may suggest adding a checkout (plan text only).

## Usability score — `scores.usability` (0–100)

**Product name:** repo health.  
A higher score means “easier to build, reason about, and continue.”

Signals (implemented today):

- project manifest present (`manifest_present`)
- README / title present (`readme_present`)
- license metadata (lightweight onboarding signal: `license_signal_usability`)
- scan fingerprint when present (`content_fingerprint`)

Planned / adapter-driven signals (docs target): tests, CI, changelog, secret findings, SBOM.

## Recoverability — `scores.recoverability` (0–100)

**Product name:** recoverability.  
A higher score means “you can likely resync, restore, or reason about lineage without heroics.”

Signals (implemented today):

- `.git` present (`git_object_db`)
- HEAD oid recorded (`resolved_head`)
- default / active branch known
- clean canonical worktree (`clean_worktree_recover`)
- recent commit on canonical (`recent_sync_signal`)
- cluster has linked remotes (`remote_backup_path`)

## Publish readiness (JSON: `scores.oss_readiness`) (0–100)

**Product name:** publish readiness (not “OSS readiness” as the default narrative).  
A higher score means “signals that usually help handoff or publication” (license, docs, hygiene).

Signals:

- usability baseline
- license present
- security scan clean
- secret scan clean
- SBOM extractable
- docs quality
- contribution metadata

**Open Source Readiness** (stricter profile: CONTRIBUTING, SECURITY, CoC, etc.) is planned as an **optional** layer on top—see `docs/PRODUCT_STRATEGY.md`.

## Risk score — `scores.risk` (0–100)

**Product name:** maintenance risk.  
A higher score means “touch this carefully.”

Signals:

- ambiguous cluster
- many clones with similar freshness
- missing remote linkage
- dirty tree without branch hygiene
- missing docs/tests
- secrets or security findings
- stale dependencies
- large unexplained divergence

## Evidence discipline

Every important score movement should be tied to evidence items:

```json
[
  {"kind": "remote_url_match", "delta": 25, "detail": "matched github.com:demo/example"},
  {"kind": "fresh_commit", "delta": 12, "detail": "newest commit in cluster"},
  {"kind": "ci_present", "delta": 5, "detail": ".github/workflows/ci.yml exists"}
]
```

Scores without supporting evidence are a bug in the engine or report layer.

## Failure modes and blind spots

The model is intentionally **shallow** so it stays explainable. Treat low scores and `Ambiguous` status as “investigate,” not ground truth.

- **Canonical confidence** can be wrong when remotes are missing, forks share names, or clones are grouped by display name only (`name:` buckets). Merge-base evidence helps only when git object databases overlap locally.
- **Repo health** is scan-heuristic (manifest, README, license metadata)—not a build or test result.
- **Recoverability** assumes recorded git metadata and links match reality; shallow clones and sparse checkouts may look worse than they are.
- **Publish readiness** (`oss_readiness`) is not legal or compliance advice; it is a small set of file/metadata signals.
- **Maintenance risk** aggregates ambiguity and gaps; it will false-positive on intentional offline or experimental trees.
- **User config** (`canonical_pins`, `ignored_cluster_keys`) overrides planner *recommendations* for actions or canonical selection but does not erase underlying scan facts—read evidence alongside overrides.

**Rule set v4+** adds zero-delta **triage hints** (not score drivers) for inventory shape and user intent:

| Kind | Meaning |
| --- | --- |
| `name_bucket_duplicate_cluster` | Several clones share a **name-only** cluster bucket |
| `fingerprint_split_clusters` | Same scan **fingerprint** appears in more than one cluster |
| `duplicate_name_split_clusters` | Same **display name** ended up in multiple clusters (fork/pivot/weak signal) |
| `stale_but_artifacted` | Canonical clone’s last commit is very old but manifest + README exist |
| `user_pinned_canonical` | Clone id from `planner.canonical_pins` forced as canonical (+ small canonical bump) |
| `user_ignored_cluster` | `cluster_key` in `planner.ignored_cluster_keys` — actions cleared; scores unchanged |
| `user_archive_hint` | `cluster_key` in `planner.archive_hint_cluster_keys` — reminder only |
| `scoring_profile_active` | Non-default `planner.scoring_profile` (see `docs/SCORING_PROFILES.md`) |
