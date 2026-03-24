# Nexus

This repository is transitioning to **Nexus v2**: a local-first **repository fleet intelligence** CLI (Rust) that inventories local and GitHub repos, resolves identity clusters, scores them, and emits a deterministic plan—without mutating your trees.

See `docs/ARCHITECTURE.md`, `docs/SCORING.md`, `docs/CLI.md`, and `TODO.md`.

## Nexus v2 (Rust CLI)

### Requirements

- [Rust](https://rustup.rs/) stable (see `rust-toolchain.toml`)
- **C toolchain for build scripts:** on macOS, **Xcode Command Line Tools** (`xcode-select --install`); on Linux, `build-essential` (or your distro’s equivalent) so `cc` can compile SQLite for `rusqlite`.
- `git` on `PATH` (for repo metadata)
- Optional: `gh` CLI (for `scan` with `--github-owner`)

### Build and run

```bash
cargo build -p nexus-cli
cargo run -p nexus-cli -- doctor
```

Optional: [just](https://github.com/casey/just) recipes mirror these commands (`just test`, `just build`, `just doctor`, …).

The CLI binary name is **`nexus`** (`target/debug/nexus` or `target/release/nexus` on the host triple; cross-compiles land under `target/<triple>/…`).

### Commands

```bash
nexus scan ~/Projects ~/code --github-owner your-github-login
nexus plan --write nexus-plan.json
nexus report --format md
nexus apply --dry-run
nexus doctor
nexus tools
nexus serve --port 3030
```

Configuration precedence is documented in `docs/CLI.md`. Example file: `nexus.toml.example`.

### Crate layout

| Crate | Role |
| --- | --- |
| `nexus-core` | Domain types |
| `nexus-config` | Config loading |
| `nexus-db` | SQLite persistence |
| `nexus-scan` | Filesystem scan |
| `nexus-git` | Git metadata |
| `nexus-github` | `gh` ingest |
| `nexus-plan` | Clustering & scoring |
| `nexus-report` | Markdown / JSON reports |
| `nexus-adapters` | Optional jscpd / semgrep / gitleaks / syft CLI hooks |
| `nexus-api` | Axum JSON API (`serve`) |
| `nexus-cli` | CLI entrypoint |

---

## Legacy stack (Python + TypeScript)

The tree still contains the earlier **project-memory** experiment: Python CLI (`nexus.py`), FastAPI dashboard (`server.py`), and a TypeScript CLI under `cli/`. These are **not** the v2 product; they remain for reference and gradual retirement. To tag that era on a commit: `scripts/tag-legacy-python.sh` (see script header).

- Python: `pip install -r requirements.txt`, `python nexus.py --help`
- TS CLI: `cd cli && npm install && npm run build`

---

## License

MIT
