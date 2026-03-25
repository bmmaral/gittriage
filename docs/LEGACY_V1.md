# Legacy v1 (Python + TypeScript)

The first Nexus experiments lived in this repository as a **project-memory** stack: a Python entrypoint (`nexus.py`), a small web/dashboard layer, and a TypeScript CLI under `cli/`. That code is **not** maintained on `main` anymore.

## Where it went

- **Archival branch:** `legacy/v1-python-ts` — points at the last commit that contained the Python/TypeScript tree (before the Rust-only cleanup).
- **Tag:** `legacy-py-mvp` — annotated marker for the same era, on the tip of the archival branch.

Both are available on the remote (`git fetch origin legacy/v1-python-ts`). No legacy code, scripts, or documents exist on `main`.

## Moving to Nexus v2 (Rust)

The supported product is the Rust workspace under `crates/`:

- Install Rust (see `README.md` and `rust-toolchain.toml`).
- Use `nexus scan`, `nexus score`, `nexus plan`, `nexus report` against a local SQLite DB (see `nexus.toml.example` and `docs/CONFIG.md`).
- There is **no** automatic importer from v1 conversation or config formats; treat v1 as historical reference only.

## Historical PRD

The old "AI project memory" PRD (`prd.md`) exists only on the `legacy/v1-python-ts` branch. The executable product on `main` is the Rust CLI; use `README.md` and `docs/ARCHITECTURE.md` for the current system.
