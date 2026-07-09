# Master Acceptance Criteria Generation Prompt

You are an expert software auditor and technical writer. Your task is to generate a comprehensive `docs/acceptance_criteria.md` document for the Harsh Realm project.

## Context
Harsh Realm is a single-player MUD with a procedural world and an expert-system GM. It uses XWN rules (Stars/Worlds Without Number core). The project has completed milestones M0 through M4.9.

## Inputs
To generate this document, you should have access to:
1. `CLAUDE.md` — Project context and high-level goals.
2. `todo.md` — Current task list and historical status.
3. `docs/milestone_*_tasks.md` — Detailed task specs for each milestone.
4. `src/harsh_realm/` — Source code to verify implementation details.
5. `tests/` — Test files to verify coverage.

## Required Structure

### 1. Summary Table
At the top, provide a summary table of all milestones (M0 through M4.9):
- Milestone (e.g., M0: Foundation)
- Status (COMPLETE / PARTIAL / MISSING)
- Key Features (1-3 words)
- Notes (optional, for deviations)

### 2. Milestone Detail Blocks
For each milestone (M0, M1, M2, M3, M4, M4.5, M4.6, M4.7, M4.8, M4.9), provide a detailed block:

#### [Milestone Name]
- **Status:** [COMPLETE/PARTIAL]
- **Summary:** 1-2 sentences of the goal.
- **Coverage:**
  - [ ] Unit Tests
  - [ ] Property Tests (Hypothesis)
  - [ ] Mutation Tests (mutmut)
  - [ ] E2E Tests (Playwright)
- **Acceptance Criteria:**
  - 3-8 testable, specific criteria. 
  - Each should be verifiable by observing code, running tests, or checking UI.
  - Example: "Character creation automatically rolls attributes and proceeds to assignment after class selection."
- **Notes & Deviations:** 
  - List any features that were deferred, implemented differently than specced, or have known gaps.

## Guidelines
- **Be Empirical:** Do not invent criteria. Base them on the `milestone_N_tasks.md` documents and the actual implementation in `src/`.
- **Be Concise:** Use bullet points and clear, direct language.
- **Differentiate Coverage:** Mark which milestones have mutation and E2E coverage based on the latest audits (M4.7 and M4.9).
- **Technical Accuracy:** Ensure rules-related criteria (e.g., combat, skill checks) reflect the actual XWN implementation.

## Deliverable
Write the complete `docs/acceptance_criteria.md` file.
