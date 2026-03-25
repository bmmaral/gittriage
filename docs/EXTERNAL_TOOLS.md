# External tools

Nexus can run **without** these programs; they extend what the engine can see. The `--external` flag is supported on: `score`, `plan`, `report`, `explain`, `export --with-plan`, `tui`, and `ai-summary`. The `doctor` command checks for their presence on `PATH`.

## `git`

Required for meaningful clone metadata (remotes, branches, commits). Install from your OS or [git-scm.com](https://git-scm.com/).

## `gh` (GitHub CLI)

Used to list repositories for a GitHub user/org when you pass `--github-owner` to `scan` (or set `github_owner` in config). Install: [cli.github.com](https://cli.github.com/). Run `gh auth login` so API calls succeed.

## `jscpd` (copy/paste detector)

Optional. When on `PATH` and you use `nexus plan --external`, Nexus may attach summary evidence from jscpd runs on the canonical clone in each cluster.

## `semgrep`

Optional static analysis CLI. Same pattern as jscpd under `plan --external`.

## `gitleaks`

Optional secret scanner. Same pattern under `plan --external`.

## `syft`

Optional SBOM generator. Same pattern under `plan --external`.

## Checking availability

Run:

```bash
nexus tools
nexus tools --format json
```

This prints which of the optional scanners were found on `PATH` (text table or `kind: "nexus_tools"` JSON for scripts).

## Timeouts

Each adapter subprocess is limited to **180 seconds** by default. If a tool hangs, Nexus kills it and records evidence such as `timed out after 180s` instead of blocking `plan --external` indefinitely.

Override with `NEXUS_ADAPTER_TIMEOUT_SECS` (integer seconds, 1–86400), for example:

```bash
NEXUS_ADAPTER_TIMEOUT_SECS=300 nexus plan --write plan.json --external
```
