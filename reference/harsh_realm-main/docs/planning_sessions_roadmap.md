# Harsh Realm — Planning Sessions Roadmap

> **Purpose:** Track the design conversations needed to produce task specs for each milestone.
> Each session produces a `milestone_N_tasks.md` file and any supporting docs the coding agent needs.
> Sessions should happen just-in-time — plan a milestone when the previous one is nearing completion.

---

## Milestone 0: Skeleton
**Status:** Task spec complete (`milestone_0_tasks.md`). Ready for implementation.

---

## Session 1: Milestone 1 — The Empty World

**When:** During or immediately after Milestone 0 implementation.

**Topics to cover:**
- Grid map generation algorithm: noise-based vs. table-based terrain assignment. What balance of terrain types for the setting (lots of wasteland and forest, sparse settlements)? Hex grid for overworld, square grid for towns/dungeons.
- Terrain type list finalized for the feudal planet setting. Which terrains exist? Wasteland, forest, hills, mountain, swamp, ruins, settlement, water, desert? Others?
- Hex description template authoring: how many variants per terrain type to avoid obvious repetition? What details to include (weather mention, time-of-day flavor, distant landmarks)?
- Character creation flow: what does the GM walk you through? Name → class selection → attribute generation method (3d6 in order? array? point buy?) → skill allocation → starting equipment. How guided vs. freeform?
- XWN character model details: what data goes in the entity JSON? Need to nail down the exact fields for attributes, skills, HP, AC, equipment, XP, level.
- Command parser specifics: exact command vocabulary for Exploration scene. How to handle ambiguous input (unrecognized command → help prompt? fuzzy match?).
- Map size: how big is the starting region? 20x20 cells? 30x30? Bigger = more content needed.

**Docs to produce:**
- `docs/rules_reference/attributes.md` — XWN attribute generation, modifiers, derived stats
- `docs/rules_reference/classes.md` — Warrior, Expert, Adventurer abilities at level 1
- `docs/rules_reference/skills.md` — Full XWN skill list with controlling attributes and difficulty guidelines
- `data/skills.yaml` — Skill definitions
- `data/classes.yaml` — Class definitions
- `milestone_1_tasks.md` — Task spec with acceptance criteria

---

## Session 2: Milestone 2 — Things to Find

**When:** During Milestone 1 implementation.

**Topics to cover:**
- Random table schema finalized: confirm the YAML format, subtable reference syntax, tag filtering logic, weighted roll algorithm.
- Content table inventory: list every table needed for this milestone. Encounter tables per terrain type, NPC name tables, occupation tables, personality tables, settlement name/feature tables, POI type tables, discovery description tables. Identify which ones you'll author from source material vs. which the agent should stub.
- Settlement generation rules: what makes a settlement? Size categories (hamlet/village/town), number of NPCs, available services (inn, smith, merchant, healer), faction affiliation.
- NPC generation: minimum viable NPC — name, occupation, personality trait, motivation, location. What's the data model?
- Encounter check mechanics: when entering a new cell, what's the encounter probability? Modified by terrain type? Time of day? Faction territory? How does "nothing happens" work (is it a table result or a probability gate before the table)?
- Oracle system details: exact Mythic GME fate chart probabilities, chaos factor rules, scene check procedure, random event action/subject word lists. Need to specify enough for the agent to implement correctly.
- SVG grid map rendering: what visual information to show. Terrain colors, explored vs. fog, player position marker, POI icons. Cell size, layout, zoom/pan behavior. HexMap for overworld, SquareMap for dungeons/towns.
- Sidebar content: exactly what data to display (HP, AC, location, terrain, weather placeholder, chaos factor, party list).

**Docs to produce:**
- `docs/rules_reference/oracle.md` — Mythic GME fate chart, chaos factor, scene checks, random events
- `docs/rules_reference/encounters.md` — Encounter check procedure, probability, terrain modifiers
- Initial YAML content tables (authored by Josh from source material)
- `milestone_2_tasks.md` — Task spec

---

## Session 3: Milestone 3 — Danger

**When:** During Milestone 2 implementation.

**Topics to cover:**
- XWN combat rules in detail: initiative procedure, attack roll formula, damage formula, defense/AC, death and dying, stabilization, healing. Need a complete rules reference the agent can implement from.
- Warrior class combat ability: "once per fight, negate a hit or force a miss." Exact trigger and resolution.
- Starting weapon and armor lists: what weapons exist at TL3 + scattered TL4? Swords, spears, crossbows, crude firearms, pretech energy weapons (rare loot). Stats for each (damage die, attribute, range if applicable, encumbrance, cost).
- Monster/enemy design: what's the statblock format? HD, AC, attack bonus, damage, special abilities, morale (even though v1 enemies fight to death, morale field for later use), loot table reference.
- Initial bestiary: 10-15 creatures appropriate for the setting. Natural predators, engineered beasts, human enemies (bandits, faction soldiers, bounty hunters). Stats and encounter context.
- Combat scene state details: turn prompt format, what information to show each round (initiative order, HP bars, distance/positioning or just "melee range"?), how flee works mechanically (opposed check? automatic with consequences?).
- Loot generation: what do enemies drop? Currency, mundane items, occasional pretech? Loot table structure.
- XP awards: how much XP per encounter? XWN standard is based on HD of defeated enemies. Confirm the XP-to-level table.
- Death handling specifics: respawn location (nearest settlement?), respawn penalty (lose equipped gear? XP penalty? both?), difficulty setting modifiers.

**Docs to produce:**
- `docs/rules_reference/combat.md` — Complete combat procedure
- `docs/rules_reference/weapons_armor.md` — Equipment stats
- `docs/rules_reference/death.md` — Death, dying, respawn rules
- `data/weapons.yaml`, `data/armor.yaml` — Equipment definitions
- `data/tables/creatures/` — Monster statblock YAML files (authored by Josh)
- `milestone_3_tasks.md` — Task spec

---

## Session 4: Milestone 4 — People

**When:** During Milestone 3 implementation.

**Topics to cover:**
- NPC interaction model: what happens when you `talk to <npc>`? Does the GM generate dialogue? Template-based responses keyed to personality + disposition + topic? How deep does the conversation tree go?
- UNE (Universal NPC Emulator) integration: mood, bearing, focus tables. How these map to NPC behavior and dialogue generation. Need exact table data or decide to adapt/simplify.
- Social skill checks: Talk for persuasion, Lead for commanding, Connect for networking. What modifiers apply? NPC disposition? Faction reputation? Situation?
- NPC disposition system: numeric score or categorical (hostile/unfriendly/neutral/friendly/allied)? What shifts it? Successful social checks, gifts, faction reputation, player actions?
- Faction reputation mechanics: what actions change reputation? Killing faction members, completing tasks that align/conflict with faction goals, trading with faction merchants. How much per action?
- Mythic scene system: exact procedure for scene checks at boundaries. When is a "scene boundary"? Entering a new cell? Entering a building? Starting a conversation?
- Shopping: merchant inventory generation, pricing (base price modified by what?), haggling (Trade skill check for discount?), what's available in different settlement sizes.
- Expert class ability: "reroll one failed skill check per scene." What counts as a scene? How is this tracked?

**Docs to produce:**
- `docs/rules_reference/social.md` — Social skill checks, NPC disposition, reputation
- `docs/rules_reference/factions.md` — Faction reputation effects (preview, full faction turns come in M6)
- `docs/rules_reference/scene_system.md` — Mythic scene checks, chaos factor adjustment rules
- NPC personality/dialogue tables (authored by Josh or adapted from UNE)
- `milestone_4_tasks.md` — Task spec

---

## Session 5: Milestone 5 — Depth (Dungeons)

**When:** During Milestone 4 implementation.

**Topics to cover:**
- Dungeon generation algorithm: BSP, random walk, template-based, or hybrid? Room size ranges, corridor behavior, door placement, dead ends.
- Dungeon theming: pretech facility, natural cave system, ruined settlement, monster lair. How does theme affect room contents, descriptions, encounter tables, and loot?
- Room content generation: what can a room contain? Enemies, traps, treasure, environmental hazards, interactive objects (terminals, locked containers, machinery), nothing. Probability distribution.
- Trap mechanics: Notice check to detect (difficulty based on trap quality), relevant skill to disarm (Fix for mechanical, Program for electronic), damage/effect on failure. What kinds of traps? Pit, dart, gas, energy field, collapse.
- Treasure and loot generation: mundane items, coins, pretech artifacts. Rarity tiers? Dungeon depth affects loot quality?
- Inventory system: XWN encumbrance (readied vs. stowed slots). How many slots per attribute? What fills a slot? UI for managing inventory.
- Dungeon navigation: room-by-room. What does the player see on entering a room? Exits, visible contents, general description. Commands: `go <exit>`, `search`, `open`, `use`, `take`.
- Dungeon → overworld transition: exit dungeon (square grid), back to world map (hex grid). Can you re-enter? Does it persist or regenerate? Cleared rooms stay cleared?
- Skill expansion: which skills become active in dungeons? Sneak (avoid encounters), Notice (find traps/secrets), Fix (disable traps, repair objects), Exert (force doors), Heal (patch up between fights).

**Docs to produce:**
- `docs/rules_reference/dungeons.md` — Dungeon exploration procedure
- `docs/rules_reference/traps.md` — Trap detection, disarming, effects
- `docs/rules_reference/inventory.md` — XWN encumbrance system
- Dungeon theme description tables (authored by Josh)
- `milestone_5_tasks.md` — Task spec

---

## Session 6: Milestone 5.5 — Python Plugins

**When:** During Milestone 5 implementation.

**Topics to cover:**
- Plugin API surface: what exactly can a plugin do? Subscribe to events, emit events, register new commands, register table generators, modify resolver behavior? All of these? A subset?
- Plugin lifecycle: discovery (scan directory), loading order (dependencies?), initialization, hot-reload mechanism (file watcher? manual command?), unloading.
- Sandbox boundaries: what's restricted? File I/O (read-only to data/? no write outside plugins/?)? Network access (blocked entirely?)? CPU time limits? Import restrictions?
- Plugin configuration: can plugins have their own config? YAML file per plugin? Section in main config?
- Error isolation: plugin crash doesn't take down the server. How to report plugin errors to the player?
- Example plugins to ship: what would be most useful as both examples and actual functionality? Event logger, custom encounter modifier, new command, custom loot table?
- Plugin documentation: what does the README in `plugins/` need to contain for someone (Josh) to write a plugin?

**Docs to produce:**
- `docs/plugin_api.md` — Plugin authoring guide
- Example plugin code
- `milestone_5_5_tasks.md` — Task spec

---

## Session 7: Milestone 6 — The Living World

**When:** During Milestone 5.5 implementation.

**Topics to cover:**
- WWN/SWN faction turn rules in full detail: action types (attack, expand, create asset, use asset ability, repair, hide asset, sell asset), resolution mechanics, asset stat blocks, asset list for the setting.
- Faction AI decision-making: how does a faction choose its action? Priority system (defend if attacked, pursue goal otherwise, opportunistic expansion)? How smart does it need to be?
- Faction goal generation: what goals do factions pursue? Territory control, rival destruction, wealth accumulation, player capture (if hostile), artifact recovery? How do goals change?
- World tick timing: how much game time passes per action? Moving one cell = ? hours. Resting = 8 hours. Combat = minutes. How do faction turns (weekly) align with player time passage?
- NPC behavior between ticks: simple states (at_work, at_home, traveling). Do NPCs move between settlements? React to faction actions? Die in faction conflicts?
- World event generation: what kinds of events? Weather, faction conflict visible at a distance, NPC death/birth, resource discovery, plague, migration. How frequent? How impactful?
- Tension/pacing system: how does tension accumulate and release? What are the thresholds? How does it modify encounter frequency and event severity?
- Rumor system: how do rumors propagate? Hear them at settlements? From NPCs? How accurate are they? Can they be false?
- Travel scene state: long-distance movement as montage. How many encounter checks per cell traveled? Rest requirements? Random events during travel?
- Between-session summary: what happened while you were away? How much detail? Just faction actions and major events, or also NPC activities and weather?

**Docs to produce:**
- `docs/rules_reference/faction_turns.md` — Complete WWN/SWN faction turn rules
- `docs/rules_reference/world_simulation.md` — Tick system, time passage, between-session rules
- Faction asset definitions (adapted from WWN/SWN by Josh)
- Starting faction definitions for the setting (authored by Josh)
- `milestone_6_tasks.md` — Task spec

---

## Future Sessions (Post-Milestone 6)

These are not yet scheduled but represent the next wave of design work:

- **Ranged combat and firearms:** TL3 crossbows and crude firearms, TL4 energy weapons. Range penalties, ammunition, reloading.
- **Advanced combat maneuvers:** Defend, aim, charge, grapple, called shots. House rules adapted from GURPS or other sources.
- **Steampunk/tech weapons module:** Expanding the TL3-4 equipment list. Gadgets, vehicles, powered armor fragments.
- **Godbound/Legate integration:** Workings, magnitude, divine gifts alongside XWN classes. Major design session needed.
- **SWN space expansion:** Sector generation, starship systems, space travel, ship combat. Phase 2 of the project.
- **Combat enemy status panel:** A dedicated UI panel (or sidebar section) that shows a real-time enemy roster during combat — name, health bar (vague or exact based on Notice check), alive/defeated state. Updates on every hit. See playtesting note from M3: the chat narration alone makes it hard to track enemy state across rounds.
- **Advanced NPC AI:** Role-aware companion behavior, NPC daily schedules, NPC-to-NPC interactions.
- **LLM narrative enhancement:** Optional LLM integration for richer descriptions and NPC dialogue.
- **Traveller/Cepheus planet generation:** Extended world classification for Phase 2 planets.
- **Magic/Psionics system:** Mage and Psychic classes, spell/power lists, resource management.

---

## Session Checklist Template

For each planning session, cover:

- [ ] Review what's been built since last session — any design issues discovered during implementation?
- [ ] Playtest the current build — what works, what feels wrong, what's missing?
- [ ] Discuss the next milestone's mechanics in detail
- [ ] Identify content tables needed — which Josh will author, which should be stubbed
- [ ] Write rules reference docs for the agent
- [ ] Break milestone into tasks with acceptance criteria
- [ ] Update CLAUDE.md with current project state
- [ ] Update open questions in ARCHITECTURE.md
