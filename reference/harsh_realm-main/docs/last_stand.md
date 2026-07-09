# House Rule: Last Stand

> **Purpose:** Solo play survival mechanic. Gives the player one final action at 0 HP before death.
> **Implemented in:** `src/harsh_realm/house_rules/last_stand.py`

## Rule

When the player character's HP is reduced to 0 (or below) during combat, instead of immediately dying, they receive one final action.

## Trigger

- Any damage that reduces HP to 0 or below
- HP is set to 0 for display purposes (does not go negative)
- The GM prompts: **"You stagger, vision fading. One last chance — what do you do?"**

## Valid Final Actions

### Attack
- The character makes one melee or ranged attack at a **-2 penalty** to the attack roll
- If the attack hits and kills the last remaining enemy, the character **survives** at 0 HP (stabilized). Combat ends in victory. The character is alive but critically wounded — first aid or healing is urgent.
- If the attack does not end the fight, the character dies after the attack resolves.

### Use Item
- The character uses one consumable item (healing herb, potion, bandage, etc.)
- If the item restores HP above 0, the character **stabilizes** and combat continues. They rejoin the initiative order on their next normal turn.
- If the item does not bring HP above 0 (insufficient healing or non-healing item), the character dies after the item is used.

### Flee
- The character attempts to flee. This is an **automatic messy escape** — no skill check, always succeeds.
- The character always takes one consequence: parting blow (one enemy free attack, but this cannot further reduce HP below 0 — the character is already dying) and/or drops a random item.
- The character escapes combat but is **dying**. Without healing within a short time, they will die.
- If they have a healing item available after fleeing, they can use it immediately.
- If they cannot heal above 0 HP after fleeing, they die.

## After the Final Action

- If HP is above 0: character is alive, combat continues (if enemies remain) or ends (if all enemies defeated).
- If HP is still 0: character dies. Proceed to death/respawn flow.

## Design Intent

This house rule exists because solo play with no allies means there's no one to stabilize a dying character. Without Last Stand, reaching 0 HP would always mean death, which is punishing and anticlimactic. Last Stand creates dramatic moments — the player might kill the final enemy in a last desperate swing, or chug a healing potion just in time to stay in the fight.

## Implementation Notes

- Last Stand triggers only once per combat. If the character stabilizes and later drops to 0 again in the same fight, they die immediately (no second Last Stand).
- The -2 attack penalty on the final attack represents fighting while nearly unconscious.
- Enemy attacks during Last Stand flee cannot reduce HP below 0 (the character is already at death's door).
- The `last_stand_used` flag should be tracked in `CombatState`.
