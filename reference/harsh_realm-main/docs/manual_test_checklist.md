# Manual Test Checklist

Test against the deployed instance at `http://harsh-realm-01`.
Mark each item pass/fail and note any issues.

---

## 1. World Management

- [PASS] **1.1** Open the app — World Manager modal appears
- [PASS] **1.2** Create a new world (name: "Test", 20x20, no seed) — success message, modal closes
- [PASS] **1.3** Refresh page — world auto-loads, game state preserved
- [PASS] **1.4** Open World Manager again — "Test" appears in the load list
    caveat on notes 1.3, 1.4, and 1.5. this doesnt work until the world has been created, it goes to the world plage, and the user enters their PC's information.

- [PASS] **1.5** Create a second world — both appear in load list
- [PASS] **1.6** Switch between worlds via load — correct world state each time

---

## 2. Character Creation

- [PASS] **2.1** After world creation, character creation flow begins automatically
- [PASS] **2.2** Enter a character name — accepted, moves to class selection
- [PASS] **2.3** Choose each class option (warrior/expert/adventurer) — description shown for each
- [PASS] **2.4** Attribute rolling — 6 rolls generated (4d6 drop lowest), values look reasonable (3-18)

    I dont think the player should have to press any key to get the dice to roll. this should occur after selecting the class, then it should jump right into assigning attributes.

- [PASS] **2.5** Assign attributes to STR/DEX/CON/INT/WIS/CHA — all 6 assigned, none duplicated

    We should probably have a workflow and a case for if the player decides to change how they want to allocate points after assigning an attribute. This also might be a good candidate for a modal to display letting you drag the values between attributes

- [PASS] **2.6** Skill point distribution — correct number of points for chosen class

    Verify that a warrior only gets two skill points to spend. This is a good candidate for a modal that lets you select a skill, and if points remain optionally increasing proficiency in the skill(s) selected

- [PASS] **2.7** Equipment kit selection — options shown, one selectable

    provide the name instead of heavy_warrior, help with tab complete, also provide a single number or letter to make selection faster

- [PASS] **2.8** Confirm character — summary shown, creation completes

- [PASS] **2.9** Transition to exploration — narration describes starting location

---

## 3. UI Panels

- [PASS] **3.1** Status sidebar shows character name, class, HP bar, AC, gold, scene badge, chaos factor
- [PASS] **3.2** Map panel renders the grid with terrain colors and player marker

    Player marker doesnt render until the player moves.

- [PASS] **3.3** Inventory panel shows equipped and stowed items with encumbrance
- [PASS] **3.4** Chat log displays narration and command echoes
- [PASS] **3.5** Hint bar shows context-appropriate command suggestions
- [PASS] **3.6** Clicking a hint bar suggestion fills the command input
- [PASS] **3.7** Command input accepts text, Enter sends it
- [PASS] **3.8** Arrow up/down cycles through command history
- [PASS] **3.9** Panel resize by dragging borders works smoothly
- [PASS] **3.10** Map pan (drag) and zoom (scroll wheel) work

---

## 4. Exploration

- [PASS] **4.1** `look` — describes current cell terrain and features
- [PASS] **4.2** `n` / `ne` / `e` / `se` / `s` / `sw` / `w` / `nw` — moves in all 8 directions
- [PASS] **4.3** Movement updates the map (player marker moves, new cells revealed)
- [PASS] **4.4** Movement to map edge — blocked with message
- [PASS] **4.5** `search` — skill check narration (roll, success/fail, discovery or nothing)

    Doesnt show roll vs outcome.

- [PASS] **4.6** `status` — full character sheet displayed in chat
- [PASS] **4.7** `inventory` — equipment list displayed in chat

    Not displayed as command hint

- [PASS] **4.8** `help` — command reference shown

    not displayed as a command hit

- [PASS] **4.9** Invalid command (e.g. `asdf`) — error message, not a crash
- [PASS] **4.10** `save` — world snapshot created, confirmation message
- [PASS] **4.11** Navigate to a settlement hex — `look` identifies it as a settlement
- [PASS] **4.12** `look` at settlement lists NPCs present

    look describes the surrounding terrain instead of the town when exploring town. command should also be enter town not explore town

---

## 5. Town Scene

- [PASS] **5.1** `explore` at a settlement — transitions to town scene

    command should be enter not explore

- [FAIL] **5.2** `look` — describes current building/location in town

    command describes area of tile and surrounding terrain

- [FAIL] **5.3** Movement within town works (8 directions)

    cant move in the town

- [FAIL] **5.4** `leave` — returns to exploration at the settlement hex

---

## 6. Shopping

cant enter shops

- [FAIL] **6.1** `shop` at a settlement — transitions to shopping scene
- [FAIL] **6.2** `list` — shows available items with prices
- [FAIL] **6.3** `buy <item>` — gold decreases, item added to inventory, confirmation message
- [FAIL] **6.4** `buy <item>` with insufficient gold — purchase blocked, error message
- [FAIL] **6.5** `sell <item>` — gold increases (50% value), item removed, confirmation
- [FAIL] **6.6** Chat log formats purchase/sale events (shows balance)
- [FAIL] **6.7** `leave` — returns to exploration
- [FAIL] **6.8** `shop` outside a settlement — rejected with message

---

## 7. Social Interaction

- [PASS] **7.1** `talk <npc>` — transitions to social scene, shows NPC name and disposition
- [PASS] **7.2** `convince` / `persuade` — skill check narrated, disposition changes on success
- [PASS] **7.3** `intimidate` — skill check narrated
- [PASS] **7.4** `deceive` — skill check narrated
- [PASS] **7.5** `bribe <amount>` — gold spent, disposition improves
- [PASS] **7.6** Chat log formats social events (disposition changes, skill check results)
- [NOT TESTED] **7.7** Expert reroll prompt appears on failed check (expert class only)
- [PASS] **7.8** `leave` — returns to previous scene

---

## 8. Combat

- [PASS] **8.1** Random encounter triggers during exploration (movement or search)
- [PASS] **8.2** Combat scene loads with awareness state (surprise/mutual/enemy surprise)
- [PASS] **8.3** `attack` — attack roll narrated (roll + modifier vs AC), damage on hit
- [PASS] **8.4** Enemy takes its turn after player action
- [PASS] **8.5** `flee` — escape check narrated, returns to exploration on success
- [NOT TESTED] **8.6** `use <item>` — consumable used (e.g. healing potion restores HP)

No items available to test with

- [PASS] **8.7** Veteran's Luck prompt on failed attack (warrior class only)
- [PASS] **8.8** Victory — "combat over" message, loot available
- [PASS] **8.9** `harvest` after victory — loot added to inventory
- [PASS] **8.10** Chat log formats combat events (attack rolls, damage, saves)
- [NOT TESTED] **8.11** Defeat (HP reaches 0) — transitions to respawn scene

---

## 9. Respawn

- [NOT TESTED] **9.1** Respawn scene shows two options (respawn / new character)
- [NOT TESTED] **9.2** Option 1 (respawn) — HP restored to 50%, one item lost, 15% XP lost, placed at settlement
- [NOT TESTED] **9.3** Option 2 (new character) — returns to character creation

---

NOT IMPLEMENTED

## 10. Dungeon

- [NOT IMPLEMENTED] **10.1** `enter` at a dungeon/lair hex — transitions to dungeon scene
- [NOT IMPLEMENTED] **10.2** `look` — describes current room with exits
- [NOT IMPLEMENTED] **10.3** Movement between connected rooms works
- [NOT IMPLEMENTED] **10.4** Movement to non-existent exit — blocked with message
- [NOT IMPLEMENTED] **10.5** `search` — skill check for traps/treasure
- [NOT IMPLEMENTED] **10.6** `leave` — returns to exploration at dungeon hex

---

## 11. Oracle & Threads

- [FAIL] **11.1** `oracle` — fate check narrated (chaos factor roll)

Not ever displayed

- [PASS] **11.2** Chaos factor displayed in status sidebar
- [FAIL] **11.3** Thread management commands work (`add`, `resolve`, etc.)

Not able to call them

---

## 12. WebSocket & Connection

- [PASS] **12.1** Connection status indicator visible (connected/disconnected)
- [PASS] **12.2** Commands send and responses arrive in real-time (no page reload needed)
- [PASS] **12.3** Page refresh — WebSocket reconnects, game state preserved
- [PASS] **12.4** Rapid command entry (type 5+ commands fast) — all processed in order, no drops

---

## 13. Admin Panel

- [PASS] **13.1** Navigate to `/admin` — admin panel loads with tabs
- [PASS] **13.2** **Skill Mappings tab** — list loads, can edit a skill's attribute/DC, save persists

    What does the system do with an added verb/skill combo? where are the values pulled from? we should have a way to autocomplete or provide suggestions or a list of verbs and skills

- [PASS] **13.3** **Difficulties tab** — list loads, can edit DC values
- [PASS] **13.4** **Disposition tab** — list loads, can edit outcome deltas

    Same question as skill mappings - how is the information here used, what does changing the delta mean? where do the outcome key values come from and how are they used?

- [PASS] **13.5** **Encounter Weights tab** — list loads, editable

    What do the values mean? need more explanation of what values to set, what they mean, and how they're used.

- [PASS] **13.6** **Faction Assets tab** — list loads, editable

    There should be both a type that's a normal readable string and its value like "Elite Troops" vs "Elite_Troops"

- [PASS] **13.7** **Map tab** — grid renders, can select and edit a cell's terrain
- [FAIL] **13.8** **Characters tab** — lists PCs/NPCs, can filter, edit stats, create new, delete

    PC is not listed

- [PASS] **13.9** **Factions tab** — CRUD works, relationships editable, assets manageable
- [PASS] **13.10** **Dungeons tab** — can create dungeon, add rooms, connect them

    Need an auto-generate function here to help people get started.

- [PASS] **13.11** **Worlds tab** — lists worlds, clone and delete work
- [PASS] **13.12** **YAML Files tab** — lists data files, can view and edit content
- [PASS] **13.13** **World Meta tab** — metadata key-value pairs editable
- [PASS] **13.14** **Creatures tab** — CRUD works for creature templates

    Loot Table not displayed correctly.
    What about an encounter frequency or terrain where its likely to be encountered

- [PASS] **13.15** **Items tab** — CRUD works for item definitions
- [PASS] **13.16** **Random Tables tab** — can view and edit table entries
- [PASS] **13.17** **Oracle & Threads tab** — can manage threads and oracle NPCs
- [PASS] **13.18** **GM Commands** — teleport, spawn, give item, set HP, set gold all work
- [PASS] **13.19** Reset buttons revert config to YAML defaults
- [PASS] **13.20** Admin changes reflect in-game (e.g. edit skill DC, then test skill check)

---

## Notes

Write any general observations, UX issues, or suggestions here:

- What are landmarks?
- Map needs a legend that's toggleable
- What do Lairs do?
- Ruins dont do anything
- Make XP/Level progression table editable
- instead of tags for items and creatures - or maybe still tags, a mapping somewhere to a table/yaml definition of what the tag means and some way of defining the business logic for it
- ranged weapons need ammunition
- melee weapons need damage roll and shock value
- i think some things in the item table should be in an effects or abilities or actions table like bit and claws - these should be something you can define in one table and add to a creature with bonuses/penalties, damage values, shock factor, etc.
- discoveries random tables need to be able to map to a real item we can give the player. i think discovered items, should be separate from things found in a location like interesting terrain features.
- random tables probably need to use the more conventional roll value instead of or in addition to weight
- how is pocket litter mapped to the game? i have many more tables like that, so i want to understand how i can define more data from books of random tables.
- the NPC UNE files are not rendering as forms correctly
- greetings.yml is not rendering properly
- GM commands should have the ability to set character stats, give XP, etc.
- we probably need a special screen or set of screens for apportioning points when a character levels up
- we should track time of day and time should elapse while traveling
- random weather events
- i need to add more creatures
- combat should be more obvious, sometimes i miss that we entered combat when i'm busy typing keys to move around. maybe flash the chat background and/or display a dialog box pausing things that the player can hit any key to dismiss, as well as disable it from appearing in the future.
- enemy attack info likes rolls and checks should be displayed as well.
- after finishing an encounter, or fleeing the character should be able rest in the same tile
- add foraging
- character should not have talk as an option when fighting animals -- until we add bard or something like that later that can talk to transform into animals
- stack items in the inventory
- we should add admin chat commands as well so theoretically you could enter stuff like /admin or /gm and then a command plus parameters like /gm skill_check PC survive vs current terrain or something like that or /admin give [target] [x] [gold - item name or id]
