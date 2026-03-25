# FAQ

## Why not a web dashboard?

Nexus is intentionally **CLI-first** (with an optional **TUI** for inspection, not a browser app). A dashboard pulls the product toward authentication, hosted state, deployment, and competition with large internal-developer-portal products. That conflicts with **local-first**, **deterministic**, and **fast** triage: the goal is to tell you which repos matter and which copy is canonical on *your machine*, without standing up a service.

See `docs/PRODUCT_STRATEGY.md` for positioning and non-goals.

## What is `serve` then?

`nexus serve` is a small **experimental** read-only JSON API over your local SQLite DB for ad hoc inspection (e.g. scripting). It is **not** a supported dashboard, multi-user product, or stable public API until explicitly documented in release notes.

## Does Nexus require AI?

No. Scoring and planning are **deterministic**. Optional AI features (`nexus explain --ai`, `nexus ai-summary`) consume structured Nexus output and are clearly labeled as model-generated. They require `ai.enabled = true` in config and an API key; they do not replace core scoring.
