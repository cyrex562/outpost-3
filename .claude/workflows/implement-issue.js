export const meta = {
  name: 'implement-issue',
  description: 'Pick next open GitHub issue, implement it in Rust, run tests, review, judge, and ship a PR',
  phases: [
    { title: 'Select', detail: 'Choose next open issue respecting phase order and dependencies' },
    { title: 'Implement', detail: 'Write Rust/Vue code satisfying the issue spec' },
    { title: 'Test', detail: 'cargo test + clippy + fmt + frontend (vitest/type-check/build/playwright) + harness check if applicable' },
    { title: 'Review', detail: 'Haiku/Sonnet code-review subagent audits the diff; fixes on the spot or files a follow-up issue' },
    { title: 'Judge', detail: 'Haiku subagent independently confirms acceptance criteria; gaps get fixed or filed, not just flagged' },
    { title: 'Ship', detail: 'Push branch, create PR, auto-merge unless a genuine human decision is needed' },
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
Skip issues with open "Depends on" references that are not yet closed.
Skip issues whose own body says they are speculative/not currently planned/not near-term, or explicitly
"Blocked on" something not yet done — treat that language as a soft block even without a label.`}

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
1. Read docs/DESIGN.md (repo root) and extract the sections referenced in the issue body (e.g., "§5", "§7A", "§14"). Return a 300-500 word excerpt of the most relevant passages.
2. List the key files in reference/harsh_realm-main/crates/harsh-core/src/ that are most relevant to this issue (list file paths only, do not read them yet).
3. If the issue mentions prior C# behavioral specs (e.g., "ColonyTurnProcessor.cs"), list those file paths in godot/src/Core/

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
6. Write \`cargo test\`-runnable tests (Rust \`#[test]\` functions) for every "Done when" bullet.
7. Every new public type should have a doc comment (one line max).
8. Run \`cargo build\` and \`cargo test\` to verify green. Fix any compile errors before stopping.
9. Stage and commit: \`git add -A && git commit -m "<commit_message>"\`
   Commit message format: "Issue #${selection.issue_number}: <brief description>\\n\\nCloses #${selection.issue_number}"

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

const touchedFrontend = (impl.files_created || [])
  .concat(impl.files_modified || [])
  .some((f) => f.startsWith('frontend/'))

const TEST_SCHEMA = {
  type: 'object',
  properties: {
    cargo_test_passed: { type: 'boolean' },
    cargo_clippy_passed: { type: 'boolean' },
    cargo_fmt_passed: { type: 'boolean' },
    harness_passed: { type: 'boolean' },
    harness_applicable: { type: 'boolean' },
    frontend_applicable: { type: 'boolean' },
    frontend_type_check_passed: { type: 'boolean' },
    frontend_unit_passed: { type: 'boolean' },
    frontend_build_passed: { type: 'boolean' },
    frontend_e2e_applicable: { type: 'boolean' },
    frontend_e2e_env_available: { type: 'boolean' },
    frontend_e2e_passed: { type: 'boolean' },
    all_passed: { type: 'boolean' },
    failure_details: { type: 'string' },
  },
  required: ['cargo_test_passed', 'cargo_clippy_passed', 'cargo_fmt_passed', 'all_passed'],
}

const testResult = await agent(
  `You are verifying that the implementation of issue #${selection.issue_number} passes all quality checks
defined in CLAUDE.md's "Definition of Done" table.

The implementation is on branch \`${selection.branch_name}\`.

Run the following checks in order. Fix any failures before reporting (up to 3 attempts per check):

1. \`cargo test --workspace 2>&1\`
   If tests fail: read the error, fix the code, re-run. Report cargo_test_passed.

2. \`cargo clippy --workspace -- -D warnings 2>&1\`
   Fix all clippy warnings (they are errors). Report cargo_clippy_passed.

3. \`cargo fmt --check --all 2>&1\`
   If fmt fails, run \`cargo fmt --all\` then re-check. Report cargo_fmt_passed.

4. Check if a \`content/checks/\` bundle exists for this issue.
   If so, run: \`cargo run --bin outpost_harness -- check content/checks/ 2>&1\`
   Report harness_applicable and harness_passed.

5. Frontend gates — this diff ${touchedFrontend ? 'DOES' : 'does NOT'} touch \`frontend/src/\`.
   Set frontend_applicable=${touchedFrontend}. If applicable, from the \`frontend/\` directory run, in order:
   - \`npm run type-check 2>&1\` — report frontend_type_check_passed
   - \`npm run test:unit -- --run 2>&1\` — report frontend_unit_passed
   - \`npm run build 2>&1\` — report frontend_build_passed
   - If the diff affects a user-facing flow (new screen, new command wiring, changed navigation),
     add or update a Playwright spec under \`frontend/e2e/\` covering the new behavior.
     Before running it, check whether a real browser is actually available in this environment:
     \`~/.cache/ms-playwright\`, \`$PLAYWRIGHT_BROWSERS_PATH\` (verify the path exists, don't just check
     the env var is set), \`/opt/pw-browsers\`, or a system chromium/chrome/chromium-browser binary.
     - If NONE of those exist: set frontend_e2e_applicable=true, frontend_e2e_env_available=false,
       frontend_e2e_passed=false, and say so explicitly in failure_details. Do NOT run
       \`playwright install\` to work around it — CLAUDE.md forbids that. This is an environment gap,
       not a code defect, and does not block merge per CLAUDE.md's auto-merge policy — but say so
       clearly rather than silently omitting it.
     - If a browser IS available: set frontend_e2e_env_available=true, run \`npm run test:e2e 2>&1\`,
       and report frontend_e2e_passed truthfully. A genuine failure here (browser present, suite ran
       and failed) DOES block merge like any other tier — this is a real regression, not an
       environment gap, so don't paper over it.
     If the change has no user-facing surface, set frontend_e2e_applicable=false and skip it —
     say so explicitly in failure_details rather than silently omitting the field.
   If not applicable, set all four frontend_* passed fields to true (vacuously), frontend_e2e_applicable=false,
   frontend_e2e_env_available=true (vacuous).

After all checks pass, stage any fmt/lint fixes with a new commit (do NOT amend):
\`git add -A && git diff --cached --quiet || git commit -m "chore: apply cargo fmt fixes"\`

If any check cannot be made green after 3 attempts, set all_passed=false and describe the failure in failure_details.

Return a StructuredOutput with all check results.`,
  { label: 'test', phase: 'Test', schema: TEST_SCHEMA }
)

// The tiers that can actually execute in every environment — these are a
// hard gate. e2e is handled separately below since it may be genuinely
// inapplicable-to-verify-here rather than failing.
const coreTestsPassed =
  !!testResult &&
  testResult.cargo_test_passed &&
  testResult.cargo_clippy_passed &&
  testResult.cargo_fmt_passed &&
  (testResult.harness_applicable ? testResult.harness_passed : true) &&
  (!testResult.frontend_applicable ||
    (testResult.frontend_type_check_passed && testResult.frontend_unit_passed && testResult.frontend_build_passed))

if (!testResult || !coreTestsPassed) {
  const detail = testResult ? testResult.failure_details : 'agent returned null'
  log(`❌ Core checks failed: ${detail}`)
  return { success: false, stage: 'test', selection, impl, testResult }
}

// e2e blocks merge only when a browser was actually available and the suite
// genuinely failed — not when the sandbox has no Playwright browser installed
// at all (environment gap, documented in the PR instead of gating on it).
// Mirrors the existing outpost_tauri/WebKit2GTK precedent in CLAUDE.md.
const e2eGenuinelyFailed =
  testResult.frontend_e2e_applicable && testResult.frontend_e2e_env_available && !testResult.frontend_e2e_passed

if (testResult.frontend_e2e_applicable && !testResult.frontend_e2e_env_available) {
  log(`⚠️ e2e could not run — no browser available in this sandbox. Every other tier is green; proceeding. Does not block merge.`)
} else if (e2eGenuinelyFailed) {
  log(`⚠️ e2e ran and FAILED — this DOES block merge (a real regression, not an environment gap).`)
}

log(`✅ Core checks passed`)
log(`   cargo test: ${testResult.cargo_test_passed}`)
log(`   clippy: ${testResult.cargo_clippy_passed}`)
log(`   fmt: ${testResult.cargo_fmt_passed}`)

// ---------------------------------------------------------------------------
// Phase 4 — Review
// ---------------------------------------------------------------------------
phase('Review')

const REVIEW_SCHEMA = {
  type: 'object',
  properties: {
    blocking_findings: { type: 'array', items: { type: 'string' } },
    non_blocking_notes: { type: 'array', items: { type: 'string' } },
    fixed_blocking_findings: { type: 'boolean' },
    clean: { type: 'boolean' },
    summary: { type: 'string' },
    unresolved_kind: { type: 'string', enum: ['none', 'needs_human_decision', 'follow_up_filed'] },
    human_question: { type: 'string' },
    filed_issue_url: { type: 'string' },
  },
  required: ['blocking_findings', 'non_blocking_notes', 'clean', 'summary', 'unresolved_kind'],
}

// Sonnet for anything touching simulation logic in outpost_core; Haiku for
// smaller/mechanical diffs (frontend-only, content-pack-only, docs) — see
// CLAUDE.md "Automated Review & Validation".
const touchedCore = (impl.files_created || [])
  .concat(impl.files_modified || [])
  .some((f) => f.startsWith('outpost_core/'))
const reviewModel = touchedCore ? 'claude-sonnet-5' : 'claude-haiku-4-5-20251001'

let review = await agent(
  `You are an independent code-review agent auditing the diff for issue #${selection.issue_number} —
"${selection.title}" — on branch \`${selection.branch_name}\` (run \`git diff origin/${DEFAULT_BRANCH}...HEAD\`
to see it).

Review against CLAUDE.md's "Rust Best Practices" and "Critical Rules" sections (read that file first):
- Correctness bugs, unhandled edge cases, panics on reachable input
- Violations of the architecture rules (zero I/O in outpost_core, content-as-data, drive-interface-only
  mutation, SQLite snapshot-only, no simulation logic leaking into outpost_tauri/outpost_web)
- Missing doc comments on new public items
- Unjustified new external dependencies in outpost_core

Classify findings as blocking (real bugs, panics, architecture violations) or non-blocking (style nits,
optional simplifications — note and skip, these never block merge).

For each blocking finding, in this order of preference:
1. **Fix it yourself**, then re-verify (re-run the relevant \`cargo test\`/\`clippy\` commands). If every
   blocking finding is resolved this way, set fixed_blocking_findings=true, clean=true,
   unresolved_kind="none".
2. If it's real but genuinely out of scope for this diff (a pre-existing problem elsewhere, a larger
   refactor that deserves its own PR) — **file a new GitHub issue** describing it via the GitHub MCP
   tool (mcp__github__create_issue) in ${OWNER}/${REPO}, referencing issue #${selection.issue_number}.
   Set clean=true (this diff itself is fine to ship as-is), unresolved_kind="follow_up_filed", and
   filed_issue_url to the created issue's URL.
3. Only if it hinges on a genuine open question that only a human can answer (an ambiguous requirement,
   a real design/scope tradeoff, not something you can reasonably decide) — leave it. Set clean=false,
   unresolved_kind="needs_human_decision", and put the specific question in human_question. This blocks
   merge; use it sparingly, only when you truly cannot decide or fix it yourself.

Prefer options 1 or 2 whenever you reasonably can. Do not use option 3 just because a finding is
inconvenient to fix — it's for genuine judgment calls only.

Return a StructuredOutput with your findings.`,
  { label: 'code-review', phase: 'Review', schema: REVIEW_SCHEMA, model: reviewModel }
)

if (!review) {
  log('⚠️ Review agent returned null — treating as unresolved, will not auto-merge')
  review = {
    clean: false,
    blocking_findings: ['review agent failed to return a result'],
    non_blocking_notes: [],
    summary: '',
    unresolved_kind: 'needs_human_decision',
    human_question: 'Review agent failed to return a result.',
  }
}

log(
  review.clean
    ? '✅ Code review clean'
    : `⚠️ Code review found ${review.blocking_findings.length} blocking finding(s) — ${review.unresolved_kind}`
)

const reviewMergeable = review.clean === true

// ---------------------------------------------------------------------------
// Phase 5 — Judge (with one bounded fix-and-rejudge pass)
// ---------------------------------------------------------------------------
phase('Judge')

const JUDGE_SCHEMA = {
  type: 'object',
  properties: {
    criteria_checked: { type: 'array', items: { type: 'string' } },
    criteria_met: { type: 'array', items: { type: 'boolean' } },
    all_met: { type: 'boolean' },
    reasoning: { type: 'string' },
  },
  required: ['criteria_checked', 'criteria_met', 'all_met', 'reasoning'],
}

function judgePrompt() {
  return `You are an independent judge verifying that issue #${selection.issue_number} — "${selection.title}" —
is actually done, not just that its tests pass.

Issue body (extract the "Done when" / acceptance-criteria bullets from this):
${selection.body}

The implementation is on branch \`${selection.branch_name}\` (run \`git diff origin/${DEFAULT_BRANCH}...HEAD\`
to see it, and read the changed test files to see what they actually assert).

For each acceptance-criterion bullet in the issue, determine independently whether the diff and its tests
genuinely satisfy it — not just that some test with a plausible name exists, but that the test's assertions
would actually fail if the criterion were violated. List each criterion you checked and whether it's met.

Return a StructuredOutput with your verdict.`
}

// Deliberately a separate, blind agent call — it must not see `review`'s
// findings so its verdict isn't anchored by the reviewer's framing.
let judge = await agent(judgePrompt(), { label: 'judge', phase: 'Judge', schema: JUDGE_SCHEMA, model: 'claude-haiku-4-5-20251001' })

if (!judge) {
  log('⚠️ Judge agent returned null — treating as unresolved, will not auto-merge')
  judge = { all_met: false, criteria_checked: [], criteria_met: [], reasoning: 'judge agent failed to return a result' }
}

log(judge.all_met ? '✅ Judge confirms acceptance criteria met' : '⚠️ Judge found unmet acceptance criteria — attempting a fix')

let judgeUnresolvedKind = 'none'
let judgeHumanQuestion = ''
let judgeFiledIssueUrl = ''

if (judge && !judge.all_met) {
  const unmet = judge.criteria_checked.filter((c, i) => !judge.criteria_met[i])

  const FIX_SCHEMA = {
    type: 'object',
    properties: {
      fixed: { type: 'boolean' },
      unfixable_reason: { type: 'string' },
      needs_human_decision: { type: 'boolean' },
      human_question: { type: 'string' },
      summary: { type: 'string' },
    },
    required: ['fixed', 'summary'],
  }

  const fix = await agent(
    `You are fixing gaps found by an independent judge in issue #${selection.issue_number} — "${selection.title}" —
on branch \`${selection.branch_name}\`.

The judge found these acceptance criteria NOT genuinely met:
${unmet.map((c) => `- ${c}`).join('\n')}

Judge's reasoning:
${judge.reasoning}

Task: close the gap for each unmet criterion — write the missing code/tests so the criterion is genuinely
satisfied (not just a plausible-looking test that wouldn't actually catch a violation). Run
\`cargo test --workspace\`, \`cargo clippy --workspace -- -D warnings\`, and \`cargo fmt --check --all\`
(and the frontend gates, if you touch frontend/) after fixing, and commit your changes (do NOT amend the
existing commit — new commit, message "fix: address judge findings for #${selection.issue_number}").

If a gap can't be closed because it hinges on a genuine open question only a human can answer (an
ambiguous requirement, a real design/scope decision) — do NOT guess. Set fixed=false,
needs_human_decision=true, and put the specific question in human_question.

If you successfully close the gap, set fixed=true.

Return a StructuredOutput describing what happened.`,
    { label: 'fix-judge-gaps', phase: 'Judge', schema: FIX_SCHEMA }
  )

  if (fix && fix.fixed) {
    log(`🔧 Fixed judge-found gaps: ${fix.summary}`)
    judge = await agent(judgePrompt(), { label: 're-judge', phase: 'Judge', schema: JUDGE_SCHEMA, model: 'claude-haiku-4-5-20251001' })
    if (!judge) {
      judge = { all_met: false, criteria_checked: [], criteria_met: [], reasoning: 'judge agent failed to return a result on re-judge' }
    }
    log(judge.all_met ? '✅ Re-judge confirms acceptance criteria now met' : '⚠️ Re-judge still finds unmet criteria')
  }

  if (!judge.all_met) {
    if (fix && fix.needs_human_decision) {
      judgeUnresolvedKind = 'needs_human_decision'
      judgeHumanQuestion = fix.human_question || '(no question captured)'
      log(`❓ Judge gap needs a human decision: ${judgeHumanQuestion}`)
    } else {
      // Not an open question — file a follow-up issue and let this diff ship;
      // the gap is tracked separately rather than blocking indefinitely.
      const ISSUE_FILE_SCHEMA = {
        type: 'object',
        properties: { filed: { type: 'boolean' }, issue_url: { type: 'string' } },
        required: ['filed'],
      }
      const filed = await agent(
        `File a new GitHub issue in ${OWNER}/${REPO} using the GitHub MCP tool (mcp__github__create_issue) for
a gap found while verifying issue #${selection.issue_number} — "${selection.title}":

Unmet criteria:
${unmet.map((c) => `- ${c}`).join('\n')}

Judge's reasoning:
${judge.reasoning}
${fix ? `\nFix attempt notes: ${fix.unfixable_reason || fix.summary}` : ''}

Title it something like "Follow-up: <short description> (from #${selection.issue_number})". Body should
reference #${selection.issue_number} and explain the gap so someone can pick it up. Return the created
issue's URL.`,
        { label: 'file-followup-issue', phase: 'Judge', schema: ISSUE_FILE_SCHEMA }
      )
      judgeUnresolvedKind = 'follow_up_filed'
      judgeFiledIssueUrl = filed && filed.filed ? filed.issue_url : '(issue filing failed — see logs)'
      log(`📋 Filed follow-up issue for unresolved judge gap: ${judgeFiledIssueUrl}`)
    }
  }
}

const judgeMergeable = judge.all_met || judgeUnresolvedKind === 'follow_up_filed'

const gateGreen = coreTestsPassed && !e2eGenuinelyFailed && reviewMergeable && judgeMergeable

// ---------------------------------------------------------------------------
// Phase 6 — Ship
// ---------------------------------------------------------------------------
phase('Ship')

const SHIP_SCHEMA = {
  type: 'object',
  properties: {
    pushed: { type: 'boolean' },
    pr_url: { type: 'string' },
    pr_number: { type: 'number' },
    pr_created: { type: 'boolean' },
  },
  required: ['pushed', 'pr_url', 'pr_number', 'pr_created'],
}

const e2eSection = !testResult.frontend_e2e_applicable
  ? ''
  : testResult.frontend_e2e_passed
    ? `\n**e2e**: ✅ \`npm run test:e2e\` passed.\n`
    : !testResult.frontend_e2e_env_available
      ? `\n**e2e**: ⚠️ could not run — no Playwright browser available in the environment that produced this PR (checked \`~/.cache/ms-playwright\`, \`$PLAYWRIGHT_BROWSERS_PATH\`, \`/opt/pw-browsers\`, system chromium — none present). Every other Definition of Done tier is green. Per CLAUDE.md this is an environment gap, not a code defect, and does not block merge — a human or CI environment with real browsers should confirm before relying on it.\n`
      : `\n**e2e**: ❌ ran and FAILED — this blocks merge (a real regression, not an environment gap).\n\n${testResult.failure_details}\n`

const reviewSection = `## Review
${e2eSection}
**Code review** (${reviewModel}): ${review.clean ? '✅ clean' : `⚠️ ${review.blocking_findings.length} blocking finding(s)`}
${review.blocking_findings.length ? review.blocking_findings.map((f) => `- [blocking] ${f}`).join('\n') + '\n' : ''}${review.non_blocking_notes.length ? review.non_blocking_notes.map((f) => `- [note] ${f}`).join('\n') + '\n' : ''}${review.unresolved_kind === 'needs_human_decision' ? `\n❓ **Needs a human decision:** ${review.human_question}\n` : ''}${review.unresolved_kind === 'follow_up_filed' ? `\n📋 **Follow-up filed:** ${review.filed_issue_url}\n` : ''}${review.summary}

**Judge** (haiku): ${judge.all_met ? '✅ acceptance criteria met' : '⚠️ unmet acceptance criteria'}
${judge.criteria_checked.map((c, i) => `- [${judge.criteria_met[i] ? 'x' : ' '}] ${c}`).join('\n')}
${judgeUnresolvedKind === 'needs_human_decision' ? `\n❓ **Needs a human decision:** ${judgeHumanQuestion}\n` : ''}${judgeUnresolvedKind === 'follow_up_filed' ? `\n📋 **Follow-up filed:** ${judgeFiledIssueUrl}\n` : ''}${judge.reasoning}`

// Always push + open the PR — the review/judge findings above go straight
// into the PR description regardless of whether the gate is green, so a
// human skimming later can see what was checked. Merging is a separate,
// gated step below (never done inside this agent call).
const ship = await agent(
  `You are creating a pull request for issue #${selection.issue_number} — "${selection.title}".

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
   - body: include, in order:
     - "Closes #${selection.issue_number}"
     - Summary of what was implemented (from: ${impl ? impl.summary : '(see commit)'})
     - Test plan: list the cargo test / vitest / Playwright cases added
     - The following "## Review" section VERBATIM (do not paraphrase it):

${reviewSection}

Do NOT merge the PR — that happens in a separate step only if the gate is green.

Return a StructuredOutput with push/PR-creation results.`,
  { label: 'open-pr', phase: 'Ship', schema: SHIP_SCHEMA }
)

if (!ship || !ship.pr_created) {
  log(`⚠️ PR creation failed — manual push/PR may be needed`)
  return { success: false, stage: 'ship', selection, impl, testResult, review, judge, ship }
}

log(`📬 PR opened: ${ship.pr_url}`)

if (!gateGreen) {
  log(
    `⏸️  Gate not green (core_tests=${coreTestsPassed}, e2e_blocking_failure=${e2eGenuinelyFailed}, review=${reviewMergeable}, judge=${judgeMergeable}) — leaving PR open for human review, not merging`
  )
  return {
    success: true,
    merged: false,
    issue_number: selection.issue_number,
    title: selection.title,
    branch: selection.branch_name,
    pr_url: ship.pr_url,
    review,
    judge,
  }
}

const MERGE_SCHEMA = {
  type: 'object',
  properties: { merged: { type: 'boolean' } },
  required: ['merged'],
}

const merge = await agent(
  `Merge pull request #${ship.pr_number} in ${OWNER}/${REPO} (branch \`${selection.branch_name}\` → \`${DEFAULT_BRANCH}\`).

Core tests, code review, and judge are all gated green per CLAUDE.md's auto-merge policy — e2e either
passed or could not run in this environment (documented in the PR, not a blocker), and any review/judge
follow-ups were fixed on the spot or filed as separate tracked issues rather than left blocking.

Steps:
1. Use (mcp__github__merge_pull_request) to merge with method "squash".
2. After merge, close issue #${selection.issue_number} using (mcp__github__issue_write) method "update" with
   state "closed" and state_reason "completed".
3. Delete the local branch: \`git checkout ${DEFAULT_BRANCH} && git branch -d ${selection.branch_name}\`

Return a StructuredOutput with the merge result.`,
  { label: 'merge', phase: 'Ship', schema: MERGE_SCHEMA }
)

if (!merge || !merge.merged) {
  log(`⚠️ Merge failed — PR is open and gated green, merge manually`)
  return { success: true, merged: false, issue_number: selection.issue_number, title: selection.title, branch: selection.branch_name, pr_url: ship.pr_url, review, judge }
}

log(`🚀 Shipped and merged!`)
log(`   PR: ${ship.pr_url}`)

return {
  success: true,
  merged: true,
  issue_number: selection.issue_number,
  title: selection.title,
  branch: selection.branch_name,
  pr_url: ship.pr_url,
  files_created: impl ? impl.files_created : [],
  files_modified: impl ? impl.files_modified : [],
  test_count: impl ? impl.test_count : 0,
  review,
  judge,
}
