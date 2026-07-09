# Milestone 5: Dungeons — Task Specification

> **Goal:** Implement core dungeon exploration gameplay, including room navigation, searching, and combat integration.
> Dungeon structures are created via the Admin Editor; this milestone makes them playable.

## 5.1 Dungeon Scene Handler (Technical Core)

**Description:** Complete the `DungeonScene` implementation in `src/harsh_realm/gm/scenes/dungeon.py` to support full gameplay loop.

**Tasks:**
- [ ] **Fix Initialization:** Correct the `__init__` method in `DungeonScene` to properly handle `DungeonRoom` models (fix the `room_models` reference bug).
- [ ] **Navigation Refinement:**
  - [ ] Support cardinal directions (north, south, east, west) and diagonal aliases.
  - [ ] Ensure `_get_room_exits` and `_build_room_exits` handle bidirectional connections correctly.
  - [ ] Implement `look` and `status` with rich narration including room type flavor.
- [ ] **Search System:**
  - [ ] Implement `search` command using the room's `loot` data.
  - [ ] Searching a room for the first time reveals loot.
  - [ ] (Optional) Add a skill check (Notice) for hidden loot.
- [ ] **Combat Integration:**
  - [ ] If a room has an `encounter` and it's the `first_visit`, trigger a combat transition.
  - [ ] Wire `_pending_combat_transition` to transition to `SceneState.COMBAT`.
  - [ ] Ensure returning from combat stays in the dungeon scene at the same room.
- [ ] **Exit Logic:**
  - [ ] `exit` or `leave` command at the "entrance" room returns to `SceneState.EXPLORATION`.
  - [ ] Update character position on hex map upon exit to `entry_q, entry_r`.

## 5.2 Dungeon Content & Rules

**Description:** Define the rules for dungeon crawling and provide starter content.

**Tasks:**
- [ ] **Rules Reference:** Create `docs/rules_reference/dungeons.md`.
  - Define room types (entrance, corridor, chamber, vault, etc.).
  - Define movement costs (if any) and light source requirements (placeholder for now).
- [ ] **Dungeon Generator (Admin):**
  - [ ] Implement a basic procedural dungeon generator in `src/harsh_realm/generators/dungeon_gen.py`.
  - [ ] Generator should produce a list of `DungeonRoom` and `DungeonConnection` objects.
  - [ ] Add an "Auto-Generate" button to the Dungeon Editor tab in the Admin panel.

## 5.3 UX & Visibility

**Description:** Ensure dungeon state is visible to the player.

**Tasks:**
- [ ] **Dungeon HUD:**
  - [ ] Update `StatusSidebar.vue` to show current room name when in a dungeon.
  - [ ] (Optional) Add a simple text-based "minimap" or "visited rooms" list.
- [ ] **Command Suggestions:**
  - [ ] Ensure `get_suggestions()` returns valid directions and dungeon-specific commands.

## Acceptance Criteria

1. **Entry/Exit:** Player can `enter` a dungeon hex, explore rooms, and `exit` back to the world map.
2. **Navigation:** Moving through rooms updates the "current room" and provides unique descriptions.
3. **Searching:** `search` command retrieves items defined in the room's loot table.
4. **Combat:** Entering an encounter room for the first time triggers a combat scene.
5. **Persistence:** Dungeon progress (visited rooms, looted items) is persisted (may require schema updates).

## Dependencies
- Requires Milestone 4.5 (Dungeon CRUD) to be functional for testing content.
- Requires Milestone 4.6 (Combat) for encounter resolution.
