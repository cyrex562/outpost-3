# XWN Rules Reference: Weapons & Armor

> **Purpose:** Reference for coding agents implementing equipment stats.
> **Status:** Placeholders marked with `[PLACEHOLDER]` need to be filled from source books.

## Weapons

### TL3 Melee Weapons (Common)

| Weapon | Damage | Attribute | Skill | Encumbrance | Cost | Notes |
|---|---|---|---|---|---|---|
| Dagger / Knife | [PLACEHOLDER] | STR or DEX | Stab | [PLACEHOLDER] | [PLACEHOLDER] | Light, concealable |
| Short Sword | [PLACEHOLDER] | STR or DEX | Stab | [PLACEHOLDER] | [PLACEHOLDER] | |
| Sword / Longsword | [PLACEHOLDER] | STR | Stab | [PLACEHOLDER] | [PLACEHOLDER] | Standard military weapon |
| Spear | [PLACEHOLDER] | STR or DEX | Stab | [PLACEHOLDER] | [PLACEHOLDER] | Reach, throwable |
| Axe | [PLACEHOLDER] | STR | Stab | [PLACEHOLDER] | [PLACEHOLDER] | |
| Mace / Hammer | [PLACEHOLDER] | STR | Stab | [PLACEHOLDER] | [PLACEHOLDER] | Effective vs. armor |
| Greataxe / Greatsword | [PLACEHOLDER] | STR | Stab | [PLACEHOLDER] | [PLACEHOLDER] | Two-handed, high damage |
| Staff | [PLACEHOLDER] | STR or DEX | Stab | [PLACEHOLDER] | [PLACEHOLDER] | Two-handed, cheap |
| Club | [PLACEHOLDER] | STR | Stab | [PLACEHOLDER] | [PLACEHOLDER] | Improvised, cheap |

### TL3 Ranged Weapons

| Weapon | Damage | Attribute | Skill | Range | Encumbrance | Cost | Notes |
|---|---|---|---|---|---|---|---|
| Bow | [PLACEHOLDER] | DEX | Shoot | [PLACEHOLDER] | [PLACEHOLDER] | [PLACEHOLDER] | Requires arrows |
| Crossbow | [PLACEHOLDER] | DEX | Shoot | [PLACEHOLDER] | [PLACEHOLDER] | [PLACEHOLDER] | Slow reload |
| Throwing Knife | [PLACEHOLDER] | DEX | Shoot | [PLACEHOLDER] | [PLACEHOLDER] | [PLACEHOLDER] | Short range |
| Javelin | [PLACEHOLDER] | STR | Shoot | [PLACEHOLDER] | [PLACEHOLDER] | [PLACEHOLDER] | One use per throw |
| Crude Firearm | [PLACEHOLDER] | DEX | Shoot | [PLACEHOLDER] | [PLACEHOLDER] | [PLACEHOLDER] | TL3, unreliable, loud, slow reload |

### TL4 Weapons (Rare — Pretech Relics)

| Weapon | Damage | Attribute | Skill | Range | Encumbrance | Cost | Notes |
|---|---|---|---|---|---|---|---|
| Laser Pistol | [PLACEHOLDER] | DEX | Shoot | [PLACEHOLDER] | [PLACEHOLDER] | [PLACEHOLDER] | Rare, requires power cell |
| Laser Rifle | [PLACEHOLDER] | DEX | Shoot | [PLACEHOLDER] | [PLACEHOLDER] | [PLACEHOLDER] | Rare, requires power cell |
| Monoblade | [PLACEHOLDER] | STR or DEX | Stab | — | [PLACEHOLDER] | [PLACEHOLDER] | Pretech melee, ignores some armor |
| Stun Baton | [PLACEHOLDER] | STR or DEX | Stab | — | [PLACEHOLDER] | [PLACEHOLDER] | Non-lethal, requires power cell |

[PLACEHOLDER — fill all weapon stats from SWN/WWN source books. Include damage dice, attribute modifiers, costs in setting-appropriate currency.]

## Armor

| Armor | AC Bonus | Encumbrance | Cost | TL | Notes |
|---|---|---|---|---|---|
| No Armor | +0 | 0 | 0 | — | AC = 10 + DEX mod |
| Leather / Hide | [PLACEHOLDER] | [PLACEHOLDER] | [PLACEHOLDER] | TL3 | Light, no penalty |
| Chain Mail | [PLACEHOLDER] | [PLACEHOLDER] | [PLACEHOLDER] | TL3 | Medium |
| Plate Armor (partial) | [PLACEHOLDER] | [PLACEHOLDER] | [PLACEHOLDER] | TL3 | Heavy, rare |
| Shield | [PLACEHOLDER] | [PLACEHOLDER] | [PLACEHOLDER] | TL3 | +1 AC, occupies one hand |
| Pretech Armor (light) | [PLACEHOLDER] | [PLACEHOLDER] | [PLACEHOLDER] | TL4 | Rare relic |
| Pretech Armor (heavy) | [PLACEHOLDER] | [PLACEHOLDER] | [PLACEHOLDER] | TL4 | Very rare relic |

[PLACEHOLDER — fill all armor stats from source books]

## Starting Equipment Kits

Defined in `data/equipment_kits.yaml`. Each kit provides a complete starting loadout.

### Warrior Kits

**Heavy Fighter:**
- [PLACEHOLDER — e.g., Chain mail, sword, shield, dagger, backpack, 7 days rations, waterskin, bedroll, 3d6 × 10 coin]

**Light Fighter:**
- [PLACEHOLDER — e.g., Leather armor, sword, bow, 20 arrows, dagger, backpack, 7 days rations, waterskin, 2d6 × 10 coin]

**Scavenger Soldier:**
- [PLACEHOLDER — e.g., Leather armor, crude firearm, 10 rounds, short sword, backpack, rations, tinderbox, 2d6 × 10 coin]

### Expert Kits

**Scout:**
- [PLACEHOLDER — e.g., Leather armor, short sword, bow, 20 arrows, rope 50ft, tinderbox, backpack, rations, 2d6 × 10 coin]

**Tinker:**
- [PLACEHOLDER — e.g., No armor, dagger, tool kit, mechanical supplies, lantern, oil, backpack, rations, 3d6 × 10 coin]

**Face:**
- [PLACEHOLDER — e.g., Fine clothing, concealed dagger, writing supplies, backpack, rations, 4d6 × 10 coin]

### Adventurer Kits

**Versatile:**
- [PLACEHOLDER — e.g., Leather armor, sword, dagger, rope 50ft, tool kit, backpack, rations, 2d6 × 10 coin]

## Currency

The setting uses a coin-based economy:
- **Copper piece (cp):** Common currency for peasants and basic goods
- **Silver piece (sp):** Standard currency for trade and military pay
- **Gold piece (gp):** Rare, used by nobility and for high-value transactions

[PLACEHOLDER — exchange rate: 1 gp = ? sp = ? cp. Verify from source or define for setting.]
[PLACEHOLDER — starting coin per kit: random roll as noted above, or fixed amount?]

Pretech items do not have fixed prices — their value depends on who's buying. A feudal lord might pay 100 gp for an energy cell; a peasant has no use for it.

## Notes for the Coding Agent

- All `[PLACEHOLDER]` values must be filled by the developer from source books.
- If implementing before placeholders are filled, use these reasonable defaults:
  - Dagger: 1d4, Sword: 1d8, Greatsword: 1d10, Spear: 1d6
  - Bow: 1d6, Crossbow: 1d8, Crude Firearm: 1d8
  - Leather: AC +2, Chain: AC +4, Plate: AC +6, Shield: AC +1
  - Starting coin: 3d6 × 10 sp
- Mark all defaults with `# PLACEHOLDER: verify against source`
- Weapon and armor data should be stored in `data/weapons.yaml` and `data/armor.yaml` for easy editing.
- The equipment kit system must be data-driven (YAML) so new kits can be added without code changes.
