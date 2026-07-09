# Milestone 3: Danger — Task Specification

> **Goal:** Combat works. The player can fight enemies, make tactical decisions, win or die. Encounters from Milestone 2 now resolve mechanically. Loot, XP, healing, and death/respawn are functional.
> **Estimated time:** 2-3 weeks (AI-assisted development)
> **Prerequisite:** Milestone 2 complete. Read CLAUDE.md, AGENTS.md, and `docs/rules_reference/combat.md`.

## Success Criteria

When this milestone is complete:

1. `pytest` passes with all tests green.
2. Hostile encounters trigger the combat awareness check (Notice/Survive). Three outcomes: player surprise, mutual awareness, enemy surprise.
3. Player surprise gives choice to engage, avoid, sneak, or prepare. Avoiding a detected encounter skips combat.
4. Combat scene state activates with turn-based initiative order.
5. Player can `attack <target>`, `flee`, or `use <item>` on their turn.
6. Attack rolls follow XWN rules: d20 + attack bonus + skill + attribute mod vs. AC. Damage follows weapon die + modifiers. Warrior bonus damage applies.
7. Enemy AI executes simple behavior: approach and attack each round, fight to the death.
8. Enemy health displayed as vague descriptions by default. Successful Notice check at combat start reveals exact HP/AC.
9. Multiple enemies are auto-numbered. Player can target by name or number.
10. Flee always succeeds (player is never trapped), but clean escape requires a skill check. Failure incurs damage, item loss, or displacement.
11. At 0 HP, player gets one final action before collapsing (house rule).
12. On death: respawn at nearest settlement, lose equipped weapon or random gear, lose 10-20% XP. Can buy a basic kit from settlement.
13. Warrior's Veteran's Luck ability works: once per fight, negate a hit.
14. On combat victory: XP awarded based on enemy HD. Loot rolled from enemy-type loot tables.
15. Post-combat first aid: Heal skill check restores HP (once per fight).
16. `rest` command restores HP slowly over time (tick cost).
17. Healing items found through `search` usable in and out of combat via `use <item>`.
18. Town healers restore full HP for a fee via `talk to <healer>`.

---

## Task 3.1: Combat Rules Reference

**What:** Create the rules reference document the coding agent needs to implement XWN combat correctly.

**Files:**
- `docs/rules_reference/combat.md` (NEW)
- `docs/rules_reference/weapons_armor.md` (NEW)
- `docs/rules_reference/death.md` (NEW)
- `docs/house_rules/last_stand.md` (NEW)

**Deliverables:**

`combat.md` — Complete XWN combat procedure:
- Initiative: d8 + DEX modifier. Higher goes first. Ties broken by DEX score, then alphabetical.
- Turn structure: move + one action per turn.
- Attack roll: d20 + attack bonus + relevant skill (Stab/Shoot/Punch) + attribute modifier vs. target AC. Natural 1 always misses. Natural 20 always hits.
- Damage: weapon damage die + attribute modifier (STR for melee, DEX for ranged if applicable). Warriors add half level (round up) to damage.
- Saving throws: when applicable, d20 vs. save target (15 - half level).
- [PLACEHOLDER — confirm exact attack bonus progression by class/level from source books]
- [PLACEHOLDER — confirm critical hit rules if any beyond natural 20 always hits]

`weapons_armor.md` — Equipment stats:
- Weapon table with: name, damage die, attribute, skill, range (if any), encumbrance, cost, TL, special notes
- Armor table with: name, AC bonus, encumbrance, cost, TL, special notes
- [PLACEHOLDER — fill stats from WWN/SWN source books. Include both TL3 weapons (swords, spears, crossbows, crude firearms) and rare TL4 weapons (energy weapons, pretech blades)]

`death.md` — Death, dying, and respawn:
- At 0 HP: character is dying. In standard XWN, allies can stabilize with Heal check, death in 6 rounds without aid.
- Solo play modification: since no allies exist yet, 0 HP triggers the Last Stand house rule (see below), then death.
- Respawn procedure:
  1. GM narrates death: "Everything goes dark..."
  2. GM presents respawn options:
     - Respawn at nearest settlement (default)
     - Create a new character (offered but not forced)
  3. On respawn:
     - Location set to nearest settlement hex
     - HP restored to 50% of max
     - One equipped item lost (weapon preferred, otherwise random gear piece). Item remains at death hex as a retrievable entity.
     - XP reduced by 15% of current XP (never causes de-level; minimum 0)
     - GM narrates: "You wake in [settlement name]. [Narrative explanation]. Your [lost item] is gone — left behind at [death location]."
  4. Player can buy a basic equipment kit from settlement (same kits as character creation, priced in coin)
  5. Death location is marked on the map with a special icon. Lost items persist there until retrieved.

`last_stand.md` — House rule:
- When the player character reaches 0 HP, before collapsing they get one final action.
- Valid final actions: `attack` (one attack at -2 penalty), `use <item>` (use a healing item — if it brings HP above 0, the character stabilizes and combat continues), `flee` (attempt escape, auto-success but take no skill check — always messy flee with consequences).
- After the final action resolves, if HP is still 0 or below, the character dies.
- This ability triggers automatically — the GM prompts: "You stagger, vision fading. One last chance — what do you do?"

**Developer action required:** Fill in all `[PLACEHOLDER]` values in `combat.md` and `weapons_armor.md` from source books before combat implementation begins.

---

## Task 3.2: Enemy/Creature Data Model & Bestiary

**What:** Define the creature statblock format and create a starting bestiary of ~20 enemy types in YAML.

**Files:**
- `src/harsh_realm/models/creature.py` — Creature data model (NEW)
- `data/creatures/` — Creature definition YAML files (NEW directory)
- `data/schemas/creature_schema.yaml` — Creature YAML schema (NEW)

**Deliverables:**

`CreatureData` frozen dataclass:
```python
@dataclass(frozen=True)
class CreatureData:
    id: str                        # "wild_dog", "bandit", "ancient_automaton"
    name: str                      # Display name: "Wild Dog"
    hd: int                        # Hit dice (determines HP, attack bonus, XP)
    hp_per_hd: int                 # HP per HD (default 4 for average, can vary)
    ac: int                        # Armor class
    attack_bonus: int              # Base attack bonus
    damage: str                    # Damage expression: "1d6", "1d8+2", "2d6"
    damage_type: str               # "melee", "ranged", "special"
    attack_skill: str              # "stab", "punch", "shoot" — for narration
    attack_description: str        # "bites at", "swings a rusty blade at", "fires at"
    num_attacks: int               # Attacks per round (usually 1)
    behavior: str                  # "aggressive", "territorial", "defensive", "ambush"
    awareness_difficulty: int      # Difficulty for player to detect (6-14)
    flee_difficulty: int           # Difficulty for clean escape (6-14)
    unavoidable: bool              # If true, skip awareness check — combat is forced
    morale: int                    # Future use: threshold for enemy to flee (not used in M3)
    loot_table: str | None         # Reference to loot table ID
    harvestable: dict | None       # {"material": "wolf pelt", "skill": "survive", "difficulty": 8}
    description_unseen: str        # Before combat: "You hear growling from the brush."
    description_seen: str          # At combat start: "Three wild dogs bare their teeth."
    description_short: str         # During combat: "wild dog"
    xp_value: int                  # XP awarded on defeat. [PLACEHOLDER: derive from HD per XWN table]
    tags: list[str]                # ["beast", "pack", "wilderness"] for table filtering
    special_abilities: list[str]   # Future use: special attack/defense abilities
```

Creature YAML schema:
```yaml
# data/creatures/wolves.yaml
creatures:
  - id: wolf
    name: Wolf
    hd: 1
    hp_per_hd: 4
    ac: 13
    attack_bonus: 1
    damage: "1d6"
    damage_type: melee
    attack_skill: punch
    attack_description: "lunges and bites at"
    num_attacks: 1
    behavior: aggressive
    awareness_difficulty: 8
    flee_difficulty: 6
    unavoidable: false
    morale: 8
    loot_table: null
    harvestable:
      material: "wolf pelt"
      skill: "survive"
      difficulty: 8
    description_unseen: "You hear howling in the distance, growing closer."
    description_seen: "A pack of gaunt wolves emerges from the shadows, eyes gleaming."
    description_short: "wolf"
    xp_value: 15   # [PLACEHOLDER: verify from XWN XP table]
    tags: [beast, pack, wilderness, forest, hills, plains]
    special_abilities: []
```

**Creature YAML files to create (stubs — developer fills stats from source material):**

| File | Creatures | Notes |
|---|---|---|
| `data/creatures/beasts.yaml` | Wolf, Wild Buffalo, Giant Crab, Giant Scorpion, Giant Insects | Natural animals/megafauna |
| `data/creatures/humanoids.yaml` | Bandit, Scavenger, Kobold, Goblin, Lizardfolk, Minotaur | Intelligent enemies with gear |
| `data/creatures/undead.yaml` | Zombie, Shade (Angry Ghost) | Undead types |
| `data/creatures/constructs.yaml` | Ancient Automaton, Gargoyle | Pretech and magical constructs |
| `data/creatures/mythical.yaml` | Dragon (template with size/element params), Basilisk, Sphinx, Griffon | Powerful/rare creatures |
| `data/creatures/elemental.yaml` | Elemental Spirit (fire, water, earth, air variants) | Elemental types |

Each file should have 2-5 creature entries with full statblocks. Developer will verify and adjust stats from source books. Agent should use reasonable defaults based on HD:
- Attack bonus ≈ HD (1 HD creature has +1 attack)
- HP ≈ HD × 4 (average)
- AC ranges from 10 (unarmored) to 18+ (heavily armored/natural)
- XP value: [PLACEHOLDER — use HD × 15 as rough default until developer fills XWN XP table]

Dragon template note: create a base dragon entry, then generate specific dragons by combining size (small/medium/large/huge) and element (fire, ice, lightning, poison). Size modifies HD, HP, damage, AC. Element modifies damage type and description. The agent should implement this as a generator or parameterized function, not 16 separate statblocks.

**Tests:** `test_creatures.py`
- Load creatures from YAML → all required fields present
- Creature with `harvestable` field has valid skill and difficulty
- Dragon template generates valid creatures for each size/element combination
- Creature tags filter correctly (find all "wilderness" creatures, all "beast" creatures)
- XP value is positive for all creatures
- At least 15 unique creature IDs loaded from stubs

---

## Task 3.3: Combat Awareness Check

**What:** When a hostile encounter triggers, roll an awareness check to determine how combat starts. Three outcomes: player surprise, mutual awareness, enemy surprise.

**Files:**
- `src/harsh_realm/engine/combat.py` — Combat initialization logic (NEW)
- `src/harsh_realm/gm/scenes/exploration.py` — Modify encounter handling to use awareness check

**Deliverables:**

`CombatInitiation` class (or functions in combat module):
- `awareness_check(character: Character, creature: CreatureData, terrain: str) -> AwarenessResult`
  1. If creature is flagged `unavoidable`: return `AwarenessResult.ENEMY_SURPRISE` (skip check entirely)
  2. Determine skill: wilderness terrains → Survive + WIS, urban/ruins → Notice + WIS or INT
  3. Roll skill check vs. creature's `awareness_difficulty`
  4. Determine creature's check: roll 2d6 vs. character's effective Sneak (or base difficulty 8 if no Sneak consideration)
  5. Results:
     - Player succeeds, creature fails → `PLAYER_SURPRISE`: player detects enemy first
     - Both succeed or both fail → `MUTUAL_AWARENESS`: both see each other
     - Player fails, creature succeeds → `ENEMY_SURPRISE`: ambushed

`AwarenessResult` enum:
```python
class AwarenessResult(Enum):
    PLAYER_SURPRISE = "player_surprise"
    MUTUAL_AWARENESS = "mutual_awareness"
    ENEMY_SURPRISE = "enemy_surprise"
```

Exploration scene changes:
- When encounter system returns a hostile encounter, run `awareness_check`
- **Player surprise:**
  - GM narrates detection: "You spot movement ahead — [creature description_unseen]"
  - GM presents options: "You have the advantage. What do you do?"
  - Valid commands: `attack` (initiate combat with initiative bonus), `avoid` (skip encounter entirely, continue exploring), `sneak` (attempt to pass without engagement — Sneak check vs. awareness difficulty)
  - If player chooses `attack`: start combat with player getting a free first round (enemies don't act round 1)
  - If player chooses `avoid`: encounter skipped, narrate "You carefully circle around the threat."
  - If player chooses `sneak`: Sneak check. Success = avoid. Failure = combat starts at mutual awareness.
- **Mutual awareness:**
  - GM narrates: "[creature description_seen]. They've spotted you too."
  - Valid commands: `attack` (initiate combat, normal initiative), `flee` (immediate flee attempt, no combat), `talk` (if creature is intelligent — attempt communication, usually fails for beasts)
  - If player chooses `attack`: start combat with normal initiative
  - If player chooses `flee`: immediate flee roll (same mechanics as in-combat flee)
- **Enemy surprise:**
  - GM narrates: "[creature description_seen] — they were waiting for you!"
  - Combat starts immediately. Enemies get a free first round (player doesn't act round 1).
  - No choice to avoid — combat is forced.

Emit events:
- `combat.awareness_check` with result, skill check details
- `combat.start` with combatant list, awareness result, initiative order

**Tests:** `test_awareness.py`
- Unavoidable creature → always returns ENEMY_SURPRISE
- With mocked dice: player succeeds, creature fails → PLAYER_SURPRISE
- With mocked dice: both succeed → MUTUAL_AWARENESS
- With mocked dice: player fails, creature succeeds → ENEMY_SURPRISE
- Player surprise → `avoid` command skips encounter (no combat started)
- Player surprise → `attack` command starts combat with player free round
- Player surprise → `sneak` success → encounter avoided
- Player surprise → `sneak` failure → combat at mutual awareness
- Mutual awareness → `flee` triggers flee mechanic
- Enemy surprise → combat starts with enemy free round
- Correct skill selected for terrain (Survive for wilderness, Notice for ruins)

---

## Task 3.4: Combat Scene State & Turn System

**What:** The combat scene handler that manages initiative, turns, and the combat loop.

**Files:**
- `src/harsh_realm/gm/scenes/combat.py` — Combat scene handler (NEW)
- `src/harsh_realm/engine/combat.py` — Extend with turn management

**Deliverables:**

`CombatState` dataclass tracking active combat:
```python
@dataclass
class CombatState:
    combatants: list[Combatant]           # All entities in the fight
    initiative_order: list[str]           # Entity IDs sorted by initiative
    current_turn_index: int               # Whose turn it is
    round_number: int                     # Current round
    player_surprise: bool                 # Player gets free first round
    enemy_surprise: bool                  # Enemies get free first round
    veteran_luck_used: bool               # Warrior ability tracking
    first_aid_used: bool                  # Post-combat first aid tracking
    enemy_detail_revealed: bool           # Notice check passed at combat start
    fled: bool                            # Player has fled
    combat_over: bool                     # Combat has ended
```

`Combatant` dataclass:
```python
@dataclass
class Combatant:
    entity_id: str
    name: str
    display_name: str                     # "Wolf (1)", "Bandit"
    is_player: bool
    initiative: int
    hp: int
    max_hp: int
    ac: int
    attack_bonus: int
    damage_expr: str
    attack_description: str
    behavior: str                         # Enemy AI behavior tag
    alive: bool
```

`CombatScene` handler implementing `SceneHandler` protocol:
- Valid commands: `attack <target>`, `flee`, `use <item>`, `status`, `help`
- `get_prompt`: Display round number, whose turn it is, enemy status (vague or detailed based on Notice check)
- `handle_command`:
  - Processes player action
  - Then runs all enemy turns for this round
  - Checks for combat end conditions (all enemies dead, player dead, player fled)
  - Returns narration events for everything that happened

Combat initialization (called by Exploration scene when combat starts):
1. Generate combatants from encounter creature data
   - Roll HP for each enemy: HD × hp_per_hd (or roll HD × d8 for variety)
   - Number duplicates: "Wolf (1)", "Wolf (2)"
2. Roll initiative for all combatants: d8 + DEX mod
   - Apply surprise modifiers: if player surprise, enemies don't act round 1. If enemy surprise, player doesn't act round 1.
3. Perform Notice check for enemy detail:
   - Notice + WIS/INT vs. difficulty 8 (or creature-specific difficulty)
   - Success: show exact HP and AC for all enemies
   - Failure: show only vague descriptions
4. Display combat start narration:
   ```
   --- Combat Begins ---
   [Creature description_seen]
   
   Initiative order: You (12), Wolf (1) (8), Wolf (2) (6)
   
   [If Notice check passed]: Wolf (1): 5/5 HP, AC 13. Wolf (2): 4/5 HP, AC 13.
   [If Notice check failed]: Wolf (1) looks healthy. Wolf (2) looks healthy.
   
   --- Round 1 ---
   Your turn.
   ```

Turn prompt format:
```
--- Round N ---
Your turn. 
[Enemy status lines — vague or detailed]
[Available actions: attack <target>, flee, use <item>]
```

**Tests:** `test_combat_scene.py`
- Combat initializes with correct number of combatants
- Initiative order is sorted correctly (highest first)
- Player surprise: enemies skip round 1
- Enemy surprise: player skips round 1
- Turn advances correctly through initiative order
- Combat ends when all enemies are dead
- Combat ends when player flees
- Combat ends when player dies
- Notice check success reveals exact enemy stats
- Notice check failure shows vague descriptions
- Duplicate enemies are numbered correctly
- Invalid command in combat returns helpful error

---

## Task 3.5: Attack Resolution

**What:** Implement the XWN attack roll, damage calculation, and Warrior's Veteran's Luck ability.

**Files:**
- `src/harsh_realm/engine/combat.py` — Extend with attack resolution
- `src/harsh_realm/engine/damage.py` — Damage calculation (NEW)

**Deliverables:**

`AttackResolver` class (extension point):
- `resolve_attack(attacker: Combatant, target: Combatant, weapon: dict | None = None) -> AttackResult`
  1. Roll d20
  2. Calculate total: roll + attack_bonus + skill_modifier + attribute_modifier
  3. Compare vs. target AC
  4. Natural 1: always miss
  5. Natural 20: always hit
  6. Return AttackResult

`AttackResult` frozen dataclass:
```python
@dataclass(frozen=True)
class AttackResult:
    roll: int                      # d20 result
    total: int                     # roll + all modifiers
    target_ac: int
    hit: bool
    natural_1: bool
    natural_20: bool
    damage: DamageResult | None    # None if miss
    attacker_name: str
    target_name: str
    narration: str                 # Full narrative text for the chat log
```

`DamageResolver` class (extension point):
- `resolve_damage(attacker: Combatant, weapon_damage: str, is_warrior: bool, level: int) -> DamageResult`
  1. Roll weapon damage dice
  2. Add attribute modifier (STR for melee)
  3. If Warrior: add half level (round up) as bonus damage
  4. Minimum damage: 1 (if the attack hit, it does at least 1 damage)
  5. Subtract target armor DR if applicable (not standard XWN but extension point)
  6. Return DamageResult

`DamageResult` frozen dataclass:
```python
@dataclass(frozen=True)
class DamageResult:
    roll: int                      # Dice roll
    modifier: int                  # Attribute + class bonuses
    total: int                     # Final damage dealt
    weapon_expr: str               # "1d8+1"
```

Veteran's Luck (Warrior ability):
- Tracked in `CombatState.veteran_luck_used`
- When the player is hit (attack succeeds against them), if they are a Warrior and haven't used Luck this fight:
  - GM prompts: "You're about to take [X] damage. Use Veteran's Luck to negate this hit? (yes/no)"
  - If yes: attack is negated, no damage taken, ability marked as used
  - If no: damage applies normally
- Alternatively, when the player misses an attack, Warrior can use Luck to turn the miss into a hit:
  - GM prompts: "Your attack misses. Use Veteran's Luck to force a hit? (yes/no)"
  - If yes: attack becomes a hit, roll damage normally, ability marked as used
- Only one of these can be used per fight (negate OR force hit, not both)

Attack narration format:
```
You swing your sword at Wolf (1).
[Attack: d20(14) + 2 (AB) + 1 (Stab) + 1 (STR) = 18 vs. AC 13 — Hit!]
[Damage: d8(5) + 1 (STR) = 6]
Your blade bites deep into the wolf's flank.

Wolf (1) lunges and bites at you.
[Attack: d20(11) + 1 = 12 vs. AC 15 — Miss!]
The wolf snaps at empty air as you step aside.
```

Hit/miss flavor text:
- Create 3-5 hit narration variants and 3-5 miss narration variants per attack type (melee, bite, ranged)
- Select randomly to avoid repetition
- Store in `data/templates/combat_narration.yaml`

**Tests:** `test_attack.py`
- Natural 1 always misses regardless of bonuses
- Natural 20 always hits regardless of AC
- Successful hit calculates correct damage
- Warrior bonus damage adds half level (round up)
- Minimum damage is 1 on a hit
- Veteran's Luck: negate hit → no damage taken, ability marked used
- Veteran's Luck: force hit → miss becomes hit with damage
- Veteran's Luck: cannot use twice in same fight
- Non-Warrior cannot use Veteran's Luck
- Attack narration includes all mechanical details in brackets
- Hit/miss flavor text varies (test over 10 attacks, at least 2 unique flavor texts)

---

## Task 3.6: Enemy AI

**What:** Simple enemy behavior for combat turns. All enemies fight to the death in Milestone 3.

**Files:**
- `src/harsh_realm/engine/enemy_ai.py` — Enemy AI decision making (NEW)

**Deliverables:**

`EnemyAI` class:
- `choose_action(combatant: Combatant, combat_state: CombatState) -> EnemyAction`
  - For Milestone 3, all behavior tags resolve to the same action: attack the player character
  - The behavior tag is stored for future use (defensive enemies might prioritize different targets, ambush enemies might flee when outnumbered, etc.)
  - Returns an `EnemyAction` that the combat scene executes

`EnemyAction` frozen dataclass:
```python
@dataclass(frozen=True)
class EnemyAction:
    action_type: str               # "attack" (only option in M3)
    target_id: str                 # Target entity ID (always player in M3)
```

Enemy turn execution (in combat scene):
1. For each enemy in initiative order:
   - Skip if dead
   - Call `EnemyAI.choose_action()`
   - Resolve the action (attack roll → damage if hit)
   - Generate narration
2. After all enemy turns, check for player death (0 HP → Last Stand)

Future extension notes (document in code comments):
- `aggressive`: always attack nearest target (current default)
- `defensive`: attack only if attacked, otherwise guard
- `territorial`: attack if player is in their hex, don't pursue
- `ambush`: high initial damage, may flee if fight goes badly
- `pack`: coordinate attacks, focus fire on wounded targets

**Tests:** `test_enemy_ai.py`
- Enemy AI returns attack action targeting the player
- Dead enemies are skipped in turn order
- All enemies in a multi-enemy encounter take their turns
- Enemy attack uses correct stats from creature data
- Enemy attack follows same resolution as player attacks (d20 + AB vs AC)

---

## Task 3.7: Flee Mechanics

**What:** Flee always succeeds (player escapes combat), but clean escape requires a skill check. Failure incurs consequences.

**Files:**
- `src/harsh_realm/engine/combat.py` — Extend with flee resolution

**Deliverables:**

`FleeResolver` class:
- `resolve_flee(character: Character, enemies: list[CreatureData], current_hex: HexCoord, db: WorldDatabase) -> FleeResult`
  1. Flee always succeeds — player exits combat regardless of roll
  2. Determine flee difficulty: highest `flee_difficulty` among living enemies
  3. Roll skill check: Exert + STR (physical escape) or Sneak + DEX (stealthy escape) — player's choice or system picks best
  4. **Clean escape (check succeeds):**
     - No consequences
     - Player returns to previous hex
     - GM: "You disengage and retreat to safety."
  5. **Messy escape (check fails):**
     - One or more consequences from the following (roll or pick based on margin of failure):
       - Take a parting blow: one enemy gets a free attack (resolved normally)
       - Drop an item: a random non-essential equipped item is left behind at the combat hex
       - Wrong direction: player ends up in a random adjacent passable hex instead of their previous hex
     - GM narrates the consequence
  6. Combat ends after flee resolution

`FleeResult` frozen dataclass:
```python
@dataclass(frozen=True)
class FleeResult:
    success: bool                  # Always True (flee always works)
    clean: bool                    # Whether the skill check passed
    skill_check: SkillCheckResult  # The flee skill check details
    consequence: str | None        # "parting_blow", "dropped_item", "wrong_direction", or None
    destination: HexCoord          # Where the player ends up
    damage_taken: int              # From parting blow (0 if clean)
    item_lost: str | None          # Item dropped (None if clean)
    narration: str                 # Full narrative text
```

Combat scene integration:
- On `flee` command: resolve flee, end combat, transition back to Exploration at destination hex
- Emit events: `combat.flee` with flee result details, `combat.end` with outcome "fled"

**Tests:** `test_flee.py`
- Flee always ends combat (combat_over = True, fled = True)
- Clean escape: no damage, no item loss, returns to previous hex
- Messy escape — parting blow: player takes damage from one enemy attack
- Messy escape — dropped item: item removed from character, placed at combat hex
- Messy escape — wrong direction: player location is a random adjacent hex
- Skill check uses correct skill and attribute
- Flee difficulty matches highest enemy flee_difficulty
- Narration describes what happened

---

## Task 3.8: Death, Last Stand & Respawn

**What:** When the player hits 0 HP, trigger Last Stand (one final action), then death and respawn.

**Files:**
- `src/harsh_realm/engine/combat.py` — Extend with death handling
- `src/harsh_realm/gm/scenes/combat.py` — Integrate Last Stand into combat flow
- `src/harsh_realm/gm/scenes/respawn.py` — Respawn scene handler (NEW)

**Deliverables:**

Last Stand trigger (in combat scene):
- After any damage that reduces player HP to 0 or below:
  1. Set HP to 0 (don't go negative for display purposes)
  2. GM narrates: "You stagger, vision fading. One last chance — what do you do?"
  3. Valid commands change to: `attack <target>` (at -2 penalty to attack roll), `use <item>`, `flee` (auto-messy, no skill check)
  4. Resolve the final action:
     - Attack: apply -2 penalty, resolve normally. If this kills the last enemy, player survives at 0 HP (stabilized, not dead — GM narrates the dramatic finish)
     - Use item: if healing item and it brings HP above 0, player stabilizes and combat continues. If not enough healing, player dies after using the item.
     - Flee: auto-success but always messy (parting blow + dropped item). If parting blow would "kill" again, ignore it — you're already at 0. You escape but are dying.
  5. If HP is still 0 or below after final action: player dies.

Death handling:
- Emit `character.death` event with death location, killer, circumstances
- Mark death location on the hex map (add "death_marker" feature with dropped items list)
- Transition to Respawn scene

`RespawnScene` handler:
- GM narrates death and blackout
- GM presents options:
  ```
  Darkness takes you...
  
  1. Respawn at [nearest settlement name] (lose [item], lose [X] XP)
  2. Create a new character
  
  What do you choose?
  ```
- On respawn (option 1):
  1. Set location to nearest settlement hex
  2. Restore HP to 50% of max
  3. Remove one equipped item (prefer weapon → armor → other gear). Create an item entity at the death hex.
  4. Reduce XP by 15% of current XP (round down, minimum 0, never causes de-level)
  5. Update character in database
  6. Transition to Exploration scene
  7. GM: "You wake in [settlement]. A traveler found you and dragged you to safety. Your [lost item] was left behind near [death location coordinates]."
- On new character (option 2):
  1. Transition to CharacterCreation scene
  2. The world persists — new character enters the same world
  3. Old character entity marked as dead in database (preserved for world history)

Death marker on map:
- Death hexes get a feature: `"death_marker"` with data including dropped items
- Items at death location can be retrieved in future visits: `take <item>` at the death hex
- Death marker visible on hex map (distinct icon)

**Tests:** `test_death.py`
- Player at 0 HP triggers Last Stand prompt
- Last Stand attack resolves with -2 penalty
- Last Stand use healing item above 0 HP → player stabilizes, combat continues
- Last Stand use healing item still at 0 → player dies
- Last Stand flee → messy escape, player dying
- Death → respawn at nearest settlement
- Respawn: HP at 50% of max
- Respawn: one item removed from character, placed at death hex
- Respawn: XP reduced by 15%, no de-level
- Death marker feature added to death hex
- Items at death hex retrievable with `take` command
- New character option transitions to CharacterCreation
- Old character preserved in database as dead entity

---

## Task 3.9: Loot System

**What:** On combat victory, roll for loot from enemy-type loot tables. Harvestable materials require a Survive check.

**Files:**
- `src/harsh_realm/engine/loot.py` — Loot generation (NEW)
- `data/tables/loot/` — Loot table YAML files (NEW directory)

**Deliverables:**

`LootGenerator` class:
- `generate_combat_loot(enemies: list[CreatureData], character: Character) -> LootResult`
  1. For each defeated enemy:
     a. If enemy has `loot_table` reference: roll on the table
     b. If enemy has `harvestable`: offer harvest opportunity
  2. Combine all results into a single `LootResult`

- `roll_loot(table_id: str) -> list[LootItem]`
  - Roll on loot table. May produce 0-N items depending on table structure.
  - Each result is a `LootItem` with name, description, value, and type.

- `attempt_harvest(creature: CreatureData, character: Character) -> HarvestResult`
  1. GM prompts: "The [creature] has harvestable [material]. Attempt to harvest? (yes/no)"
  2. If yes: Survive skill check vs. creature's harvest difficulty
  3. Success: material added to player's discoverable items
  4. Failure: material ruined in the attempt

`LootItem` frozen dataclass:
```python
@dataclass(frozen=True)
class LootItem:
    name: str
    description: str
    item_type: str                 # "weapon", "armor", "consumable", "material", "junk", "pretech", "currency"
    value: int                     # Base value in coin
    data: dict                     # Additional item data (weapon stats, healing amount, etc.)
```

`LootResult` frozen dataclass:
```python
@dataclass(frozen=True)
class LootResult:
    items: list[LootItem]
    currency: int                  # Total coin found
    harvestable: list[dict]        # Pending harvest opportunities
    narration: str
```

Post-combat loot flow (in combat scene, after victory):
1. GM: "The fight is over."
2. Run `generate_combat_loot()` for all defeated enemies
3. Display found items and currency in the chat
4. For each harvestable opportunity, prompt player
5. Items are placed at the current hex (or directly in player's inventory if inventory system exists — for M3, place at hex with `take` command available)

**Loot table YAML files (stubs — developer populates from source material):**

| File | Content | Notes |
|---|---|---|
| `data/tables/loot/humanoid_common.yaml` | Bandit/scavenger loot | Coin, crude weapons, rations, clothing scraps |
| `data/tables/loot/humanoid_military.yaml` | Soldier/guard loot | Better weapons, armor pieces, orders/documents |
| `data/tables/loot/beast.yaml` | Animal drops | Mostly nothing, occasional useful materials |
| `data/tables/loot/construct.yaml` | Automaton/gargoyle loot | Pretech components, metal scraps, energy cells |
| `data/tables/loot/undead.yaml` | Zombie/shade loot | Items from former life, cursed objects, coin |
| `data/tables/loot/mythical.yaml` | Dragon/basilisk/etc. loot | Rare materials, significant treasure, pretech |
| `data/tables/loot/pocket_litter.yaml` | Generic minor loot | Buttons, string, bent nails, scraps of cloth |

Each table should have entries weighted across tiers:
- Junk/nothing (weight 4-5): Worthless items, flavor text
- Common (weight 3): Basic useful items, small coin amounts
- Uncommon (weight 2): Good equipment, moderate coin, useful consumables
- Rare (weight 1): Pretech fragments, valuable items, significant coin

**Tests:** `test_loot.py`
- Combat victory triggers loot generation
- Enemy with loot_table produces items from that table
- Enemy without loot_table produces nothing
- Harvest prompt appears for harvestable creatures
- Successful harvest check adds material
- Failed harvest check ruins material
- Loot narration lists all found items
- Currency is summed correctly across multiple enemies
- Items placed at hex are retrievable

---

## Task 3.10: XP Awards & Level Up Detection

**What:** Award XP after combat based on enemy HD. Detect level-up threshold and handle advancement.

**Files:**
- `src/harsh_realm/engine/advancement.py` — XP and leveling (NEW or extend)
- `src/harsh_realm/gm/scenes/combat.py` — Integrate XP awards into post-combat flow

**Deliverables:**

`AdvancementSystem` class:
- `award_combat_xp(character: Character, defeated_enemies: list[CreatureData]) -> XPAwardResult`
  - Sum XP values of all defeated enemies
  - Add XP to character
  - Check if XP crosses next level threshold
  - Return result with XP gained, new total, and whether level-up occurred

- `check_level_up(character: Character) -> bool`
  - Compare current XP to XP needed for next level
  - Return True if threshold crossed

- `apply_level_up(character: Character) -> LevelUpResult`
  - Increment level
  - Roll new HD for HP gain (class-appropriate die + CON modifier, minimum 1)
  - Update attack bonus per class progression
  - Update save targets
  - Award skill points per class rules
  - GM narrates: "You've reached Level [N]! HP increased by [X]. You have [N] skill points to allocate."
  - Skill point allocation can be immediate (GM prompts for each point) or deferred (player allocates via `status` or dedicated command later)

XP-to-level table:
```python
XP_TABLE = {
    1: 0,
    2: 1500,       # [PLACEHOLDER — verify from source]
    3: 3000,       # [PLACEHOLDER]
    4: 6000,       # [PLACEHOLDER]
    5: 12000,      # [PLACEHOLDER]
    6: 24000,      # [PLACEHOLDER]
    7: 48000,      # [PLACEHOLDER]
    8: 96000,      # [PLACEHOLDER]
    9: 192000,     # [PLACEHOLDER]
    10: 384000,    # [PLACEHOLDER]
}
```

Post-combat XP flow:
1. After loot, before returning to Exploration
2. GM: "You earned [X] XP from the encounter. (Total: [Y] / [Z] for level [N+1])"
3. If level-up triggered: run level-up flow immediately
4. Emit `character.xp_gained` event (updates frontend sidebar)
5. Emit `character.level_up` event if applicable

**Tests:** `test_advancement.py`
- XP awards sum correctly across multiple enemies
- XP added to character total
- Level-up detected when threshold crossed
- Level-up: HP increases by at least 1
- Level-up: level increments by 1
- Level-up: skill points awarded
- XP table values are correct (once placeholders filled)
- No level-up when XP is below threshold
- XP gained event emitted with correct data

---

## Task 3.11: Healing System

**What:** Post-combat first aid, rest recovery, consumable healing items, and town healers.

**Files:**
- `src/harsh_realm/engine/healing.py` — Healing mechanics (NEW)
- `src/harsh_realm/gm/scenes/exploration.py` — Add `rest` command and healer interaction
- `src/harsh_realm/gm/scenes/combat.py` — Integrate post-combat first aid

**Deliverables:**

`HealingSystem` class:
- `first_aid(character: Character) -> HealingResult`
  - Roll Heal skill check vs. difficulty 8 (standard)
  - Success: restore 1d6 + Heal skill level HP (minimum 1)
  - Failure: no healing, but no harm
  - Can only be used once per combat encounter (tracked in CombatState)
  - GM: "You tend to your wounds. [Heal check: rolled X + Y = Z vs. 8 — Success/Failure] [Restored N HP / No improvement.]"

- `rest(character: Character, ticks: int) -> HealingResult`
  - Restore 1 HP per rest period
  - Rest period costs a configurable number of ticks (default: 10 ticks per rest, representing ~1 hour)
  - Full rest (longer): restore level + CON modifier HP per full rest period (costs more ticks)
  - Emit `world.tick` events for time passed during rest (may trigger world events in Milestone 6)
  - GM: "You rest for a while... [Restored N HP. Time passes.]"
  - [PLACEHOLDER — verify XWN natural healing rates from source books]

- `use_healing_item(character: Character, item: LootItem) -> HealingResult`
  - Apply item's healing effect (stored in item data: `{"healing": "1d6+1"}`)
  - Remove item from character's inventory/hex items
  - Can be used in combat (via `use <item>`) or outside combat
  - GM: "You use [item name]. [Restored N HP.]"

- `town_healer(character: Character, healer_npc: Entity, db: WorldDatabase) -> HealingResult`
  - Restore full HP
  - Cost: configurable per settlement size. [PLACEHOLDER — 10 coin per HP healed? Flat fee? Depends on setting economy]
  - If player can't afford: "The healer shakes their head. 'I need payment. Come back when you have coin.'"
  - If affordable: deduct coin, restore HP
  - GM: "The healer works over your injuries. [Fully healed. Paid X coin.]"

`HealingResult` frozen dataclass:
```python
@dataclass(frozen=True)
class HealingResult:
    healed: bool
    hp_restored: int
    new_hp: int
    max_hp: int
    method: str                    # "first_aid", "rest", "item", "healer"
    cost: int                      # Coin cost (0 for non-healer methods)
    skill_check: SkillCheckResult | None
    narration: str
```

Exploration scene additions:
- `rest` command:
  - Available everywhere
  - GM: "You find a spot to rest..."
  - Run healing, advance ticks
  - Check for random encounter during rest (low probability, ~10%)
  - If encounter: interrupt rest, trigger encounter check

- Healer interaction:
  - When talking to a healer NPC in a settlement: `talk to <healer>`
  - Healer offers healing if player HP < max HP
  - GM: "[Healer name] examines you. 'You're hurt. I can fix you up for [X] coin. You have [Y] coin.' (yes/no)"
  - `yes` → heal and pay. `no` → decline.

Post-combat first aid:
- After combat ends in victory and loot is resolved:
  - If player HP < max HP:
    - GM: "You take a moment to tend to your wounds."
    - Auto-attempt first aid (Heal check)
    - Display result

**Tests:** `test_healing.py`
- First aid success restores HP (1d6 + skill)
- First aid failure restores 0 HP
- First aid usable once per combat only
- Rest restores HP based on time spent
- Rest advances world ticks
- Healing item restores specified HP
- Healing item consumed after use
- Town healer restores full HP for correct cost
- Town healer refuses if player can't afford
- HP never exceeds max_hp after healing
- Rest can be interrupted by encounter

---

## Task 3.12: Item Use in Combat

**What:** Enable `use <item>` command during combat for consumable items (healing herbs, bandages, etc.).

**Files:**
- `src/harsh_realm/gm/scenes/combat.py` — Add `use` command handling
- `src/harsh_realm/engine/items.py` — Item effect resolution (NEW)

**Deliverables:**

`ItemSystem` class:
- `use_item(character: Character, item_name: str, context: str, db: WorldDatabase) -> ItemUseResult`
  1. Find matching item in character's available items (at current hex or in inventory)
  2. Validate item is usable in current context ("combat" or "exploration")
  3. Apply item effect:
     - Healing items: restore HP
     - Other consumable effects: apply (future expansion)
  4. Remove item (consumed)
  5. Return result

`ItemUseResult` frozen dataclass:
```python
@dataclass(frozen=True)
class ItemUseResult:
    success: bool
    item_name: str
    effect: str                    # "healed 5 HP", "applied poison to weapon", etc.
    narration: str
    error: str | None              # "Item not found", "Can't use that here", etc.
```

Item matching:
- Case-insensitive partial matching against item names
- If multiple matches: "Which item did you mean? [list matches]"
- If no matches: "You don't have anything called '[name]'."

Combat integration:
- `use <item>` consumes the player's action for the turn
- Healing items restore HP immediately
- After item use, enemy turns proceed as normal

**Tests:** `test_item_use.py`
- Use healing item in combat → HP restored, item consumed
- Use healing item outside combat → HP restored, item consumed
- Use non-existent item → error message
- Partial name matching works
- Multiple matches prompt disambiguation
- Item use in combat consumes the turn
- Item use in Last Stand can save the character if it heals above 0

---

## Task 3.13: Combat Integration & Full Flow

**What:** Wire everything together. Verify the full combat flow from encounter through resolution.

**Files:**
- Various — integration tests and fixes

**Deliverables:**

Integration test covering:
1. Explore until hostile encounter triggers
2. Awareness check runs → appropriate outcome
3. If player surprise: choose to engage → combat starts with free round
4. Combat: attack an enemy → hit/miss narration with dice details
5. Enemy turn: enemy attacks player → hit/miss narration
6. Warrior's Veteran's Luck: prompted on hit, negates damage
7. Fight continues for multiple rounds
8. All enemies defeated → combat victory
9. Loot generated and displayed
10. Harvestable materials offered if applicable
11. First aid attempted automatically
12. XP awarded, level-up triggered if applicable
13. Return to Exploration scene at same hex

Additional integration tests:
14. Enemy surprise → player skips round 1 → takes damage → fights back
15. Flee in combat → skill check → clean or messy escape → return to exploration
16. Death: player HP reaches 0 → Last Stand prompt → attack kills last enemy → survive at 0 HP
17. Death: player HP reaches 0 → Last Stand fails → death → respawn at settlement → lost items at death hex → return to death hex → retrieve items
18. Multiple enemies: target specific enemy by number, enemies numbered correctly
19. Use healing item mid-combat → HP restored → combat continues
20. `status` during combat shows current HP, enemy status

Verify frontend updates:
21. Status sidebar HP updates during combat
22. Combat narration displays correctly in chat panel
23. On death/respawn: map updates to show new position at settlement
24. Death marker visible on map at death location

Update `CLAUDE.md` with "Milestone 3 complete" and note any deviations.

---

## Dependency Graph

```
Task 3.1 (Rules docs — developer fills) ←── prerequisite for all
  ↓
Task 3.2 (Creature data model + bestiary)
  ↓
Task 3.3 (Awareness check) 
  ↓
Task 3.4 (Combat scene + turn system)
  ↓
Task 3.5 (Attack resolution + Warrior ability) ← core combat loop
  ↓
Task 3.6 (Enemy AI) 
  ↓
Task 3.7 (Flee mechanics)
  ↓
Task 3.8 (Death, Last Stand, respawn)
  ↓
Task 3.9 (Loot system) ← post-combat
  ↓
Task 3.10 (XP + level up) ← post-combat
  ↓
Task 3.11 (Healing system) ← post-combat + exploration
  ↓
Task 3.12 (Item use in combat)
  ↓
Task 3.13 (Integration) ← everything connected
```

Tasks 3.5-3.8 form the core combat loop and should be built sequentially. Tasks 3.9-3.12 are post-combat systems that can be developed in parallel once the core loop works.

---

## Content Stubs Needed

| File | Content | Stub Size | Notes |
|---|---|---|---|
| `data/creatures/beasts.yaml` | Wolves, buffalo, giant crabs/scorpions/insects | 5 creatures | Developer fills stats |
| `data/creatures/humanoids.yaml` | Bandits, scavengers, kobolds, goblins, lizardfolk, minotaurs | 6 creatures | Developer fills stats |
| `data/creatures/undead.yaml` | Zombies, shades | 2 creatures | Developer fills stats |
| `data/creatures/constructs.yaml` | Ancient automatons, gargoyles | 2 creatures | Developer fills stats |
| `data/creatures/mythical.yaml` | Dragon template, basilisks, sphinxes, griffons | 4 creatures + template | Developer fills stats |
| `data/creatures/elemental.yaml` | Elemental spirits (4 variants) | 4 creatures | Developer fills stats |
| `data/tables/loot/humanoid_common.yaml` | Bandit/scavenger drops | 8 entries | Developer populates |
| `data/tables/loot/humanoid_military.yaml` | Soldier/guard drops | 8 entries | Developer populates |
| `data/tables/loot/beast.yaml` | Animal drops | 6 entries | Developer populates |
| `data/tables/loot/construct.yaml` | Automaton/gargoyle drops | 6 entries | Pretech focus |
| `data/tables/loot/undead.yaml` | Undead drops | 6 entries | Developer populates |
| `data/tables/loot/mythical.yaml` | Dragon/rare drops | 6 entries | High value |
| `data/tables/loot/pocket_litter.yaml` | Generic minor loot | 10 entries | Flavor items |
| `data/templates/combat_narration.yaml` | Hit/miss/flee flavor text | 30+ entries | 5 per attack type × hit/miss |
| `docs/rules_reference/combat.md` | XWN combat rules | Complete | Developer fills PLACEHOLDERs |
| `docs/rules_reference/weapons_armor.md` | Equipment stats | Complete | Developer fills from source |
| `docs/rules_reference/death.md` | Death/respawn rules | Complete | Defined in this spec |
| `docs/house_rules/last_stand.md` | Last Stand house rule | Complete | Defined in this spec |

---

## Notes for the Coding Agent

- Read `docs/rules_reference/combat.md` and `docs/rules_reference/weapons_armor.md` before implementing. Many values will be `[PLACEHOLDER]` — use reasonable defaults and mark in code comments.
- The combat scene is the most complex scene handler so far. Test each component (attack resolution, flee, death) independently before integrating into the scene.
- Veteran's Luck interrupts the normal flow — it requires prompting the player mid-resolution. Implement this as a special state in the combat scene that pauses for player input.
- Last Stand similarly interrupts normal flow — at 0 HP, instead of immediately dying, the scene prompts for one more action.
- Enemy AI is deliberately simple in this milestone. Build the extension point but don't over-engineer the decision logic.
- Combat narration should feel cinematic despite being text. Vary the flavor text. Don't use the same "you swing your sword" every round.
- Loot tables should feel setting-appropriate. Bandits have coin and crude weapons. Automatons have pretech fragments and metal. Beasts have harvestable parts.
- The respawn system needs to handle edge cases: what if the nearest settlement was destroyed? (Shouldn't happen in M3 but design for it.) Fall back to starting settlement or any known settlement.
- Death markers on the map create a strong motivation to return for gear. Make sure items at the death hex persist correctly and are retrievable.
- After completing all tasks, update `CLAUDE.md` with "Milestone 3 complete" and note any deviations.
