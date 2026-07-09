# Harsh Realm — Content Key Reference

_Generated from the IR schema (SCHEMA_VERSION 0.2.0). Do not edit by hand — run `hrctl content reference` to regenerate._

Each record is one YAML document with a single top-level key (the record type). Effects in `do:` lists use the verb-first form (see the template); the keys below are the raw record fields.

## Record types

### `creature`

A creature/monster record. Mirrors `CreatureData` + the action-model fields.

| key | type | required | description |
|---|---|---|---|
| `ac` | integer | **yes** |  |
| `actions` | list of string | no | Action ids the creature can perform (subsumes the former `innate_attacks`). |
| `attack_bonus` | integer | **yes** |  |
| `attack_description` | string | no |  |
| `awareness_difficulty` | integer | no |  |
| `behavior` | string | no |  |
| `damage` | string | **yes** |  |
| `damage_type` | string | no |  |
| `defenses` | object | no | Named defenses. Empty → the host derives `{ac}` from the `ac` field. |
| `description_seen` | string | no |  |
| `description_short` | string | no |  |
| `description_unseen` | string | no |  |
| `flee_difficulty` | integer | no |  |
| `harvestable` | [`Harvestable`](#-harvestable) | no |  |
| `hd` | integer | **yes** |  |
| `hp_per_hd` | integer | no |  |
| `id` | string | **yes** |  |
| `loot_table` | string (optional) | no |  |
| `morale` | integer | no |  |
| `name` | string | **yes** |  |
| `num_attacks` | integer | no |  |
| `pools` | list of [`Pool`](#-pool) | no | Typed health pools. Empty → the host derives an `hp` pool from `hd * hp_per_hd`. |
| `special_abilities` | list of string | no |  |
| `tags` | list of string | no |  |
| `unavoidable` | boolean | no |  |
| `xp_value` | integer | no |  |

### `item`

An item record. Mirrors `ItemData` + `grants_traits`.

| key | type | required | description |
|---|---|---|---|
| `ac` | integer (optional) | no |  |
| `ac_bonus` | integer (optional) | no |  |
| `ammo_type` | string (optional) | no |  |
| `attribute` | string (optional) | no |  |
| `cost` | integer | no |  |
| `damage` | string (optional) | no |  |
| `damage_type` | string (optional) | no |  |
| `effect` | [`ItemEffect`](#-itemeffect) | no |  |
| `enc` | number | no | Encumbrance (may be fractional). |
| `grants_traits` | list of string | no | Traits this item grants while equipped (carries the item's modifiers — C2). |
| `id` | string | **yes** |  |
| `name` | string | **yes** |  |
| `power_charges` | integer (optional) | no |  |
| `range_band` | string (optional) | no |  |
| `save_bonuses` | [`SaveBonusProfile`](#-savebonusprofile) | no |  |
| `shock_ac_threshold` | integer | no |  |
| `shock_damage` | integer | no |  |
| `tags` | list of string | no |  |
| `weapon_tags` | list of string | no |  |

### `trait`

A trait, feature, edge, advantage, gift, feat, talent, or ability record.

| key | type | required | description |
|---|---|---|---|
| `category` | string | **yes** | advantage, disadvantage, edge, hindrance, gift, feat, talent, etc. |
| `conflicts` | list of string | no | Qualified trait IDs that cannot coexist with this trait. |
| `cost` | [`TraitCost`](#-traitcost) | no |  |
| `description` | string | no |  |
| `id` | string | **yes** |  |
| `modifiers` | list of [`Modifier`](#-modifier) | no |  |
| `name` | string | **yes** |  |
| `prerequisites` | list of [`Prerequisite`](#-prerequisite) | no |  |
| `provides_tags` | list of string | no |  |
| `source` | string (optional) | no |  |
| `tags` | list of string | no |  |
| `triggers` | list of [`Trigger`](#-trigger) | no |  |

### `status_effect`

A status effect content record.

| key | type | required | description |
|---|---|---|---|
| `default_duration_ticks` | integer | no | 0 means permanent until explicitly removed. |
| `description` | string | no |  |
| `icon` | string (optional) | no |  |
| `id` | string | **yes** |  |
| `modifiers` | list of [`Modifier`](#-modifier) | no |  |
| `name` | string | **yes** |  |
| `provides_tags` | list of string | no |  |
| `stacking` | one of `replace` · `extend` · `stack` | no |  |
| `tags` | list of string | no |  |
| `triggers` | list of [`Trigger`](#-trigger) | no |  |

### `tag`

A tag definition. Tags are lightweight markers; `implies` lets one tag pull in others for query purposes.

| key | type | required | description |
|---|---|---|---|
| `description` | string | no |  |
| `id` | string | **yes** |  |
| `implies` | list of string | no |  |

### `action`

An action record (L1 archetype or L2 concrete via `base`).

| key | type | required | description |
|---|---|---|---|
| `activation` | [`Activation`](#-activation) | no |  |
| `base` | string (optional) | no | The archetype this action specializes (L2); `None` for an L1 archetype. |
| `id` | string | **yes** |  |
| `kind` | one of `contest` · `apply` | no |  |
| `modifiers` | list of [`Modifier`](#-modifier) | no | Inline modifiers on the action itself (e.g. aimed shot +1, snap shot −2). |
| `outcome` | list of [`OutcomeBranch`](#-outcomebranch) | no |  |
| `prerequisites` | list of [`ActionPrerequisite`](#-actionprerequisite) | no |  |
| `resolution` | [`ActionResolution`](#-actionresolution) | no |  |
| `tags` | list of string | no |  |
| `targeting` | [`Targeting`](#-targeting) | no |  |

### `skill`

A skill record (L3 overlay): governs/modifies actions and grants actions.

| key | type | required | description |
|---|---|---|---|
| `governs` | list of [`SkillGoverns`](#-skillgoverns) | no |  |
| `grants` | list of string | no | Actions/reactions this skill makes available. |
| `id` | string | **yes** |  |
| `name` | string | **yes** |  |
| `tags` | list of string | no |  |
| `triggers` | list of [`Trigger`](#-trigger) | no | Special reactions/triggers the skill adds. |

### `trigger`

A declarative trigger: an event subscription with a condition and effects.

Mirrors `triggers/schema.py`. `when` is the *string* form of a condition expression (the R1 parser turns it into a [`super::condition::Expr`]); it defaults to `"true"` (always fires).

| key | type | required | description |
|---|---|---|---|
| `description` | string | no |  |
| `do` | list of [`Effect`](#-effect) | no | Effects to run when the trigger fires, in order. |
| `id` | string | **yes** | Unique within the owning record (trait/status/etc). |
| `on` | string | **yes** | Event type to subscribe to. |
| `when` | string | no | Condition expression in string form. Default: `"true"`. |

### `table`

A random table content record. Entries stay open (`serde_json::Value`) until the table engine is ported in R4 and can formalize entry shapes.

| key | type | required | description |
|---|---|---|---|
| `category` | string | **yes** |  |
| `entries` | list of any | no |  |
| `id` | string | **yes** |  |
| `name` | string | **yes** |  |
| `tags` | list of string | no |  |

### `procedure`

A declarative, multi-step generator.

| key | type | required | description |
|---|---|---|---|
| `description` | string | no |  |
| `id` | string | **yes** |  |
| `inputs` | list of [`ProcedureInput`](#-procedureinput) | no |  |
| `name` | string | no |  |
| `output` | [`ProcedureOutput`](#-procedureoutput) | no |  |
| `steps` | list of [`ProcedureStep`](#-procedurestep) | no |  |
| `tags` | list of string | no |  |

## Shared sub-objects

### `ActionResolution`

A contest's resolution config.

| key | type | required | description |
|---|---|---|---|
| `roll_spec` | [`RollSpec`](#-rollspec) | **yes** |  |
| `roller` | one of `actor` · `defender` | no |  |
| `tn_source` | [`TnSource`](#-tnsource) | **yes** |  |

### `Activation`

An action's activation spec.

| key | type | required | description |
|---|---|---|---|
| `cooldown` | integer (optional) | no |  |
| `costs` | list of [`Cost`](#-cost) | no |  |
| `economy` | [`Economy`](#-economy) | no |  |
| `uses` | [`Uses`](#-uses) | no |  |

### `Cost`

A resource cost (PP, strain, slots, ammo, charges — all resources).

| key | type | required | description |
|---|---|---|---|
| `amount` | integer | **yes** |  |
| `resource` | string | **yes** |  |

### `Effect`

A declarative effect: a verb (`kind`) plus an open map of parameters.

The set is intentionally open (a plain string) so packs can introduce new verbs; [`EffectKind`] documents the verbs the core engine lowers to intents.

| key | type | required | description |
|---|---|---|---|
| `kind` | string | **yes** |  |
| `params` | object | no |  |

### `Harvestable`

A material harvestable from a creature's corpse.

| key | type | required | description |
|---|---|---|---|
| `difficulty` | integer | no |  |
| `material` | string | **yes** |  |
| `skill` | string | no |  |

### `Modifier`

A single modifier contribution from some source.

| key | type | required | description |
|---|---|---|---|
| `condition` | [`ModifierCondition`](#-modifiercondition) | no |  |
| `description` | string | no |  |
| `priority` | integer | no | For replace mode; higher priority wins. |
| `stacking` | one of `additive` · `multiplicative` · `replace` · `max` · `min` | no |  |
| `target` | string | **yes** | Namespaced target, e.g. `attribute.str`. |
| `value` | integer | **yes** | Magnitude; sign indicates bonus or penalty. |

### `ModifierCondition`

A modifier condition: a predicate plus an optional argument.

| key | type | required | description |
|---|---|---|---|
| `arg` | string (optional) | no |  |
| `predicate` | one of `always` · `entity_has_tag` · `target_has_tag` · `entity_has_trait` | **yes** |  |

### `OutcomeBranch`

One outcome branch: a `when` predicate over the result (`crit`/`success`/ `miss`/`fumble`, or a `degree`/`raw_margin` expression) and the effects to run.

| key | type | required | description |
|---|---|---|---|
| `do` | list of [`Effect`](#-effect) | no |  |
| `when` | string | **yes** |  |

### `Pool`

A typed health pool on an entity.

| key | type | required | description |
|---|---|---|---|
| `can_go_negative` | boolean | no |  |
| `current` | integer | **yes** |  |
| `id` | string | **yes** |  |
| `max` | integer | **yes** |  |
| `tags` | list of string | no | Routing tags, e.g. `["sd"]`, `["md"]`, `["hp"]`. A pool tagged `md` only takes `Md` packets; others take `Sd`. |

### `Prerequisite`

A trait prerequisite.

| key | type | required | description |
|---|---|---|---|
| `arg` | string | **yes** |  |
| `kind` | one of `trait` · `attribute_min` · `level_min` · `skill_min` | **yes** |  |
| `value` | integer | no |  |

### `ProcedureInput`

A named input parameter accepted by a procedure.

| key | type | required | description |
|---|---|---|---|
| `default` | any | no |  |
| `name` | string | **yes** |  |
| `required` | boolean | no |  |
| `type` | one of `string` · `integer` · `boolean` · `any` | no |  |

### `ProcedureOutput`

An explicit output field mapping for a procedure (`var.path` per output key).

| key | type | required | description |
|---|---|---|---|
| `fields` | object | **yes** |  |

### `ProcedureStep`

One executable step in a declarative procedure.

| key | type | required | description |
|---|---|---|---|
| `assign` | string | **yes** | Output variable name. |
| `count` | integer | no |  |
| `function` | string (optional) | no |  |
| `kind` | one of `roll` · `compute` · `procedure` · `format` | **yes** |  |
| `params` | object | no |  |
| `procedure` | string (optional) | no |  |
| `table` | string (optional) | no |  |
| `template` | string (optional) | no |  |

### `RollSpec`

What the roll draws on. `mechanic` defaults to the world's mechanic; `skill`/ `attribute` may be null (e.g. a save the defender rolls).

| key | type | required | description |
|---|---|---|---|
| `attribute` | string (optional) | no |  |
| `mechanic` | [`RollMechanic`](#-rollmechanic) | no |  |
| `skill` | string (optional) | no |  |

### `SaveBonusProfile`

Save bonuses an item confers (mirrors `SaveBonusProfile`).

| key | type | required | description |
|---|---|---|---|
| `evasion` | integer | no |  |
| `luck` | integer | no |  |
| `mental` | integer | no |  |
| `physical` | integer | no |  |

### `SkillGoverns`

One action a skill governs: per-level modifiers, optionally gated by a tag.

| key | type | required | description |
|---|---|---|---|
| `action` | string | **yes** |  |
| `per_level` | list of [`Modifier`](#-modifier) | no |  |
| `requires_tag` | string (optional) | no |  |

### `Targeting`

An action's targeting spec.

| key | type | required | description |
|---|---|---|---|
| `range_band` | string (optional) | no |  |
| `reach` | integer | no | Extra cells of reach beyond the band's base (e.g. a polearm). Default 0. |
| `requires_los` | boolean | no |  |
| `shape` | [`TargetShape`](#-targetshape) | **yes** |  |

### `TraitCost`

How acquiring a trait costs a character in a source system.

| key | type | required | description |
|---|---|---|---|
| `description` | string | no |  |
| `points` | integer | no |  |
| `slot` | string (optional) | no |  |

### `Trigger`

A declarative trigger: an event subscription with a condition and effects.

Mirrors `triggers/schema.py`. `when` is the *string* form of a condition expression (the R1 parser turns it into a [`super::condition::Expr`]); it defaults to `"true"` (always fires).

| key | type | required | description |
|---|---|---|---|
| `description` | string | no |  |
| `do` | list of [`Effect`](#-effect) | no | Effects to run when the trigger fires, in order. |
| `id` | string | **yes** | Unique within the owning record (trait/status/etc). |
| `on` | string | **yes** | Event type to subscribe to. |
| `when` | string | no | Condition expression in string form. Default: `"true"`. |

### `Uses`

A cap on how many times an action may be used within a scope.

| key | type | required | description |
|---|---|---|---|
| `n` | integer | **yes** |  |
| `scope` | one of `round` · `scene` · `day` | **yes** |  |
