# XWN Rules Reference: Death & Respawn

> **Purpose:** Reference for coding agents implementing death, dying, and respawn mechanics.
> **Status:** This document contains both XWN base rules and Harsh Realm house rules.

## Dying (XWN Base)

When a character reaches 0 HP, they are **dying**.

In standard XWN:
- Allies can stabilize the dying character with a successful Heal skill check (difficulty 8)
- Without stabilization, the character dies in 6 rounds
- [PLACEHOLDER — verify dying/death rules from XWN source. Do characters die at 0 or go to negative HP?]

## Solo Play Modification

Since Harsh Realm is solo (no allies to stabilize), reaching 0 HP triggers the **Last Stand** house rule (see `docs/house_rules/last_stand.md`) before death.

If the Last Stand action does not save the character, they die.

## Death

On death:
1. Combat ends immediately
2. GM narrates: "Everything goes dark..."
3. The character's death is recorded in the event log
4. A **death marker** is placed at the hex where the character died
5. Any dropped items are placed at the death marker
6. The game transitions to the Respawn scene

## Respawn Options

The player chooses one:

### Option 1: Respawn at Nearest Settlement

- **Location:** Character wakes at the nearest settlement hex
- **HP:** Restored to 50% of maximum (round up)
- **Item loss:** One equipped item is lost (priority: weapon → armor → other gear). The item remains at the death hex as a retrievable entity.
- **XP penalty:** Lose 15% of current XP (round down). This never causes a de-level — XP cannot go below the threshold for the current level.
- **Currency:** No currency lost
- **Narration:** "You wake in [settlement name]. A passing traveler found you and dragged you to safety. Your [lost item] was left behind near [death hex coordinates]."
- **Equipment recovery:** A basic equipment kit can be purchased at the settlement (same kits as character creation). The lost items persist at the death hex until retrieved.

### Option 2: Create a New Character

- The current character is marked as dead in the database (preserved for world history)
- The player enters the Character Creation flow
- The new character enters the same world — all exploration, faction state, NPC relationships persist
- The new character starts at the starting settlement (or another settlement of the player's choice if multiple are known)

## Death Marker

- Death hexes gain a `"death_marker"` feature with data including a list of dropped items
- Items at the death location can be retrieved by visiting the hex and using the `take <item>` command
- The death marker is visible on the grid map as a distinct icon
- Death markers persist until all items are retrieved, then they are removed

## Difficulty Settings (Future)

Planned but not implemented in Milestone 3:

| Setting | Effect |
|---|---|
| Normal | Standard rules as above |
| Forgiving | Respawn at 75% HP, lose only 5% XP, no item loss |
| God Mode | Respawn at full HP, no XP or item loss |
| Ironman | Permadeath — no respawn, must create new character |

## Anchor System (Future House Rule)

A planned house rule for later implementation:

- Anchors can be set at towns, temples, and other safe locations
- When HP reaches 0, the character respawns at their most recently set anchor
- Respawn at 50% HP with equipment intact (the anchor magically preserves you)
- Until magic is implemented, equipment is NOT retained — the character must acquire a basic kit at the anchor location
- Anchors provide a player-controlled respawn point rather than always defaulting to nearest settlement
