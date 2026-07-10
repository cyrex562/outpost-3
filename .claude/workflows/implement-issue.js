export const meta = {
  name: 'implement-issue',
  description: 'Pick next open GitHub issue, implement it in Rust, run tests, and create a PR',
  phases: [
    { title: 'Select', detail: 'Choose next open issue respecting phase order and dependencies' },
    { title: 'Implement', detail: 'Write Rust code satisfying the issue spec' },
    { title: 'Test', detail: 'cargo test + clippy + fmt check + harness check if applicable' },
    { title: 'Ship', detail: 'Push branch, create PR, merge on green' },
  ],
}

// ---------------------------------------------------------------------------
// Config
// ---------------------------------------------------------------------------
const OWNER = 'cyrex562'
const REPO = 'outpost-3'
const DEFAULT_BRANCH = 'main'

// Milestone label priority order — earlier milestones block later ones
const PHASE_ORDER = [
  'M1',
  'M2',
  'M3',
  'M4',
  'M5',
  'M6',
  'M7',
  'M8',
  'M9',
  // Legacy phase labels (all closed)
  'phase-1-core',
  'phase-2-colony',
  'phase-3-harness',
  'phase-4-control',
  'phase-4b-tech',
  'phase-5-planet',
  'phase-6-ui',
  'phase-7-pop-events',
  'phase-8-orbital-system',
  'phase-9-expeditions',
  'phase-10-endgame',
  'phase-11-ci',
]

// Issue numbers that must be closed before a phase can start
// M-series milestones have no hard gates — all M1 issues can proceed immediately
const PHASE_GATES = {
  'M1': [],
  'M2': [],
  'M3': [],
  'M4': [],
  'M5': [],
  'M6': [],
  'M7': [],
  'M8': [],
  'M9': [],
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function slugify(title) {
  return title
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-|-$/g, '')
    .slice(0, 50)
}

function phaseOf(labels) {
  for (const phase of PHASE_ORDER) {
    if (labels.includes(phase)) return phase
  }
  return null
}

// ---------------------------------------------------------------------------
// Phase 1 — Select issue
// ---------------------------------------------------------------------------
phase('Select')

const ISSUE_SCHEMA = {
  type: 'object',
  properties: {
    issue_number: { type: 'number' },
    title: { type: 'string' },
    body: { type: 'string' },
    labels: { type: 'array', items: { type: 'string' } },
    phase_label: { type: 'string' },
    branch_name: { type: 'string' },
    reasoning: { type: 'string' },
    can_proceed: { type: 'boolean' },
    block_reason: { type: 'string' },
  },
  required: ['issue_number', 'title', 'body', 'labels', 'phase_label', 'branch_name', 'can_proceed'],
}

// If args specifies an issue number, target it directly; otherwise pick next
const targetIssue = args && args.issue_number ? args.issue_number : null

const selection = await agent(
  `You are selecting the next GitHub issue to implement for the Outpost 3 project.

Repository: ${OWNER}/${REPO}

${targetIssue ? `The user requested issue #${targetIssue} specifically.` : `Pick the NEXT open issue to work on using this priority order:
Phase priority (lowest number = highest priority): ${PHASE_ORDER.join(' > ')}
Within a phase: lowest issue number first.
Skip issues labeled "in-progress" or "blocked".
Skip issues with open "Depends on" references that are not yet closed.`}

Phase gates (issue numbers that must be CLOSED before starting a later phase):
${JSON.stringify(PHASE_GATES, null, 2)}

Steps:
1. Use the GitHub MCP tool (mcp__github__list_issues) to list open issues for ${OWNER}/${REPO}. The new issues use milestone labels M1–M9 (not phase-N-* labels). Issues #81 and above are the active backlog.
2. ${targetIssue ? `Find issue #${targetIssue} specifically — it is open and ready to implement, set can_proceed=true.` : 'Apply the phase priority and dependency rules to select the best candidate. Prefer M1 issues first (lowest number).'}
3. Check the PHASE_GATES: if any gate issues for this issue's phase are still open, set can_proceed=false and explain in block_reason. For M-series issues there are no gates — always set can_proceed=true.
4. Generate a branch name: "issue-{number}-{slug}" where slug is a short kebab-case version of the title.
5. Return the selected issue's full body text in the "body" field.

Return a StructuredOutput matching the schema.`,
  { label: 'select-issue', phase: 'Select', schema: ISSUE_SCHEMA }
)

if (!selection || !selection.can_proceed) {
  const reason = selection ? selection.block_reason : 'agent returned null'
  log(`⛔ Cannot proceed: ${reason}`)
  return { blocked: true, reason, selection }
}

log(`✅ Selected: #${selection.issue_number} — ${selection.title}`)
log(`   Branch: ${selection.branch_name}`)
log(`   Phase: ${selection.phase_label}`)

// ---------------------------------------------------------------------------
// Phase 2 — Implement
// ---------------------------------------------------------------------------
phase('Implement')

const CONTEXT_SCHEMA = {
  type: 'object',
  properties: {
    design_excerpt: { type: 'string' },
    reference_files: { type: 'array', items: { type: 'string' } },
  },
  required: ['design_excerpt', 'reference_files'],
}

const context = await agent(
  `You are gathering context for implementing GitHub issue #${selection.issue_number} in the Outpost 3 repo.

Issue title: ${selection.title}
Issue body:
${selection.body}

Tasks:
1. Read /home/user/outpost-3/docs/DESIGN.md and extract the sections referenced in the issue body (e.g., "§5", "§7A", "§14"). Return a 300-500 word excerpt of the most relevant passages.
2. List the key files in /home/user/outpost-3/reference/harsh_realm-main/crates/harsh-core/src/ that are most relevant to this issue (list file paths only, do not read them yet).
3. If the issue mentions prior C# behavioral specs (e.g., "ColonyTurnProcessor.cs"), list those file paths in /home/user/outpost-3/godot/src/Core/

Return a StructuredOutput with the gathered context.`,
  { label: 'gather-context', phase: 'Implement', schema: CONTEXT_SCHEMA }
)

const IMPL_SCHEMA = {
  type: 'object',
  properties: {
    files_created: { type: 'array', items: { type: 'string' } },
    files_modified: { type: 'array', items: { type: 'string' } },
    test_count: { type: 'number' },
    summary: { type: 'string' },
    commit_message: { type: 'string' },
  },
  required: ['files_created', 'files_modified', 'test_count', 'summary', 'commit_message'],
}

const impl = await agent(
  `You are implementing GitHub issue #${selection.issue_number} for the Outpost 3 project.

## Issue

**Title:** ${selection.title}

**Full spec:**
${selection.body}

## Design Context

${context ? context.design_excerpt : '(context agent returned null — read docs/DESIGN.md yourself)'}

## Architecture Rules (from docs/DESIGN.md §14)

- \`outpost_core\` is a pure Rust library: ZERO I/O, ZERO framework deps, ZERO async runtime in lib code.
- All game logic is in \`outpost_core\`. A separate \`outpost_harness\` binary crate is allowed for CLI tooling.
- Content is data (YAML/JSON pack files), never hardcoded in the kernel.
- The drive interface is the only mutation point: \`GameEngine::apply(cmd) -> Result<Vec<Event>, EngineError>\`.
- SQLite is snapshot-only (between turns), never per-mutation live state.
- Rust edition 2021; use \`serde\`, \`serde_yaml\`, \`rusqlite\` where needed; no tokio/async in core.

## Relevant Reference Files (from harsh_realm — structural patterns only, different domain)

${context ? context.reference_files.join('\n') : '(none listed)'}
You may read these for structural patterns but do NOT copy game logic — it is a different game.

## Steps

1. First run \`git status\` and \`git log --oneline -5\` to understand the current state.
2. Check out branch \`${selection.branch_name}\` from ${DEFAULT_BRANCH}:
   \`git checkout -B ${selection.branch_name} origin/${DEFAULT_BRANCH}\`
3. Examine the existing Rust workspace structure (if it exists): look in the repo root for \`Cargo.toml\`, \`crates/\`, or \`outpost_core/\`.
4. If this is a scaffold issue (#7), CREATE the Rust workspace. Otherwise, extend the existing one.
5. Implement the feature exactly as specified in the issue "Task" and "Done when" sections.
6. Write \`cargo test\`-runnable NUnit-equivalent tests (Rust \`#[test]\` functions) for every "Done when" bullet.
7. Every new public type should have a doc comment (one line max).
8. Run \`cargo build\` and \`cargo test\` to verify green. Fix any compile errors before stopping.
9. Stage and commit: \`git add -A && git commit -m "<commit_message>"\`
   Commit message format: "Phase N: <brief description>\\n\\nCloses #${selection.issue_number}"

Return a StructuredOutput describing what was done.`,
  { label: 'implement', phase: 'Implement', schema: IMPL_SCHEMA }
)

if (!impl) {
  log('⚠️ Implementation agent returned null — manual intervention needed')
  return { success: false, stage: 'implement', selection }
}

log(`📝 Implementation complete`)
log(`   Files created: ${impl.files_created.length}`)
log(`   Files modified: ${impl.files_modified.length}`)
log(`   Tests: ${impl.test_count}`)

// ---------------------------------------------------------------------------
// Phase 3 — Test
// ---------------------------------------------------------------------------
phase('Test')

const TEST_SCHEMA = {
  type: 'object',
  properties: {
    cargo_test_passed: { type: 'boolean' },
    cargo_clippy_passed: { type: 'boolean' },
    cargo_fmt_passed: { type: 'boolean' },
    harness_passed: { type: 'boolean' },
    harness_applicable: { type: 'boolean' },
    all_passed: { type: 'boolean' },
    failure_details: { type: 'string' },
  },
  required: ['cargo_test_passed', 'cargo_clippy_passed', 'cargo_fmt_passed', 'all_passed'],
}

const testResult = await agent(
  `You are verifying that the implementation of issue #${selection.issue_number} passes all quality checks.

The implementation is on branch \`${selection.branch_name}\`.

Run the following checks in order. Fix any failures before reporting (up to 3 attempts per check):

1. \`cargo test --workspace 2>&1\`
   If tests fail: read the error, fix the code, re-run. Report cargo_test_passed.

2. \`cargo clippy --workspace -- -D warnings 2>&1\`
   Fix all clippy warnings (they are errors). Report cargo_clippy_passed.

3. \`cargo fmt --check --all 2>&1\`
   If fmt fails, run \`cargo fmt --all\` then re-check. Report cargo_fmt_passed.

4. Check if the issue is Phase 3 or later AND a \`content/checks/\` bundle exists for it.
   If so, run: \`cargo run --bin outpost_harness -- check content/checks/ 2>&1\`
   Report harness_applicable and harness_passed.

After all checks pass, stage any fmt fixes with a new commit (do NOT amend):
\`git add -A && git diff --cached --quiet || git commit -m "chore: apply cargo fmt fixes"\`

If any check cannot be made green after 3 attempts, set all_passed=false and describe the failure in failure_details.

Return a StructuredOutput with all check results.`,
  { label: 'test', phase: 'Test', schema: TEST_SCHEMA }
)

if (!testResult || !testResult.all_passed) {
  const detail = testResult ? testResult.failure_details : 'agent returned null'
  log(`❌ Tests failed: ${detail}`)
  return { success: false, stage: 'test', selection, impl, testResult }
}

log(`✅ All checks passed`)
log(`   cargo test: ${testResult.cargo_test_passed}`)
log(`   clippy: ${testResult.cargo_clippy_passed}`)
log(`   fmt: ${testResult.cargo_fmt_passed}`)

// ---------------------------------------------------------------------------
// Phase 4 — Ship
// ---------------------------------------------------------------------------
phase('Ship')

const SHIP_SCHEMA = {
  type: 'object',
  properties: {
    pushed: { type: 'boolean' },
    pr_url: { type: 'string' },
    pr_number: { type: 'number' },
    merged: { type: 'boolean' },
  },
  required: ['pushed', 'pr_url', 'pr_number', 'merged'],
}

const ship = await agent(
  `You are creating and merging a pull request for issue #${selection.issue_number} — "${selection.title}".

Branch: \`${selection.branch_name}\`
Base: \`${DEFAULT_BRANCH}\`
Repository: ${OWNER}/${REPO}

Steps:
1. Push the branch:
   \`git push -u origin ${selection.branch_name}\`
   On failure, retry up to 4 times with exponential backoff (2s, 4s, 8s, 16s).

2. Use the GitHub MCP tool (mcp__github__create_pull_request) to create a PR:
   - title: "${selection.title}"
   - base: "${DEFAULT_BRANCH}"
   - head: "${selection.branch_name}"
   - body: include:
     - "Closes #${selection.issue_number}"
     - Summary of what was implemented (from: ${impl ? impl.summary : '(see commit)'})
     - Test plan: list the cargo test cases added
     - Phase label: ${selection.phase_label}

3. Use (mcp__github__merge_pull_request) to merge with method "squash".

4. After merge, close issue #${selection.issue_number} on GitHub using (mcp__github__issue_write) method "update" with state "closed" and state_reason "completed".

5. Delete the local branch: \`git checkout ${DEFAULT_BRANCH} && git branch -d ${selection.branch_name}\`

Return a StructuredOutput with push/PR/merge results.`,
  { label: 'ship', phase: 'Ship', schema: SHIP_SCHEMA }
)

if (!ship || !ship.merged) {
  log(`⚠️ Ship stage incomplete — manual PR/merge may be needed`)
  return { success: false, stage: 'ship', selection, impl, testResult, ship }
}

log(`🚀 Shipped!`)
log(`   PR: ${ship.pr_url}`)
log(`   Merged: ${ship.merged}`)

return {
  success: true,
  issue_number: selection.issue_number,
  title: selection.title,
  branch: selection.branch_name,
  pr_url: ship.pr_url,
  files_created: impl ? impl.files_created : [],
  files_modified: impl ? impl.files_modified : [],
  test_count: impl ? impl.test_count : 0,
}
