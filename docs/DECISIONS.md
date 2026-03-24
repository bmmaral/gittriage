# Architectural decisions

## ADR-001: Rust rewrite is allowed
Decision: yes.

Reason:
- repo scanning is I/O-heavy
- CLI ergonomics matter
- local-first binary distribution matters
- product needs a clear deterministic core

## ADR-002: SQLite before services
Decision: yes.

Reason:
- no infrastructure dependency
- easy reproducibility
- ideal for a local-first operator tool

## ADR-003: `gh` before raw GitHub API
Decision: yes for v1.

Reason:
- faster implementation
- leverages existing authentication
- good enough for inventory ingest

## ADR-004: no AI in the decision loop
Decision: yes.

Reason:
- wrong canonical selection is more expensive than missing a nice explanation
- AI should explain evidence, not generate truth

## ADR-005: no destructive actions in v1
Decision: yes.

Reason:
- user trust
- lower blast radius
- plan-first workflow is sufficient
