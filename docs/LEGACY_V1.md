# Legacy v1 (Python + TypeScript)

The first Nexus experiments lived in this repository as a **project-memory** stack: a Python entrypoint (`nexus.py`), a small web/dashboard layer, and a TypeScript CLI under `cli/`. That code is **not** maintained on `main` anymore.

## Where it went

- **Archival branch:** `legacy/v1-python-ts` — should point at the **last commit that still contained** the Python/TypeScript tree (before the Rust-only cleanup). Push it with `git push origin legacy/v1-python-ts` so clones can fetch it.
- **Tag:** `legacy-py-mvp` — optional annotated marker for the same era. On the tip of `legacy/v1-python-ts`, run `./scripts/tag-legacy-python.sh` (idempotent if the tag already points at `HEAD`), then `git push origin legacy-py-mvp`.
- If you checked out the archival branch and its tree has an older copy of the script, use the version from `main`: `bash <(git show main:scripts/tag-legacy-python.sh)` (after `git fetch`).

## Moving to Nexus v2 (Rust)

The supported product is the Rust workspace under `crates/`:

- Install Rust (see `README.md` and `rust-toolchain.toml`).
- Use `nexus scan`, `nexus score`, `nexus plan`, `nexus report` against a local SQLite DB (see `nexus.toml.example` and `docs/CONFIG.md`).
- There is **no** automatic importer from v1 conversation or config formats; treat v1 as historical reference only.

## Historical PRD

`prd.md` describes the old “AI project memory” vision. The executable product on `main` is the Rust CLI; use `README.md` and `docs/ARCHITECTURE.md` for the current system.
