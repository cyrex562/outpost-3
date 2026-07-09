# Harsh Realm — Milestone 4: People
**Version:** 1.0  
**Date:** 2026-03-27  
**Estimated Duration:** 3–4 weeks (scope is large — full Mythic GME + Adventure Crafter + social + factions + shopping)

---

## Scope Summary

| Subsystem | Scope |
|---|---|
| Social Scene | Full scene state, UNE personality, skill checks, disposition |
| Faction System | Full WWN faction turns, assets, goals, automatic weekly tick, simple AI, reputation, encounter table modification |
| Mythic Oracle | Full GME (fate chart, scene checks, random events, chaos) + full Adventure Crafter (plotlines, threads, themed scenes) |
| Shopping | Simple store: browse, buy/sell, inventory updates |
| Expert Class | Reroll one failed skill check per scene |

---

## ⚠️ Unresolved Design Question (Needs Answer Before Section 4.1 Implementation)

**Skill mapping inside social scenes.** The design doc defines Talk, Connect, Convince, and Trade as separate skills but doesn't specify which social command maps to which skill. Suggested mapping — confirm or revise before coding:

| Command | Skill | Attribute | Notes |
|---|---|---|---|
| `talk <npc>` | — | — | Opens social scene; no check on entry |
| `convince <npc> to <goal>` | Talk | CHA | Friendly persuasion |
| `intimidate <npc>` | Talk | STR | Hostile persuasion; disposition cost on success |
| `deceive <npc>` | Talk | CHA | Lies; large disposition cost if caught |
| `bribe <npc>` | Trade | CHA | Requires item/gold in hand |
| `connect <npc>` | Connect | CHA | Leverage existing network ties |
| `ask <npc> about <topic>` | — | — | Free info exchange; no check unless sensitive |

**Open sub-questions:**
- Does `intimidate` use STR or is it always CHA? (GURPS would say ST; XWN is less explicit)
- Does a failed `deceive` always flip NPC hostile, or is there a partial failure state?
- Is there a `perform` command (Perform skill) for bards/entertainers, or defer that to M5?

---

## Section 4.1 — Social Scene System

**Priority:** 1 (highest)  
**Estimated effort:** ~18 hours  
**Files:** `src/harsh_realm/gm/scenes/social.py`, `src/harsh_realm/models/npc.py` (extend), `data/tables/npc/une_*.yaml`

### 4.1.1 UNE Personality Generation

**Description:** Implement verbatim UNE tables as YAML and a generator that produces NPC personality on first contact.

**YAML files to create:**
```
data/tables/npc/une_power_level.yaml       # 7 entries: wretched → superb
data/tables/npc/une_descriptors.yaml       # d100 adjective table
data/tables/npc/une_motivation_verbs.yaml  # d100
data/tables/npc/une_motivation_nouns.yaml  # d100
data/tables/npc/une_bearings.yaml          # 8 bearings × 5 sub-entries each + focus
data/tables/npc/une_moods.yaml             # disposition scale + chaos modification
```

**Tasks:**
- [ ] Encode all 6 UNE YAML files verbatim from the rulebook
  - **Test:** Each table has correct entry count (power=7, descriptors=100, verbs=100, nouns=100, bearings=40, moods=7)
- [ ] Implement `UNEGenerator` class in `engine/npc_personality.py`
  - [ ] `generate_personality(power_level: str | None) -> NPCPersonality`
  - [ ] `generate_motivation() -> Motivation` (verb + noun pair)
  - [ ] `generate_bearing(chaos_factor: int, relationship: str) -> Bearing`
  - **Test:** Calling `generate_personality()` 100 times produces valid, varied results within table bounds
- [ ] Extend `Entity` data JSON schema to store `une_personality` block
  - ```json
    "une_personality": {
      "power_level": "average",
      "descriptor": "scheming",
      "motivation_verb": "advance",
      "motivation_noun": "wealth",
      "bearing": "scheming",
      "bearing_focus": "future action",
      "base_disposition": "neutral"
    }
    ```
  - **Test:** NPC entity round-trips through SQLite with personality intact
- [ ] NPCs generated without personality get one generated on first `talk` command
  - **Test:** NPC with no `une_personality` in data gets one generated and persisted on first social entry

### 4.1.2 Disposition System

**Description:** Track NPC disposition as a numeric score (-3 to +3) that maps to the UNE mood table. Score persists per NPC. Changes flow through the event bus.

**Tasks:**
- [ ] Add `disposition` integer field to entity data JSON (default 0 = Indifferent)
  - **Test:** Default NPCs start at 0
- [ ] Implement disposition → mood label mapping:
  ```
  -3: Hostile
  -2: Unsteady  
  -1: Guarded
   0: Indifferent
  +1: Sociable
  +2: Friendly
  +3: Helpful
  ```
- [ ] Implement `DispositionChangeEvent(entity_id, old_score, new_score, reason)`
  - **Test:** Score clamped to [-3, +3]; event fired on every change
- [ ] Chaos factor modifies effective disposition during scene checks
  - **Test:** At chaos 9, hostile NPC behaves one step more hostile; at chaos 1, one step more friendly

### 4.1.3 Social Scene State Handler

**Description:** Implement `SocialSceneHandler` following the `SceneHandler` protocol. Handles all social scene logic.

**Tasks:**
- [ ] Implement `SocialSceneHandler` in `gm/scenes/social.py`
- [ ] **Entry triggers:**
  - [ ] Player command `talk <npc>` while NPC is in same hex → transition to Social
  - [ ] GM auto-triggers when player encounters non-hostile NPC (disposition ≥ -1)
  - **Test:** `talk bandit` while bandit.disposition = -3 (Hostile) → refused entry, GM narrates hostility
  - **Test:** Encounter with disposition 0 NPC → auto-enters social scene
- [ ] **Exit triggers:**
  - [ ] Player types `leave` or `goodbye` → exit to Exploration
  - [ ] Hostility threshold crossed (disposition drops to -3 mid-conversation) → exit to Combat
  - [ ] Scene check fires an interrupt → exit per interrupt result
  - **Test:** Three consecutive failed intimidate rolls → disposition hits -3 → scene transitions to Combat
- [ ] Implement `get_valid_commands()` for social mode:
  - `talk/ask`, `convince`, `intimidate`, `deceive`, `bribe`, `connect`, `leave`, `oracle`, `status`, `inventory`
  - **Test:** `attack` is not a valid command in social mode (must `leave` first, then combat triggers)
- [ ] Implement `get_prompt()`: GM narrates NPC's current bearing + what they're doing/saying
  - **Test:** Prompt includes NPC name, power level flavor, current bearing sub-entry text

### 4.1.4 Social Skill Check Resolution

**Description:** Wire social commands to the Rules Engine skill check system with social-specific outcomes.

**Tasks:**
- [ ] Implement skill check resolution for each social command per the mapping table above (resolve the ⚠️ design question first)
- [ ] Implement **margin-of-success outcomes** for social checks:
  ```
  Failure by 4+:  Large disposition penalty, NPC clams up / turns hostile
  Failure by 1-3: Small disposition penalty, NPC is unimpressed
  Success by 0-1: Goal achieved, no disposition change
  Success by 2-3: Goal achieved, small disposition bonus
  Success by 4+:  Goal achieved, large disposition bonus, NPC volunteers extra info
  ```
  - **Test:** Each margin band produces correct disposition change and narrative
- [ ] Fire `action.skill_check` and `social.dialogue` events with full context
  - **Test:** Event log contains roll, total, target, margin, outcome for every social check
- [ ] Narrator generates contextual text per NPC bearing + outcome
  - **Test:** A "hostile bearing / large success" produces different narration than "friendly bearing / large success"

---

## Section 4.2 — Faction System (Full WWN)

**Priority:** 2  
**Estimated effort:** ~20 hours  
**Files:** `src/harsh_realm/faction/faction_turn.py`, `faction/faction_ai.py`, `faction/assets.py`, `data/factions/`, `data/tables/factions/`

### 4.2.1 Faction Data Model & YAML Content

**Description:** The SQLite schema already exists. This section populates starting faction data and implements the data access layer.

**Tasks:**
- [ ] Create `data/factions/` directory with YAML definitions for starting factions
  - Minimum viable: 3–5 factions for the starting region
  - Each faction needs: name, HP, Force/Cunning/Wealth ratings, starting assets, goals, home hex, initial relationships
  - **Test:** All YAML factions load cleanly into SQLite on world creation
- [ ] Create `data/tables/factions/faction_assets.yaml` — full WWN asset list:
  - All three categories: Force, Cunning, Wealth
  - Each asset: name, type, cost, upkeep, HP, attack/counter stats, special abilities
  - **Test:** Asset table count matches WWN rulebook asset list
- [ ] Create `data/tables/factions/faction_actions.yaml` — valid actions per turn
- [ ] Implement `FactionRepository` in `db.py` — CRUD for factions, assets, relations, reputation
  - **Test:** Create faction → read → update HP → delete asset round-trips cleanly

### 4.2.2 Faction Turn Engine

**Description:** Implement the WWN faction turn resolution system.

**Tasks:**
- [ ] Implement `FactionTurnEngine` in `faction/faction_turn.py`
- [ ] Implement all WWN faction actions:
  - [ ] **Attack:** Select asset, select target asset in range, roll attack vs. counter, apply damage
    - Attack formula: `attacker_asset.attack_stat` vs `defender_asset.counter_stat` (faction-scale dice, not d20)
    - **Test:** Asset with 0 HP is removed; faction takes 1 HP if asset was defended
  - [ ] **Expand Influence:** Move or place asset into adjacent/uncontrolled hex
    - **Test:** Asset cannot expand into hex controlled by hostile faction without attacking first
  - [ ] **Create Asset:** Spend Wealth/Force/Cunning to add new asset
    - **Test:** Cannot create asset if faction lacks required attribute minimum
  - [ ] **Repair:** Restore HP to damaged asset (cost = ½ asset purchase price in Wealth)
  - [ ] **Seize Territory:** Take control of hex from rival
  - [ ] **Sell Asset:** Remove asset, recover partial cost
  - [ ] **Refit:** Spend turn to recover faction HP
  - [ ] **Harvest:** Generate Wealth from controlled assets
  - **Test (integration):** Run 4 faction turns; faction HP, asset HP, and territory change coherently
- [ ] Implement asset combat resolution
  - **Test:** Both attacking and defending assets can be damaged in the same exchange
- [ ] Faction XP and advancement (WWN: factions gain XP from successful actions, spend for stat gains)
  - **Test:** Faction gains 1 XP on successful attack; threshold triggers stat gain event

### 4.2.3 Simple Faction AI

**Description:** Automated decision-making for faction turns. Intentionally simple — advanced AI is M6.

**Tasks:**
- [ ] Implement `FactionAI` in `faction/faction_ai.py` with priority-based action selection:
  ```
  Priority order:
  1. Attack enemy assets if within reach and HP > 50%
  2. Repair if any asset at < 50% HP and Wealth available
  3. Create asset if Wealth > threshold and below asset cap
  4. Expand influence into neutral hexes adjacent to territory
  5. Harvest if no better option
  6. Refit if faction HP < 50%
  ```
  - **Test:** Faction with damaged assets prioritizes repair over expansion
  - **Test:** Faction with 0 Wealth cannot create assets (falls through to expand/harvest)
- [ ] Faction AI respects relationships — Allied factions don't attack each other's assets
  - **Test:** Faction with `allied` relationship to another never selects attack against that faction
- [ ] Faction AI targets weakest enemy asset when attacking
  - **Test:** Given two valid targets, AI selects the one with lowest HP

### 4.2.4 Weekly Tick Integration

**Description:** Wire faction turns into the world clock. One faction turn fires per in-game week elapsed.

**Tasks:**
- [ ] Implement week tracking in `gm_state` table (`current_week` key)
- [ ] Trigger `FactionTurnEngine.run_all_turns()` when week advances
  - **Test:** Advancing time by 1 day does NOT trigger faction turn
  - **Test:** Advancing time to next week boundary DOES trigger faction turn
- [ ] All faction turn events logged to `event_log` table with tick
  - **Test:** After one faction turn, event log contains at least one `world.faction_action` event per active faction
- [ ] GM narrates significant faction events to player (asset destroyed, territory taken, new faction conflict)
  - **Test:** When a faction takes a hex within 3 hexes of the player, player receives a narrative message

### 4.2.5 Reputation & Encounter Table Modification

**Description:** Player actions affect faction reputation scores; scores affect what the player encounters in faction territory.

**Tasks:**
- [ ] Implement reputation change events from social/combat actions:
  - Killing faction member: −10 reputation with that faction
  - Completing faction task: +15 reputation
  - Bribing faction member: +5 reputation
  - **Test:** Killing a faction guard fires `reputation.change` event with correct delta
- [ ] Implement disposition → encounter weight modifier:
  ```python
  FACTION_ENCOUNTER_MODIFIERS = {
      "allied":      {"patrol_friendly": +3, "patrol_hostile": -999},
      "friendly":    {"patrol_hostile": -2, "trade_opportunity": +2},
      "neutral":     {},  # no modification
      "unfriendly":  {"patrol_hostile": +2, "spy_encounter": +1},
      "hostile":     {"patrol_hostile": +4, "bounty_hunter": +2, "ambush": +1},
  }
  ```
  - **Test:** In hostile faction territory, encounter roll produces faction patrol 4× more often than neutral territory
  - **Test:** In allied territory, `patrol_hostile` entry is never rolled
- [ ] Reputation score → disposition label mapping (implement thresholds)
  - **Test:** Score of −30 maps to "hostile"; +20 maps to "friendly"

---

## Section 4.3 — Mythic GME + Adventure Crafter

**Priority:** 3  
**Estimated effort:** ~22 hours  
**Files:** `src/harsh_realm/engine/oracle.py` (replace placeholder), `data/tables/oracle/`

### 4.3.1 Fate Chart

**Description:** The full 9×9 Mythic fate chart — 9 likelihood levels × 9 chaos factor levels → probability thresholds.

**Tasks:**
- [ ] Encode the Mythic fate chart as `data/tables/oracle/fate_chart.yaml`
  - 9 likelihood values × 9 chaos factors = 81 probability entries
  - Each entry: `{yes_threshold: int, exceptional_yes: int, exceptional_no: int}`
  - **Test:** Spot-check: Likelihood=LIKELY, Chaos=5 → correct threshold from rulebook
- [ ] Implement `FateCheck.resolve(likelihood, chaos_factor) -> FateResult`
  - Results: Exceptional Yes / Yes / No / Exceptional No
  - **Test:** Roll of 1–4 always produces Exceptional Yes at CERTAIN + chaos 9
  - **Test:** Roll of 96–100 always produces Exceptional No at IMPOSSIBLE + chaos 1
- [ ] Fire `oracle.fate_check` event with full context (likelihood, chaos, roll, result)
- [ ] Format result for narrator: "Fate check: **Yes** (rolled 34 vs threshold 65)"
  - **Test:** All four result types produce distinct narrative formats

### 4.3.2 Chaos Factor

**Description:** Track and modify chaos factor (1–9) based on scene outcomes.

**Tasks:**
- [ ] Store chaos factor in `gm_state` table (key: `oracle_chaos_factor`, default: 5)
- [ ] Implement chaos adjustment:
  - Player wins / controlled outcome → chaos decreases by 1 (min 1)
  - Player loses / things go wrong → chaos increases by 1 (max 9)
  - **Test:** Two player victories in a row decreases chaos from 5 to 3
- [ ] Fire `oracle.chaos_change` event on every adjustment
- [ ] Expose current chaos factor in `status` command output
  - **Test:** `status` shows "Chaos: 6" after several bad outcomes

### 4.3.3 Scene Checks

**Description:** At the start of each new scene, roll a scene check to see if the expected scene plays out or is interrupted/modified.

**Tasks:**
- [ ] Implement `SceneCheck.roll(chaos_factor) -> SceneModification`
  - Roll d10 vs chaos factor
  - `roll > chaos`: Scene plays out as expected (Altered Scene)
  - `roll ≤ chaos && roll is odd`: Interrupt (new random event fires)
  - `roll ≤ chaos && roll is even`: Altered Scene (scene plays out but differently)
  - **Test:** At chaos 9, ~90% of scenes are interrupted or altered
  - **Test:** At chaos 1, ~10% of scenes are interrupted or altered
- [ ] Wire scene checks to GM Controller scene transitions
  - Scene check fires whenever GM transitions to a new scene (exploration hex entry, social entry, rest, etc.)
  - **Test:** Entering a new hex triggers a scene check event in the log

### 4.3.4 Random Event Tables

**Description:** The Mythic random event system — Event Focus (d100) × Event Action (d100) × Event Subject (d100).

**Tasks:**
- [ ] Encode three YAML tables verbatim:
  - `data/tables/oracle/event_focus.yaml` — 100 entries
  - `data/tables/oracle/event_action.yaml` — 100 entries  
  - `data/tables/oracle/event_subject.yaml` — 100 entries
  - **Test:** Each table has exactly 100 entries
- [ ] Implement `RandomEventGenerator.generate() -> RandomEvent`
  - **Test:** Generated events produce grammatically sensible combinations (NPC thread + action + subject)
- [ ] Random events integrated into scene interrupts (4.3.3)
  - **Test:** When interrupt fires, a random event is generated and narrated

### 4.3.5 Thread & NPC Tracking

**Description:** Mythic tracks active threads (story threads + character threads) and the NPC cast list as living in-game state.

**Tasks:**
- [ ] Add `threads` and `oracle_npcs` tables to world SQLite schema:
  ```sql
  CREATE TABLE threads (
      id        TEXT PRIMARY KEY,
      type      TEXT NOT NULL,  -- "story" | "character"
      title     TEXT NOT NULL,
      status    TEXT DEFAULT 'active',  -- active | resolved | abandoned
      progress  INTEGER DEFAULT 0,
      data      TEXT DEFAULT '{}'
  );
  CREATE TABLE oracle_npcs (
      id        TEXT PRIMARY KEY,
      name      TEXT NOT NULL,
      status    TEXT DEFAULT 'active',
      notes     TEXT,
      entity_id TEXT REFERENCES entities(id)
  );
  ```
- [ ] Implement `add thread <title>`, `resolve thread <id>`, `list threads` commands
  - **Test:** Thread persists in DB; `list threads` shows active threads with IDs
- [ ] Implement `add npc <name>`, `remove npc <id>`, `list npcs` commands
  - **Test:** Oracle NPC list independent from entity list (can track NPCs not yet encountered)
- [ ] Thread progress increments when random events reference that thread
  - **Test:** Random event targeting a character thread increments its progress counter

### 4.3.6 Adventure Crafter — Plotlines & Themes

**Description:** The Adventure Crafter adds structured story generation on top of the core GME.

**Tasks:**
- [ ] Encode Adventure Crafter YAML tables:
  - `data/tables/oracle/ac_themes.yaml` — 5 themes (Action, Tension, Mystery, Social, Personal) with weighted sub-tables
  - `data/tables/oracle/ac_characters.yaml` — character elements table
  - `data/tables/oracle/ac_plots.yaml` — plot element table
  - **Test:** Theme distribution: rolling 100 times produces roughly correct theme frequency per AC rules
- [ ] Implement `Plotline` data model (title, theme, status, scenes list)
- [ ] Add `plotlines` table to SQLite:
  ```sql
  CREATE TABLE plotlines (
      id        TEXT PRIMARY KEY,
      title     TEXT NOT NULL,
      theme     TEXT NOT NULL,
      status    TEXT DEFAULT 'active',
      scenes    TEXT DEFAULT '[]',
      data      TEXT DEFAULT '{}'
  );
  ```
- [ ] Implement `create plotline`, `list plotlines`, `advance plotline <id>` commands
  - **Test:** Plotline with 3 scenes advances correctly through AC progression rules
- [ ] Scene generation pulls from active plotline's theme table for narrative flavor
  - **Test:** A "Tension" themed plotline generates scene prompts weighted toward tension elements
- [ ] **Thread progression system:** As scenes complete, threads advance; at threshold, thread resolves
  - **Test:** Completing 3 scenes on a character thread triggers resolution check per AC rules

---

## Section 4.4 — Shopping

**Priority:** 4  
**Estimated effort:** ~6 hours  
**Files:** `src/harsh_realm/gm/scenes/shopping.py`, `data/items/`

### 4.4.1 Shopping Scene State

**Tasks:**
- [ ] Implement `ShoppingSceneHandler` in `gm/scenes/shopping.py`
- [ ] Entry trigger: player in settlement hex + `shop`, `buy`, or `visit merchant` command
  - **Test:** `shop` command outside a settlement: GM responds "There's nothing to buy out here."
- [ ] Exit trigger: `leave`, `done`, or `exit`
- [ ] Valid commands in shopping mode: `list`, `buy <item>`, `sell <item>`, `examine <item>`, `leave`

### 4.4.2 Inventory & Transaction Resolution

**Tasks:**
- [ ] Implement `list` command — display available stock grouped by category (weapons, armor, gear, consumables)
  - **Test:** Output shows item name, description, weight (slots), price
- [ ] Implement `buy <item>` — deduct gold, add item to inventory
  - **Test:** Cannot buy if insufficient funds → GM narrates "You can't afford that."
  - **Test:** XWN encumbrance slots update correctly after purchase
- [ ] Implement `sell <item>` — remove item from inventory, add gold (50% base value)
  - **Test:** Selling equipped armor un-equips it first, then removes from inventory
- [ ] `examine <item>` shows full item stats without buying
  - **Test:** Examine a weapon → shows damage die, attribute modifier, encumbrance, price, special properties
- [ ] All transactions fire `shopping.purchase` or `shopping.sale` events
  - **Test:** Event log contains correct gold delta and item name for every transaction

---

## Section 4.5 — Expert Class Ability

**Priority:** 5  
**Estimated effort:** ~3 hours  
**Files:** `src/harsh_realm/engine/skill_checks.py`, `src/harsh_realm/models/character.py`

**Tasks:**
- [ ] Add `expert_reroll_available: bool` to character data (resets to `true` on each new scene)
  - **Test:** Flag resets to `true` on `gm.scene_change` event
- [ ] Implement reroll trigger: after any failed skill check, if character is Expert class and `expert_reroll_available == true`, GM prompts "You can reroll this check. Use your Expert ability? (yes/no)"
  - **Test:** Expert prompted after failure; non-Expert never prompted
- [ ] On confirmation: roll again, use better result, set flag to `false`
  - **Test:** Flag set to `false` after use; no second prompt for remainder of scene
- [ ] Fire `character.expert_reroll` event
  - **Test:** Event log records original roll, reroll, and which result was used

---

## Acceptance Tests (Full Milestone)

The milestone is complete when ALL of the following pass in a live playtest session:

1. **Social flow:** `talk <npc>` → NPC has UNE-generated personality → player makes skill check → disposition changes → check result narrated with dice visible → `leave` exits social scene
2. **Combat escalation from social:** Intimidate NPC → fail 3 times → disposition hits −3 → scene auto-transitions to Combat → combat resolves per M3 rules
3. **Shopping:** Enter settlement → `shop` → `list` shows items → `buy sword` deducts gold and adds to inventory → `sell dagger` returns gold → `leave` exits shopping
4. **Oracle:** `oracle is there a hidden passage? (likely)` → fate check resolves → chaos factor adjusts → result narrated
5. **Scene check fires:** Enter new hex → scene check triggers → interrupt fires → random event generated and narrated → chaos factor changes
6. **Adventure Crafter:** `create plotline "Find the Starship" action` → `list plotlines` shows it → `advance plotline 1` → scene generated with Action theme flavor
7. **Thread tracking:** `add thread "Find the Starship"` → `list threads` shows active → completing adventure crafter scenes progresses thread
8. **Faction turn:** Advance time by 7 days → faction turn fires automatically → event log shows faction actions → if player was hostile to a faction, encounter table includes patrol entries in that faction's territory
9. **Reputation:** Kill a faction guard → `status` shows reduced reputation with that faction → entering faction territory now shows hostile encounter modifier
10. **Expert reroll:** Expert character fails a skill check → prompted for reroll → confirm → second roll used → ability unavailable for remainder of scene

---

## Dependencies & Ordering

```
4.1.1 UNE YAML tables
  ↓
4.1.2 Disposition system
  ↓
4.1.3 Social scene state ← requires: 4.1.1, 4.1.2
  ↓
4.1.4 Skill check resolution ← requires: 4.1.3 + ⚠️ design decision

4.2.1 Faction data model + YAML
  ↓
4.2.2 Faction turn engine
  ↓
4.2.3 Simple faction AI ← requires: 4.2.2
  ↓
4.2.4 Weekly tick integration ← requires: 4.2.3
  ↓
4.2.5 Reputation + encounter modification ← requires: 4.2.4

4.3.1 Fate chart
  ↓
4.3.2 Chaos factor ← requires: 4.3.1
  ↓
4.3.3 Scene checks ← requires: 4.3.2
  ↓
4.3.4 Random event tables ← requires: 4.3.3
  ↓
4.3.5 Thread + NPC tracking ← requires: 4.3.4
  ↓
4.3.6 Adventure Crafter ← requires: 4.3.5

4.4.1 Shopping scene ← no dependencies (only needs M3 inventory)
4.4.2 Transactions ← requires: 4.4.1

4.5 Expert ability ← requires: M3 skill check system (already done)
```

Recommended build order given priorities:
1. 4.1 (social — highest value, unblocked)
2. 4.3.1–4.3.4 (GME core — unblocks scene checks which affect social)
3. 4.2.1–4.2.4 (faction turns — substantial but self-contained)
4. 4.3.5–4.3.6 (Adventure Crafter — builds on GME core)
5. 4.2.5 (reputation + encounter modification — requires faction turns running)
6. 4.4 (shopping — fast, independent)
7. 4.5 (Expert — smallest task, easiest win at end)

---

## Estimated Effort Summary

| Section | Hours |
|---|---|
| 4.1 Social Scene + UNE | 18 |
| 4.2 Faction System (full WWN) | 20 |
| 4.3 Mythic GME + Adventure Crafter | 22 |
| 4.4 Shopping | 6 |
| 4.5 Expert Class Ability | 3 |
| **Total** | **~69 hours** |

At ~3 hours/day this is approximately 3.5 weeks. At ~5 hours/day this is ~2.5 weeks. The 2-week estimate in the design doc is optimistic given the full Mythic GME + Adventure Crafter scope decision.

**Risk:** Section 4.3.6 (Adventure Crafter) is the highest-risk section. The AC thread progression rules are more intricate than the core GME and require careful encoding of the probability tables. Budget extra time here or consider deferring to M4.5 if the milestone is running long.
