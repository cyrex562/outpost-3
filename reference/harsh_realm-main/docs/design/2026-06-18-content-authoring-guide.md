# Harsh Realm — Content Authoring Guide

**Date:** 2026-06-18
**Audience:** the person writing content (you)
**Companion doc:** `2026-06-18-content-grammar-compiler-spec.md` (how the compiler works, for the implementing agent)

This guide teaches you how to write game content as YAML — monsters, items, NPCs, feats,
status effects, random tables, and generators — without touching the engine, *as long as
the mechanics you need already exist*. When they don't, the compiler tells you exactly what
to build first. That boundary is the whole point of the system.

> **⚠️ Two surfaces, read §8 for what runs today.** Sections 1–7 describe the *aspirational
> authoring sugar* (the verb-first "form 2") and were written before the runtime existed; a
> few examples are intentionally-unimplemented teaching cases (e.g. `world.tick`). **[§8 —
> The live IR runtime](#8-the-live-ir-runtime--triggers-effects-and-the-damage-model-as-implemented)**
> documents the **raw IR** (`kind`/`params`) that the engine actually consumes today, the real
> effect/event vocabulary, and worked examples drawn from the shipped, test-pinned starter pack
> ([content/xwn-core/content/ir/starter.yaml](../../content/xwn-core/content/ir/starter.yaml)).
> If you are authoring content to run *now*, write raw IR and follow §8. See also the
> [IR interpreter roadmap](2026-06-28-ir-interpreter-roadmap.md) and the "Design Vision" section
> of [AGENTS.md](../../AGENTS.md).

---

## 1. The big picture

You author content in YAML. A compiler (written in Rust) turns it into **IR records** — the
engine's canonical data format — and validates them. The IR is the source of truth. The YAML
is just the way content gets *in*; once compiled, the YAML is discarded.

```
You write YAML  ─────────────┐
                              ├──► Rust compiler ──► IR records ──► stored (canonical)
Content inspector (GUI) edits ┘                     (validated)
```

Two consequences worth internalizing:

- **There are two ways to author: bulk YAML and the content inspector.** They produce the same
  IR. Write a lot at once in YAML; tweak individual records later in the inspector.
- **The content inspector is not the god-view.** The content inspector edits *content* (the
  rulebook). The god-view inspector browses *live world state* (a savegame). Different tools,
  different data. This guide is only about content.

---

## 2. The four things you author

Almost everything you'll write is one of four shapes. Getting these four mental models right
makes the rest mechanical.

### Entities — typed stat blocks that reference rules

An entity is a flat record of stats (`creature`, `item`, `npc`, `character`). It mostly holds
plain numbers and strings, **plus references to rules records by ID**. A monster doesn't
*contain* its special ability; it lists the ability's ID, and the ability is its own record.
That reference-by-ID is the seam between "stat block" and "rules."

### Rules — graphs of records rooted at a container

A "rule" (a feat, a creature ability, a magic-like effect) is almost never one record. It's a
**container** record — usually a `trait` — that owns some pieces inline (modifiers, triggers)
and references others by ID (statuses, tags). Triggers hold effects; effects reference statuses
and resources. Authoring a new rule means wiring that graph. No new code is needed **as long as
every node in the graph already exists in the engine.**

### Tables — side-effect-free oracles

A `table` maps a roll (or a weight) to a result. It *returns a value*; it never *does* anything.
A wandering-monster table returns `{spawn: scrip-hound, count: "1d4"}` — it doesn't spawn the
hound. Whatever consumes the roll interprets the result. Keeping tables inert is what makes them
reusable and deterministic.

### Generators — recipes that compose tables

A `procedure` is an ordered recipe: roll on a table, call another procedure, run a registered
function, format a string, emit a structured result. "Make a scavenger NPC," "generate a
settlement," UNE personality — all procedures. When a recipe needs *real logic* (conditional
math, anything that isn't roll/lookup/format), it uses a `compute` step that calls a registered
Python function by name. That `compute` step is the controlled escape hatch into code.

> **Rule of thumb:** tables are nouns-of-chance, procedures are recipes, `compute` is where a
> recipe calls code.

---

## 3. How you write things — the sugar dial

The surface deliberately mixes two styles, and the split is not arbitrary:

- **Structured YAML keys** for everything stable and enumerable: which record type, the target,
  the event, stacking, duration, and **verbs with named arguments**.
- **Quoted strings** for the only two things that are genuinely little expression languages:
  **dice** (`2d8+2`) and **conditions** (`event.hit and target.hp > 0`).

Here is the same effect written three ways. You write **form 2**. Forms 1 and 3 exist so you
understand what's happening underneath.

```yaml
# Form 1 — string micro-syntax (terse, but needs a parser; not the default)
do: ["apply bleeding to target for 3"]

# Form 2 — keyed verb-first  ← THIS IS HOW YOU WRITE IT
do:
  - apply: bleeding
    to: target
    for: 3

# Form 3 — raw IR (what form 2 compiles to; you never write this by hand)
do:
  - { kind: apply_status, params: { status_id: bleeding, entity_id: target, duration_ticks: 3 } }
```

Form 2 is verb-first (the verb is the key), unambiguous, and reads close to the source text. New
verbs are added to the engine's verb registry without changing the grammar, so the vocabulary
grows but the way you write never does.

---

## 4. Quick reference

### Record types

| You write | Produces | Use it for |
|---|---|---|
| `creature:` | creature entity record | monsters, beasts, engineered horrors |
| `npc:` | npc entity record | people you can talk to |
| `character:` | character entity record | the player (rarely hand-authored) |
| `item:` | item entity record | weapons, armor, gear, consumables |
| `trait:` | trait record | feats, creature abilities, granted bonuses |
| `status_effect:` | status_effect record | poisoned, bleeding, blessed, corroded |
| `tag:` | tag record | category labels with implications |
| `trigger:` | trigger record | standalone event→effect rules |
| `table:` | table record | random/lookup tables |
| `procedure:` | procedure record | generators |

### Effect verbs (current vocabulary)

Write these as keyed forms. Each verb's argument keys come from its registry signature.

| Verb | Keys | Compiles to |
|---|---|---|
| `damage:` / `heal:` | `<resource-or-entity>`, `amount` | `change_resource` |
| `change:` | resource, `by`, `on` (entity) | `change_resource` |
| `apply:` | `<status>`, `to`, `for` | `apply_status` |
| `remove:` | `<status>`, `from` | `remove_status` |
| `add_modifier:` | `<modifier>`, `to`, `source` | `apply_modifier` |
| `drop_modifier:` | `source`, `from` | `remove_modifier` |
| `emit:` | `<event>`, `with` (payload) | `emit_event` |
| `log:` | `<message>` | `log` |
| `roll:` / `run:` | (compute-style; see procedures) | `roll_dice` / `run_procedure` |

If you need a verb that isn't here, write it anyway — the compiler will report it as an
unimplemented primitive (see §6), and you add it to the registry once.

### Condition cheat-sheet (the existing condition DSL — unchanged)

- **Paths:** `entity` `event` `world` `target` `self` `local`, dotted: `entity.hp`, `event.damage`.
- **Comparison:** `==` `!=` `<` `<=` `>` `>=` (word forms `eq` `neq` `lt` `lte` `gt` `gte`), plus `in`.
- **Boolean:** `and` `or` `not`. **Arithmetic:** `+` `-` `*` `/`.
- **Functions:** `has_tag(e,t)` `has_trait(e,t)` `has_status(e,s)` `len(x)` `min(...)` `max(...)` `abs(x)`.
- **Literals:** ints, floats, `'strings'`, `true`, `false`, `null`, `[lists]`.

Example: `has_tag(self, 'flammable') and entity.hp > 0`

### Dice

`1d6`, `2d8+2`, `1d4-1`, `1d100`. A bare integer (`3`) is a flat value. Dice always live in a
string field or a string value.

---

## 5. Worked examples

### 5.1 A monster with an attack and a special ability

The creature is a flat stat block; its special ability is a separate `trait` referenced by ID;
the ability applies a `status_effect`.

```yaml
creature:
  id: scrip-hound
  name: Scrip-Hound
  hd: 2
  hp_per_hd: 8
  ac: 14
  attack_bonus: 3
  damage: 1d8
  damage_type: melee
  attack_skill: stab
  attack_description: "snaps with corroded fangs"
  num_attacks: 1
  behavior: aggressive
  awareness_difficulty: 8
  flee_difficulty: 9
  unavoidable: false
  morale: 9
  loot_table: scrip-hound-loot
  harvestable: { material: corrosive-gland, skill: Heal, difficulty: 9 }
  description_unseen: "Something metallic clicks in the dark."
  description_seen: "A skeletal hound plated in scavenged armor, jaws weeping acid."
  description_short: "scrip-hound"
  xp_value: 30
  tags: [engineered, beast, guard]
  special_abilities: [corrosive-bite]   # ← reference to a trait record
  innate_attacks: []
```

```yaml
trait:
  id: corrosive-bite
  name: Corrosive Bite
  category: creature-ability
  description: "A successful bite corrodes the target's armor."
  triggers:
    - on: combat.attack
      when: "event.attacker eq self.id and event.hit and event.attack_skill eq 'stab'"
      do:
        - apply: corroded
          to: target
          for: 0          # 0 ticks = until removed
```

```yaml
status_effect:
  id: corroded
  name: Corroded Armor
  default_duration_ticks: 0
  stacking: stack
  modifiers:
    - target: combat.ac    # ← see note
      value: -1
      stacking: additive
      description: "-1 AC while armor is corroded"
```

> **Note on `combat.ac`:** modifier targets are a registered vocabulary (`attribute.str`,
> `skill.stab`, …). If `combat.ac` isn't a registered target yet, the compiler will list it as an
> unimplemented primitive — telling you to register an AC modifier target before this status can
> change combat math. This is normal: you found a gap.
>
> **Note on the trigger payload:** `event.attacker`, `event.hit`, `event.attack_skill` assume the
> `combat.attack` payload carries those fields. Confirm against the real payload; an unknown
> payload field is a validation error you'll see at compile time.

### 5.2 A combat feat with a conditional bonus and a triggered effect

One `trait` that owns a conditional modifier and a trigger, references a status and a tag, and
has a prerequisite. This is the canonical "a rule is a graph rooted at a trait" shape.

```yaml
trait:
  id: sand-reaver-training
  name: Sand-Reaver Training
  category: combat
  description: "Brutal desert-raider knife-work."
  prerequisites:
    - { kind: attribute_min, arg: DEX, value: 13 }
  provides_tags: [reaver]
  modifiers:
    - target: skill.stab
      value: 1
      stacking: additive
      condition: { predicate: entity_has_tag, arg: bloodied }
      description: "+1 Stab while bloodied"
  triggers:
    - on: combat.attack
      when: "event.attack_skill eq 'stab' and event.hit and not has_status(target, 'bleeding')"
      do:
        - apply: bleeding
          to: target
          for: 3
```

### 5.3 A status effect that does something over time

`bleeding` ticks damage. This is the example that *intentionally* reveals a missing primitive:
there is no per-tick event in the current event surface, so this won't compile until one exists.

```yaml
status_effect:
  id: bleeding
  name: Bleeding
  default_duration_ticks: 3
  stacking: stack
  triggers:
    - on: world.tick          # ← no such event yet — compiler will flag it
      do:
        - damage: self
          amount: 1
```

When you compile this, `world.tick` shows up in the unimplemented-primitives list. You add a
tick event to the engine once; afterward, every "damage/heal/decay per tick" effect is pure data.

### 5.4 A d100 table with a subtable, and the generator that uses it

The table is inert — it only returns values:

```yaml
table:
  id: wasteland-encounters
  category: encounter
  name: Wasteland Wandering Monsters
  roll: 1d100                                          # presence of `roll:` ⇒ ranged (d100-style)
  entries:
    - { range: [1, 40],   result: "nothing" }
    - { range: [41, 70],  result: { spawn: scrip-hound, count: "1d4" } }
    - { range: [71, 90],  result: { table: wasteland-hazards } }   # subtable reference
    - { range: [91, 100], result: { spawn: rust-revenant, count: 1 } }
```

A weighted table omits `roll:` and uses `weight`:

```yaml
table:
  id: scrip-hound-loot
  category: loot
  name: Scavenged Scrap
  entries:
    - { weight: 5, result: { give: scrap-metal, count: "2d6" } }
    - { weight: 2, result: { give: power-cell,  count: 1 } }
    - { weight: 1, result: { give: pretech-fragment, count: 1 } }
```

A generator composes tables, sub-procedures, computed logic, and formatting into a structured
NPC. Note the `compute` step — the escape hatch into Python:

```yaml
procedure:
  id: generate-scavenger
  name: Generate Scavenger NPC
  inputs:
    - { name: faction_id, type: string, default: null }
  steps:
    - { kind: roll,      assign: occ,     table: scavenger-occupations }
    - { kind: procedure, assign: persona, procedure: une-personality }
    - { kind: compute,   assign: dispo,   function: "xwn-core:disposition_from_chaos", params: { persona: var.persona } }
    - { kind: format,    assign: greeting, template: "The {occ} eyes you warily.", params: { occ: var.occ } }
  output:
    fields:                         # maps onto NPCData fields; id/name come from the envelope
      occupation:       var.occ
      une_personality:  var.persona
      disposition:      var.dispo
      greeting:         var.greeting
      faction_id:       input.faction_id
```

### 5.5 An item

Items carry flat mechanical fields. A weapon:

```yaml
item:
  id: reaver-knife
  name: Reaver's Knife
  enc: 1
  cost: 15
  tags: [weapon, melee, knife]
  damage: 1d6
  damage_type: melee
  attribute: DEX
  shock_damage: 1
  shock_ac_threshold: 15
  weapon_tags: [light, concealable]
```

A shield that adds AC:

```yaml
item:
  id: scrap-buckler
  name: Scrap Buckler
  enc: 1
  cost: 10
  tags: [armor, shield]
  ac_bonus: 1
```

> **Known limitation (flagged for a design decision):** `item` has no general `modifiers[]`
> field, so an item that grants something like "+1 to Stab" can't be expressed with current
> fields. Two ways to resolve it: (a) add a `grants_traits: []` field to `item` so an item can
> reference a `trait` that carries the modifier (consistent with how `creature` references
> `special_abilities`), or (b) add a `modifiers[]` field to `item` directly. This is an open
> question — see the spec doc.

---

## 6. Reading compiler output

When you compile, the compiler reads the **entire** set of documents, collects **every**
problem, sorts them into two buckets, and — if either bucket is non-empty — fails without writing
any records. Nothing partial gets committed.

```
✗ Compile failed — 2 syntax errors, 3 unimplemented primitives. No records written.

SYNTAX ERRORS — your text is malformed; fix these:
  creatures/scrip-hound.yaml:6
    field 'attack_skill' = "bite" is not one of: punch, stab, shoot
  traits/sand-reaver-training.yaml:14
    condition parse error near "eq eq": expected a value after 'eq'

UNIMPLEMENTED PRIMITIVES — well-formed, but the engine has no such thing yet.
  These are either content you're prototyping ahead of the engine, OR a typo in a
  reference. Scan this list: if you meant to reference something that exists, it's a typo.
  status_effects/bleeding.yaml:7
    trigger on event 'world.tick' — no such event is registered
  status_effects/corroded.yaml:6
    modifier target 'combat.ac' — no such modifier target is registered
  procedures/generate-scavenger.yaml:5
    compute function 'xwn-core:disposition_from_chaos' — not registered
```

How to read it:

- **Syntax errors** are always mistakes in your text — typos, wrong types, malformed dice or
  conditions, unknown record types, broken internal references. Fix them.
- **Unimplemented primitives** are well-formed references to things the engine doesn't have:
  unknown verbs, events, compute functions, resources, status IDs, tags, or modifier targets. This
  bucket does double duty — it's both your "to-build" list when prototyping and your typo-catcher
  for references. If a line in here surprises you, it's probably a typo; if you expected it, it's
  the work you signed up to do.

### The prototyping-ahead workflow

You can transcribe a whole sourcebook in one sitting, referencing verbs and events that don't
exist yet. The compile will fail, but the unimplemented-primitives list becomes your precise
to-do list for the engine. Implement those primitives, recompile, done.

(An optional `--draft` mode can downgrade unimplemented primitives to warnings and emit IR for the
implemented parts only, for when you want to playtest a partially-wired pack mid-transcription. The
default is strict fail. See the spec doc.)

---

## 7. Composing new rules without new code — and knowing when you can't

The test for "can I do this as pure data?" is: **does every piece of this rule already exist as a
primitive?** Walk the graph — the container trait, each modifier target, each trigger event, each
effect verb, each referenced status and tag. If all of them are registered, it's data. If one
isn't, that one is your engineering task, and the compiler names it exactly. After you build it
once, everything that needs it is data forever.

This is the loop the whole system is built around: write the content, let the compiler surface the
gaps, fill the gaps in the engine, recompile. The grammar never has to change to add new mechanics —
only the registries (verbs, events, modifier targets, compute functions) grow.

---

## 8. The live IR runtime — triggers, effects, and the damage model (as implemented)

Everything below is what the engine runs **today** (roadmap Phases 1–3). It is the **raw IR**
form — `{ kind, params }` effects under `do:` — which is exactly what the shipped starter pack
([content/xwn-core/content/ir/starter.yaml](../../content/xwn-core/content/ir/starter.yaml))
uses and what `tests/starter_pack_ir.rs` pins. Author in this form for content that must run now.
(The form-2 sugar in §3 is a future compiler convenience; the raw IR is the ground truth.)

### 8.1 The interpreter loop

One cycle drives every reaction (see the "Design Vision" in AGENTS.md):

```text
event ──▶ gather the acting entity's triggers (statuses, equipped items, intrinsic traits)
          + standalone/global trigger records
      ──▶ evaluate each trigger's `when` (the condition DSL, §4 cheat-sheet)
      ──▶ run its `do` effects in order → typed Intents → applied to state
      ──▶ cascade any emitted events (bounded depth) ──▶ (loop)
```

A trigger is `{ id, on, when, do }` — `on` is the event type, `when` defaults to `"true"`,
`do` is the ordered effect list. Triggers live on a `trait`, a `status_effect`, or as a
standalone `trigger:` record.

### 8.2 Where triggers come from (sourcing order)

For an event on an acting entity, the runtime fires, **in this order**:

1. the entity's active **status effects'** triggers,
2. triggers from traits granted by the entity's **equipped items** (`item.grants_traits`),
3. the entity's **intrinsic traits'** triggers (`creature.traits` / `character.traits`),
4. **standalone/global `trigger:` records** subscribing to the event (sorted by id) — these fire
   without being carried by anyone (rooms/objects/world reactions).

Within the `when`/effects, `self` is the acting entity and `target` is the event's target (if any).

### 8.3 Effect verbs (raw IR `kind`)

| `kind` | params | does |
|---|---|---|
| `change_resource` | `resource`, `delta`, `entity_id?` | add `delta` to a resource (e.g. `hp`) |
| `apply_status` | `status_id`, `duration_ticks?`, `source?`, `entity_id?` | apply a status effect |
| `remove_status` | `status_id`, `entity_id?` | remove a status |
| `apply_modifier` | `modifier` (an IR Modifier object), `source_id?`, `entity_id?` | add a transient modifier |
| `remove_modifier` | `source_id`, `entity_id?` | remove modifiers by source |
| `emit_event` | `event_type`, `payload?` | emit a new event (cascades) |
| `log` | `message` | append a `gm.narrate` line |
| `emit_damage` | `packet` (a DamagePacket), `entity_id?` | deal damage through the pipeline (§8.6) |
| `roll_dice` | `dice` (e.g. `"2d6+1"`), `bind` | **compute**: roll and bind the total into the local scope (§8.5) |
| `run_procedure` | — | **deferred** (errors): needs the procedure-runner context wired in |
| `run_action` / `set_event` | — | resolution control-flow, not lowerable to an intent |

**`entity_id` roles:** omit it to default to `self`; the literal strings `self` and `target`
resolve against the event; any other value is a literal entity id.

### 8.4 The event catalog (what you can subscribe `on:`)

| Event | Payload fields | Acting `self` / `target` |
|---|---|---|
| `combat.attack` | `attacker_id`, `target_id`, `hit` (bool) | self = attacker, target = defender |
| `time.tick` | `self_id`, `tick` | self = each status-bearing entity (the world clock fans this out per turn) |
| `exploration.enter_hex` | `self_id`, `terrain`, `features` (list), `q`, `r`, `first_visit` | self = the player entering |

The runtime reads the acting entity from `attacker_id` or `self_id`, and the target from
`target_id`. Any event your own effects `emit_event` can also be subscribed to (bounded cascade).

### 8.5 Compute effects — roll dice, then spend the result

`roll_dice` binds an integer into a per-trigger **local scope**; later effects reference it. A
numeric param can be a literal, `{ "ref": "name" }` (the bound value verbatim), or
`{ "expr": "0 - local.name" }` (a DSL expression with `local` merged in):

```yaml
do:
  - kind: roll_dice
    params: { dice: "1d2", bind: gnaw }
  - kind: change_resource
    params: { resource: hp, delta: { expr: "0 - local.gnaw" } }   # spend the roll as damage
```

### 8.6 The damage model — pools, defenses, mitigation

`emit_damage` routes a `packet` through the IR damage pipeline against the **target's** live
pools and mitigations (it is not a flat `hp -= amount`):

- **`packet`**: `{ amount, types: [..], tier: sd|md }`. The amount is pre-rolled (use `roll_dice`
  then `{ref}` if it should vary).
- **Pools** (`creature.pools`, carried into combat): `{ id, tags: [..], current, max, can_go_negative }`.
  Every entity also has a derived `hp` pool. A packet routes to the first pool that accepts its
  tier (`md` packets bypass non-`md` pools — Rifts-style).
- **Named defenses** (`creature.defenses`, a `{name: value}` map): two roles.
  - *Damage mitigation*: a defense named `dr` reduces incoming damage (damage-resistance),
    `armor` negates hits at/below its value (threshold).
  - *Avoidance* (the to-hit contest): a **melee** attack is opposed by the `ac` defense, a
    **ranged** attack by `evasion` (falling back to `ac`). When no named defense is authored the
    legacy `ac` field is used, so existing content is unchanged.

```yaml
creature:
  id: ash_crawler
  # ...standard stat block...
  traits: [caustic_bite]                 # intrinsic trait whose trigger fires on its bite
  defenses: { ac: 12, dr: 1 }            # dr mitigates incoming IR damage
  pools:
    - { id: carapace, tags: [sd], current: 3, max: 3 }   # extra pool beyond derived hp
```

### 8.7 Worked examples (all live in the starter pack)

- **Item grants a trait that afflicts on hit** — the shard-lance grants `shard_laceration`
  (`on: combat.attack`, `when: "event.hit == true"` → `apply_status lacerated`).
- **Intrinsic creature trait** — `caustic_bite` on the ash crawler applies a status on its bite
  with no item involved.
- **Over-time on the world clock** — the `dread` status's `on: time.tick` trigger rolls `1d2` and
  spends it as hp loss each turn until it expires by `default_duration_ticks`.
- **Global/world reaction** — the standalone `ruins_watch` trigger logs a line whenever *any*
  entity enters a ruins hex, carried by no one.
- **Exploration reaction carried by the player** — the `ruin_dread` trait applies `dread` on
  `exploration.enter_hex when event.terrain == 'ruins'`.

Copy any of these from [starter.yaml](../../content/xwn-core/content/ir/starter.yaml), change the
ids, and drop them in a pack's `content/ir/` directory — they compile at world creation and fire
in live play.

### 8.8 What isn't wired yet

- `run_procedure` inside a trigger (the runner context isn't threaded in).
- Authored `creature.actions` perform in the **demo** turn loop (HR-765 resolver +
  HR-767): a creature with an `action` id performs it (contest → outcome → effects)
  on its turn. Wiring into the **live** `CombatScene` turn, plus action
  economy/targeting/multi-action selection, is still pending. Note an action's
  effects live in its `outcome` branches — an action does **not** emit a
  `combat.attack` event, so on-attack triggers (e.g. a weapon's `grants_traits`)
  belong in the action's outcome, not a separate `combat.attack` subscription.
- Globals firing on entity-less world events, and room/object-owned `self`.

When you reference something the engine doesn't have yet, the compiler lists it as an
unimplemented primitive (§6) — that list is your build queue.
