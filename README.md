<p align="center">
  <strong>GitTriage</strong><br>
  <em>Workspace truth and preflight safety for coding agents.</em>
</p>

<p align="center">
  <a href="https://github.com/bmmaral/gittriage/actions/workflows/rust-ci.yml"><img src="https://img.shields.io/github/actions/workflow/status/bmmaral/gittriage/rust-ci.yml?branch=main&style=flat-square&label=CI" alt="CI"></a>
  <a href="https://github.com/bmmaral/gittriage/actions/workflows/security.yml"><img src="https://img.shields.io/github/actions/workflow/status/bmmaral/gittriage/security.yml?branch=main&style=flat-square&label=security&color=blueviolet" alt="Security"></a>
  <a href="https://github.com/bmmaral/gittriage/blob/main/LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue?style=flat-square" alt="License"></a>
  <img src="https://img.shields.io/badge/rust-1.82%2B-orange?style=flat-square&logo=rust" alt="Rust">
  <img src="https://img.shields.io/badge/platform-linux%20%7C%20macOS%20%7C%20win-lightgrey?style=flat-square" alt="Platform">
</p>

<p align="center">
  <a href="https://github.com/bmmaral/gittriage/releases/latest"><img src="https://img.shields.io/github/v/release/bmmaral/gittriage?sort=semver&style=flat-square&logo=github&label=release" alt="GitHub release"></a>
  <a href="https://github.com/bmmaral/gittriage/blob/main/docs/DISTRIBUTION.md#cargo-from-cratesio"><img src="https://img.shields.io/badge/crates.io-install%20--git%20%7C%20--path-lightgrey?style=flat-square&logo=rust&label=crates.io" alt="crates.io (not published as a single crate yet)"></a>
  <a href="https://github.com/bmmaral/gittriage/pkgs/npm/gittriage"><img src="https://img.shields.io/github/package-json/v/bmmaral/gittriage/main?filename=packaging%2Fnpm%2Fpackage.json&style=flat-square&logo=github&label=%40bmmaral%2Fgittriage" alt="npm package version (declared in repo; published via GitHub Packages workflow)"></a>
  <a href="https://github.com/bmmaral/gittriage/tree/main/packaging/chocolatey"><img src="https://img.shields.io/badge/chocolatey-template%20in%20repo-8B4513?style=flat-square&logo=chocolatey&logoColor=white&label=chocolatey" alt="Chocolatey (community feed not published)"></a>
  <a href="https://github.com/bmmaral/gittriage/tree/main/packaging/homebrew"><img src="https://img.shields.io/badge/homebrew-formula%20(shipped)-FBB040?style=flat-square&logo=homebrew&logoColor=white" alt="Homebrew formula"></a>
  <a href="https://github.com/bmmaral/gittriage/tree/main/packaging/scoop"><img src="https://img.shields.io/badge/scoop-manifest%20(shipped)-7E56FF?style=flat-square" alt="Scoop manifest"></a>
</p>

---

**GitTriage** builds a deterministic picture of your local git workspace: which checkout is **canonical**, which paths are **unsafe for automation**, and how duplicates and nested repos relate — without touching your working trees. Optional `gh` ingest augments local remotes; clustering and scores stay rule-based.

**Before:** agents and humans edit the wrong clone, or guess which folder is “the” repo.
**After:** `preflight`, `resolve`, `check-path`, and `verdict` give stable JSON (and a read-only HTTP API) you can hand to tooling before any read or write.

> **Who it's for:** anyone running coding agents or scripts over messy local trees who needs *which repo is real*, *when automation must stop*, and *what a human should review first*.
>
> **Optional:** `explain --ai` and `ai-summary` add narrative on top of deterministic output — they are not required for the core workflow.
>
> **Not for:** hosted PR review bots, code search platforms, or compliance-first catalog products.

---

## Quick start

```bash
cargo build --release -p gittriage          # → target/release/gittriage
cp gittriage.toml.example gittriage.toml        # edit db_path / default_roots / github_owner

gittriage scan ~/Projects --github-owner your-login
gittriage preflight my-repo --format json                       # agent manifest (canonical path + verdict + warnings)
gittriage summary --agent ~/Projects --format json             # compact rollups (duplicates, unsafe, dirty, nested)
gittriage plan --write plan.json
gittriage report --format md
gittriage tui                                # interactive terminal browser (automation column + canonical path)
gittriage score --profile security           # optional scoring profile override
```

## Install

**Prebuilt binaries:** [GitHub Releases](https://github.com/bmmaral/gittriage/releases) (Linux musl x86_64, macOS arm64/x86_64, Windows x86_64) with `.sha256` checksum files.

**From source** (needs [Rust](https://rustup.rs/) stable ≥ 1.82 and a **C toolchain** for `rusqlite`: macOS Xcode CLT, Linux `build-essential`, Windows MSVC build tools):

```bash
cargo install --locked --path crates/gittriage
# or
cargo build --release -p gittriage
```

**Package managers & wrappers:** Homebrew formula, Scoop, Chocolatey, **`@bmmaral/gittriage` on GitHub Packages** (npm), AUR PKGBUILD, and Nix are documented in [`docs/DISTRIBUTION.md`](docs/DISTRIBUTION.md).

| Channel | Notes |
| :--- | :--- |
| **GitHub Releases** | Prebuilt `gittriage` binaries (see link above) |
| **`cargo install --path` / `cargo build`** | From this repo; needs Rust ≥ 1.82 + C toolchain for SQLite |
| **Homebrew / Scoop** | Formulas in `packaging/` |
| **npm (`@bmmaral/gittriage`)** | Wrapper around the published binary; install matrix in `docs/DISTRIBUTION.md` |

- `git` on `PATH`
- Optional: [`gh`](https://cli.github.com/) for `scan --github-owner`

## Commands

### Agent / coding-agent (stable JSON + provenance)

| Command | What it does |
| :--- | :--- |
| `preflight <TARGET>` | Compact manifest: canonical path, alternates, warnings, automation verdict (label / path / URL) |
| `resolve <QUERY>` | Resolve label / path / URL → canonical path, cluster, alternates, verdict |
| `verdict <TARGET>` | Deterministic `safe_to_*` flags and `unsafe_for_automation` |
| `check-path <PATH>` | Wrong-clone check: canonical vs alternate disposition |
| `summary --agent <WORKSPACE>…` | Token-light workspace rollup (duplicates, unsafe, dirty canon, nested) |

### Stable core

| Command | What it does |
| :--- | :--- |
| `scan` | Discover repos into SQLite; default **`git_only`** mode inventories real `.git` roots (best for agents); `project_roots` is broader — see [`docs/CLI.md`](docs/CLI.md#git-workspaces-vs-manifest-only-discovery) |
| `score` | Compute scores + evidence from inventory (`--profile` to override) |
| `plan` | Resolve clusters → score → actions → write JSON plan (`--profile`) |
| `report` | Markdown or JSON from a fresh plan (`--profile`; agent-first sections when unscoped) |
| `doctor` | Environment and DB sanity checks |
| `tools` | Which optional scanners are on `PATH` |
| `export` | JSON inventory (optional embedded plan via `--with-plan`) |
| `import` | Restore inventory from export JSON (`--force` required) |
| `explain` | Per-cluster deep dive: scores, evidence, actions (optional `--ai` narrative) |
| `ai-doctor` | Show AI config / key presence; optional `--probe-network` checks `{api_base}/models` |

### Secondary

| Command | What it does |
| :--- | :--- |
| `tui` | Interactive terminal table — sort, filter, inspect, pin, export; automation verdict column |

### Experimental

| Command | What it does |
| :--- | :--- |
| `ai-summary` | Optional AI-generated narrative over the full plan (not deterministic) |
| `apply --dry-run` | Count proposed actions (read-only preview) |
| `serve` | Read-only JSON API: `/v1/*` inventory/plan + `/v2/agent/*` preflight-style operations |

See [`docs/CLI.md`](docs/CLI.md) for flags, examples, and TUI keybindings.

---

## Architecture

```
          ┌──────────────────────────────────────────────┐
          │                    gittriage                 │
          │         clap commands · tokio runtime        │
          └──┬────┬────┬────┬────┬────┬────┬────┬────┬───┘
             │    │    │    │    │    │    │    │    │
         scan│ git│ gh │plan│ db │ rpt│ tui│ ai │ api│
             ▼    ▼    ▼    ▼    ▼    ▼    ▼    ▼    ▼
         ┌──────────────────────────────────────────────┐
         │               gittriage-core                 │
         │    CloneRecord · ClusterRecord · PlanDoc     │
         └──────────────────────────────────────────────┘
                                  │
                          ┌───────┴───────┐
                          │  SQLite (db)  │
                          └───────────────┘
```

14 crates, one workspace:

| Crate | Role |
| :--- | :--- |
| `gittriage-core` | Domain types (`CloneRecord`, `ClusterRecord`, `PlanDocument`, etc.) |
| `gittriage-config` | Config loading (`gittriage.toml`) |
| `gittriage-db` | SQLite persistence (WAL mode, schema versioning) |
| `gittriage-scan` | Filesystem walking, SPDX sniffing, project cue detection |
| `gittriage-git` | Git metadata extraction |
| `gittriage-github` | `gh` CLI ingest (5000-repo pagination) |
| `gittriage-plan` | Clustering, scoring engine, action generation |
| `gittriage-agent` | Resolve, verdict, preflight, path check, agent summary (deterministic) |
| `gittriage-report` | Markdown / JSON report rendering |
| `gittriage-adapters` | Optional external tool hooks (gitleaks, semgrep, syft, jscpd) |
| `gittriage-tui` | Ratatui interactive terminal browser |
| `gittriage-ai` | Optional AI explanations (OpenAI-compatible) |
| `gittriage-api` | Axum API for `serve` (experimental, loopback default) |
| `gittriage` | CLI crate and `gittriage` binary |

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
- `serve` is experimental (loopback-only by default). `/v2/agent/*` is the versioned agent contract; field additions are expected; breaking changes will be noted in release notes.
- GitHub ingest caps at 5000 repos per owner; warns on truncation.
- Core usefulness does **not** depend on AI.

## Docs

| Doc | Purpose |
| :--- | :--- |
| [`CLI.md`](docs/CLI.md) | Full command reference, flags, TUI keybindings |
| [`SCORING.md`](docs/SCORING.md) | Scoring model, evidence kinds, failure modes |
| [`SCORING_PROFILES.md`](docs/SCORING_PROFILES.md) | Optional scoring profiles |
| [`PLAN_SCHEMA.md`](docs/PLAN_SCHEMA.md) | Plan JSON schema |
| [`CONFIG.md`](docs/CONFIG.md) | `gittriage.toml` configuration |
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
