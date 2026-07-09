# HR-107 — Adjustable Difficulty

> Status: design approved 2026-07-05. Issue: #107 (enhancement).
> Player-facing, per-world difficulty controls presented as graded easy→hard
> sliders, applied at combat/XP/loot resolver points.

## Goal

Give the player per-world, adjustable difficulty. Seven independent
dimensions, each a slider graded from **Easiest** to **Hardest**, that scale
concrete game outcomes: XP earned, to-hit (both directions), enemy HP, player
HP, loot amount, and loot probability. The design is deliberately expandable —
adding an eighth dimension is one seed row plus one runtime hook.

## Decisions (from planning)

- **UI:** a player-facing settings screen (not the `/admin` panel, not gated by
  `admin_mode`).
- **HP buff:** two independent knobs — enemy HP and player HP.
- **Presentation:** one slider per dimension, discrete grades easy→hard.
- **Scope:** all seven sliders ship together (each hook is a one-liner; the UI
  is generic).
- **Player HP is live:** moving the player-HP slider mid-game recomputes current
  max HP (base/effective split), not just future level-ups.

## Model

### Dimensions and grades

Seven dimensions. Each has **7 grades**, index `0..=6`, where **grade 3 is
Normal** (identity — ×1.0 or +0). Slider left (grade 0) is always *easier*,
right (grade 6) always *harder*, regardless of the underlying multiplier's
direction. Grade→value is an explicit constant table per dimension (no
interpolation — explicit values are transparent and unit-testable).

| key | kind | grade 0 (Easiest) | 1 | 2 | **3 (Normal)** | 4 | 5 | 6 (Hardest) |
|---|---|---|---|---|---|---|---|---|
| `enemy_hp`         | mult (f64) | 0.5 | 0.7 | 0.85 | **1.0** | 1.2 | 1.5 | 2.0 |
| `player_hp`        | mult (f64) | 1.6 | 1.4 | 1.2 | **1.0** | 0.9 | 0.8 | 0.7 |
| `xp`               | mult (f64) | 1.5 | 1.3 | 1.15 | **1.0** | 0.9 | 0.8 | 0.7 |
| `player_to_hit`    | mod (i32)  | +3 | +2 | +1 | **0** | -1 | -2 | -3 |
| `enemy_to_hit`     | mod (i32)  | -3 | -2 | -1 | **0** | +1 | +2 | +3 |
| `loot_amount`      | mult (f64) | 2.0 | 1.6 | 1.3 | **1.0** | 0.85 | 0.7 | 0.5 |
| `loot_probability` | mult (f64) | 1.6 | 1.4 | 1.2 | **1.0** | 0.85 | 0.7 | 0.5 |

Grade count (7) is fixed in code and easy to change; it sits in the requested
5–10 range and is odd so a true Normal center exists.

### Semantics per dimension

- `enemy_hp` — multiplies enemy max HP at spawn. Higher grade = tougher enemies.
- `player_hp` — multiplies the player's base max HP. Higher grade = frailer PC.
- `xp` — multiplies XP granted per encounter. Higher grade = slower leveling.
- `player_to_hit` — additive modifier on the **player's** attack rolls vs
  monsters. Higher grade = player misses more.
- `enemy_to_hit` — additive modifier on **monster** attack rolls vs the player.
  Higher grade = monsters hit more.
- `loot_amount` — multiplies dropped item count and gold. Higher grade = less.
- `loot_probability` — multiplies drop chance. Higher grade = fewer drops.

All to-hit modifiers are additive terms on the roll side of the XWN `>= AC`
compare, so the nat-1/nat-20 override and authored AC/defense are untouched.
All multipliers scale a computed outcome, never the underlying XWN tables (XP
progression, hit-die math, loot weights' relative rarity all survive).

## Storage (per-world SQLite)

A row-per-dimension config table, mirroring the `difficulty_targets` code path
so it inherits seed/read/write plumbing:

```sql
CREATE TABLE IF NOT EXISTS difficulty_settings (
    key   TEXT PRIMARY KEY,
    grade INTEGER NOT NULL
);
```

- Added to `SCHEMA_SQL` and the `REQUIRED_TABLES` list in
  `crates/harsh-core/src/db_schema.rs` (bump the array length; insert
  `"difficulty_settings"` alphabetically — a validation test enforces presence).
- Seeded at world creation to grade 3 for every key, via a new
  `seed_difficulty_settings()` called from
  `AdminService::seed_all_from_yaml` (`admin/service.rs`), which already runs
  during `create_world` (`crates/harsh-web/src/worldsvc.rs`). Defaults are
  inlined (no YAML needed — the key set is fixed in code).
- Row-per-dimension means adding a dimension later = one new seed row + one
  runtime hook, no migration.

### Player-HP base/effective split

To make the player-HP slider live, the character gains a `base_max_hp` field
(the unbuffed XWN value). Effective `max_hp = round(base_max_hp * player_hp_mult)`,
floored at 1. It is recomputed at:

1. character creation (`character_build.rs::compute_derived_stats`),
2. level-up (`advancement.rs::apply_level_up`), and
3. difficulty save of the `player_hp` key (a recalc step in the PUT handler).

On recalc, current `hp` is scaled proportionally to the new max
(`hp = round(hp * new_max / old_max)`, clamped to `1..=max_hp`) so a slider move
never leaves `hp > max_hp` or drops the PC to 0. `base_max_hp` is the source of
truth; enemy HP needs no such split (it is recomputed every spawn).

## Runtime application

### DifficultyProfile

A small value struct holding the *resolved* values:

```rust
pub struct DifficultyProfile {
    pub enemy_hp_mult: f64,
    pub player_hp_mult: f64,
    pub xp_mult: f64,
    pub player_to_hit_mod: i32,
    pub enemy_to_hit_mod: i32,
    pub loot_amount_mult: f64,
    pub loot_probability_mult: f64,
}
```

Loaded by a lightweight `DifficultySettingsRepository::load(db) ->
DifficultyProfile` (mirrors `SkillConfigRepository`), which reads the grade rows
and maps each through its const table. `Default` returns the all-Normal profile
(identity), so any code path without a DB handle degrades to vanilla XWN.

The combat scene loads the profile once at combat start and holds it; loot and
XP resolution read from that held profile. No global cache; the read is cheap
and per-encounter.

### Hook points (mapped to current `main`)

| Dimension | File:line | Application |
|---|---|---|
| player→monster to-hit | `gm/scenes/combat_scene.rs:748` (normal), `:1605` (Last Stand) | set `AttackParams.attack_modifier = profile.player_to_hit_mod` |
| monster→player to-hit | `gm/scenes/combat_scene.rs:1210` (legacy), `:1301` (IR action `actor_modifier`) | add `profile.enemy_to_hit_mod` |
| enemy HP | `combat/creation.rs:52` | `((base_hp + variation) as f64 * enemy_hp_mult).round().max(1) as i32` — after variation, so template spread survives; threaded via a param on `create_combat` |
| XP | `gm/scenes/combat_scene.rs:1506` | `(sum as f64 * xp_mult).round() as i32` before payload + narration |
| player HP | `character_build.rs` + `advancement.rs` + PUT recalc | base/effective split (above) |
| loot amount | `loot_gen.rs` (`generate_combat_loot` quantity loop + currency total) | scale item count and gold by `loot_amount_mult` |
| loot probability | `loot_gen.rs` (`roll_loot`) | scale the `type: nothing` weight (or non-nothing weights) by `loot_probability_mult`, preserving relative rarity |

`LootGenerator` gains the two loot scalars (constructor or `generate_combat_loot`
argument), set at the `combat_scene.rs:1516` construction site from the profile.
`create_combat` (`combat/creation.rs`) gains an `enemy_hp_mult` parameter.

Rounding rule everywhere: `f64::round`, then floor at the domain minimum (HP ≥ 1,
XP ≥ 0, loot count ≥ 0).

## API (player-facing, non-admin)

New routes, registered outside the `/api/admin` namespace (no `admin_mode`
gate), backed by the same `WorldDatabase` via the session actor:

- `GET /api/difficulty` → `DifficultySettingsResponse { dimensions: Vec<DifficultyDimension> }`
  where each dimension is self-describing so the frontend renders generically:

  ```ts
  interface DifficultyDimension {
    key: string;            // "enemy_hp"
    label: string;          // "Enemy HP"
    description: string;    // "Tougher enemies survive longer."
    grade: number;          // current 0..6
    grade_count: number;    // 7
    easy_label: string;     // "Weakest"
    hard_label: string;     // "Toughest"
    normal_grade: number;   // 3
    current_effect: string; // "×1.5" | "+2" — display of the resolved value at `grade`
  }
  ```

- `PUT /api/difficulty/:key` with `{ grade: number }` → validates
  `0 <= grade < grade_count` and the key; writes the row; for `player_hp` runs
  the HP recalc; returns the updated dimension.

Labels/descriptions/effect formatting live in the same Rust const table as the
values, so a new dimension is fully described in one place and appears in the UI
automatically.

## Frontend

- **`frontend/src/views/DifficultyView.vue`** (route `/difficulty`) or a game
  modal reached from the game header — a generic list rendering one labeled
  slider per dimension from `GET /api/difficulty`. Slider snaps to integer
  grades `0..grade_count-1`, endpoints labeled `easy_label`/`hard_label`, with a
  Normal tick and a live `current_effect` readout. On change, `PUT` the key.
- **`frontend/src/stores/difficulty.ts`** — `dimensions` ref, `load()`,
  `setGrade(key, grade)` (optimistic + reconcile), explicit return types.
- **`frontend/src/types/api.ts`** — `DifficultyDimension`,
  `DifficultySettingsResponse` interfaces.
- Access point: a "Difficulty" control in the game header/sidebar
  (`views/GameView.vue`), consistent with existing header links (Admin,
  Content). No `localStorage` — state is the per-world DB.

The sliders are data-driven: the frontend hardcodes no dimension keys, so
backend additions surface with zero frontend change.

## Testing

Per the four-layer rule (realized as Rust + vitest + Playwright):

**Rust unit (`crates/harsh-core`):**
- Grade→value tables: each dimension returns the documented value at grades 0,
  3, 6; grade 3 is exactly identity (×1.0 / +0); out-of-range grade clamps.
- Enemy HP: `enemy_hp_mult` scales and floors at 1; grade 3 leaves HP unchanged
  vs the pre-feature value.
- XP: scales and rounds; grade 3 unchanged; result ≥ 0.
- To-hit: `player_to_hit_mod` / `enemy_to_hit_mod` shift `total` and preserve the
  nat-1/nat-20 override and `>=` compare (a hit at Normal that a −3 turns to a
  miss, etc.).
- Loot: probability scalar shifts drop rate (statistical, seeded RNG); amount
  scalar scales item count and gold; relative rarity preserved.
- Player-HP recalc: `max_hp = round(base * mult)`, current `hp` scaled
  proportionally and clamped to `1..=max_hp`; round-trip grade 3 restores base.
- Seed: a fresh world has all seven keys at grade 3; `DifficultyProfile::default`
  is identity.

**vitest (frontend):** store `load`/`setGrade` (optimistic update + reconcile),
grade↔effect display, clamp to valid grade range.

**Playwright (`frontend/e2e/difficulty.spec.ts`):** open the difficulty screen,
assert seven sliders render with labels, move a slider, and confirm the grade
persists across a reload (per-world DB round-trip). This is the required
UI-behavior e2e.

Every change ships a regression test that fails without it.

## Files

**Backend add/edit:**
- `crates/harsh-core/src/db_schema.rs` — table DDL + `REQUIRED_TABLES`.
- `crates/harsh-core/src/difficulty.rs` (new) — grade→value const tables,
  `DifficultyProfile`, `DifficultyDimension` metadata, resolution + formatting.
- `crates/harsh-core/src/repositories/difficulty.rs` (new) —
  `DifficultySettingsRepository` (load profile, read/write grade, list
  dimensions).
- `crates/harsh-core/src/admin/service.rs` — `seed_difficulty_settings` +
  call from `seed_all_from_yaml`.
- `crates/harsh-core/src/combat/creation.rs` — `enemy_hp_mult` param.
- `crates/harsh-core/src/gm/scenes/combat_scene.rs` — load profile at combat
  start; apply to-hit mods, XP mult, loot scalars.
- `crates/harsh-core/src/loot_gen.rs` — loot amount + probability scalars.
- `crates/harsh-core/src/character_build.rs`, `advancement.rs` — `base_max_hp`
  + effective HP.
- `crates/harsh-core/src/character.rs` (or the character model module) —
  `base_max_hp` field.
- `crates/harsh-web/src/difficulty_routes.rs` (new) + `routes.rs` merge — the
  `GET/PUT /api/difficulty` surface.

**Frontend add/edit:**
- `frontend/src/views/DifficultyView.vue` (new), `stores/difficulty.ts` (new),
  `types/api.ts`, `views/GameView.vue` (access point + route), router
  registration, `e2e/difficulty.spec.ts` (new).

## Non-goals / future

- No presets (Easy/Normal/Hard bundles) — sliders only, per decision.
- Difficulty does not retroactively rescale already-spawned enemies or
  already-granted XP; it applies from the next spawn/encounter. Player HP is the
  one live-recomputed value by explicit decision.
- Config-driven grade→value mapping (min/max/steps stored in the DB) is a
  possible future expansion; for now the mapping is code-defined and the DB
  stores only the chosen grade.
