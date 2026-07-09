# XWN Rules Reference: Attributes

> **Purpose:** Reference for coding agents implementing XWN attribute mechanics.
> **Status:** Placeholders marked with `[PLACEHOLDER]` need to be filled from source books.

## Primary Attributes

| Attribute | Abbreviation | Description |
|---|---|---|
| Strength | STR | Physical power, melee damage, carrying capacity |
| Dexterity | DEX | Agility, ranged combat, AC bonus, initiative |
| Constitution | CON | Toughness, hit points, physical endurance |
| Intelligence | INT | Reasoning, technical skills, knowledge |
| Wisdom | WIS | Perception, willpower, awareness |
| Charisma | CHA | Leadership, social influence, force of personality |

## Attribute Modifiers

| Score | Modifier |
|---|---|
| 3 | -2 |
| 4-7 | -1 |
| 8-13 | 0 |
| 14-17 | +1 |
| 18 | +2 |

## Attribute Generation: 4d6 Drop Lowest

1. Roll 4d6, discard the lowest die, sum the remaining three.
2. Repeat six times to generate six scores.
3. Player assigns each score to one attribute of their choice.

The GM presents all six scores at once, then the player assigns them one at a time.

## Derived Statistics

| Stat | Formula |
|---|---|
| Hit Points (level 1) | Class HD roll + CON modifier (minimum 1) |
| Armor Class | 10 + DEX modifier + armor bonus |
| Melee Attack Bonus | [PLACEHOLDER — base attack bonus by class at level 1] + STR modifier + Stab/Punch skill |
| Ranged Attack Bonus | [PLACEHOLDER — base attack bonus by class at level 1] + DEX modifier + Shoot skill |
| Physical Saving Throw | [PLACEHOLDER — 15 minus half level, round down] |
| Evasion Saving Throw | [PLACEHOLDER — 15 minus half level, round down] |
| Mental Saving Throw | [PLACEHOLDER — 15 minus half level, round down] |
| Initiative Modifier | DEX modifier |

## Notes

- No attribute can be below 3 or above 18 at character creation.
- Attribute modifiers are used in skill checks, attack rolls, damage rolls, and saving throws.
- Which attribute applies to a skill check depends on the situation — the GM (system) decides based on context. Common pairings are documented in `skills.md`.
