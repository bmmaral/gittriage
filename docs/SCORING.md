# Scoring model

Nexus uses a **small, explainable** scoring model (see `docs/PRODUCT_STRATEGY.md`). The engine is **deterministic**; optional profiles (e.g. stricter open-source readiness) may layer on later without changing the default headline experience.

## JSON fields vs product language (v0)

`plan.json` and `nexus score --format json` expose a `ScoreBundle` with **stable Rust/JSON field names** today. They map to the product strategy as follows:

| JSON field (`ScoreBundle`) | Product concept (strategy) | Notes |
| --- | --- | --- |
| `canonical` | **Canonical confidence** | How sure we are about the canonical working copy |
| `usability` | **Repo health** (and partially **recoverability**) | v0 combines operational “health” signals in one bucket; split is a future refinement |
| `oss_readiness` | **Publish readiness** signals | License/docs/publish cues—not “OSS compatibility” as a headline for all users |
| `risk` | **Maintenance risk** | Higher = more caution / time sink |

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
- optional manual pin (future)

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

**Product name:** repo health (recoverability overlaps in v0).  
A higher score means “easier to build, reason about, and continue.”

Signals:

- README present and non-trivial
- manifest/lockfile present
- tests present
- CI present
- license present
- changelog/contributing present
- install/run commands inferable
- secret findings absent
- dependency inventory extractable

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
