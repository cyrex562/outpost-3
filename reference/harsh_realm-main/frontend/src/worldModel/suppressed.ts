/**
 * Events the backend forwards that the worldModel intentionally does NOT
 * process (no reducer registered, no store update).
 *
 * Listed as string literals so the Rust
 * `frontend_handles_all_client_facing_events` coverage gate can verify
 * exhaustive coverage across the worldModel files.
 *
 * HR-795 (104c): the gate was repointedfrom `_websocketHandlers.ts` to the
 * worldModel directory — every `CLIENT_FACING_EVENT_TYPES` entry must appear
 * as a quoted string in at least one file under `frontend/src/worldModel/`.
 */
export const SUPPRESSED_GAME_EVENTS: readonly string[] = [
  /**
   * `gm.narrate` is forwarded by the backend as a `narration` WebSocket
   * frame (`msg.type === "narration"`), not as a `game_event` frame.
   * The WS handler in `_websocketHandlers.ts` already processes it there;
   * no game_event reducer is needed.
   */
  "gm.narrate",
  /**
   * `combat.fled` carries no player-facing text and drives no UI state —
   * the scene change (`gm.scene_change`) that immediately follows it is
   * the meaningful signal.
   */
  "combat.fled",
  /**
   * `exploration.enter_hex` fires on every move; position is already
   * applied via `exploration.moved`.  Suppressed to avoid duplicate updates.
   */
  "exploration.enter_hex",
  /**
   * `social.healer` acknowledges a healing transaction; the HP update
   * arrives via `character.hp_changed` and the gold delta via
   * `shopping.purchase`.  No additional display line needed.
   */
  "social.healer",
  /**
   * Oracle query results — displayed by the GM narration that accompanies
   * them; no separate chat line is required.
   */
  "oracle.fate_check",
  "oracle.scene_check",
  "oracle.random_event",
] as const;
