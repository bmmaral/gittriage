# Nexus

**Nexus** is a **local-first, Rust CLI for repo fleet triage**: it inventories directories for git repos, ingests GitHub metadata (via `gh`), groups clones and remotes into **clusters**, scores them, and writes a deterministic **plan** — without modifying your working trees.

**Before:** dozens of checkouts and remotes with unclear "source of truth," risky for humans and agents. **After:** one SQLite-backed inventory, reproducible scores and a `plan.json`, and human-readable reports you can diff and review.

This is **not** a web dashboard or internal developer portal; the product stays lightweight and deterministic.

**Who it's for:** solo developers, freelancers, small teams, and AI-heavy workflows with lots of local clones — anyone who needs **which repos matter**, **which copy is canonical**, and **what to do next**, without platform overhead.

**Who it's not for:** enterprises wanting a hosted catalog, approval workflows, or compliance-first tooling — those are different products.

## Install / build

- [Rust](https://rustup.rs/) stable (see `rust-toolchain.toml`)
- **C toolchain** for `rusqlite` (bundled SQLite): macOS Xcode CLT (`xcode-select --install`); Linux `build-essential` or equivalent
- `git` on `PATH`
- Optional: `gh` for `scan --github-owner` (see `docs/EXTERNAL_TOOLS.md`)

```bash
cargo build --release -p nexus-cli
# binary: target/release/nexus
```

Or debug: `cargo build -p nexus-cli` → `target/debug/nexus`.

Optional: [just](https://github.com/casey/just) recipes (`just test`, `just build`, …).

## Golden path

```bash
cp nexus.toml.example nexus.toml   # edit db_path / default_roots / github_owner
nexus scan ~/Projects --github-owner your-login
nexus score --format text
nexus plan --write nexus-plan.json
nexus report --format md
nexus doctor
nexus tools
# optional: nexus export -o backup.json && nexus import backup.json --force
```

**`plan.json`:** `schema_version: 1`; fields in `docs/PLAN_SCHEMA.md`.

## Commands

### Stable core

| Command | Role |
| --- | --- |
| `scan` | Discover local repos; optional GitHub ingest |
| `score` | Compute scores and evidence from inventory (text or JSON; does not persist plan) |
| `plan` | Resolve clusters, score, attach actions; write JSON plan; persist plan to SQLite |
| `report` | Markdown or JSON from a fresh plan built from inventory |
| `doctor` | Environment and DB sanity |
| `tools` | Which optional scanners are on `PATH` |
| `export` | JSON inventory (optional embedded plan via `--with-plan`) |
| `import` | Restore inventory from export JSON; clears persisted plan (`--force` required) |
| `explain` | One cluster: scores, evidence, actions (`cluster` / `clone` / `remote` subcommands; optional `--ai` narrative) |

### Secondary (real, shipped)

| Command | Role |
| --- | --- |
| `tui` | Interactive terminal table: sort/filter clusters, evidence inspection, pin hint, plan export; read-only |

### Experimental

| Command | Role |
| --- | --- |
| `ai-summary` | AI-generated executive summary of the full plan (requires `ai.enabled = true` + API key) |
| `apply --dry-run` | Count proposed actions (read-only preview; mutating apply disabled) |
| `serve` | Read-only JSON API over local SQLite (not a dashboard; unstable API) |

See `docs/CLI.md` for flags, examples, and TUI keybindings.

## Crate layout

| Crate | Role |
| --- | --- |
| `nexus-core` | Domain types |
| `nexus-config` | Config loading |
| `nexus-db` | SQLite |
| `nexus-scan` | Filesystem scan |
| `nexus-git` | Git metadata |
| `nexus-github` | `gh` ingest |
| `nexus-plan` | Clustering & scoring |
| `nexus-report` | Markdown / JSON |
| `nexus-adapters` | Optional tool hooks |
| `nexus-tui` | Ratatui terminal browser |
| `nexus-ai` | Optional AI explanations (OpenAI-compatible) |
| `nexus-api` | Axum API for `serve` (experimental) |
| `nexus-cli` | CLI entrypoint |

## Limitations / non-goals (v1)

- No web dashboard; no automatic delete/move/archive of repos; no commits or PRs from the tool.
- Scoring and clustering are heuristics — review `plan` and `report` for high-stakes decisions.
- `serve` is experimental; do not rely on it as a stable public API yet.
- Core usefulness does **not** depend on AI.

## Docs

| Doc | Purpose |
| --- | --- |
| [`docs/CLI.md`](docs/CLI.md) | Full command reference, flags, TUI keybindings |
| [`docs/SCORING.md`](docs/SCORING.md) | Scoring model, evidence kinds, failure modes |
| [`docs/SCORING_PROFILES.md`](docs/SCORING_PROFILES.md) | Optional scoring profiles |
| [`docs/PLAN_SCHEMA.md`](docs/PLAN_SCHEMA.md) | Plan JSON schema |
| [`docs/CONFIG.md`](docs/CONFIG.md) | `nexus.toml` configuration |
| [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) | Crate layout and data flow |
| [`docs/PRODUCT_STRATEGY.md`](docs/PRODUCT_STRATEGY.md) | Positioning and roadmap |
| [`docs/EXTERNAL_TOOLS.md`](docs/EXTERNAL_TOOLS.md) | Optional scanner adapters |
| [`docs/EXAMPLES.md`](docs/EXAMPLES.md) | Copy-paste scenarios |
| [`docs/FAQ.md`](docs/FAQ.md) | Common questions |
| [`docs/DECISIONS.md`](docs/DECISIONS.md) | Architecture decision records |
| [`docs/LEGACY_V1.md`](docs/LEGACY_V1.md) | Python/TS prototype migration notes |

Backlog: [`TODO_updated.md`](TODO_updated.md).

## Legacy v1

Python/TypeScript prototypes are **not** on `main`. They are preserved on branch `legacy/v1-python-ts` and tag `legacy-py-mvp`. Details: `docs/LEGACY_V1.md`.

## License

MIT — see `LICENSE`.
