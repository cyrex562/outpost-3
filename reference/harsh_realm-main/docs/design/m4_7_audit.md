# M4.7 Town & Social Frontend Audit

> Authoritative source for which WebSocket events are emitted vs. missing,
> and the current state of frontend surfaces for M4 features.
> Produced 2026-03-29 from full codebase audit.

## Town Scene Wiring (Backend — All Functional)

| Command | Handler | Requires Settlement? | Notes |
|---|---|---|---|
| `explore town` | `_handle_explore` | Yes | Lists establishments + operators + resident NPCs |
| `talk <npc>` | `_handle_talk` | No (any hex with NPCs) | Queries entities table, partial name match, transitions to SOCIAL scene |
| `shop` | `_handle_shop` | Yes | Transitions to SHOPPING scene with settlement data |
| `examine <npc>` | `_handle_examine` | No | Shows NPC description |
| `heal` | `_handle_heal` (in SocialScene) | Via social scene with healer NPC | 5 gold/HP, healer detected by occupation keywords |

### NPC Persistence
- NPCs generated once at world creation by `SettlementGenerator`, stored in `entities` table
- Disposition changes persist across visits (written to entity JSON data)
- Settlement size affects NPC count: hamlet 3-4, village 5-8, town 8-12

### Settlement Size vs. Services
- NPC count varies by size (correctly implemented)
- Shop inventory is hardcoded (10 items, same for all settlement sizes) — **gap**
- No per-establishment shopping (all shops use generic inventory) — **gap**

## WebSocket Events — Emitted by Backend

| Event Type | Source File | Data Fields |
|---|---|---|
| `social.disposition_change` | social.py:188 | npc, old_score, new_score, reason |
| `social.healer` | social.py:417 | npc, cost, hp_restored |
| `action.skill_check` | social.py:204 | verb, skill, roll, total, difficulty, margin, success |
| `character.expert_reroll` | social.py:297 | original_total, reroll_total |
| `shopping.purchase` | shopping.py:153 | item, price, gold_remaining |
| `shopping.sale` | shopping.py:190 | item, price, gold_total |
| `oracle.fate_check` | exploration.py:910 | question, likelihood, result, roll |
| `oracle.scene_check` | controller.py:259 | roll, chaos_factor, modification |
| `oracle.random_event` | controller.py:271 | focus, action, subject |
| `gm.scene_change` | controller.py:197 | from, to |
| `gm.narrate` | multiple | text |
| `gm.suggestions` | multiple | commands |

## WebSocket Events — NOT Emitted (Gaps)

| Event Type | Impact | Fix Location |
|---|---|---|
| `faction.turn_completed` | Faction turns invisible to player | `faction/faction_turn.py` — emit after `run_all_turns()` |
| `faction.reputation_changed` | Reputation changes silent | `faction/reputation.py` — emit in adjust methods |
| `oracle.chaos_changed` | Chaos factor adjustments not broadcast | `engine/oracle.py` — emit in `ChaosTracker.increase()/decrease()` |

Note: `social.scene_entered`/`social.scene_exited` are NOT needed — `gm.scene_change` already fires with `{from, to}` fields.

## Frontend — What IS Displayed

| Component | Displays |
|---|---|
| StatusSidebar | name, class, level, HP bar, AC, XP bar, location (q,r), terrain, features, conditions |
| ChatLog | Player input (green), GM narration (amber), system events (grey italic) |
| CommandInput | Text input, arrow-key history, tab autocomplete |

## Frontend — What is NOT Displayed (Gaps)

| Missing | Component | Fix |
|---|---|---|
| Gold / currency | StatusSidebar | Add `gold` to CharacterState, read from character API |
| Current scene type | StatusSidebar | Track `gm.scene_change` `to` field in game store |
| Chaos factor | StatusSidebar | Expose via character API or world_meta, display |
| Social event formatting | ChatLog / useWebSocket | Handle `social.disposition_change` + `action.skill_check` |
| Shopping event formatting | ChatLog / useWebSocket | Handle `shopping.purchase` / `shopping.sale` |
| Faction event display | ChatLog | Depends on `faction.turn_completed` event (backend gap) |
| NPC disposition | StatusSidebar | Not tracked in game store |
| Inventory | StatusSidebar | Not tracked in game store |
| Skill check roll breakdown | ChatLog | Events carry data but frontend shows as grey `[event_type]` |

## Frontend Event Handling

All events are broadcast via WebSocket (no server-side filtering). The frontend
in `useWebSocket.ts` selectively handles:

- **Processed:** `exploration.enter_hex`, `gm.suggestions`, `character.hp_changed`,
  `character.xp_gained`, `character.created`, `gm.scene_change`, `character.level_up`,
  `character.respawn`
- **Suppressed (not displayed):** `gm.narrate` (special path), `action.move`,
  `combat.attack`, `combat.enemy_defeated`, `combat.fled`, `exploration.encounter`,
  `combat.awareness_check`, `combat.start`
- **Unhandled (shown as grey `[event_type]`):** Everything else including all
  social, shopping, faction, and oracle events

## Test Coverage Gaps

| Area | Tested? | Notes |
|---|---|---|
| `shop` from non-settlement hex | No | `_handle_shop` checks feature but no test for rejection |
| NPC persistence across visits | No | NPCs stored in DB but no test verifies cross-visit survival |
| Settlement-size shop variation | No | All settlements share same inventory (no variation to test) |
| `explore town` NPC listing | No | Command works but no unit test |
| Playwright E2E for social/shopping | No | Zero E2E tests for these flows |
