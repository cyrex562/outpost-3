# Milestone 4.7: Town Depth — Task Specification

> **Goal:** Surface town interaction mechanics through richer frontend
> and backend features. This milestone adds settlement-size-gated shop
> tiers (YAML-driven, admin-editable), a collapsible PC inventory panel,
> unit and Playwright test coverage for town flows, and minor backend
> fixes for missing edge case handling. Economic depth (variable pricing,
> shop inventory tracking, item availability) is explicitly deferred to
> a future milestone.
> **Estimated time:** 3–4 days (AI-assisted)
> **Prerequisite:** M4.6 complete (item registry must exist before shop
> tiers can reference item IDs). Read CLAUDE.md, AGENTS.md before starting.

## Success Criteria

When this milestone is complete:

1. `pytest` passes with all tests green.
2. Shop inventory is driven by settlement size tier. A hamlet offers ~4
   basic items, a village ~7 standard items, a town ~10+ items including
   better equipment. All inventories are defined in YAML and seeded into
   SQLite at world creation, editable per-world via the admin panel.
3. Shop prices are fixed per item (from `items.yaml` cost field) with no
   dynamic variation. This is explicitly a v1 simplification.
4. `look` in a settlement lists present NPCs (completed in M4.9 — verify
   still passing; do not re-implement).
5. `shop` command at a non-settlement hex returns an error message.
   This path is tested.
6. NPCs in settlements are verified persistent across re-entry (test
   confirms same NPC IDs are present on second visit).
7. A collapsible PC inventory panel exists in the frontend. It opens and
   closes via a toggle button. It lists current items with enc values and
   running total enc used / enc capacity.
8. Playwright E2E tests cover: settlement entry → shop → buy item,
   settlement entry → talk NPC, shop outside settlement → error.
9. mutmut ≥85% on new and modified modules.

---

## Task 4.7.1: Shop Tier YAML Files

> **What:** Create per-tier shop inventory YAML files. Each tier stocks
> a curated list of item IDs at fixed prices. All items must exist in the
> item registry.
> **Estimated time:** 1.5 hours (content + validation)

**Directory:** `data/shops/`

**Three files — one per settlement size tier:**

**`data/shops/hamlet.yaml`**
```yaml
tier: hamlet
description: "Minimal supplies. A travelling peddler or a farmer's surplus."
items:
  - item_id: consumable.rations
    quantity_available: null    # null = unlimited stock (v1 simplification)
  - item_id: gear.rope_50ft
    quantity_available: null
  - item_id: weapon.knife
    quantity_available: null
  - item_id: consumable.pretech_medkit
    quantity_available: null
```

**`data/shops/village.yaml`**
```yaml
tier: village
description: "General store and a blacksmith. Common equipment available."
items:
  - item_id: consumable.rations
    quantity_available: null
  - item_id: gear.rope_50ft
    quantity_available: null
  - item_id: gear.tinderbox
    quantity_available: null
  - item_id: weapon.knife
    quantity_available: null
  - item_id: weapon.short_sword
    quantity_available: null
  - item_id: armor.leather
    quantity_available: null
  - item_id: ammo.arrow
    quantity_available: null
```

**`data/shops/town.yaml`**
```yaml
tier: town
description: "Full market. Most standard equipment available for purchase."
items:
  - item_id: consumable.rations
    quantity_available: null
  - item_id: gear.rope_50ft
    quantity_available: null
  - item_id: gear.tinderbox
    quantity_available: null
  - item_id: gear.tools_basic
    quantity_available: null
  - item_id: weapon.knife
    quantity_available: null
  - item_id: weapon.short_sword
    quantity_available: null
  - item_id: weapon.longsword
    quantity_available: null
  - item_id: weapon.short_bow
    quantity_available: null
  - item_id: weapon.hand_crossbow
    quantity_available: null
  - item_id: armor.leather
    quantity_available: null
  - item_id: armor.chain_mail
    quantity_available: null
  - item_id: armor.shield
    quantity_available: null
  - item_id: ammo.arrow
    quantity_available: null
  - item_id: ammo.bolt
    quantity_available: null
  - item_id: consumable.pretech_medkit
    quantity_available: null
```

**Note on `quantity_available: null`:** v1 uses unlimited stock. A future
milestone will add inventory tracking. The field is present in the schema
to make that extension non-breaking.

**Note on deferred economics:** Prices come from the item registry
(`ItemData.cost`). No per-shop price override in v1. This is explicitly
noted in comments in each YAML file.

**Validation on load:** The shop seeder must verify that every `item_id`
in each tier file exists in the `ItemRegistry`. If any item ID is unknown,
raise a `ValueError` at startup with the offending ID.

**Tests:**
- Unit: all three tier files load without error
- Unit: every item_id in each tier resolves in ItemRegistry
- Unit: hamlet has 4 items, village has 7, town has 15

**Acceptance:** Three YAML files exist and pass validation. Item IDs
all resolve.

---

## Task 4.7.2: Shop Tier Seeding & AdminService

> **What:** Seed shop tier data into SQLite at world creation. Make it
> editable via the admin panel.
> **Estimated time:** 2 hours

**New SQLite table: `shop_tiers`**
```sql
CREATE TABLE shop_tiers (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    tier TEXT NOT NULL,              -- hamlet | village | town
    item_id TEXT NOT NULL,
    quantity_available INTEGER,      -- NULL = unlimited
    price_override INTEGER,          -- NULL = use item registry cost
    UNIQUE(tier, item_id)
);
```

**`admin/service.py`:** Add `seed_shop_tiers_from_yaml()` method.
Called from `seed_all_from_yaml()` (so it fires at world creation).

Add CRUD methods:
- `get_shop_tier(tier: str) -> list[ShopTierEntry]`
- `update_shop_tier_item(tier, item_id, quantity, price_override) -> None`
- `reset_shop_tier(tier: str) -> None`

**`admin/admin_routes.py`:** Add REST endpoints:
- `GET /api/admin/shop-tiers/{tier}` — list items for tier
- `PUT /api/admin/shop-tiers/{tier}/{item_id}` — update quantity/price
- `DELETE /api/admin/shop-tiers/{tier}/{item_id}` — remove item from tier
- `POST /api/admin/shop-tiers/{tier}/reset` — re-seed from YAML

**Admin Vue panel:** Add "Shop Tiers" tab to the admin panel with three
sub-tabs (Hamlet, Village, Town). Each shows the item list with inline
price override editing. Save and Reset buttons per tier.

**`gm/scenes/shopping.py`:** Replace hardcoded 10-item inventory with
a query to `shop_tiers` table based on the settlement's size field.

Settlement size mapping:
- `hamlet` → tier "hamlet"
- `village` → tier "village"
- `town` or `city` → tier "town"

If settlement has no size field (legacy data): default to "village".

**Tests:**
- Unit: `seed_shop_tiers_from_yaml()` populates all three tiers correctly
- Unit: `get_shop_tier("hamlet")` returns 4 items
- Unit: `get_shop_tier("town")` returns 15 items
- Unit: shop scene in hamlet shows only hamlet-tier items
- Unit: shop scene in town shows town-tier items
- Unit: reset re-seeds from YAML, overwriting any edits
- Unit: `price_override` non-null → use override price; null → use registry cost
- Playwright: Admin "Shop Tiers" tab renders three sub-tabs
- Playwright: editing a price override and saving persists the change

**Acceptance:** Shop inventory scales with settlement size. Admin panel
allows per-world customisation of shop contents and prices.

---

## Task 4.7.3: Shop Outside Settlement — Error Handling & Test

> **What:** Verify and test that `shop` command outside a settlement
> returns a clear error. Add missing rejection-path test.
> **Estimated time:** 0.5 hours

**Audit finding:** `_handle_shop()` checks for settlement feature and
returns an error, but no test covers this path.

**File:** `tests/test_shopping.py`

Add:
```python
async def test_shop_outside_settlement_returns_error(game):
    # Move character to a non-settlement hex
    # Send "shop" command
    # Assert response contains "not in a settlement" (or equivalent)
```

If the error message is not user-friendly, update `_handle_shop()` to
return: `"There's nowhere to shop here. You need to be in a settlement."`

**Tests:**
- Unit: `shop` at plains hex → error message
- Unit: `shop` at ruins hex → error message
- Unit: `shop` at settlement hex → shop opens (existing test — verify still passes)

**Acceptance:** Rejection path tested and produces a clear player-facing message.

---

## Task 4.7.4: NPC Persistence Verification

> **What:** Add tests confirming that settlement NPCs persist across
> re-entry and that disposition changes survive re-entry.
> **Estimated time:** 1 hour

**Audit finding:** NPCs stored in `entities` table (correct) but no test
verifies they survive a settlement exit and re-entry.

**File:** `tests/test_npc_persistence.py` (new)

```python
async def test_npcs_persist_across_settlement_reentry(game):
    # Enter settlement, get NPC IDs via look
    # Leave settlement (move to adjacent hex)
    # Re-enter settlement
    # Assert same NPC IDs present

async def test_npc_disposition_persists_across_reentry(game):
    # Enter settlement, talk to NPC, convince → disposition changes to +1
    # Leave settlement
    # Re-enter settlement
    # Assert NPC disposition is still +1 (not reset to 0)

async def test_dead_npc_not_present_after_reentry(game):
    # Enter settlement, trigger combat with NPC (disposition → -3)
    # Defeat NPC in combat
    # Leave and re-enter settlement
    # Assert dead NPC not listed in look
```

**Acceptance:** Three persistence tests pass. Settlement NPCs are
genuinely persistent, not re-generated on entry.

---

## Task 4.7.5: PC Inventory Panel

> **What:** Add a collapsible inventory panel to the frontend showing
> the PC's current items, enc values, and enc capacity.
> **Estimated time:** 3 hours

**Component:** `frontend/src/components/InventoryPanel.vue` (new)

**Toggle:** Add an "Inventory" button to the main game layout (near
the StatusSidebar or as a floating button). Clicking opens/closes the panel.
The panel is closed by default.

**Panel contents:**
```
INVENTORY
━━━━━━━━━━━━━━━━━━━━━
Equipped:
  ⚔ Short Sword (1 enc)
  🛡 Leather Armor (1 enc)

Stowed:
  Rope (50 ft) (1 enc)
  Rations × 3 (1 enc)
  Crossbow Bolts × 14 (1 enc)

━━━━━━━━━━━━━━━━━━━━━
Enc: 5 / 10
```

**Data source:** The character API response already includes equipment
data. Add an `inventory` field to `CharacterState` in the game store
populated from the API response.

**Enc capacity calculation:**
- Readied slots = STR score ÷ 2 (round down), minimum 4
- Stowed slots = STR score (typically 10 = 10 stowed slots)
- For v1 simplification: show total enc used vs. a single capacity
  value. Readied/stowed distinction deferred to future milestone.

**WebSocket updates:** On `shopping.purchase`, `shopping.sale`,
and `gm.scene_change` events, re-fetch character data from
`GET /api/character` to refresh inventory panel contents.

**Deferred explicitly:**
- Item detail view (click item to see description)
- Item encyclopedia links
- Readied vs. stowed slot distinction
- Drag-and-drop reordering

**Tests:**
- Playwright: inventory button visible in main game UI
- Playwright: clicking button opens panel with item list
- Playwright: clicking button again closes panel
- Playwright: after buying item in shop, panel shows new item
- Playwright: enc total updates correctly after purchase

**Acceptance:** Panel opens and closes. Shows current items and enc.
Updates after transactions.

---

## Task 4.7.6: Playwright E2E — Town Flows

> **What:** Write Playwright E2E tests for the core settlement
> interaction flows.
> **Estimated time:** 4 hours

**File:** `frontend/tests/e2e/town.spec.ts` (new)

**Setup:** `beforeAll` creates a world and character, pathfinds to
a settlement hex (use a known settlement from world seed or generate
a world with settlement at a predictable location).

**Test flows:**

```typescript
test('enter settlement and explore town', async ({ page }) => {
  // Send "explore town"
  // Assert response contains NPC names and establishment names
})

test('shop in settlement — buy item', async ({ page }) => {
  // Send "shop"
  // Assert shop listing appears with items
  // Send "buy rations"
  // Assert purchase message appears in ChatLog
  // Assert gold display in sidebar decremented
  // Assert rations appear in inventory panel
})

test('shop outside settlement — error', async ({ page }) => {
  // Navigate to non-settlement hex
  // Send "shop"
  // Assert error message in chat
})

test('talk to NPC in settlement', async ({ page }) => {
  // Send "look" — assert NPC names visible
  // Send "talk <npc_name>"
  // Assert social scene entered (response contains NPC personality)
  // Send "leave"
  // Assert returned to exploration
})

test('NPC disposition persists across re-entry', async ({ page }) => {
  // Talk to NPC, send "convince" until disposition changes
  // Move away and back
  // Talk to same NPC — assert disposition still changed
})

test('inventory panel opens and shows items', async ({ page }) => {
  // Click inventory toggle
  // Assert panel visible with equipped weapon and armor
  // Assert enc total displayed
})
```

**Acceptance:** All 6 E2E tests pass. Zero console errors during flows.

---

## Dependency Order

```
4.7.1 (shop YAML) → must be before 4.7.2 (seeder needs files)
  ↓
4.7.2 (seeding + admin + shopping scene) → needs M4.6 item registry
4.7.3 (shop rejection test) → independent, do first
4.7.4 (NPC persistence tests) → independent, no code changes
4.7.5 (inventory panel) → needs character API data (already exists)
  ↓
4.7.6 (Playwright E2E) → needs 4.7.2 + 4.7.5 complete
```

Recommended order:
1. 4.7.3 (shop rejection test — 30 min, fast win)
2. 4.7.4 (NPC persistence tests — 1h, no code changes)
3. 4.7.1 (shop YAML content)
4. 4.7.2 (seeding + admin panel + shopping scene update)
5. 4.7.5 (inventory panel)
6. 4.7.6 (Playwright E2E — last)

---

## Explicitly Deferred to Future Milestones

The following items are intentionally out of scope for M4.7. Do not
implement them. They are recorded here to prevent scope creep.

| Item | Future milestone |
|------|-----------------|
| Dynamic pricing (supply/demand, haggling) | M6+ (economics) |
| Shop inventory tracking (items sell out) | M6+ (economics) |
| Item quality tiers (poor/standard/fine) | M6+ (economics) |
| Per-NPC shopkeeper personality affecting price | M6+ (economics) |
| Item detail view in inventory panel | M5+ (inventory depth) |
| Item encyclopedia / lore entries | M6+ (content) |
| Readied vs. stowed slot distinction in panel | M5 (inventory system) |
| Dedicated ShoppingPanel component | M6+ (UI polish) |
| Dedicated SocialPanel component | M6+ (UI polish) |
| `visit <establishment>` command | M6+ (town depth) |

---

## Notes for the Coding Agent

- Read CLAUDE.md and AGENTS.md before starting.
- M4.6 item registry (Task 4.6.2) must be complete before this milestone.
  Specifically: `ItemRegistry` must be loadable and all item IDs in
  `data/items/` must be registered before shop tier YAML can be validated.
- The `quantity_available: null` field in shop tier YAML means unlimited
  stock. In v1 the shopping scene never decrements stock. Do not add stock
  tracking logic — leave the field as metadata for the future milestone.
- Settlement size field: check `SettlementGenerator` or the `entities`
  table for how settlement size is currently stored. The size values may
  be `hamlet`, `village`, `town` or may use different labels. Map them
  to the three tier names; document any mapping in a comment.
- The inventory panel is read-only in v1. Do not add drag-and-drop,
  item use, or equip/unequip from the panel. The command interface
  remains the primary interaction mechanism.
- After completing all tasks, update CLAUDE.md:
  - Mark Milestone 4.7 complete with date
  - Record final test count
  - Document the v1 simplifications (unlimited stock, fixed prices)
    in the deviations section
