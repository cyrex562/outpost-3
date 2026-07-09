# XWN Rules Reference: Classes

> **Purpose:** Reference for coding agents implementing XWN class mechanics.
> **Status:** Placeholders marked with `[PLACEHOLDER]` need to be filled from source books.

## Available Classes (Milestone 1)

### Warrior

The combat specialist. Best attack progression, toughest in a fight, bonus damage.

| Feature | Detail |
|---|---|
| Hit Die | d8 per level (roll at level 1, + CON modifier, minimum 1) |
| Attack Bonus | [PLACEHOLDER — +1 per level?] |
| Starting Skill Points | [PLACEHOLDER — number of points and any restrictions] |
| **Veteran's Luck** | Once per fight, the Warrior can negate a successful hit against them OR turn one of their own missed attacks into a hit. This ability refreshes at the start of each new combat. |
| **Killing Blow** | [PLACEHOLDER — Warriors add damage bonus. Half level rounded up to damage?] |
| Saving Throws | [PLACEHOLDER — Physical/Evasion/Mental base values at level 1] |

### Expert

The skilled specialist. More skill points, skill reroll ability, wide skill access.

| Feature | Detail |
|---|---|
| Hit Die | d6 per level (+ CON modifier, minimum 1) |
| Attack Bonus | [PLACEHOLDER — progression rate] |
| Starting Skill Points | [PLACEHOLDER — number of points, which skills available] |
| **Quick Learner** | [PLACEHOLDER — bonus skill point per level? Non-combat skill?] |
| **Masterful Expertise** | Once per scene, the Expert can reroll one failed skill check. They must take the new result. This ability refreshes when the scene changes. |
| Saving Throws | [PLACEHOLDER — Physical/Evasion/Mental base values at level 1] |

### Adventurer

A hybrid class that picks two partial class abilities. Available combinations for Milestone 1 (no magic):

- Partial Warrior + Partial Expert

| Feature | Detail |
|---|---|
| Hit Die | [PLACEHOLDER — d6? Depends on partial classes chosen?] |
| Attack Bonus | [PLACEHOLDER — depends on partial classes] |
| Starting Skill Points | [PLACEHOLDER] |
| **Partial Warrior** | [PLACEHOLDER — reduced version of Warrior abilities. Veteran's Luck every other fight? Reduced damage bonus?] |
| **Partial Expert** | [PLACEHOLDER — reduced version of Expert abilities. Reroll once per day instead of per scene?] |
| Saving Throws | [PLACEHOLDER] |

## XP and Leveling

| Level | Total XP Required |
|---|---|
| 1 | 0 |
| 2 | [PLACEHOLDER] |
| 3 | [PLACEHOLDER] |
| 4 | [PLACEHOLDER] |
| 5 | [PLACEHOLDER] |
| 6 | [PLACEHOLDER] |
| 7 | [PLACEHOLDER] |
| 8 | [PLACEHOLDER] |
| 9 | [PLACEHOLDER] |
| 10 | [PLACEHOLDER] |

### On Level Up

[PLACEHOLDER — what happens at each level up? Roll new HD for HP? Gain skill points? Attack bonus increase? New abilities at specific levels?]

## XP Awards

[PLACEHOLDER — how much XP per encounter/discovery/objective? Based on enemy HD? Flat amounts? Guidelines for the automated GM to determine awards.]

## Starting Equipment Kits

Equipment kits are predefined loadouts selected during character creation. Each kit is appropriate for the setting (TL3 feudal planet with scattered pretech).

### Warrior Kits

**Heavy Fighter**
[PLACEHOLDER — chain mail or equivalent, melee weapon, shield, backpack, rations, waterskin, basic supplies. List items with stats.]

**Light Fighter**
[PLACEHOLDER — leather armor, two melee weapons or one melee + ranged, backpack, rations, basic supplies.]

**Scavenger Soldier**
[PLACEHOLDER — mixed armor, crude firearm + melee weapon, backpack, ammunition, rations.]

### Expert Kits

**Scout**
[PLACEHOLDER — light armor, short blade, survival gear, rope, tinderbox, rations.]

**Tinker**
[PLACEHOLDER — no armor or light armor, tools, mechanical supplies, short blade, rations.]

**Face**
[PLACEHOLDER — decent clothing, light concealed weapon, money pouch with extra starting coin, rations.]

### Adventurer Kits

**Versatile**
[PLACEHOLDER — medium armor, one weapon, some tools, rations. A bit of everything.]

---

## Notes for the Coding Agent

- All `[PLACEHOLDER]` entries must be filled by the developer from XWN source books before implementation.
- If implementing before placeholders are filled, use reasonable defaults and mark them clearly in code comments: `# PLACEHOLDER: verify against source book`.
- The starting kit system should be data-driven (YAML) so new kits can be added without code changes.
- Class abilities that reference "per fight" or "per scene" need the GM controller to track scene/fight boundaries and reset the ability flags.
