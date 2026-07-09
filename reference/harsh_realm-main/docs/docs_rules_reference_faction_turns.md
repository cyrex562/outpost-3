# Faction Turn Rules Reference
> For use by coding agents implementing Milestone 4 faction system.
> Source: Worlds Without Number (WWN) by Kevin Crawford — Faction rules chapter.

## Overview

Factions are organizations that operate on a strategic scale above individual characters. They have their own stats, assets, goals, and take actions each week. The faction system generates the living world backdrop — factions expand, fight each other, react to the player, and create situations the player encounters.

One faction turn fires per in-game **week** (7 days of game time). This is accelerated from WWN's default monthly for solo play pacing.

## Faction Stats

| Stat | Description |
|---|---|
| **HP** | Faction health. Reaches 0 → faction is destroyed or surrenders |
| **Force** | Military/violent power. Required for Force-type assets |
| **Cunning** | Espionage, deception, information power. Required for Cunning-type assets |
| **Wealth** | Economic power. Required for Wealth-type assets |
| **XP** | Experience points. Earned through actions, spent to create assets and improve stats |

Stats range from 1–8 for most factions. Player-hostile factions typically have higher Force; merchant factions have higher Wealth.

## Assets

Assets are the units factions control. Each asset has:

| Field | Description |
|---|---|
| **Type** | Unique identifier (e.g., "Warriors", "Informers", "Smugglers") |
| **Category** | Force, Cunning, or Wealth |
| **Min Attribute** | Minimum faction attribute needed to purchase this asset |
| **Cost** | XP cost to create |
| **Upkeep** | XP cost per faction turn to maintain |
| **HP** | Asset health. Reaches 0 → asset destroyed |
| **Attack Stat** | What roll the asset uses to attack (e.g., Force vs Force) |
| **Counter Stat** | What stat defends against attacks on this asset |
| **Attack Roll** | Dice expression for damage when attacking (e.g., "1d6") |
| **Special** | Optional special ability |

### Asset Attack Resolution

When one faction's asset attacks another:
1. Attacker rolls their `attack_roll` dice (e.g., 1d6)
2. Compare vs defender's `counter_stat` of the defending asset
3. Attack hits if roll exceeds counter (no d20 — faction combat uses direct dice comparison)
4. On hit: defender asset loses HP equal to damage rolled
5. If defending asset reaches 0 HP: asset is destroyed, removed from DB
6. When a faction loses an asset: that faction takes 1 HP damage

**Attack stat format:** `"force_vs_force"` means attacker uses Force, defender uses Force to resist. `"cunning_vs_wealth"` means attacker uses Cunning, defender uses Wealth.

### Example Assets (encode full list from WWN rulebook into `data/faction_assets.yaml`)

| Type | Category | Min | Cost | HP | Attack | Counter | Roll | Special |
|---|---|---|---|---|---|---|---|---|
| Warriors | Force | Force 1 | 2 | 6 | Force vs Force | Force | 1d6 | — |
| Informers | Cunning | Cunning 1 | 2 | 4 | Cunning vs Cunning | Cunning | 1d4 | Reveal enemy assets |
| Smugglers | Wealth | Wealth 1 | 2 | 4 | Wealth vs Wealth | Wealth | 1d4 | — |
| Militia | Force | Force 2 | 4 | 8 | Force vs Force | Force | 1d8 | — |
| Spies | Cunning | Cunning 2 | 4 | 4 | Cunning vs Force | Cunning | 1d6 | — |
| Merchant Caravan | Wealth | Wealth 2 | 4 | 6 | Wealth vs Cunning | Wealth | 1d6 | — |

Full asset list must be encoded verbatim from the WWN faction chapter into `data/faction_assets.yaml`.

## Faction Actions (one per turn)

### Attack
Target an enemy asset within range (same hex or adjacent). Roll combat as described above.
- Faction can only attack if it has at least one Force or Cunning asset
- Cannot attack Allied or Friendly factions

### Expand Influence
Move an existing asset to an adjacent hex, OR place a newly created asset in an adjacent hex.
- Cannot expand directly into hexes controlled by a Hostile faction (must Attack first)
- Each asset can move at most 1 hex per faction turn

### Create Asset
Pay the asset's XP cost. Faction must have the minimum required attribute.
- New asset appears at faction's home hex or any hex the faction controls
- Cannot exceed faction's maximum asset count (optional rule — default no cap for M4)

### Repair
Select a damaged asset. It recovers `1d6 HP`.
- Cost: XP equal to ½ the asset's purchase cost (round down, minimum 1)

### Seize Territory
Claim control of a hex that is uncontrolled or held by a weakened faction.
- Cannot seize hexes held by factions with assets present (must destroy assets first)

### Sell Asset
Remove one of the faction's assets from play.
- Faction recovers ½ the asset's purchase cost in XP

### Refit
The faction recuperates. It recovers `1d6 HP`.
- No other action taken this turn

### Harvest
The faction exploits its economic assets. It gains `1d6 XP`.
- Only available if faction has at least one Wealth asset

## Faction XP and Advancement

Factions earn XP from:
- Successful Attack action: +1 XP
- Successful Expand action: +1 XP
- Harvest action: +1d6 XP
- Player completing a faction task: +2 XP
- Selling an asset: ½ asset cost in XP

Factions spend XP to:
- Create assets (cost = asset's listed cost)
- Repair assets (cost = ½ asset cost)
- Improve stats: Force/Cunning/Wealth can be raised by spending XP equal to (new rating × 2). Max stat rating = 8.

## Faction Relationships

Factions have disposition toward each other and toward the player:

| Disposition | Effect |
|---|---|
| Allied | Never attack each other's assets. Share territory. Cooperate. |
| Friendly | Rarely conflict. May share resources. |
| Neutral | Standard competition. No cooperation, no active hostility. |
| Unfriendly | Compete actively. May take actions disadvantaging the other. |
| Hostile | Actively attack each other's assets when possible. |

Relationships are symmetric: if Iron Pact is Hostile to the player, the player is Hostile to Iron Pact.

## Faction Disposition → Encounter Table Effect

A faction's disposition toward the player directly modifies encounter tables in hexes that faction controls or patrols:

| Player-Faction Disposition | Encounter Effect |
|---|---|
| Allied | Faction patrols help the player. `patrol_hostile` effectively impossible. `patrol_friendly` increased. |
| Friendly | Reduced hostile encounters. NPCs helpful. `trade_opportunity` increased. |
| Neutral | Standard encounter tables unmodified. |
| Unfriendly | `patrol_hostile` +2. `spy_encounter` +1. NPCs suspicious. |
| Hostile | `patrol_hostile` +4. `bounty_hunter` +2. `ambush` +1. Wanted status. |

Encounter weight modifiers are stored in the `encounter_weights` SQLite table (seeded from `data/encounter_weights.yaml`). Apply them by looking up the faction controlling the current hex and the player's reputation/disposition with that faction.

## Player Reputation

Player actions change reputation with factions:

| Action | Reputation Change |
|---|---|
| Kill faction member | -10 |
| Kill faction leader | -25 |
| Complete faction task | +15 |
| Bribe faction member | +5 |
| Destroy faction asset (player action) | -20 |
| Help faction against enemy | +10 |
| Publicly humiliate faction | -15 |

Reputation score → disposition label:
```
score ≤ -30: hostile
-29 to -10: unfriendly
-9 to +9:   neutral
+10 to +29: friendly
score ≥ +30: allied
```

## Faction Turn Procedure (code this sequence)

```python
for each faction in world (ordered by Force descending):
    1. Collect upkeep: deduct asset upkeep costs from faction XP
       - If can't pay upkeep: asset degrades (loses 1 HP)
    2. Run FactionAI to select action
    3. Execute action, update DB
    4. Fire world.faction_action event
    5. Award XP for successful actions
    6. Check faction HP: if 0, faction collapses
       - Remove all assets
       - Fire world.faction_destroyed event
       - Update relationships (all factions become neutral to destroyed faction)
```

## Narration to Player

After all faction turns complete, narrate significant events:
- Asset destroyed within 3 hexes of player → "You hear rumors that [Faction]'s [Asset] was destroyed near [Location]."
- Territory seized in player's current region → "Word spreads that [Faction] has taken control of [Area]."
- Faction hunting the player (Hostile) → "You spot [Faction] patrol in the distance. They seem to be looking for someone."
- Faction destroyed → "The [Faction] has collapsed. Their former territory is now contested."

Keep narration atmospheric, not mechanical. Don't say "faction_action event fired."

## Simple AI Priority Rules (M4 — advanced AI deferred to M6)

```
1. ATTACK if:
   - Enemy asset exists within range (same or adjacent hex)
   - This faction HP > 50%
   - Target faction is NOT allied or friendly
   - Select target: lowest HP enemy asset

2. REPAIR if:
   - Any own asset at < 50% HP
   - Faction XP ≥ repair cost of cheapest damaged asset

3. CREATE ASSET if:
   - Faction XP > (most expensive affordable asset × 1.5)
   - Faction meets min_attribute for that asset
   - Prioritize: Force asset if Force is lowest stat, etc.

4. EXPAND if:
   - Faction has assets that can move
   - Adjacent uncontrolled hexes exist

5. HARVEST if:
   - Faction has Wealth assets
   - No better option above applies

6. REFIT if:
   - Faction HP < 50%
   - No other action possible
```

Allied factions are never attacked (check both directions of faction_relations before selecting attack target).
