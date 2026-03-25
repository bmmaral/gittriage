# Examples

Short, copy-paste oriented scenarios. Nexus stays **read-only**; it never deletes repos or opens PRs for you.

## Duplicate local clones (same GitHub remote)

You have two folders that both point at `github.com/you/app`.

1. `nexus scan ~/Projects --github-owner you`
2. `nexus score` — inspect canonical confidence and look for `not_canonical_clone` evidence on the older checkout.
3. `nexus plan --write plan.json` — review **Warnings** and **Actions** in `nexus report --format md`.
4. Manually archive or delete the non-canonical tree only after you confirm there is no unpushed work.

## Recoverability / repo health (scores)

Health and publish-readiness signals come from scan-time heuristics (manifest, README, license, etc.), not from running your full test suite.

1. Run `scan` then `score --format text`.
2. Read **Repo health** and **Publish readiness** lines per cluster; cross-check **Evidence** for `manifest_present`, `readme_present`, `license_present`.
3. Use `plan` / `report` for suggested next steps (still descriptive only).

## Publish readiness (not a full OSS audit)

The `scores.oss_readiness` field is documented as **publish readiness** in reports (`docs/SCORING.md`). It is **not** a guarantee that a repo is ready for public OSS maintainership.

- Use `nexus plan --external` only when optional scanners are installed (`nexus tools`) and you accept the runtime cost.
- Optional **Open Source Readiness** and other scoring profiles are available via `planner.scoring_profile` in `nexus.toml` (see `docs/SCORING_PROFILES.md`).

## Explaining a cluster

`nexus explain` gives you a detailed deterministic breakdown of one cluster:

```bash
nexus explain cluster my-repo              # text
nexus explain cluster my-repo --format json
nexus explain cluster my-repo --ai         # add AI narrative (requires config)
```

## Scoring profiles

Override the config-file profile with `--profile` on `score`, `plan`, `report`, or `explain`:

```bash
nexus score --profile security
nexus plan --write plan.json --profile publish
nexus explain cluster my-repo --profile ai_handoff
```

Available profiles: `default`, `publish`, `open_source`, `security`, `ai_handoff` (see `docs/SCORING_PROFILES.md`).

## Scan modes and filtering

By default, Nexus only considers directories with `.git` as project roots (`git_only` mode). To also include directories with manifest files (Cargo.toml, package.json, etc.):

```toml
# nexus.toml
[scan]
scan_mode = "project_roots"
max_depth = 5
```

Place a `.nexusignore` file in any scan root to exclude directories:

```text
# .nexusignore
node_modules
.cache
vendor
```

## AI-optional flow

Core commands (`scan`, `score`, `plan`, `report`, `doctor`) are fully deterministic. No API keys are required.

- `nexus explain --ai` and `nexus ai-summary` call user-configured OpenAI-compatible endpoints; they consume structured Nexus output, not arbitrary repo trees (`docs/CLI.md`).
- AI output is clearly labeled as model-generated.
