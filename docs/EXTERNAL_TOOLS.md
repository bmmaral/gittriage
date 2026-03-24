# External tools

These are optional in v1, but strongly recommended after the core pipeline is stable.

- **jscpd**: duplicate code / copy-paste detection across many languages
- **Semgrep CE**: static analysis and rule enforcement
- **Gitleaks**: secret detection
- **Syft**: SBOM generation

Integrate them as adapters that contribute evidence into assessments.

They must not become hard blockers for the initial `scan -> plan -> report` flow.

## Nexus wiring

- **`nexus tools`** — shows which binaries are visible on `PATH`.
- **`nexus plan --external`** — runs available tools on each cluster’s **canonical** local clone and appends evidence (best-effort, non-blocking if a tool errors).

Implementation: `crates/nexus-adapters` (wraps CLI invocations; no embedded engines).
