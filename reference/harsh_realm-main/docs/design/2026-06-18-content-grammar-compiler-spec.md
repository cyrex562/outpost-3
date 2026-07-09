# Harsh Realm — Content Grammar & Compiler Design Spec

**Date:** 2026-06-18
**Audience:** the agent implementing the compiler (Rust) and the IR extensions
**Companion doc:** `2026-06-18-content-authoring-guide.md` (how to author, for the human)
**Grounding:** the current-state doc (entity field sets, primitive vocabulary, event/verb
surface, Rust/Python split). Where this conflicts with earlier briefs, the current-state doc wins.

This spec defines a deterministic YAML authoring grammar that compiles to the engine's IR. No LLM
runs at compile time or run time. The IR is the canonical stored form; the YAML is an import
surface that is discarded after a successful compile.

---

## 1. Locked decisions

1. **Parser/compiler lives in Rust** (`crates/harsh-core`), alongside the existing IR, condition
   parser, and JSON Schema export. The compiler emits IR records and validates them against the
   exported JSON Schema before they are handed to the Python host for storage.
2. **The IR is the source of truth.** YAML compiles to IR; the IR is stored and loaded. The YAML is
   discarded after compile. Ongoing edits happen on IR via the content inspector, not by re-editing
   YAML.
3. **Two authoring surfaces, one IR:** bulk YAML (this grammar) and the content-inspector GUI. Both
   produce identical IR. (The god-view inspector is a separate runtime-state browser over per-world
   SQLite and is out of scope here.)
4. **Surface style — the sugar dial is fixed at "form 2":** structured YAML keys for everything
   enumerable (record type, target, event, stacking, duration, and verbs-with-named-args); quoted
   strings only for the two genuine expression languages, **dice** and **conditions**, parsed by the
   recursive-descent parser. This minimizes custom-parser surface in Rust to exactly two small
   grammars.
5. **Entity records are added to the IR.** `creature`, `item`, `npc`, `character` become IR record
   types whose field sets mirror the existing Python models exactly. The compiler emits them; the
   Python host maps IR → Pydantic (`CreatureData`, `ItemData`, `NPCData`, `Character`) at load/spawn.
6. **Gap handling is strict-fail with a full categorized report** (see §7).

---

## 2. IR record types

### 2.1 Existing rules records (already in the Rust IR)

`trait`, `tag`, `status_effect`, `trigger`, `procedure`, `table`, plus the condition AST, `effect`
(`{kind, params}`), and `modifier`. These are unchanged; the grammar targets them as-is. Their
field shapes are as documented in the existing IR.

### 2.2 New entity records (to add)

Add these four record types. **Field sets are the hard bound — the compiler must populate exactly
these fields, mirroring the Python models.** No silent additions.

**`creature`** → maps to `CreatureData`:
`id, name, hd, hp_per_hd, ac, attack_bonus, damage, damage_type, attack_skill, attack_description,
num_attacks, behavior, awareness_difficulty, flee_difficulty, unavoidable, morale, loot_table,
harvestable{material,skill,difficulty}, description_unseen, description_seen, description_short,
xp_value, tags[], special_abilities[] (ability IDs), innate_attacks[] (ability IDs)`

**`item`** → maps to `ItemData`:
`id, name, enc, cost, tags[], damage, damage_type, attribute, shock_damage, shock_ac_threshold,
range_band, ammo_type, ac, ac_bonus, weapon_tags[], effect (discriminated: food|heal|save_bonus),
power_charges, save_bonuses{}`

**`npc`** → maps to `NPCData`:
`occupation, personality_traits[], motivation, appearance, greeting, faction_id, disposition (−3..+3),
une_personality, tags[], traits[]` (plus envelope `id`, `name`)

**`character`** → maps to `Character`:
`id, name, character_class, level, xp, xp_next, attributes{}, attr_mods{}, skills{}, hp, max_hp, ac,
attack_bonus, physical_save, evasion_save, mental_save, save_bonuses{physical,evasion,mental,luck},
equipment[], class_abilities{}, unspent_skill_points, tags[], traits[], position_q, position_r`

`Combatant` is a derived runtime view and is **not** an authorable target.

### 2.3 The envelope

Persistence/transport unifies entities as
`EntityRecord = { id, name, entity_data: dict, entity_type: str }`, where `entity_type ∈
{character, npc, creature}` (and creatures when spawned). The compiler emits the typed entity record;
the host wraps it in the envelope. `entity_type` is the discriminator; `entity_data` is the
type-specific blob.

### 2.4 Entity↔rules join

Entities reference rules records by ID: `creature.special_abilities`, `creature.innate_attacks`,
`npc.traits`, `character.traits`. These IDs must resolve to existing `trait`/`trigger`/`status_effect`
records (internal references — unresolved ones are syntax errors, see §7).

---

## 3. Surface → IR mapping

| Surface construct | Produces |
|---|---|
| `creature:` / `item:` / `npc:` / `character:` block | the corresponding entity record (§2.2), wrapped in envelope by host |
| `trait:` block | `trait` record |
| `status_effect:` block | `status_effect` record |
| `tag:` block | `tag` record |
| `trigger:` block (top-level) | `trigger` record |
| `table:` block | `table` record; `roll:` present ⇒ ranged entries, absent ⇒ weighted |
| `procedure:` block | `procedure` record |
| `modifiers:` list item | `modifier` (`{target,value,stacking,priority,condition,description}`) |
| `triggers:` list item (inside trait/status) | inline `trigger`, owned by the container |
| `on:` | trigger `on` (event type string) |
| `when:` (string) | trigger `when` → condition AST (parsed) |
| `do:` list item (keyed verb form) | `effect` `{kind, params}` via verb registry (§5) |
| `prerequisites:` list item | `Prerequisite {kind, arg, value}` |
| `provides_tags:` | trait `provides_tags[]` |
| `cost:` | trait `Cost {points, slot?, description}` |
| condition `{predicate, arg}` (on a modifier) | modifier `condition` (note: this is the modifier's restricted predicate form, *not* the full condition DSL) |
| `entries:` `{range:[a,b], result}` | ranged table entry |
| `entries:` `{weight:n, result}` | weighted table entry |
| `result: {table: id}` | subtable reference |
| `steps:` `{kind, assign, table?/procedure?/function?/template?, params, count}` | procedure `Step` |
| `output: {fields: {name: "var.path"}}` | procedure `Output` |
| dice string (`2d8+2`) | parsed dice value (§6.1) |

**Two condition forms exist and must not be conflated:**
- Trigger `when:` uses the **full condition DSL** (§6.2), as a string.
- Modifier `condition:` uses the **restricted predicate form** `{predicate, arg}` where
  `predicate ∈ always | entity_has_tag | target_has_tag | entity_has_trait`. This is intentionally
  not the full DSL.

---

## 4. Inline-record lifting

A document may declare multiple records. Inline definitions are sugar: the compiler lifts them to
top-level records and replaces the inline block with a generated reference.

- An inline `status_effect`/`trait`/`table` defined inside a container is lifted to a top-level
  record with a derived ID: `<container-id>.<local-name>` (e.g. `sand-reaver-training.bleeding`).
- Reference-by-ID is used when the referenced record is shared; inline is used when it's private to
  one container. Both yield identical IR — an inline definition is exactly equivalent to defining the
  record separately and referencing its derived ID.
- Lifting is deterministic: same input ⇒ same derived IDs ⇒ same IR.

---

## 5. Verb registry (effects)

Effects are an **open vocabulary**. The grammar never changes when verbs are added; only the registry
grows. The registry is engine/pack-provided data, authored once per verb, not per content record.

Each verb entry declares: the IR `kind` it lowers to, a **signature** (the named argument keys it
accepts and which are required/optional), and a **binding** from signature args to `effect.params`.

```yaml
# verb registry (illustrative shape)
verbs:
  apply:
    kind: apply_status
    args:
      _self:    { role: status_id, required: true }   # the value on the verb key itself
      to:       { role: entity_id, required: false, default: "<acting entity>" }
      for:      { role: duration_ticks, required: false }
      source:   { role: source, required: false }
  damage:
    kind: change_resource
    args:
      _self:    { role: entity_id, required: true }
      amount:   { role: delta, required: true, transform: negate }   # damage ⇒ negative delta
      resource: { role: resource, required: false, default: "hp" }
```

Lowering: parse the keyed effect → look up verb → bind args to `params` per the signature → emit
`{kind, params}`. An unknown verb is an unimplemented primitive (§7), not a grammar error.

Known initial verbs and their `kind`: `apply`→`apply_status`, `remove`→`remove_status`,
`damage`/`heal`/`change`→`change_resource`, `add_modifier`→`apply_modifier`,
`drop_modifier`→`remove_modifier`, `emit`→`emit_event`, `log`→`log`. Reserved compute-style:
`roll`→`roll_dice`, `run`→`run_procedure`.

---

## 6. Formal grammars (the only two parsed strings)

### 6.1 Dice

```ebnf
dice      = flat | roll
flat      = integer
roll      = [ count ] "d" sides [ modifier ]
count     = integer            (* default 1 *)
sides     = integer            (* 100 for d100-style tables *)
modifier  = ("+" | "-") integer
integer   = digit { digit }
```

Accepts `1d6`, `2d8+2`, `1d4-1`, `1d100`, and bare integers (`3`). A malformed dice string is a
syntax error.

### 6.2 Condition DSL (existing — restated, not redesigned)

This is the engine's existing condition language. The grammar is restated here for completeness; the
implementation is the existing recursive-descent parser/evaluator in `crates/harsh-core/dsl`.

```ebnf
expr       = or_expr
or_expr    = and_expr { "or" and_expr }
and_expr   = not_expr { "and" not_expr }
not_expr   = [ "not" ] comparison
comparison = sum [ comp_op sum ]
comp_op    = "==" | "!=" | "<" | "<=" | ">" | ">="
           | "eq" | "neq" | "lt" | "lte" | "gt" | "gte" | "in"
sum        = term { ("+" | "-") term }
term       = factor { ("*" | "/") factor }
factor     = literal | path | func_call | "(" expr ")"
path       = root { "." ident }
root       = "entity" | "event" | "world" | "target" | "self" | "local"
func_call  = ident "(" [ args ] ")"
args       = expr { "," expr }
literal    = int | float | string | "true" | "false" | "null" | list
list       = "[" [ expr { "," expr } ] "]"
ident      = letter { letter | digit | "_" }
```

Functions: `has_tag(e,t)`, `has_trait(e,t)`, `has_status(e,s)`, `len(x)`, `min(...)`, `max(...)`,
`abs(x)`. A parse failure is a syntax error. A well-formed reference to an unknown function is an
unimplemented primitive.

---

## 7. Diagnostics model

The compiler performs a **full pass over all documents**, collects **all** diagnostics, then decides
the outcome. It never stops at the first problem and never commits partial output.

### 7.1 Two diagnostic classes

**`SyntaxError`** — the author's text is malformed. The author must fix it. Includes:
- Malformed YAML.
- Unknown record type (top-level key not in the record-type set).
- Missing required field; wrong field type; value not in a closed enum
  (e.g. `attack_skill` not in `{punch, stab, shoot}`; `stacking` not in its allowed set).
- Unparseable dice or condition string.
- Unresolved **internal** reference (an inline/derived ID or a record-to-record reference that names
  a record not present in this compile unit and not in the loaded pack set).
- Unknown event-payload field, **if** event payloads are typed and checkable (see §7.3).

**`UnimplementedPrimitive`** — the construct is well-formed but names something the engine doesn't
have. The author decides whether it's prototyping-ahead or a typo. Includes well-formed references
to an unknown:
- effect verb,
- event (`on:` value not in the registered event catalog),
- compute function (`function:` not registered),
- resource, status ID, tag, or modifier target not in the registered vocabulary.

The distinguishing rule: **malformed structure or a broken internal reference ⇒ `SyntaxError`; a
well-formed reference into an open/extensible vocabulary that doesn't resolve ⇒
`UnimplementedPrimitive`.** The unimplemented bucket intentionally also catches reference typos — the
author scans it to tell intent apart.

### 7.2 Outcome

- If **either** bucket is non-empty: **fail**, write nothing, print both buckets (grouped, with
  file:line and a one-line explanation each), and report counts.
- If **both** are empty: validate every emitted record against the JSON Schema; if validation passes,
  hand records to the host for storage.

### 7.3 Reference resolution inputs

The compiler needs, at minimum, the registered catalogs to resolve references:
event catalog, verb registry, compute-function registry, resource records, status IDs, tags, and
modifier-target vocabulary. These come from the loaded pack set (the same packs the world is bound
to). Event-payload field checking is **optional and recommended**: if per-event typed payloads are
available, an `event.<field>` path that names a non-existent field on that event's payload is a
`SyntaxError`; if payload schemas aren't available, payload-field checks are skipped.

### 7.4 Optional `--draft` mode

Default is strict fail. An optional `--draft` flag downgrades `UnimplementedPrimitive` to warnings,
emits IR for the fully-resolved records only, and writes a **gap manifest** (the list of unimplemented
primitives) so a partially-wired pack can be playtested mid-transcription. `SyntaxError` still fails
even in draft mode. This mode is secondary; it exists to support long transcription sessions, not as
the normal path.

---

## 8. Validation & determinism

- Every emitted record is validated against the exported JSON Schema before storage. Schema
  validation failure is a compiler bug or an IR drift, not author error — surface it distinctly from
  the two author-facing buckets.
- The compile is a pure function of (documents + loaded pack catalogs): same inputs ⇒ same IR,
  including all derived IDs from inline lifting. No randomness, no ordering dependence beyond
  document order for derived-ID stability.

## 9. Round-trip

Round-trip is **not load-bearing** because the YAML is discarded after compile. However, an
IR → canonical-YAML emitter is worth having for the content inspector's "export" affordance and for
diffing. If implemented, it should emit form-2 keyed verbs and the canonical field order, such that
re-compiling the emitted YAML yields byte-identical IR. This is a nice-to-have, not a requirement.

## 10. Open questions (flagged for decision)

- **Item modifiers.** `item` has no general `modifiers[]` field. To express an item that grants a
  skill/attribute modifier, either (a) add `grants_traits: []` to `item` (reference a `trait` that
  carries the modifier — consistent with `creature.special_abilities`), or (b) add `modifiers[]` to
  `item` directly. Recommendation: (a), to reuse the trait/modifier machinery and avoid a second
  modifier-bearing shape. Needs sign-off since it touches the entity field-set bound.
- **AC as a modifier target.** Several effects want to modify AC (`corroded` → −1 AC). Confirm
  whether `combat.ac` (or similar) is a registered modifier target; if not, it's the first
  unimplemented primitive a real monster import will hit.
- **Event-payload typing for `when:` validation.** Decide whether to wire per-event typed payloads
  into the compiler so `event.<field>` paths are checkable (§7.3). Strongly recommended — it turns a
  large class of silent run-time mismatches into compile-time syntax errors.
- **Derived-ID scheme for inline lifting.** Confirm `<container-id>.<local-name>` is acceptable as
  the generated ID convention, or specify an alternative.
