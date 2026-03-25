# Nexus Product Strategy

## Positioning

**Nexus is a local-first, Rust-first CLI for repo fleet triage.**

It helps developers answer three painful questions fast:

1. Which repos matter right now?
2. Which copy of a project is the real one?
3. What should I do next: keep, merge, revive, publish, or archive?

Nexus should not become a web platform, internal developer portal, or always-on analysis service. The product should stay:

- **CLI-first**
- **optional TUI**
- **deterministic by default**
- **fast and local-first**
- **useful without AI**
- **Rust-dominant in implementation and runtime**

## Strategic choice

### Chosen path

Build Nexus as a **lightweight repo fleet triage tool** with:

- a strong Rust CLI core
- a thin optional TUI for inspection and overrides
- optional external adapters
- optional AI explanation/suggestion features
- broad binary distribution through multiple package channels

### Rejected path

Do **not** build Nexus into:

- a heavy web dashboard product
- a centralized software catalog
- a multi-user enterprise portal
- an always-on server/service
- an AI-first repo analysis product that becomes weak without API access

A web dashboard would pull the product toward auth, persistence complexity, remote state, deployment burden, and competition with much larger platform products. That is misaligned with the intended audience and with the constraint that the codebase should remain at least 65–70% Rust.

## Product category

Nexus belongs in a distinct category:

**repo fleet triage**

It is adjacent to cleanup tools and repository summary tools, but the differentiator is this combination:

- local inventory
- clone/duplicate/pivot resolution
- deterministic scoring
- prioritization
- plan generation

The mental model is not “another AI repo tool” and not “a tiny Backstage.”

## Users

### Primary users

- solo developers with many local repos
- indie hackers with abandoned or pivoted experiments
- consultants and freelancers juggling many codebases
- AI-heavy developers whose repo sprawl exceeds memory
- small engineering teams that want a shared cleanup/reporting tool without platform overhead

### Secondary users

- OSS maintainers curating which projects are publishable
- technical founders deciding which prototypes deserve investment
- devtools users who want a local catalog of active vs dead code

### Not the target

- enterprises wanting a developer portal
- orgs wanting approval workflows or policy enforcement
- teams primarily buying security/compliance software
- users who need cloud syncing and centralized dashboards first

## Core jobs to be done

Users hire Nexus to:

- inventory all local and selected remote repos
- detect duplicates, forks, clones, and pivots
- determine the canonical working copy
- understand repo quality and recoverability
- prioritize what deserves attention
- export a deterministic plan for humans or agents

## Product principles

1. **Deterministic first**
   - Scoring and planning must work without AI.
   - Canonical selection must always be evidence-based.

2. **Local-first**
   - The best experience should work entirely on local repos.
   - External tools and APIs enhance, not define, the product.

3. **Fast by default**
   - First-run usefulness should happen in under a minute for normal developer machines.
   - Expensive adapters should be opt-in or profile-based.

4. **Explainable output**
   - Every important score should explain itself.
   - Every plan action should show evidence and trade-offs.

5. **Single engine, multiple shells**
   - CLI is primary.
   - TUI is a second shell over the same engine.
   - No separate web logic.

6. **Rust-native distribution**
   - The binary is the product.
   - Package managers wrap or distribute the binary.
   - No JS rewrite for npm/bun.

## Product surface

### Primary interface: CLI

Core commands for v1:

- `nexus scan` — inventory
- `nexus score` — scores and evidence (stdout; does not replace persisted plan)
- `nexus plan` — full plan with actions + persistence
- `nexus report` — human/machine reports
- `nexus doctor`
- `nexus tools`
- `nexus explain` — deterministic per-cluster view (text/JSON)
- `nexus export` / `nexus import` — inventory JSON backup and full DB inventory replace (clears persisted plan)

Shipped alongside core:

- `nexus tui` — interactive terminal browser (secondary interface)
- `nexus ai-summary` — AI-generated plan summary (experimental; requires config + API key)
- `nexus suggest` — AI-assisted suggestions (planned, not yet shipped)

### Secondary interface: TUI

The TUI should exist only if it materially improves inspection and decision-making.

Good TUI use cases:

- browse clusters and repo groups
- sort/filter by score, risk, status, or profile
- inspect canonical evidence
- preview recommended actions
- apply manual overrides or pin canonical choices
- export filtered reports/plans

Bad TUI use cases:

- replacing the CLI
- hiding evidence behind too much interaction
- becoming a pseudo-dashboard

## AI strategy

### Decision

**AI should be optional.**

Nexus must run perfectly without AI. AI is a value-add, not a dependency.

### What AI should do

- explain scores in natural language
- summarize evidence for a repo or cluster
- suggest next steps based on a deterministic plan
- help draft cleanup or publish-readiness checklists
- optionally summarize a repo for humans before handoff

### What AI should not do

- decide canonical identity on its own
- be required for scoring
- silently mutate scores or weights
- be the only path to useful output

### API model

Support:

- OpenAI-compatible endpoints
- user-supplied API key and base URL
- optional local model endpoints later if simple to support

The AI layer should consume structured Nexus data, not raw repo trees by default.

## Packaging and distribution

### Rule

The **Rust binary is the real product**.

### First-class channels

- `cargo`
- GitHub Releases / direct binary download
- Homebrew
- Chocolatey

### Wrapper channels

- `npm`
- `npx`
- `bunx`

The npm/bun package should be a thin wrapper that fetches the correct prebuilt binary. It should not reimplement Nexus in JavaScript.

### Nice-to-have later

- Scoop
- AUR
- Nix

## Scoring system

Nexus should not rely on a single magic score. It should provide a **small, explainable scoring model**.

### Default scores

#### 1. Canonical Confidence

How sure is Nexus that this clone/repo is the canonical working copy?

Signals:

- normalized remote URL match
- branch/head relationships
- merge-base evidence
- freshness and activity
- sibling clone similarity
- path/history consistency
- user override or manual pinning

#### 2. Repo Health

Is this repo operationally sane?

Signals:

- clean git state
- meaningful recent activity
- manifest presence
- lockfiles
- build/test cues
- documented scripts/tasks
- absence of obvious junk or broken structure

#### 3. Recoverability

How likely is the repo to be usable again after time has passed?

Signals:

- install/run instructions
- lockfiles and environment templates
- example commands
- tests or smoke checks
- dependency clarity
- config discoverability

#### 4. Maintenance Risk

How likely is this repo to waste time?

Signals:

- dependency sprawl
- duplicated codebase variants
- stale branches
- fragile build chains
- high complexity + low documentation
- secrets/binaries/generated artifacts committed

### Optional profiles

These should not distort the default scoring for all users.

#### Publish Readiness

Can this repo be handed to another person or published with confidence?

Signals:

- README quality
- LICENSE
- setup instructions
- examples
- tests
- changelog / release notes
- security/config hygiene

#### Open Source Readiness

A stricter profile layered on top of Publish Readiness for public OSS maintainers.

Additional signals may include:

- CONTRIBUTING
- CODE_OF_CONDUCT
- SECURITY policy
- issue templates / release process clarity

#### Security / Supply-Chain Posture

Optional adapter-backed profile using available tooling.

Signals may include:

- secret scan cleanliness
- static-analysis findings
- SBOM/dependency visibility
- dependency risk indicators

#### AI Handoff Readiness

How well can a repo be handed to an agent or AI-assisted workflow?

Signals:

- docs completeness
- obvious entry points
- stable command surface
- reproducible setup
- absence of ambiguous duplicate roots

## Why “OSS readiness” should not be the default

Not all users are OSS maintainers. If OSS readiness becomes a headline score, Nexus becomes biased toward public packaging rather than repo triage.

The better model is:

- default scores for everyone
- optional profiles for specific intents

Therefore:

- keep **Publish Readiness** as a broadly useful profile
- keep **Open Source Readiness** as an optional stricter profile
- do not make OSS compatibility part of the default overall score

## Command model

### Stable core

- `scan`: discover and inventory repos
- `score`: compute scores and evidence
- `plan`: generate prioritized actions
- `report`: render human/machine-readable output
- `doctor`: validate environment, adapters, and config
- `tools`: inspect optional adapter availability

### Shipped alongside core

- `export` / `import` (inventory JSON envelope; optional `--with-plan`)
- deterministic `explain` (text/JSON; optional `--ai` narrative)
- `tui`: interactive inspection and overrides
- `ai-summary`: AI-generated plan summary (experimental)

### Planned

- `suggest`: AI-assisted suggestions grounded in existing evidence

### Not a priority

- long-running server mode
- web dashboard mode
- destructive apply in stable releases

## Competitive position

Nexus should not try to beat broad platforms at their own game.

The winning position is:

- lighter than platform products
- more analytical than cleanup utilities
- more deterministic than AI wrappers
- more local-first than web-native catalogs

This gives Nexus a clear wedge:

**a trustworthy repo triage tool for developers with too many repos and too little certainty**

## Roadmap

### Phase 1 — sharp CLI value

Goal: useful in 60 seconds.

Ship and harden:

- `scan`
- `score` (inspect scores without persisting plan)
- `plan`
- `report`
- `doctor`

Success criteria:

- clear top-level scores
- clear canonical selection
- clear next-step plan
- machine-readable JSON output

### Phase 2 — TUI (shipped)

Goal: make inspection pleasant without changing product type.

Shipped:

- cluster browser
- score sorting/filtering
- canonical evidence panel
- manual override/pinning (TOML snippet)
- plan preview/export

### Phase 3 — adapter ecosystem

Goal: deepen analysis without bloating defaults.

Ship official best-effort adapters for:

- duplicate analysis
- static analysis
- secret scanning
- SBOM/dependency inspection

### Phase 4 — optional AI (partially shipped)

Goal: improve explanation and suggestions.

Shipped:

- optional AI on top of deterministic `explain` (`--ai` flag)
- `ai-summary` for plan-wide narrative
- OpenAI-compatible endpoint config (`[ai]` in `nexus.toml`)
- strict grounding in deterministic Nexus output

Remaining:

- `suggest` command

### Phase 5 — distribution breadth

Goal: low-friction install everywhere relevant.

Priority order:

1. cargo
2. GitHub Releases
3. Homebrew
4. Chocolatey
5. npm / npx / bunx wrapper
6. Scoop / AUR / Nix later

## Success metrics

Early product success should be measured by:

- time to first useful scan
- percentage of users who understand their canonical repos after first run
- number of actionable findings per scan
- quality and trust in score explanations
- number of users who can use Nexus without AI
- install success across package channels

## Non-goals

For the foreseeable roadmap, Nexus should avoid:

- a web dashboard
- multi-user collaboration features
- automatic commits, PRs, or repo mutations
- AI-only functionality
- enterprise portal ambitions
- scoring that cannot explain itself

## One-line positioning

**Nexus is the local-first CLI that tells you which repos matter, which copy is real, and what to do next.**
