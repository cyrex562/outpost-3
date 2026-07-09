# Rules Reference: Dungeons

> **Goal:** Standard procedures for dungeon exploration in Harsh Realm.

## 1. Dungeon Entrance & Exit
- **Entry:** Using the `enter` command at a hex with a **lair** or **dungeon** feature starts a dungeon exploration scene.
- **Exit:** To leave a dungeon, the player must return to the **entrance** room and use the `exit` or `leave` command.
- **Surface Connection:** Exiting returns the player to the exploration scene at the hex coordinates where the dungeon was entered.

## 2. Room Navigation
- **Movement:** Movement within a dungeon uses cardinal directions (`north`, `south`, `east`, `west`) and their diagonal counterparts.
- **Room Types:**
    - **Entrance:** The only room from which the player can return to the surface.
    - **Corridor:** Narrow passages primarily used for connection.
    - **Chamber:** Larger rooms that may contain encounters or loot.
    - **Vault/Crypt/Laboratory:** Specialized rooms often containing higher-value loot or dangerous encounters.
- **Fog of War:** Rooms are marked as **visited** once entered. The `status` command shows the number of unique rooms visited out of the total known to the system.

## 3. Searching & Loot
- **Search Command:** The `search` command can be used in any room to look for hidden items or loose loot.
- **Loot Table:** Each room has a predefined list of loot. Searching reveals these items immediately.
- **Hidden Loot:** Rooms may contain high-value hidden loot. Discovery requires a successful **Notice** skill check during the `search` command.
- **Darkness Penalty:** Searching in pitch darkness (no active light source) increases the difficulty of discovering hidden loot by +4.

## 4. Traps & Hazards
- **Discovery:** When moving into a room with traps, the system automatically performs a **Notice** skill check.
- **Spotted Traps:** If successful, the trap is revealed, and the player is warned before it triggers. A revealed trap can be bypassed or disarmed.
- **Triggering:** If the check fails (or if the player moves through a known trap without disarming), the trap triggers, dealing damage or applying status effects.
- **Disarming:** Spotted traps can be neutralized using the `disarm` command, which requires a successful **Fix** skill check. Failure to disarm may trigger the trap immediately.

## 5. Light & Visibility
- **Illumination:** Certain items (torches, lanterns) provide the **Illuminated** status effect.
- **Darkness:** Without an active light source, rooms are described as pitch black, and many details (including visible exits) are obscured.
- **Mechanical Impact:** Darkness imposes a +4 difficulty penalty to all **Notice** checks (searching and trap discovery) and may affect combat effectiveness (future).

## 6. Map Visibility
- **Current Room:** The current room name is displayed in the **StatusSidebar** under the "Exploring" (Dungeon) badge.
- **Look Command:** Use `look` to see a detailed description of the room and list all visible exits. Descriptions vary based on illumination level.
