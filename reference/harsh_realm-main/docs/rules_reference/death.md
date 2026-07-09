# Death and Dying Reference (XWN + House Rules)

## Reaching 0 HP

In standard XWN, a character who reaches 0 HP is dying. In solo play, the **Last Stand** house rule applies before death.

See `docs/house_rules/last_stand.md` for the Last Stand procedure.

## After Last Stand

If the player fails to survive the Last Stand (does not kill the final enemy, heal above 0 HP, or escape), they die.

## Death Consequences (Harsh Realm)

Upon death, the player has two options:

1. **Respawn** — The character is recovered by locals or crawls to safety:
   - Transported to the nearest settlement.
   - Restored to 50% maximum HP (rounded down, minimum 1).
   - Loses one equipped item (prefer weapon → armor → other gear).
   - Loses 15% of current XP (rounded down, minimum 0; cannot de-level).
   - A death marker is placed at the location of death; the lost item can be recovered.

2. **New Character** — The old character is gone. Start character creation fresh.

## Death Marker

When a character dies, the hex where they fell receives a death marker in its data. If the player returns to that hex, they may use `take <item>` to retrieve the lost item.

## XP Loss on Respawn

XP is reduced by 15% of the current XP total, rounded down. The character never drops below the XP threshold for their current level (they cannot de-level from respawn XP loss).

## Respawn Location

The nearest settlement is found by searching outward from the death hex. If no settlement exists, the character respawns at a safe adjacent hex (any passable terrain without hostile features).
