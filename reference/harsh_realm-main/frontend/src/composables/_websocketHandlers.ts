/**
 * WebSocket message dispatch — HR-795 104c cutover.
 *
 * After 104c the `game_event` path is a single line:
 *   apply(worldModel, event as ClientEvent)
 *
 * The reducer registry (populated by instance.ts side-effect import) does all
 * the work; the projection subscriber (installed once in useWebSocket.ts) maps
 * model changes back to Pinia stores.
 *
 * This file retains the non-game_event branches that need direct store
 * access or special parsing:
 *   - Admin live-event feed (ADM-09)  → adminStore (game_event/narration/
 *                                       player_input all mirrored here)
 *   - narration frame                 → gameStore (attribute-roll detection +
 *                                       multi-paragraph split)
 *   - error frame                     → gameStore
 *
 * `player_input` frames only feed the admin log (above); the chat echo is
 * already added synchronously in `send()`, so there is no separate handling
 * branch for them.
 */
import { useAdminStore } from "../stores/admin";
import { useGameStore } from "../stores/game";
import { worldModel } from "../worldModel/instance";
import { apply } from "../worldModel/model";
import type { ClientEvent } from "../types/events.gen";

export interface WebSocketHandlerDeps {
  gameStore: ReturnType<typeof useGameStore>;
  adminStore: ReturnType<typeof useAdminStore>;
}

export function handleWebSocketMessage(
  msg: Record<string, unknown>,
  deps: WebSocketHandlerDeps,
): void {
  const { gameStore, adminStore } = deps;

  // --- Admin Live Event Feed (ADM-09) ---
  // Push raw message to admin store for the event log panel.
  if (msg.type === "game_event") {
    if (msg.event && typeof msg.event === "object") {
      adminStore.addEvent(msg.event as Record<string, unknown>);
    }
  } else if (msg.type === "narration" || msg.type === "player_input") {
    adminStore.addEvent(msg);
  }

  // --- narration frame ---
  if (msg.type === "narration") {
    const text = msg.text as string | undefined;
    if (text) {
      // Detect the character-creation attribute-roll announcement and extract
      // the 6 scores so the drag-and-drop modal can open.
      if (text.includes("Rolled attribute scores:")) {
        const nums = text.match(/\b\d{1,2}\b/g);
        if (nums && nums.length >= 6) {
          const scores = nums.slice(0, 6).map((n) => parseInt(n, 10));
          gameStore.setAttributeRoll(scores);
        }
      }
      // Split multi-paragraph narration into separate messages
      const paragraphs = String(text)
        .split("\n")
        .filter((p) => p.trim());
      for (const para of paragraphs) {
        gameStore.addMessage("gm", para, "narration");
      }
    }

  // --- game_event frame ---
  } else if (msg.type === "game_event") {
    const event = msg.event as Record<string, unknown>;
    // Route through the reducer registry → model → projection → stores.
    // apply() dev-warns on unhandled events (HR-792) and no-ops on suppressed.
    apply(worldModel, event as unknown as ClientEvent);

  // --- error frame ---
  } else if (msg.type === "error") {
    gameStore.addMessage(
      "system",
      `Error: ${String(msg.message ?? "")}`,
      "system",
    );
  }
}
