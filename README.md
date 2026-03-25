<p align="center">
  <strong>nexus</strong><br>
  <em>Local-first repo fleet triage</em>
</p>

<p align="center">
  <a href="https://github.com/bmmaral/nexus/actions/workflows/rust-ci.yml"><img src="https://img.shields.io/github/actions/workflow/status/bmmaral/nexus/rust-ci.yml?branch=main&style=flat-square&label=CI" alt="CI"></a>
  <a href="https://github.com/bmmaral/nexus/actions/workflows/security.yml"><img src="https://img.shields.io/github/actions/workflow/status/bmmaral/nexus/security.yml?branch=main&style=flat-square&label=security&color=blueviolet" alt="Security"></a>
  <a href="https://github.com/bmmaral/nexus/blob/main/LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue?style=flat-square" alt="License"></a>
  <img src="https://img.shields.io/badge/rust-1.82%2B-orange?style=flat-square&logo=rust" alt="Rust">
  <img src="https://img.shields.io/badge/platform-linux%20%7C%20macOS%20%7C%20win-lightgrey?style=flat-square" alt="Platform">
</p>

---

**Nexus** inventories your local git clones, ingests GitHub metadata (via `gh`), groups everything into **clusters**, scores them, and writes a deterministic **plan** — without touching your working trees.

**Before:** dozens of checkouts and remotes with unclear "source of truth."
**After:** one SQLite-backed inventory, reproducible scores, a `plan.json`, and human-readable reports you can diff and review.

> **Who it's for:** solo devs, freelancers, small teams, AI-heavy workflows — anyone who needs *which repos matter*, *which copy is canonical*, and *what to do next*.
>
> **Not for:** enterprises wanting a hosted catalog, approval workflows, or compliance-first tooling.

---

## Quick start

```bash
cargo build --release -p nexus-cli      # → target/release/nexus
cp nexus.toml.example nexus.toml        # edit db_path / default_roots / github_owner

nexus scan ~/Projects --github-owner your-login
nexus score --format text
nexus plan --write plan.json
nexus report --format md
nexus tui                                # interactive terminal browser
nexus score --profile security           # override scoring profile from CLI
```

## Install

**Prebuilt binaries:** [GitHub Releases](https://github.com/bmmaral/nexus/releases) (Linux musl x86_64, macOS arm64/x86_64, Windows x86_64) with `.sha256` checksum files.

**From source** (needs [Rust](https://rustup.rs/) stable ≥ 1.82 and a **C toolchain** for `rusqlite`: macOS Xcode CLT, Linux `build-essential`, Windows MSVC build tools):

```bash
cargo install --locked --path crates/nexus-cli
# or
cargo build --release -p nexus-cli
```

**Package managers & wrappers:** Homebrew formula, Scoop, Chocolatey, npm thin wrapper, AUR PKGBUILD, and Nix are documented in [`docs/DISTRIBUTION.md`](docs/DISTRIBUTION.md).

- `git` on `PATH`
- Optional: [`gh`](https://cli.github.com/) for `scan --github-owner`

## Commands

### Stable core

| Command | What it does |
| :--- | :--- |
| `scan` | Discover local repos; optional GitHub ingest |
| `score` | Compute scores + evidence from inventory (`--profile` to override) |
| `plan` | Resolve clusters → score → actions → write JSON plan (`--profile`) |
| `report` | Markdown or JSON from a fresh plan (`--profile`) |
| `doctor` | Environment and DB sanity checks |
| `tools` | Which optional scanners are on `PATH` |
| `export` | JSON inventory (optional embedded plan via `--with-plan`) |
| `import` | Restore inventory from export JSON (`--force` required) |
| `explain` | Per-cluster deep dive: scores, evidence, actions (`--ai` for narrative) |

### Secondary

| Command | What it does |
| :--- | :--- |
| `tui` | Interactive terminal table — sort, filter, inspect, pin, export |

### Experimental

| Command | What it does |
| :--- | :--- |
| `ai-summary` | AI-generated executive summary of the full plan |
| `apply --dry-run` | Count proposed actions (read-only preview) |
| `serve` | Read-only JSON API over local SQLite |

See [`docs/CLI.md`](docs/CLI.md) for flags, examples, and TUI keybindings.

---

## Architecture

```
          ┌──────────────────────────────────────────────┐
          │                  nexus-cli                    │
          │         clap commands · tokio runtime         │
          └──┬────┬────┬────┬────┬────┬────┬────┬────┬───┘
             │    │    │    │    │    │    │    │    │
         scan│ git│ gh │plan│ db │ rpt│ tui│ ai │ api│
             ▼    ▼    ▼    ▼    ▼    ▼    ▼    ▼    ▼
         ┌──────────────────────────────────────────────┐
         │               nexus-core                     │
         │    CloneRecord · ClusterRecord · PlanDoc     │
         └──────────────────────────────────────────────┘
                             │
                     ┌───────┴───────┐
                     │  SQLite (db)  │
                     └───────────────┘
```

13 crates, one workspace:

| Crate | Role |
| :--- | :--- |
| `nexus-core` | Domain types (`CloneRecord`, `ClusterRecord`, `PlanDocument`, etc.) |
| `nexus-config` | Config loading (`nexus.toml`) |
| `nexus-db` | SQLite persistence (WAL mode, schema versioning) |
| `nexus-scan` | Filesystem walking, SPDX sniffing, project cue detection |
| `nexus-git` | Git metadata extraction |
| `nexus-github` | `gh` CLI ingest (5000-repo pagination) |
| `nexus-plan` | Clustering, scoring engine, action generation |
| `nexus-report` | Markdown / JSON report rendering |
| `nexus-adapters` | Optional external tool hooks (gitleaks, semgrep, syft, jscpd) |
| `nexus-tui` | Ratatui interactive terminal browser |
| `nexus-ai` | Optional AI explanations (OpenAI-compatible) |
| `nexus-api` | Axum API for `serve` (experimental, loopback default) |
| `nexus-cli` | CLI entrypoint |

## External tools (optional)

| Tool | Support | What it adds |
| :--- | :--- | :--- |
| `gitleaks` | **Official** | Secret leak detection evidence |
| `semgrep` | **Official** | Static analysis findings |
| `syft` | **Official** | SBOM / dependency inventory |
| `jscpd` | Best effort | Copy/paste duplication evidence |

Missing tools are **silently skipped** — they never break the pipeline. See [`docs/EXTERNAL_TOOLS.md`](docs/EXTERNAL_TOOLS.md).

---

## Limitations (v1)

- No web dashboard; no automatic delete/move/archive of repos.
- Scoring and clustering are heuristics — review `plan` and `report` for high-stakes decisions.
- `serve` is experimental (loopback-only by default); do not rely on it as a stable API.
- GitHub ingest caps at 5000 repos per owner; warns on truncation.
- Core usefulness does **not** depend on AI.

## Docs

| Doc | Purpose |
| :--- | :--- |
| [`CLI.md`](docs/CLI.md) | Full command reference, flags, TUI keybindings |
| [`SCORING.md`](docs/SCORING.md) | Scoring model, evidence kinds, failure modes |
| [`SCORING_PROFILES.md`](docs/SCORING_PROFILES.md) | Optional scoring profiles |
| [`PLAN_SCHEMA.md`](docs/PLAN_SCHEMA.md) | Plan JSON schema |
| [`CONFIG.md`](docs/CONFIG.md) | `nexus.toml` configuration |
| [`ARCHITECTURE.md`](docs/ARCHITECTURE.md) | Crate layout and data flow |
| [`PRODUCT_STRATEGY.md`](docs/PRODUCT_STRATEGY.md) | Positioning and roadmap |
| [`EXTERNAL_TOOLS.md`](docs/EXTERNAL_TOOLS.md) | Optional scanner adapters |
| [`DISTRIBUTION.md`](docs/DISTRIBUTION.md) | Homebrew, Scoop, Chocolatey, npm, AUR, Nix |
| [`EXAMPLES.md`](docs/EXAMPLES.md) | Copy-paste scenarios |
| [`FAQ.md`](docs/FAQ.md) | Common questions |
| [`DECISIONS.md`](docs/DECISIONS.md) | Architecture decision records |
| [`LEGACY_V1.md`](docs/LEGACY_V1.md) | Python/TS prototype migration notes |

## Legacy v1

Python/TypeScript prototypes are **not** on `main`. Preserved on branch `legacy/v1-python-ts` and tag `legacy-py-mvp`. Details: [`docs/LEGACY_V1.md`](docs/LEGACY_V1.md).

## License

MIT — see [`LICENSE`](LICENSE).
