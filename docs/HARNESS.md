# Outpost 3 — Implementation Harness

## What It Does

The harness is a Claude Code workflow that automates the implementation loop:

1. **Select** — picks the next open GitHub issue respecting phase order and dependency gates
2. **Implement** — spawns a Rust/Vue coding agent with full design context to write the feature
3. **Test** — runs `cargo test`, `cargo clippy`, `cargo fmt --check`, the balance harness, and — when the diff touches `frontend/src/` — `npm run type-check`, `npm run test:unit`, `npm run build`, and (for user-facing changes) `npm run test:e2e`. See CLAUDE.md's [Definition of Done](../CLAUDE.md#definition-of-done) for the full gate.
4. **Review** — an independent code-review subagent (Sonnet if the diff touches `outpost_core/`, Haiku otherwise) audits the diff against CLAUDE.md's Rust Best Practices and Critical Rules; fixes blocking findings itself and re-verifies
5. **Judge** — a separate, blind Haiku subagent checks the issue's acceptance criteria against the diff and its tests independently of the reviewer's findings
6. **Ship** — pushes the branch and opens a PR with a `## Review` section summarizing both agents' verdicts; squash-merges and closes the issue **only if** tests + review + judge are all green. If the gate isn't green, the PR stays open for a human instead.

## Running the Harness

### Via Claude Code CLI

```bash
# Pick and implement the next issue automatically
claude workflow implement-issue

# Target a specific issue
claude workflow implement-issue --args '{"issue_number": 7}'

# Target a specific phase
claude workflow implement-issue --args '{"phase": "phase-1-core"}'
```

### Via Claude Code Chat

In the chat interface, trigger the workflow:

```
/workflow implement-issue
```

Or with a specific issue:

```
/workflow implement-issue {"issue_number": 7}
```

## Issue Selection Rules

The harness picks the **lowest-numbered open issue** in the **earliest open phase**:

| Phase | Label | Gate issues (must be closed first) |
|---|---|---|
| 1 | `phase-1-core` | none |
| 2 | `phase-2-colony` | #7, #8, #9, #10, #11 |
| 3 | `phase-3-harness` | #12, #14 |
| 4 | `phase-4-control` | #15, #16 |
| 4b | `phase-4b-tech` | #17, #9 |
| 5 | `phase-5-planet` | #16 |
| 6 | `phase-6-ui` | #10 |
| 7 | `phase-7-pop-events` | #23 |
| 8 | `phase-8-orbital-system` | — |
| 9 | `phase-9-expeditions` | — |
| 10 | `phase-10-endgame` | #25 |
| 11 | `phase-11-ci` | #7 |

Issues labeled `in-progress` or `blocked` are skipped.

## Branch Naming

`issue-{number}-{slug}` where slug is a kebab-case abbreviation of the issue title.

Example: `issue-7-scaffold-rust-workspace-pure-library-sim-core`

## All Issues

| # | Title | Phase | Fidelity |
|---|---|---|---|
| #7 | Scaffold Rust workspace | phase-1-core | high |
| #8 | In-memory turn model + two-cadence turn loop | phase-1-core | high |
| #9 | Content-pack loading pipeline | phase-1-core | high |
| #10 | Programmatic drive interface | phase-1-core | high |
| #11 | SQLite snapshot / restore between turns | phase-1-core | high |
| #12 | Commodity + recipe data model | phase-2-colony | high |
| #13 | Building model + finite tech-gated build slots | phase-2-colony | high |
| #14 | Production step in the turn pipeline | phase-2-colony | high |
| #15 | Population aggregate pool + labor | phase-2-colony | high |
| #16 | Needs resolution + stability dynamics | phase-2-colony | high |
| #17 | Research as a commodity | phase-2-colony | high |
| #18 | Static flow-balance calculator | phase-3-harness | high |
| #19 | Harness CLI + report output | phase-3-harness | high |
| #20 | Prototyping-loop runner hook | phase-3-harness | high |
| #21 | Condition/predicate language + evaluator | phase-4-control | high |
| #22 | Directive system (auto-handle) + manual override | phase-4-control | high |
| #23 | Interrupt tiers + threshold + "wait N turns" | phase-4-control | high |
| #24 | Tech DAG + unlock application | phase-4b-tech | high |
| #25 | Effect/modifier descriptor | phase-4b-tech | high |
| #26 | Hex map + colonies-as-nodes + infrastructure | phase-5-planet | medium |
| #27 | Inter-colony trade + expansion | phase-5-planet | medium |
| #28 | Vue frontend spine | phase-6-ui | medium |
| #29 | Colony screen + planet map + interrupt digest UI | phase-6-ui | medium |
| #30 | Population dynamics, migration, predictive warnings | phase-7-pop-events | medium |
| #31 | Orbital layer (epic) | phase-8-orbital-system | coarse |
| #32 | System zoom — world specialization, megaprojects (epic) | phase-8-orbital-system | coarse |
| #33 | Expeditions (epic) | phase-9-expeditions | coarse |
| #34 | Difficulty, existential clock, victory (epic) | phase-10-endgame | coarse |
| #35 | CI pipeline | phase-11-ci | coarse |

## Architectural Constraints (enforced by the harness agent)

Every implementation agent is given these rules extracted from `docs/DESIGN.md §14`:

- `outpost_core` is a **pure Rust library** — zero I/O, zero async, zero framework deps
- Content is **data files** (YAML/JSON packs), never hardcoded in kernel
- SQLite is **snapshot-only** (between turns), never per-mutation live state
- The **drive interface** (`GameEngine::apply()`) is the only mutation point
- All public types get a **one-line doc comment**
- **Every applicable Definition of Done tier must be green** — not just `cargo test` — before the PR is created
- **The code-review and judge agents must both pass** before the harness merges the PR (see `CLAUDE.md`'s [Automated Review & Validation](../CLAUDE.md#automated-review--validation))

## Reference Material

The harness agent reads these when implementing each issue:

- `docs/DESIGN.md` — authoritative design spec (sections referenced in each issue)
- `reference/harsh_realm-main/` — Rust + Vue structural patterns (different game domain; borrow architecture, not logic)
- `godot/src/Core/` — prior C# behavioral spec (163 passing tests); re-express in Rust, do not port

## Adding Balance Check Bundles

When a Phase 3 issue is complete, create a bundle at `content/checks/<name>/`:

```
content/checks/<name>/
  pack.yaml         ← pack manifest (id, name, version, description)
  commodities.yaml  ← commodities this test needs
  buildings.yaml    ← buildings this test needs
  recipes.yaml      ← recipes this test needs
  colony.yaml       ← which buildings the colony has, and any imports
  assertions.yaml   ← what the balance calculator should report
```

(Flat files, not a `pack/` subdirectory, and `colony.yaml` rather than `config.yaml` — see `cmd_check` in `outpost_harness/src/main.rs` for the authoritative list.)

The harness automatically runs `outpost_harness check content/checks/` on Phase 3+ issues.

### Two things a bundle reports

`check` prints two blocks:

- **ASSERTION CHECK RESULTS** — per-commodity net rates against the bundle's `assertions.yaml`. This is what passes or fails the bundle, and what sets the exit code.
- **COLONY FOOTPRINT** — what the configuration *costs*: build slots, worker slots, summed `power_delta` (negative in the data means a net producer; the block prints it the way round a reader expects), longest single build time, and the total construction bill by commodity. Informational only, never asserted.

The footprint exists because `BalanceCalculator` reads recipe flows and **nothing else** — not `power_delta`, not `worker_slots`, not `slot_cost`, not `maintenance`. That makes it excellent at "does this chain close?" and blind to "at what price?", which is a whole class of balance question. Issue #272 gap 5 — is one consolidated building in one build slot well-tuned against three standalone buildings in three? — is entirely about price, and could not be answered at all before this block existed.

It is deliberately not assertable. A cost figure is only meaningful *relative to an alternative*, so the way to use it is to author two bundles covering the same functions and diff their footprints; `content/checks/colony_hq_efficiency/` and `content/checks/standalone_trio/` are that pair. A `min_net`-style threshold on a slot count would be a number invented from nothing.

**If you want a bundle's footprint to mean anything, author the cost fields in its local `buildings.yaml`.** They all have `#[serde(default)]`, so a bundle that omits them reports zero power, zero workers, and one slot per building regardless of the real content — silently, and the assertions still pass. Bundles whose purpose is commodity closure can leave them out; bundles used for cost comparison must mirror `content/base` exactly, and should say so in a comment, because a drifted number there invalidates the comparison without failing anything.

## Failure Modes

If the harness returns `success: false`:

- `stage: "implement"` — the agent couldn't compile the code; check the branch manually
- `stage: "test"` — tests, clippy, fmt, or a frontend/Playwright gate failed; the branch exists with the code, fix and push
- `stage: "ship"` — the PR couldn't even be created (push or `create_pull_request` failed); branch is pushed, open the PR manually

If the harness returns `success: true, merged: false`, everything up through Ship worked — a PR is open — but the auto-merge gate wasn't green: `testResult.all_passed`, `review.clean`, and `judge.all_met` must **all** be true to merge. Check the returned `review`/`judge` objects (or the PR's `## Review` section) for which one failed, decide whether to fix and re-run or merge manually, then act.

After fixing, re-run the harness targeting the specific issue number to retry from Test onward.
