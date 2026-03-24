# Scoring model

## Canonical score (0–100)

A higher score means "this is most likely the source of truth for the project cluster."

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
- optional manual pin

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

## Usability score (0–100)

A higher score means "this repo is easier to build, reason about, and continue."

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

## OSS readiness score (0–100)

A higher score means "this repo can more safely be polished and published."

Signals:
- usability baseline
- license present
- security scan clean
- secret scan clean
- SBOM extractable
- docs quality
- contribution metadata

## Risk score (0–100)

A higher score means "touch this carefully."

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

Every score must include an evidence list:

```json
[
  {"kind": "remote_url_match", "delta": 25, "detail": "matched github.com:demo/example"},
  {"kind": "fresh_commit", "delta": 12, "detail": "newest commit in cluster"},
  {"kind": "ci_present", "delta": 5, "detail": ".github/workflows/ci.yml exists"}
]
```

Scores without evidence are invalid.
