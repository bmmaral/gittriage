# Optional scoring profiles

Nexus’s **default** experience is a single `ScoreBundle` per cluster (`canonical`, `usability`, `recoverability`, `oss_readiness`, `risk`). Optional profiles **do not rewrite those axes**; they add a `scoring_profile_active` evidence item and adjust **which publish/hygiene plan actions** appear (via the `oss_candidate_threshold` baseline). Headline numbers stay comparable across runs unless you change inventory or config.

Configure with `planner.scoring_profile` in `nexus.toml` (see `nexus.toml.example`). Accepted values (case-insensitive, `-` or `_` allowed):

| Config value | Enum | Effect |
| --- | --- | --- |
| `default` (or omit) | Default | Baseline thresholds only |
| `publish`, `publish_readiness` | PublishReadiness | Stricter hygiene nudges: effective OSS threshold −5 vs config |
| `open_source`, `open_source_readiness`, `oss` | OpenSourceReadiness | Stronger nudges: effective OSS threshold −10 |
| `security`, `security_supply_chain`, `supply_chain` | SecuritySupplyChain | Same threshold as default; marker evidence for supply-chain–focused review |
| `ai_handoff`, `ai` | AiHandoff | Slightly stricter hygiene nudges: effective OSS threshold −5 |

## Publish Readiness

**Goal:** Ship or hand off a repo with fewer “paper cuts” (license, CI, basic scans).

**Signals already in default scores:** `oss_readiness` and `usability` capture license, manifest, README, etc.

**Profile behavior:** Lowers the bar at which license/CI/security **plan actions** are suggested (see `crates/nexus-plan/src/lib.rs`, `effective_oss_threshold`). Scores are unchanged.

## Open Source Readiness

**Goal:** Stricter bar for public collaboration (CONTRIBUTING, SECURITY, CoC are roadmap targets for adapters/docs; not all are scored in-engine yet).

**Profile behavior:** Largest threshold shift (−10) so more hygiene actions surface earlier. Use when you explicitly want “public repo checklist” pressure.

## Security / Supply-Chain Posture

**Goal:** Emphasize review of dependencies, secrets, and provenance; optional tools via `nexus plan --external` / adapters.

**Profile behavior:** Marker evidence only in v1 (threshold unchanged). Pair with `--external` and `nexus tools` for scanner-backed evidence.

## AI Handoff Readiness

**Goal:** A clone that another human or agent can navigate quickly (docs, structure, hygiene).

**Profile behavior:** Same magnitude as Publish (−5 on the hygiene action threshold). Complements default `usability` / `recoverability` without inflating scores artificially.

## Default headline experience

With **`scoring_profile` unset or `default`**, no `scoring_profile_active` item is emitted. Profiles never hide the default five-axis view in `score` / `plan` / `report`.
