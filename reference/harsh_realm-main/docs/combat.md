# XWN Rules Reference: Combat

> **Purpose:** Reference for coding agents implementing XWN combat mechanics.
> **Status:** Placeholders marked with `[PLACEHOLDER]` need to be filled from source books.

## Combat Sequence

### 1. Initiative

Roll d8 + DEX modifier for each combatant. Higher goes first. Ties broken by DEX score, then alphabetical by name.

- **Player surprise:** Enemies do not act in Round 1.
- **Enemy surprise:** Player does not act in Round 1.
- **Mutual awareness:** Normal initiative, all act from Round 1.

### 2. Turn Structure

Each combatant gets one turn per round, in initiative order. On your turn:
- **Move:** Reposition (not mechanically tracked in text mode — future extension for tactical grid)
- **Action:** One of: attack, use item, skill check, or other action

### 3. Attack Roll

Roll d20 + attack bonus + relevant combat skill + attribute modifier vs. target's Armor Class.

```
d20 + attack_bonus + skill_level + attribute_modifier >= target_AC → Hit
```

- **Melee attacks:** Use Stab skill + STR modifier (or DEX modifier if weapon allows)
- **Ranged attacks:** Use Shoot skill + DEX modifier
- **Unarmed:** Use Punch skill + STR or DEX modifier
- **Natural 1:** Always misses, regardless of modifiers
- **Natural 20:** Always hits, regardless of target AC
- [PLACEHOLDER — are there critical hit rules beyond auto-hit on 20? Extra damage?]

### 4. Attack Bonus by Class

| Class | Level 1 | Level 2 | Level 3 | Level 4 | Level 5 | Level 6+ |
|---|---|---|---|---|---|---|
| Warrior | [PLACEHOLDER] | [PLACEHOLDER] | [PLACEHOLDER] | [PLACEHOLDER] | [PLACEHOLDER] | [PLACEHOLDER] |
| Expert | [PLACEHOLDER] | [PLACEHOLDER] | [PLACEHOLDER] | [PLACEHOLDER] | [PLACEHOLDER] | [PLACEHOLDER] |
| Adventurer (Partial Warrior) | [PLACEHOLDER] | [PLACEHOLDER] | [PLACEHOLDER] | [PLACEHOLDER] | [PLACEHOLDER] | [PLACEHOLDER] |
| Adventurer (Partial Expert) | [PLACEHOLDER] | [PLACEHOLDER] | [PLACEHOLDER] | [PLACEHOLDER] | [PLACEHOLDER] | [PLACEHOLDER] |

### 5. Damage

On a hit, roll the weapon's damage die + attribute modifier.

- **Melee weapons:** Add STR modifier to damage
- **Ranged weapons:** [PLACEHOLDER — do ranged weapons add DEX to damage in XWN?]
- **Warrior bonus:** Warriors add half their level (round up) to all damage rolls
- **Minimum damage:** 1 (a hit always deals at least 1 damage)

### 6. Armor Class

AC = 10 + DEX modifier + armor bonus + shield bonus (if any)

| Armor | AC Bonus | Encumbrance | Cost | TL |
|---|---|---|---|---|
| [PLACEHOLDER — fill from source] | | | | |

### 7. Saving Throws

When required, roll d20 vs. save target.

Save target = 15 - (level / 2, round down)

[PLACEHOLDER — are there separate Physical/Evasion/Mental saves in XWN, or one unified save? Fill from source.]

| Level | Save Target |
|---|---|
| 1 | 15 |
| 2 | 14 |
| 3 | 14 |
| 4 | 13 |
| 5 | 13 |
| 6 | 12 |
| 7 | 12 |
| 8 | 11 |
| 9 | 11 |
| 10 | 10 |

### 8. Hit Points and Injury

- HP = sum of all HD rolls + (CON modifier × level)
- At 0 HP: dying (see `death.md`)
- [PLACEHOLDER — do characters die at 0 HP or at negative HP? XWN-specific rule?]

### 9. Healing

- **First aid:** After combat, one attempt. Heal skill check vs. difficulty 8. Success: restore 1d6 + Heal skill HP.
- **Natural rest:** [PLACEHOLDER — HP restored per day of rest? Level + CON mod? Verify from source.]
- **Full night's rest:** [PLACEHOLDER — different from short rest?]

### 10. Class Combat Abilities

**Warrior — Veteran's Luck:**
Once per combat, a Warrior may either:
- Negate one successful hit against them (the attack is treated as a miss), OR
- Turn one of their own missed attacks into a hit

This choice is made in the moment when the triggering event occurs. Only one use per combat, and the Warrior must choose to negate OR force hit — not both.

**Warrior — Killing Blow:**
[PLACEHOLDER — Warriors add half level (round up) to damage? Verify exact wording from source.]

**Expert — Masterful Expertise:**
[PLACEHOLDER — does Expert ability apply in combat? Once per scene reroll — does "scene" include a combat scene?]

### 11. XP Awards

[PLACEHOLDER — fill XP-per-HD table from source books]

| Enemy HD | XP Value |
|---|---|
| < 1 | [PLACEHOLDER] |
| 1 | [PLACEHOLDER] |
| 2 | [PLACEHOLDER] |
| 3 | [PLACEHOLDER] |
| 4 | [PLACEHOLDER] |
| 5 | [PLACEHOLDER] |
| 6 | [PLACEHOLDER] |
| 7 | [PLACEHOLDER] |
| 8+ | [PLACEHOLDER] |

## Notes for the Coding Agent

- All `[PLACEHOLDER]` values must be filled by the developer.
- If implementing before placeholders are filled, use these reasonable defaults:
  - Warrior attack bonus: +1 per level
  - Expert attack bonus: +1 per 2 levels
  - Warrior damage bonus: half level round up
  - XP per HD: HD × 15
  - Natural healing: 1 HP per day of rest, level + CON mod per full rest
- Mark all default values in code with `# PLACEHOLDER: verify against source`
