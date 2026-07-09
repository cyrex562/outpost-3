/**
 * Reducer registry — 104b extensions entry-point (HR-795, Layer 2).
 *
 * Side-effect imported by `reducers.ts`; do not import directly.
 *
 * Delegates to three focused sub-modules so each stays under 400 lines:
 *   reducers-b-character.ts — character.* + status.* events
 *   reducers-b-combat.ts    — oracle.chaos_changed + combat.* + exploration.actions
 *   reducers-b-misc.ts      — gm.suggestions, dungeon, shopping, travel,
 *                             inventory, action.skill_check, social.*
 *
 * SUPPRESSED events (intentionally produce no output; no reducer registered):
 *   combat.fled           exploration.enter_hex   social.healer
 *   oracle.fate_check     oracle.scene_check      oracle.random_event
 *
 * (gm.narrate is a WS `narration` frame, not a game_event — absent from
 * the ClientEvent union and therefore not listed here.)
 */
import "./reducers-b-character";
import "./reducers-b-combat";
import "./reducers-b-misc";
