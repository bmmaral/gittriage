# Nexus

**Nexus** is a **local-first, Rust-first CLI for repo fleet triage**: it inventories directories for git repos, ingests GitHub metadata (via `gh`), groups clones and remotes into **clusters**, scores them, and writes a deterministic **plan**—without modifying your working trees.

**Before:** dozens of checkouts and remotes with unclear “source of truth,” risky for humans and agents. **After:** one SQLite-backed inventory, reproducible scores and a `plan.json`, and human-readable reports you can diff and review.

This is **not** a web dashboard or internal developer portal; the product stays lightweight and deterministic. An optional **`nexus tui`** terminal browser sits on the same planner as `score`/`plan` (see `docs/CLI.md`). **Optional AI** explanations may come later; they do not define the core engine.

**Who it’s for:** solo developers, freelancers, small teams, and AI-heavy workflows with lots of local clones—anyone who needs **which repos matter**, **which copy is canonical**, and **what to do next**, without platform overhead.

**Who it’s not for:** enterprises wanting a hosted catalog, approval workflows, or compliance-first buying—those are different products.

Docs: [`docs/PRODUCT_STRATEGY.md`](docs/PRODUCT_STRATEGY.md), [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md), [`docs/SCORING.md`](docs/SCORING.md), [`docs/SCORING_PROFILES.md`](docs/SCORING_PROFILES.md), [`docs/CLI.md`](docs/CLI.md), [`docs/PLAN_SCHEMA.md`](docs/PLAN_SCHEMA.md), [`docs/CONFIG.md`](docs/CONFIG.md), [`docs/EXTERNAL_TOOLS.md`](docs/EXTERNAL_TOOLS.md), [`docs/FAQ.md`](docs/FAQ.md), [`docs/EXAMPLES.md`](docs/EXAMPLES.md). Backlog: [`TODO_updated.md`](TODO_updated.md) (current), [`TODO.md`](TODO.md) (earlier tracker).

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

## Golden path (documented workflow)

```bash
cp nexus.toml.example nexus.toml   # edit db_path / default_roots / github_owner
nexus scan ~/Projects --github-owner your-login
nexus score --format text          # scores + evidence (stdout only; does not persist plan)
nexus plan --write nexus-plan.json
nexus report --format md
nexus apply --dry-run
nexus doctor
nexus tools
# optional: nexus export -o backup.json && nexus import backup.json --force
```

**Sample `report` output shape (markdown):** title `Nexus Report`, run metadata, then per cluster: scores, evidence, proposed actions (descriptive only; no automatic mutations).

**`plan.json`:** `schema_version: 1`; fields in `docs/PLAN_SCHEMA.md`.

## Commands (stable core + helpers)

| Command | Role |
| --- | --- |
| `scan` | Discover local repos; optional GitHub ingest |
| `score` | Compute scores and evidence from inventory (text or JSON; does not write plan file or persist plan row) |
| `plan` | Resolve clusters, score, attach actions; write JSON plan; persist plan to SQLite |
| `report` | Markdown or JSON from a fresh plan built from inventory |
| `doctor` | Environment and DB sanity |
| `tools` | Which optional scanners are on `PATH` |
| `apply --dry-run` | Count proposed actions (read-only preview; mutating apply disabled) |
| `serve` | **Experimental** read-only JSON API over local SQLite (not a dashboard; unstable API) |
| `export` | JSON inventory (optional embedded plan via `--with-plan`) |
| `import` | Restore inventory from export JSON; clears persisted plan (`--force` required) |
| `explain` | One cluster: scores, evidence, actions (`cluster` / `clone` / `remote` subcommands) |

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
| `nexus-api` | Axum API for `serve` (experimental) |
| `nexus-cli` | CLI entrypoint |

## Limitations / non-goals (v1)

- No web dashboard; no automatic delete/move/archive of repos; no commits or PRs from the tool.
- Scoring and clustering are heuristics—review `plan` and `report` for high-stakes decisions.
- `serve` is experimental; do not rely on it as a stable public API yet.
- Core usefulness does **not** depend on AI.

## Legacy v1

Python/TypeScript prototypes are **not** on `main`. They are preserved on branch `legacy/v1-python-ts` and tag `legacy-py-mvp`. Details: `docs/LEGACY_V1.md`.

## License

MIT — see `LICENSE`.
