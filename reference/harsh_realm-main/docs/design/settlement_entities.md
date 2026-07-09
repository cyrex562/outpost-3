# Settlement Entity Hierarchy

> Settlements are promoted to first-class entities in the database.
> This pattern should be used for future features that need structured
> world content hierarchies.

## Entity Types

| entity_type | Parent | Description |
|---|---|---|
| `settlement` | (hex) | A settlement at a hex location. References hex via location_q/r. |
| `building` | settlement | A building within a settlement. References parent via `settlement_id` in data. |
| `npc` | building or settlement | An NPC. May have `building_id` in data if they're a building operator. |

## Data Column Schemas

### Settlement Entity (`entity_type='settlement'`)
```json
{
  "name": "Ashford",
  "size": "village",
  "description": "A quiet village nestled in the hills.",
  "building_ids": ["building-uuid-1", "building-uuid-2"],
  "resident_npc_ids": ["npc-uuid-1", "npc-uuid-2"]
}
```

### Building Entity (`entity_type='building'`)
```json
{
  "building_type": "blacksmith",
  "building_name": "The Rusty Anvil",
  "tier": "medium",
  "settlement_id": "settlement-uuid",
  "operator_npc_id": "npc-uuid"
}
```

## Settlement Size → Building Tier

| Settlement Size | Building Tier | Required Buildings |
|---|---|---|
| hamlet | small | healer, general_store |
| village | medium | healer, general_store, blacksmith |
| town | large | healer, general_store, blacksmith, tavern |
| city | large | healer, general_store, blacksmith, tavern |

## Shop Inventory

Each building type has a YAML file in `data/shops/` with inventory tiers:
- `data/shops/blacksmith.yaml` — weapons, armor, ammo
- `data/shops/general_store.yaml` — food, rope, torches, basic gear
- `data/shops/healer.yaml` — medical supplies, healing items
- `data/shops/tavern.yaml` — food, drink

Tier determines which items are available. A hamlet blacksmith won't
stock plate armor; a town emporium has everything.

## Player Interaction

```
explore town → lists buildings and NPCs
shop blacksmith → enters blacksmith shop with tier-appropriate inventory
shop → if only one shop type, enters it; otherwise lists available shops
talk <npc> → social scene with NPC
```

## Future: Economic Model

The current tier system uses static YAML definitions. A future milestone
will replace this with a dynamic economic model where:
- Building inventory is generated based on local resources
- Trade routes affect item availability
- Faction control affects prices
- Supply/demand affects stock levels
