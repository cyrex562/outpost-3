/**
 * 104b reducers — GM suggestions, dungeon, shopping, travel, inventory,
 * action skill-check, and social events.
 *
 * Side-effect imported by `reducers-b.ts`; do not import directly.
 *
 * Covers:
 *   gm.suggestions         dungeon.enter_room
 *   shopping.purchase      shopping.sale
 *   travel.interrupted     travel.resumed      travel.started
 *   travel.completed       travel.cancelled    travel.blocked
 *   inventory.item_given   inventory.item_lost inventory.ammo_consumed
 *   action.skill_check     social.disposition_change
 */
import type {
  GmSuggestionsNotice,
  DungeonEnterRoomNotice,
  ShoppingPurchaseNotice,
  ShoppingSaleNotice,
  TravelInterruptedNotice,
  TravelJourneyNotice,
  TravelEndNotice,
  InventoryItemGivenNotice,
  InventoryItemLostNotice,
  InventoryAmmoConsumedNotice,
  ActionSkillCheckNotice,
  SocialDispositionChangeNotice,
} from "../types/events.gen";
import { reducerRegistry, type WorldModel, type Change } from "./model";

// ---------------------------------------------------------------------------
// gm.suggestions
// ---------------------------------------------------------------------------

reducerRegistry["gm.suggestions"] = (
  model: WorldModel,
  data: unknown,
): Change[] => {
  // Safe: only called by apply() when event_type === "gm.suggestions".
  // Mirrors: if (commands && commands.length > 0) gameStore.addSuggestions(commands)
  const d = data as GmSuggestionsNotice;
  if (d.commands.length > 0) {
    model.suggestions = d.commands;
    return [{ kind: "suggestions" }];
  }
  return [];
};

// ---------------------------------------------------------------------------
// dungeon.enter_room — update character location to room coordinates
// ---------------------------------------------------------------------------

reducerRegistry["dungeon.enter_room"] = (
  model: WorldModel,
  data: unknown,
): Change[] => {
  // Safe: only called by apply() when event_type === "dungeon.enter_room".
  // Mirrors: gameStore.updateLocation(-1, -1, roomType, [roomName])
  const d = data as DungeonEnterRoomNotice;
  if (model.character === null) return [];
  model.character.locationQ = -1;
  model.character.locationR = -1;
  model.character.terrain = d.room_type;
  model.character.features = [d.room_name];
  return [{ kind: "character" }];
};

// ---------------------------------------------------------------------------
// shopping.purchase
// ---------------------------------------------------------------------------

reducerRegistry["shopping.purchase"] = (
  model: WorldModel,
  data: unknown,
): Change[] => {
  // Safe: only called by apply() when event_type === "shopping.purchase".
  const d = data as ShoppingPurchaseNotice;
  const changes: Change[] = [
    {
      kind: "message",
      text: `[Purchased] ${d.item}  —  ${d.price} gp  (Balance: ${d.gold_remaining} gp)`,
      style: "purchase",
    },
    { kind: "refetch-character" },
  ];
  if (model.character !== null) {
    model.character.gold = d.gold_remaining;
    changes.unshift({ kind: "character" });
  }
  return changes;
};

// ---------------------------------------------------------------------------
// shopping.sale
// ---------------------------------------------------------------------------

reducerRegistry["shopping.sale"] = (
  model: WorldModel,
  data: unknown,
): Change[] => {
  // Safe: only called by apply() when event_type === "shopping.sale".
  const d = data as ShoppingSaleNotice;
  const changes: Change[] = [
    {
      kind: "message",
      text: `[Sold] ${d.item}  —  ${d.price} gp  (Balance: ${d.gold_total} gp)`,
      style: "sale",
    },
    { kind: "refetch-character" },
  ];
  if (model.character !== null) {
    model.character.gold = d.gold_total;
    changes.unshift({ kind: "character" });
  }
  return changes;
};

// ---------------------------------------------------------------------------
// travel.interrupted — store destination so UI can show resume banner
// ---------------------------------------------------------------------------

reducerRegistry["travel.interrupted"] = (
  model: WorldModel,
  data: unknown,
): Change[] => {
  // Safe: only called by apply() when event_type === "travel.interrupted".
  const d = data as TravelInterruptedNotice;
  model.pendingTravel = { q: d.target_q, r: d.target_r };
  return [{ kind: "pendingTravel" }];
};

// ---------------------------------------------------------------------------
// travel.resumed / travel.started / travel.completed / travel.cancelled /
// travel.blocked — clear pending travel destination
// ---------------------------------------------------------------------------

function clearPendingTravel(model: WorldModel): Change[] {
  model.pendingTravel = null;
  return [{ kind: "pendingTravel" }];
}

// travel.resumed + travel.started carry TravelJourneyNotice.
(["travel.resumed", "travel.started"] as const).forEach((evType) => {
  reducerRegistry[evType] = (model, data) => {
    void (data as TravelJourneyNotice);
    return clearPendingTravel(model);
  };
});

// travel.completed + travel.cancelled + travel.blocked carry TravelEndNotice.
(["travel.completed", "travel.cancelled", "travel.blocked"] as const).forEach((evType) => {
  reducerRegistry[evType] = (model, data) => {
    void (data as TravelEndNotice);
    return clearPendingTravel(model);
  };
});

// ---------------------------------------------------------------------------
// inventory.item_given
// ---------------------------------------------------------------------------

reducerRegistry["inventory.item_given"] = (
  _model: WorldModel,
  data: unknown,
): Change[] => {
  // Safe: only called by apply() when event_type === "inventory.item_given".
  // Handler: scheduleInventoryRefresh (debounced loadCharacter) + addMessage.
  // Model emits refetch-character per event; 104c subscriber can debounce.
  const d = data as InventoryItemGivenNotice;
  const item = d.item;
  const itemType = typeof item.type === "string" ? item.type : "";
  let text: string;
  if (itemType === "currency") {
    const amount = typeof item.value === "number" ? item.value : 0;
    text = `You received ${amount} gp.`;
  } else {
    const itemName = typeof item.name === "string" ? item.name : "an item";
    text = `You obtained ${itemName}.`;
  }
  return [
    { kind: "message", text, style: "inventory" },
    { kind: "refetch-character" },
  ];
};

// ---------------------------------------------------------------------------
// inventory.item_lost
// ---------------------------------------------------------------------------

reducerRegistry["inventory.item_lost"] = (
  _model: WorldModel,
  data: unknown,
): Change[] => {
  // Safe: only called by apply() when event_type === "inventory.item_lost".
  const d = data as InventoryItemLostNotice;
  return [
    { kind: "message", text: `You lost ${d.item_name}.`, style: "inventory" },
    { kind: "refetch-character" },
  ];
};

// ---------------------------------------------------------------------------
// inventory.ammo_consumed
// ---------------------------------------------------------------------------

reducerRegistry["inventory.ammo_consumed"] = (
  _model: WorldModel,
  data: unknown,
): Change[] => {
  // Safe: only called by apply() when event_type === "inventory.ammo_consumed".
  const d = data as InventoryAmmoConsumedNotice;
  return [
    {
      kind: "message",
      text: `Ammo spent: ${d.ammo_type} (${d.remaining} remaining).`,
      style: "inventory",
    },
    { kind: "refetch-character" },
  ];
};

// ---------------------------------------------------------------------------
// action.skill_check
// ---------------------------------------------------------------------------

reducerRegistry["action.skill_check"] = (
  _model: WorldModel,
  data: unknown,
): Change[] => {
  // Safe: only called by apply() when event_type === "action.skill_check".
  const d = data as ActionSkillCheckNotice;
  const mod = d.total - d.roll;
  const sign = d.margin >= 0 ? "+" : "";
  const result = d.success ? "Success" : "Fail";
  return [
    {
      kind: "message",
      text:
        `[Skill Check] ${d.verb} (${d.skill})\n` +
        `Roll: ${d.roll} + ${mod} mod = ${d.total}  vs  Difficulty ${d.difficulty}  →  ${result} (${sign}${d.margin})`,
      style: "skill_check",
    },
  ];
};

// ---------------------------------------------------------------------------
// social.disposition_change
// ---------------------------------------------------------------------------

reducerRegistry["social.disposition_change"] = (
  _model: WorldModel,
  data: unknown,
): Change[] => {
  // Safe: only called by apply() when event_type === "social.disposition_change".
  const d = data as SocialDispositionChangeNotice;
  const delta = d.new_score - d.old_score;
  const sign = delta >= 0 ? "+" : "";
  return [
    {
      kind: "message",
      text: `${d.npc}: ${d.old_mood} → ${d.new_mood}  (${sign}${delta})`,
      style: "disposition",
    },
  ];
};
