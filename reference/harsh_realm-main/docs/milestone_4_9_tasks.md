# Milestone 4.9: Cleanup & Polish — Task Specification

> **Goal:** Close the test and frontend gaps identified in the M4 and M4.7 audits.
> Backend M4 features are complete. This milestone makes them visible and verifiable:
> mutmut coverage, sidebar displays, structured event formatting, missing backend event
> emissions, and Playwright E2E for the admin panel. Also produces the master
> acceptance criteria document.
> **Estimated time:** 2–3 days (AI-assisted)
> **Prerequisite:** M4 and M4.5 complete. Read CLAUDE.md, AGENTS.md before starting.

## Success Criteria

When this milestone is complete:

1. `pytest` passes with all tests green.
2. mutmut has been run on all M4 engine modules. ≥85% mutants killed per module.
   Surviving mutants are killed or documented as known equivalents with inline comments.
3. StatusSidebar displays: gold balance, current scene type (badge), chaos factor.
   All three update in real time via WebSocket.
4. Social events in ChatLog are formatted as structured messages: disposition delta
   shown with +/- indicator and mood label, skill check shows roll + modifier +
   target + margin + pass/fail.
5. Shopping events in ChatLog show item name, unit price, and gold remaining after
   transaction.
6. `look` command in a settlement hex lists present NPCs by name and occupation.
7. Faction turn events are emitted to frontend and displayed as narration in ChatLog.
8. Faction reputation changes are emitted as WebSocket events.
9. Oracle chaos changes are emitted as WebSocket events and update chaos display in sidebar.
10. Playwright E2E tests cover all 12 admin panel tabs: each tab renders without
    errors, primary interaction (edit + save) works, and reset restores defaults.
11. `docs/acceptance_criteria.md` exists, covers M0–M4.5, and has a summary table
    showing complete/partial/missing status per milestone.

---

## Task 4.9.1: mutmut Runs — M4 Engine Modules

> **What:** Run mutmut on all M4 engine and scene modules. Kill surviving mutants
> or document equivalents.
> **Estimated time:** 4–6 hours

**Modules to target (in order):**
- `src/harsh_realm/engine/skill_checks.py`
- `src/harsh_realm/engine/npc_personality.py`
- `src/harsh_realm/engine/oracle.py`
- `src/harsh_realm/gm/scenes/social.py`
- `src/harsh_realm/gm/scenes/shopping.py`
- `src/harsh_realm/faction/faction_turn.py`
- `src/harsh_realm/faction/faction_ai.py`
- `src/harsh_realm/faction/reputation.py`
- `src/harsh_realm/admin/service.py`

**For each module:**
1. Run `mutmut run --paths-to-mutate src/harsh_realm/<module>`
2. Run `mutmut results` — record kill rate
3. For each surviving mutant: add a test that kills it, OR add a comment
   `# mutmut equivalent: <reason>` on the relevant line
4. Rerun until ≥85% killed or all survivors documented

**Deliverables:**
- All modules at ≥85% mutant kill rate or with documented survivors
- Kill rates recorded in a comment block at the top of each test file:
  ```python
  # mutmut kill rate: 91% (47/52). Survivors: see lines 203, 218 — equivalent mutants.
  ```

**Acceptance:** `mutmut results` shows ≥85% per module or inline documentation
for every survivor.

---

## Task 4.9.2: StatusSidebar — Gold, Scene Type, Chaos Factor

> **What:** Add gold balance, current scene type badge, and chaos factor to
> StatusSidebar. All three must update via WebSocket without page refresh.
> **Estimated time:** 2.5 hours

**Backend changes:**
- `src/harsh_realm/engine/oracle.py` — `ChaosTracker.increase()` and `.decrease()`
  must emit `oracle.chaos_changed` event with `{"chaos_factor": int}` payload.
  This is the only backend change needed; gold and scene type are already in
  existing events.

**Frontend changes — `frontend/src/stores/game.ts`:**
- Add `gold: number` field to `CharacterState` interface (read from
  `class_abilities.gold` in character API response)
- Add `currentScene: string` field, updated on `gm.scene_change` event
- Add `chaosFactor: number` field, updated on `oracle.chaos_changed` event

**Frontend changes — `frontend/src/composables/useWebSocket.ts`:**
- Handle `oracle.chaos_changed` → update `game.chaosFactor`
- Handle `gm.scene_change` → update `game.currentScene` (already partially handled;
  extend to store scene name)
- On `shopping.purchase` and `shopping.sale` → update `game.character.gold` from
  event payload `gold_remaining` / `gold_total`

**Frontend changes — `frontend/src/components/StatusSidebar.vue`:**
- Add gold row: coin icon + formatted number (e.g. `⦿ 42 gp`)
- Add scene type badge below character name: pill badge with scene label
  (Exploring / Social / Shopping / Combat) using distinct colours per scene
- Add chaos row below location: `Chaos: N` with colour scale (green 1–3,
  yellow 4–6, red 7–9)

**Tests:**
- Unit: `ChaosTracker.increase()` emits `oracle.chaos_changed` with correct value
- Unit: `ChaosTracker.decrease()` emits event; chaos never goes below 1 or above 9
- Playwright: StatusSidebar renders gold after character creation
- Playwright: scene badge updates after `gm.scene_change` event injected via WS
- Playwright: chaos factor updates after `oracle.chaos_changed` event

**Acceptance:** All three fields visible in sidebar and update in real time.

---

## Task 4.9.3: ChatLog — Social Event Formatting

> **What:** Handle `social.disposition_change` and `action.skill_check` events
> in the frontend. Format as structured chat messages instead of grey `[event_type]`
> placeholder text.
> **Estimated time:** 2 hours

**Audit finding:** Both events are already emitted by the backend with full payloads.
The frontend currently renders them as grey italic `[social.disposition_change]` text
(suppressed in `useWebSocket.ts` lines 119–131). This task replaces that suppression
with structured rendering.

**Frontend changes — `frontend/src/composables/useWebSocket.ts`:**

For `action.skill_check` event:
```
payload: { verb, skill, roll, total, difficulty, margin, success }
```
Render as a distinct message type in ChatLog:
```
[Skill Check] Convince (Talk/CHA)
Roll: 8 + 2 mod = 10  vs  Difficulty 10  →  Success (+0)
```

For `social.disposition_change` event:
```
payload: { npc, old_score, new_score, reason }
```
Render as:
```
Maren Coldwater: Indifferent → Sociable  (+1)
```
Use colour coding: green for positive delta, red for negative.

For `character.expert_reroll` event:
```
payload: { original_total, reroll_total }
```
Render as:
```
[Expert Reroll] Original: 7 → Reroll: 11  (new result used)
```

**Frontend changes — `frontend/src/components/ChatLog.vue`:**
- Add CSS classes for `skill-check-message`, `disposition-message`,
  `reroll-message` with distinct left-border accent colours
- These message types should be visually distinct from narration but
  not overwhelming — use muted accent, smaller font size

**Tests:**
- Playwright: `convince` command triggers skill-check message with roll breakdown
- Playwright: failed `convince` triggers disposition-change message with red delta
- Playwright: expert reroll shows both original and reroll values

**Acceptance:** Skill checks and disposition changes are readable structured messages,
not raw event type labels.

---

## Task 4.9.4: ChatLog — Shopping Event Formatting

> **What:** Handle `shopping.purchase` and `shopping.sale` events as structured
> chat messages.
> **Estimated time:** 1 hour

**Audit finding:** Both events emitted with full payloads; suppressed on frontend.

**Frontend changes — `frontend/src/composables/useWebSocket.ts`:**

For `shopping.purchase`:
```
payload: { item, price, gold_remaining }
```
Render as:
```
[Purchased] Short Sword  —  12 gp  (Balance: 38 gp)
```

For `shopping.sale`:
```
payload: { item, price, gold_total }
```
Render as:
```
[Sold] Battered Knife  —  4 gp  (Balance: 42 gp)
```

**Tests:**
- Playwright: `buy short sword` triggers purchase message with correct price
  and updated balance
- Playwright: `sell knife` triggers sale message

**Acceptance:** Purchases and sales show item, price, and running gold balance.

---

## Task 4.9.5: Backend Event Emission — Factions & Oracle

> **What:** Add the three missing backend event emissions identified in the
> M4.7 audit: faction turn results, reputation changes, oracle chaos changes.
> **Estimated time:** 3.5 hours

**5a — Faction turn events (~2h)**

File: `src/harsh_realm/faction/faction_turn.py`

In `FactionTurnEngine.run_all_turns()`: after each faction completes its turn,
emit a `faction.turn_completed` event if the action is significant (attack,
expand, seize — not repair or refit which are internal). Payload:
```python
{
  "faction": faction_name,
  "action": action_type,
  "target": target_name_or_hex,  # None if not applicable
  "summary": one_line_narration_string
}
```
The `summary` field is a short narrated string (e.g. "House Valdris expanded
into the Eastern Plains"). Use the existing narration template pattern.

In `frontend/src/composables/useWebSocket.ts`: handle `faction.turn_completed`
→ render as muted system message in ChatLog (grey, italic, distinct from
player-facing narration).

**5b — Reputation change events (~1h)**

File: `src/harsh_realm/faction/reputation.py`

In `ReputationSystem` methods that modify disposition: emit
`faction.reputation_changed` with:
```python
{"faction": faction_name, "old_disposition": str, "new_disposition": str}
```
Frontend: handle → render as brief status message
`Reputation with House Valdris: Neutral → Unfriendly`.

**5c — Oracle chaos events (~0.5h)**

File: `src/harsh_realm/engine/oracle.py`

Already covered in Task 4.9.2 (ChaosTracker emits `oracle.chaos_changed`).
This sub-task is just to confirm the frontend handler wired in 4.9.2 also
updates ChatLog with a subtle message: `[Oracle] Chaos: 4 → 5`.

**Tests:**
- Unit: `run_all_turns()` with attacking faction emits `faction.turn_completed`
- Unit: `run_all_turns()` with refit action does NOT emit event
- Unit: reputation decrease emits `faction.reputation_changed` with correct fields
- Playwright: advance world clock 7 days → faction event appears in ChatLog

**Acceptance:** Faction actions and reputation changes are visible to the player
in the chat log without being noisy for minor internal faction actions.

---

## Task 4.9.6: `look` Lists NPCs at Settlement

> **What:** When the player uses `look` at a settlement hex, list the NPCs
> present by name and occupation.
> **Estimated time:** 1 hour

**File:** `src/harsh_realm/gm/scenes/exploration.py`

In `_handle_look()`: if current hex has feature type `settlement`, query
the `entities` table for all living NPCs at this hex. Append to the look
narration:

```
Present: Maren Coldwater (merchant), Gareth Holt (blacksmith),
         Sera Ashmore (healer)
```

If no NPCs are present (shouldn't happen for a generated settlement, but
defensive): omit the line.

**Tests:**
- Unit: `look` at settlement hex → response includes NPC names and occupations
- Unit: `look` at non-settlement hex → no NPC list in response
- Unit: `look` at settlement after an NPC is killed in combat → dead NPC not listed

**Acceptance:** `look` in a settlement surfaces the NPCs the player can `talk` to.

---

## Task 4.9.7: Playwright E2E — Admin Panel Tabs

> **What:** Write Playwright E2E tests for all 12 admin panel tabs.
> **Estimated time:** 5 hours

**Files:** `frontend/tests/e2e/admin.spec.ts` (new or extend existing)

**Tabs to cover (5 config + 7 editor):**
1. Skill Mappings
2. Difficulty Targets
3. Disposition Outcomes
4. Encounter Weights
5. Faction Asset Stats
6. Hexes
7. Characters
8. Factions (World)
9. Dungeons
10. Worlds
11. YAML Files
12. World Meta

**Per tab, minimum tests:**
- Tab renders without JS errors
- Primary data loads (table or list is non-empty after world is loaded)
- Edit + save interaction works (change a value, save, reload — value persists)
- Reset (where applicable) restores default

**Setup:** Tests require a world to be loaded. Use a `beforeAll` that creates
a world via the UI or API before the tab tests run.

**Acceptance:** All 12 tabs have at least 3 passing Playwright tests each.
Zero console errors during any tab interaction.

---

## Task 4.9.8: Acceptance Criteria Document

> **What:** Have Claude Code produce `docs/acceptance_criteria.md` by reading
> all milestone task docs, CLAUDE.md, and source/test files.
> **Estimated time:** 1–2 hours agent time

**Deliverable:** `docs/acceptance_criteria.md`

**Instruction to agent:** Use the prompt in `docs/design/acceptance_criteria_prompt.md`
(create this file with the prompt text from the planning session) to guide
generation of the document.

**Structure:** Feature-level blocks, M0 through M4.5, each with:
- Status (COMPLETE / PARTIAL / MISSING)
- Milestone
- Test coverage (unit / property / mutation / E2E checkboxes)
- 3–8 testable criteria
- Notes for deviations and deferred items
- Summary table at top of document

**Acceptance:** Document exists, has summary table, covers all milestones M0–M4.5,
no criteria are implementation details, no criteria are invented without
evidence in source or tests.

---

## Dependency Order

```
4.9.1 (mutmut) — independent, run in parallel with frontend tasks
4.9.2 (sidebar) — backend oracle.chaos_changed needed first
4.9.3 (social formatting) — frontend only, no deps
4.9.4 (shopping formatting) — frontend only, no deps
4.9.5 (event emission) — backend; 4.9.5c shares work with 4.9.2
4.9.6 (look + NPCs) — backend only, no deps
4.9.7 (Playwright admin) — needs running server with world loaded
4.9.8 (acceptance doc) — no code deps; run last or in parallel
```

Recommended order:
1. 4.9.1 (mutmut — start early, runs in background)
2. 4.9.5 (event emission — unblocks sidebar and ChatLog handlers)
3. 4.9.2 (sidebar)
4. 4.9.3 + 4.9.4 (ChatLog formatting — parallel)
5. 4.9.6 (look NPCs — fast, independent)
6. 4.9.7 (Playwright admin)
7. 4.9.8 (acceptance doc — last)

---

## Notes for the Coding Agent

- Read CLAUDE.md and AGENTS.md before starting.
- The M4.7 audit (docs/design/m4_7_audit.md) is the authoritative source
  for which events are emitted vs. missing. Do not re-audit — implement
  based on those findings.
- Do NOT change the event payload shapes of existing emitted events
  (social.disposition_change, action.skill_check, shopping.purchase,
  shopping.sale). These are established contracts. Only add new events
  (faction.turn_completed, faction.reputation_changed, oracle.chaos_changed).
- Gold is stored in `class_abilities["gold"]` in the character model.
  Read it from the `/api/character` response — do not add a separate DB field.
- The `gm.scene_change` event already fires with `{from, to}` fields and
  is already handled in the frontend to trigger character reload. Extend
  that handler to also store `to` in `game.currentScene` — do not duplicate
  the handler.
- After completing all tasks, update CLAUDE.md:
  - Mark Milestone 4.9 complete with date
  - Record final test count
  - Document any deviations
