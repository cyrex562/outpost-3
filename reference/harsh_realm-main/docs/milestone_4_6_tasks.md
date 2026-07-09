# Milestone 4.6: Combat Completion — Task Specification

> **Goal:** Complete the XWN combat system. This milestone introduces the
> canonical item registry (all items get human-readable namespaced IDs),
> refactors all existing inline item definitions to use those IDs, adds
> weapons.yaml with full stat blocks, implements shock damage, range bands,
> differentiated saving throw types, ammo tracking with auto-weapon-switching,
> and ensures equipped weapon stats drive PC attack/damage rolls.
> It also enhances the ChatLog with CRPG-style structured combat messages.
> **Estimated time:** 5–7 days (AI-assisted)
> **Prerequisite:** M4.9 complete. Read CLAUDE.md, AGENTS.md, and
> `docs/rules_reference/combat.md` before starting.

## Success Criteria

When this milestone is complete:

1. `pytest` passes with all tests green.
2. Every item in the game (weapons, armor, gear, ammo, consumables, pretech)
   has a unique human-readable ID in the format `category.slug`
   (e.g. `weapon.short_sword`, `armor.chain_mail`, `ammo.crossbow_bolt`).
3. `equipment_kits.yaml`, all loot tables, and all shop YAML files reference
   item IDs only — no inline item stat definitions.
4. `data/items/` contains canonical YAML files for all item categories.
   An `ItemRegistry` service loads and validates all items at startup.
5. The PC's attack roll uses the equipped weapon's damage die and attribute.
   Unarmed (no weapon equipped) uses `1d2 + STR mod`.
6. Shock damage: when a melee attack misses but the target's AC ≤ the
   weapon's shock AC threshold, `shock_damage + STR mod` is dealt
   automatically. If the weapon has no shock value, no shock is dealt.
7. Range bands (none / melee / near / far) are tracked per combatant.
   Melee weapons require melee range. Ranged weapons require near or far.
   Moving in/out of range costs no action in v1 (movement is free).
8. Ammo is consumed on each ranged attack. When a ranged weapon runs out
   of ammo, the PC automatically switches to their equipped melee weapon
   (or unarmed if none). A `[No ammo]` message is shown.
9. Saving throws are differentiated by type: Physical (STR/CON threats),
   Evasion (DEX threats), Mental (WIS/CHA threats), Luck (other). Each
   save type has its own base target from `classes.yaml`. Items can buff
   individual save types via an extensible `save_bonuses` dict field.
10. The ChatLog shows structured combat messages: each attack displays
    roll + modifier + total, target AC, hit/miss, damage dealt, and shock
    damage if applicable. Each save shows type, roll, target, pass/fail.
11. The bot (M4.8) first-suite combat goals pass with the new combat system.
12. mutmut ≥85% on all new and modified combat modules.

---

## Task 4.6.1: Audit Existing Save Infrastructure

> **What:** Examine what currently exists for saving throws, document it,
> and decide extend vs. replace.
> **Estimated time:** 1 hour

**Files to examine:**
- `src/harsh_realm/engine/` — look for save-related functions or classes
- `src/harsh_realm/models/character.py` — look for save fields
- `data/classes.yaml` — examine existing saving_throw structure
- Any test files referencing saves

**Produce a brief written audit** (add to `docs/design/save_audit.md`):
- What save infrastructure exists (model fields, engine functions, tests)
- Current save target structure in classes.yaml
- Whether it can be extended or needs replacement
- Recommended approach

**Based on audit findings:** proceed with either extending or replacing
per the audit recommendation. The tasks below assume the most likely case
(single save target per class needs extension to 4 types). Adjust if
the audit shows otherwise.

**Acceptance:** `docs/design/save_audit.md` exists. Approach documented
before any code changes.

---

## Task 4.6.2: Item Registry

> **What:** Create the canonical item data layer. Every item gets a unique
> namespaced ID. All existing inline item definitions are migrated.
> **Estimated time:** 4 hours

### 4.6.2a — Item YAML Files

Create `data/items/` directory with these files:

**`data/items/weapons.yaml`**

Each entry:
```yaml
- id: weapon.short_sword
  name: "Short Sword"
  damage: "1d6"
  damage_type: melee
  attribute: str         # primary attack attribute
  shock_damage: 2        # flat shock damage (before STR mod)
  shock_ac_threshold: 13 # target AC must be ≤ this to deal shock
  range_band: melee      # melee | near | far
  enc: 1
  cost: 30
  tags: [melee, blade]

- id: weapon.longsword
  name: "Longsword"
  damage: "1d8"
  damage_type: melee
  attribute: str
  shock_damage: 2
  shock_ac_threshold: 15
  range_band: melee
  enc: 1
  cost: 60
  tags: [melee, blade, two_hand_optional]

- id: weapon.knife
  name: "Knife"
  damage: "1d4"
  damage_type: melee
  attribute: str
  shock_damage: 1
  shock_ac_threshold: 10
  range_band: melee
  enc: 0
  cost: 5
  tags: [melee, blade, thrown]

- id: weapon.short_bow
  name: "Short Bow"
  damage: "1d6"
  damage_type: ranged
  attribute: dex
  shock_damage: 0
  shock_ac_threshold: 0
  range_band: near        # effective at near; -2 penalty at far
  ammo_type: ammo.arrow
  enc: 1
  cost: 25
  tags: [ranged, bow]

- id: weapon.hand_crossbow
  name: "Hand Crossbow"
  damage: "1d4"
  damage_type: ranged
  attribute: dex
  shock_damage: 0
  shock_ac_threshold: 0
  range_band: near
  ammo_type: ammo.bolt
  enc: 1
  cost: 30
  tags: [ranged, crossbow]
```

Add all weapons referenced in `equipment_kits.yaml` and `creatures/*.yaml`.
Creature attack weapons (punch, bite equivalents) get IDs too but are
tagged `natural` and have no cost.

**`data/items/armor.yaml`**

```yaml
- id: armor.none
  name: "No Armor"
  ac: 10
  enc: 0
  cost: 0
  tags: []

- id: armor.leather
  name: "Leather Armor"
  ac: 13
  enc: 1
  cost: 50
  tags: [light]

- id: armor.chain_mail
  name: "Chain Mail"
  ac: 14
  enc: 2
  cost: 100
  tags: [medium]

- id: armor.shield
  name: "Shield"
  ac_bonus: 1   # adds to equipped armor AC, not base
  enc: 1
  cost: 20
  tags: [shield]
```

**`data/items/ammo.yaml`**

```yaml
- id: ammo.arrow
  name: "Arrow"
  weapon_tags: [bow]
  enc_per_20: 1   # 20 arrows = 1 enc slot
  cost_per_20: 5

- id: ammo.bolt
  name: "Crossbow Bolt"
  weapon_tags: [crossbow]
  enc_per_20: 1
  cost_per_20: 5
```

**`data/items/gear.yaml`**

```yaml
- id: gear.backpack
  name: "Backpack"
  enc: 0
  cost: 5

- id: gear.rope_50ft
  name: "Rope (50 ft)"
  enc: 1
  cost: 5

- id: gear.tinderbox
  name: "Tinderbox"
  enc: 0
  cost: 2

- id: gear.tools_basic
  name: "Tools (basic set)"
  enc: 1
  cost: 15
```

**`data/items/consumables.yaml`**

```yaml
- id: consumable.rations
  name: "Rations (3 days)"
  enc: 1
  cost: 3
  effect: {type: food, days: 3}

- id: consumable.pretech_medkit
  name: "Pretech Medkit"
  enc: 0
  cost: 150
  effect: {type: heal, dice: "2d6"}

- id: consumable.pretech_medpack
  name: "Pretech Medpack"
  enc: 0
  cost: 400
  effect: {type: heal, dice: "3d6+3"}
```

**`data/items/pretech.yaml`**

```yaml
- id: pretech.energy_cell
  name: "Energy Cell"
  enc: 0
  cost: 100
  power_charges: 5

- id: pretech.circuit_board
  name: "Pretech Circuit Board"
  enc: 0
  cost: 75

- id: pretech.data_chip
  name: "Pretech Data Chip"
  enc: 0
  cost: 50
```

### 4.6.2b — ItemRegistry Service

**File:** `src/harsh_realm/engine/item_registry.py`

```python
class ItemRegistry:
    """
    Loads all items from data/items/*.yaml at startup.
    Provides lookup by item ID.
    Validates that all IDs are unique across all files.
    """

    def load(self, data_dir: Path) -> None: ...
    def get(self, item_id: str) -> ItemData | None: ...
    def get_or_raise(self, item_id: str) -> ItemData: ...
    def all_items(self) -> list[ItemData]: ...
    def items_by_tag(self, tag: str) -> list[ItemData]: ...
```

**`models/item.py`** — Pydantic model:
```python
class ItemData(BaseModel):
    id: str                           # e.g. "weapon.short_sword"
    name: str
    category: str                     # derived from id prefix
    enc: int | float
    cost: int
    tags: list[str] = []
    # weapon-specific (optional)
    damage: str | None = None
    damage_type: str | None = None
    attribute: str | None = None
    shock_damage: int = 0
    shock_ac_threshold: int = 0
    range_band: str | None = None
    ammo_type: str | None = None
    # armor-specific
    ac: int | None = None
    ac_bonus: int | None = None
    # ammo-specific
    enc_per_20: int | None = None
    cost_per_20: int | None = None
    weapon_tags: list[str] = []
    # consumable-specific
    effect: dict | None = None
    # pretech-specific
    power_charges: int | None = None
```

Register `ItemRegistry` as a singleton loaded at world startup
(same pattern as `AdminService`).

### 4.6.2c — Refactor Existing Item References

**`data/equipment_kits.yaml`:** Replace all inline item stat blocks with
item ID references:
```yaml
# Before
- id: short_sword
  name: "Short Sword"
  type: melee_weapon
  damage: "1d6"
  enc: 1

# After
- item_id: weapon.short_sword
  quantity: 1
```

**`data/tables/loot/*.yaml`:** Replace inline item definitions with
item ID references where the item maps to a canonical item. Narrative-only
items (e.g. "a folded map fragment") that have no mechanical stats can
remain as text results. Mechanically significant items (weapons, armor,
consumables, pretech) must reference IDs.

**`data/creatures/*.yaml`:** `attack_skill` field remains as-is (punch/stab/shoot
skill reference, not item ID). Add `natural_weapon_id` field pointing to
a natural weapon entry in weapons.yaml (e.g. `weapon.natural_bite`,
`weapon.natural_claws`). These natural weapons have damage and shock but
no cost.

**Tests:**
- Unit: `ItemRegistry.load()` finds all items across all files
- Unit: duplicate ID raises `ValueError` on load
- Unit: `get("weapon.short_sword")` returns correct name and damage
- Unit: `get("nonexistent.item")` returns None
- Unit: `get_or_raise("nonexistent.item")` raises `KeyError`
- Unit: kit items all resolve to valid item IDs via registry
- Unit: loot table items that reference IDs all resolve via registry
- Property: all item IDs match pattern `^[a-z]+\.[a-z0-9_]+$`
- Property: all items have enc ≥ 0 and cost ≥ 0

**Acceptance:** ItemRegistry loads without errors. All kit, loot, and shop
YAML files reference item IDs that resolve in the registry.

---

## Task 4.6.3: Equipped Weapon Resolution

> **What:** PC attack rolls must use equipped weapon stats. Unequipped PC
> uses fists (1d2+STR mod, no shock).
> **Estimated time:** 2 hours

**Audit finding:** Current combat system ignores equipped weapon.

**File:** `src/harsh_realm/engine/combat.py` (or wherever attack resolution lives)

**Changes:**
1. When resolving a PC attack, query the character's equipped weapon slot
   from the `entities` or `characters` table.
2. If equipped weapon is ranged and PC is not at required range band → block
   attack, suggest movement or melee. (Range band tracking added in 4.6.5.)
3. Look up the weapon in `ItemRegistry`. Use `weapon.damage` for damage roll,
   `weapon.attribute` to determine attack modifier (STR for melee, DEX for ranged).
4. If no weapon equipped: use `1d2` damage, STR modifier, no shock.
5. Enemy attacks continue to use creature statblock damage (creatures don't
   use the item registry for their natural attacks).

**Tests:**
- Unit: PC with short_sword equipped → attack uses 1d6 damage
- Unit: PC with no weapon → attack uses 1d2 damage
- Unit: PC with short_bow but no ammo → attack blocked, melee fallback triggered
- Unit: enemy attack uses creature statblock, not item registry

**Acceptance:** Character's equipped weapon visibly affects attack output
in combat log.

---

## Task 4.6.4: Shock Damage

> **What:** Implement WWN shock damage mechanic.
> **Estimated time:** 2 hours

**Rule:** When a melee attack misses (attack roll + modifiers < target AC),
check if the target's AC ≤ the weapon's `shock_ac_threshold`. If yes,
deal `shock_damage + attacker STR modifier` as automatic damage regardless
of the miss.

Shock damage applies to:
- PC's melee attacks against enemies
- Enemy melee attacks against PC (use creature's natural weapon shock values
  if defined; most creatures have shock_damage: 0 by default)

**File:** `src/harsh_realm/engine/combat.py`

Add `resolve_shock(attacker, weapon: ItemData, target_ac: int) -> int`:
```python
def resolve_shock(attacker, weapon: ItemData, target_ac: int) -> int:
    """
    Returns shock damage dealt on a miss, or 0 if no shock applies.
    """
    if weapon.shock_damage == 0:
        return 0
    if target_ac > weapon.shock_ac_threshold:
        return 0
    str_mod = attacker.str_modifier  # (STR - 10) // 2
    return max(0, weapon.shock_damage + str_mod)
```

On a miss, call `resolve_shock()`. If result > 0, deal that damage and
emit a combat event:
```python
{"type": "combat.shock", "attacker": name, "target": name,
 "shock_damage": int, "weapon": item_id}
```

**Tests:**
- Unit: attack misses, target AC 12 ≤ threshold 13 → shock dealt
- Unit: attack misses, target AC 15 > threshold 13 → no shock
- Unit: attack misses, weapon has no shock (shock_damage=0) → no shock
- Unit: attack hits → no shock check performed
- Unit: shock damage is clamped to minimum 0 (negative STR mod can't make
  shock negative)
- Property: shock damage is always ≥ 0

**Acceptance:** Shock damage appears in combat log on misses against
lightly-armored targets.

---

## Task 4.6.5: Range Bands

> **What:** Track range band per combatant. Enforce weapon range requirements.
> **Estimated time:** 2.5 hours

**Range bands:** `none` (not in combat), `melee`, `near`, `far`

**Rule (simplified v1):**
- Combatants start at `melee` range when encounter is triggered (existing behaviour).
- Ranged weapons can attack at `near` or `far`. Melee weapons require `melee`.
- Movement is free (no action cost in v1) — PC can declare range band change
  before attacking. Add commands `advance` (→ melee) and `withdraw` (→ near or far).
- Enemies move to melee range if PC is at near/far and enemy has only melee attacks.

**File:** `src/harsh_realm/gm/scenes/combat.py`

Add `range_band: str = "melee"` to `CombatantState` (or equivalent model).

Add command handlers:
- `advance` → set PC range_band to melee
- `withdraw` → set PC range_band to near

On attack:
- If PC weapon is melee and PC range_band != melee → return error message
  "You need to advance to melee range first. (advance)"
- If PC weapon is ranged and PC range_band == melee → allow (point-blank)
  but note -2 penalty in log (per WWN)
- If PC weapon is ranged and ammo = 0 → auto-switch to melee weapon
  (see 4.6.6)

Enemy AI (simple v1): if enemy has melee-only attacks and PC is at near/far,
enemy uses its action to advance to melee.

**Tests:**
- Unit: PC at near range, attacks with short_sword → blocked with message
- Unit: PC at melee range, attacks with short_sword → proceeds
- Unit: PC at melee range, attacks with short_bow → proceeds (point-blank)
- Unit: `advance` command sets range_band to melee
- Unit: `withdraw` command sets range_band to near
- Unit: melee enemy at far range → enemy advances before attacking

**Acceptance:** Range bands are tracked and enforced. `advance`/`withdraw`
commands work.

---

## Task 4.6.6: Ammo Tracking

> **What:** Consume ammo on ranged attacks. Auto-switch to melee weapon
> when ammo depleted.
> **Estimated time:** 2 hours

**Model changes:**
- Character inventory tracks ammo by item ID and quantity.
  `class_abilities` JSON field extended to include `ammo: {item_id: count}`.
- Starting kit ammo quantities loaded from kit YAML (arrows_20 → 20 of
  `ammo.arrow`, bolts_20 → 20 of `ammo.bolt`).

**Engine changes (`engine/combat.py`):**
- Before resolving a ranged attack: check ammo count for the equipped
  ranged weapon's `ammo_type`. If 0, trigger auto-switch.
- On successful ranged attack: decrement ammo count by 1.
- Auto-switch procedure:
  1. Check for equipped melee weapon in character inventory.
  2. If found: equip it, emit `[No ammo — switched to <melee weapon>]`
     combat message, continue attack with melee weapon this round.
  3. If no melee weapon: equip unarmed (1d2), emit
     `[No ammo — fighting unarmed]`.

**Shop and loot:** Ammo items purchased via shop increment the ammo counter.
Ammo loot drops increment the counter.

**Tests:**
- Unit: ranged attack with 5 arrows → arrows become 4
- Unit: ranged attack with 0 arrows → auto-switch to melee weapon, message emitted
- Unit: ranged attack with 0 arrows, no melee weapon → switch to unarmed
- Unit: buy arrows in shop → ammo counter increments correctly
- Unit: ammo count never goes below 0
- Property: ammo count after N attacks ≤ initial ammo count

**Acceptance:** Ranged weapons consume ammo. Depletion auto-switches to melee.

---

## Task 4.6.7: Saving Throw Types

> **What:** Extend saving throws from a single value to four differentiated
> types. Each type has its own base target per class. Items can buff
> individual types.
> **Estimated time:** 2.5 hours

**Background from audit task 4.6.1:** Proceed with extension approach
(assumed; adjust if audit says replace).

**Four save types:**
| Type | Governs | Primary Stat |
|------|---------|-------------|
| Physical | STR/CON threats: poison, disease, exhaustion | CON mod |
| Evasion | DEX threats: traps, explosions, area effects | DEX mod |
| Mental | WIS/CHA threats: fear, charm, confusion | WIS mod |
| Luck | Everything else: random mishaps, cursed items | None |

**`data/classes.yaml` — extend saving_throws:**
```yaml
saving_throws:
  physical: 15    # base target (roll this or higher to succeed)
  evasion: 15
  mental: 15
  luck: 15
```
(All start at 15 — PLACEHOLDER comment removed once verified against source.)

**`models/character.py` — add save bonus field:**
```python
save_bonuses: dict[str, int] = {}
# e.g. {"physical": 2, "evasion": 1} — from equipped items or abilities
```

**Engine (`engine/combat.py` or `engine/saves.py`):**

```python
def resolve_save(
    character,
    save_type: Literal["physical", "evasion", "mental", "luck"],
    difficulty_modifier: int = 0
) -> SaveResult:
    """
    Roll d20 + relevant stat modifier + save_bonuses.get(save_type, 0)
    vs base_target - difficulty_modifier.
    Returns SaveResult with roll, modifier, target, passed.
    """
```

Stat modifier used per type:
- physical → CON modifier
- evasion → DEX modifier
- mental → WIS modifier
- luck → 0 (no stat mod)

**`models/item.py`:** Add optional `save_bonus: dict[str, int]` field to
`ItemData` for future items that buff saves (extensibility, not used by
any current items).

**Tests:**
- Unit: physical save with CON 14 (mod +2) → modifier applied
- Unit: evasion save with DEX 8 (mod -1) → penalty applied
- Unit: luck save → no stat modifier
- Unit: save passes when roll + mod ≥ target
- Unit: save fails when roll + mod < target
- Unit: item with `save_bonus: {physical: 2}` adds to physical save
- Property: save result always has valid roll (1-20), modifier, target, bool

**Acceptance:** Four save types implemented. Each uses correct stat modifier.
Items can buff saves via the extensible field.

---

## Task 4.6.8: Combat Log Formatting

> **What:** Enhance ChatLog with CRPG-style structured combat messages.
> **Estimated time:** 2 hours

**Backend:** Ensure all combat events carry sufficient data for display.
Add/verify these event payloads:

`combat.attack`:
```python
{
  "attacker": str,
  "target": str,
  "weapon": str,           # item_id or "unarmed"
  "roll": int,             # raw d20 result
  "modifier": int,         # attack bonus + attribute mod
  "total": int,            # roll + modifier
  "target_ac": int,
  "hit": bool,
  "damage": int | None,    # None on miss with no shock
  "shock": int,            # 0 if no shock
  "critical": bool         # natural 20
}
```

`combat.save`:
```python
{
  "character": str,
  "save_type": str,        # physical/evasion/mental/luck
  "roll": int,
  "modifier": int,
  "total": int,
  "target": int,
  "passed": bool
}
```

`combat.shock` (already defined in 4.6.4):
```python
{"attacker": str, "target": str, "shock_damage": int, "weapon": str}
```

**Frontend (`frontend/src/composables/useWebSocket.ts`):**

Handle `combat.attack`:
```
⚔ TestBot attacks Wolf  [Short Sword]
   Roll: 14 + 3 = 17  vs  AC 13  →  Hit!  Damage: 6
```
On miss with shock:
```
⚔ TestBot attacks Bandit  [Short Sword]
   Roll: 5 + 3 = 8  vs  AC 12  →  Miss  |  Shock: 3
```
On critical:
```
⚔ TestBot attacks Wolf  [Short Sword]
   Roll: 20 (CRITICAL) + 3 = 23  vs  AC 13  →  Hit!  Damage: 11
```

Handle `combat.save`:
```
🎲 Physical Save  [TestBot]
   Roll: 12 + 2 = 14  vs  15  →  Failed
```

Handle `combat.shock` (if emitted separately from attack):
```
💥 Shock damage: 3  (Wolf → TestBot)
```

**CSS classes:** `combat-attack-message`, `combat-save-message`,
`combat-shock-message` — use red/orange accent colours consistent with
the existing combat narration styling.

**Tests:**
- Unit: `combat.attack` event with hit=True emits correct payload fields
- Unit: `combat.attack` event with shock > 0 emits shock value
- Playwright: attack command produces combat-attack-message in ChatLog
- Playwright: save roll produces combat-save-message in ChatLog
- Playwright: shock message appears on miss against low-AC target

**Acceptance:** Combat is readable as a structured log. Player can see
exactly what happened on each exchange.

---

## Dependency Order

```
4.6.1 (save audit) → must be first
  ↓
4.6.2 (item registry) → foundational; all other tasks depend on it
  ↓
4.6.3 (equipped weapon resolution) → needs item registry
4.6.4 (shock damage) → needs item registry + weapon stat blocks
4.6.5 (range bands) → needs equipped weapon check from 4.6.3
4.6.6 (ammo tracking) → needs item registry + range bands
4.6.7 (save types) → needs save audit result; mostly independent
  ↓
4.6.8 (combat log) → needs all event shapes finalized
```

Recommended order:
1. 4.6.1 (audit — first, 1 hour)
2. 4.6.2 (item registry — foundational, build fully before anything else)
3. 4.6.3 + 4.6.7 in parallel (equipped weapon + saves — independent of each other)
4. 4.6.4 (shock — needs 4.6.3)
5. 4.6.5 (range bands — needs 4.6.3)
6. 4.6.6 (ammo — needs 4.6.5)
7. 4.6.8 (combat log — last, all event shapes must be final)

---

## Notes for the Coding Agent

- Read CLAUDE.md, AGENTS.md, and `docs/rules_reference/combat.md` before
  starting. The combat rules doc is the authoritative source for XWN mechanics.
- **Item IDs are immutable once assigned.** Do not rename an item ID after
  it is referenced in any YAML file. If a name needs to change, update the
  `name` field only, not the `id`.
- The item ID format is strictly `category.slug` where category is one of:
  `weapon`, `armor`, `ammo`, `gear`, `consumable`, `pretech`. No other
  prefixes. The slug uses underscores, lowercase only.
- `ItemRegistry` is loaded once at world startup. It reads from `data/items/`.
  It does NOT load from SQLite — item definitions are static authored content,
  not per-world editable config. This is consistent with the YAML-seeds-SQLite
  principle: item definitions are the seed, per-world inventory is SQLite.
- Do NOT change the `hexes` table name (retained for backwards compat per
  CLAUDE.md). Square grid is the new default but the table name stays.
- After 4.6.2c refactors kits, run the full test suite before proceeding.
  Kit refactoring touches character creation — if tests break, fix before
  continuing.
- Creatures use their statblock damage for attacks (HD-derived). Natural
  weapon IDs in creatures.yaml are for shock value lookups only.
- After completing all tasks, update CLAUDE.md:
  - Mark Milestone 4.6 complete with date
  - Record final test count
  - Note any PLACEHOLDER values resolved or still outstanding in classes.yaml
