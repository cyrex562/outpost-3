# Harsh Realm — Action & Resolution Primitives (Design Concept)

**Date:** 2026-06-19
**Audience:** the agent implementing this (and you)
**Companion:** `2026-06-19-xwn-skill-action-decomposition.md` (the action vocabulary + 19-skill catalog)
**Builds on:** the content grammar/IR spec and the current-state grounding (flat tagged entity
records, existing resolvers `resolve_hit`/`resolve_skill_check`/`resolve_save`, modifier pipeline,
condition DSL, statuses, resources, triggers).

This concept defines a small set of orthogonal primitives on top of which all skill use, combat,
and (later) magic/psionics/super and multi-system content are expressed as data. The engine knows
how to *resolve an action*, *roll a mechanic*, *push a damage packet through stages*, and *stack a
modifier*. It does not know `melee_attack`, `Stab`, `fireball`, `AC`, `MDC`, or wounding
multipliers — those are all data.

---

## 1. The abstraction stack

Primitives stay brutally simple so their units are common and reusable; legibility is recovered by
stacking named layers, each composing the one below **by reference, not inheritance** (consistent
with the flat-tagged-records principle — there is no class hierarchy).

- **L0 — primitives:** `contest` / `reaction` / `apply`, plus the roll-mechanic, the damage
  pipeline, targeting, action-economy, and the effect verbs.
- **L1 — action archetypes:** a small mechanical vocabulary (`melee_attack`, `ranged_attack`,
  `athletics_check`, `social_contest`, `perceive`, `save_vs_effect`, …). Each is a configured contest.
- **L2 — concrete actions / abilities:** named content (`sword_thrust`, `judo_throw`, `aimed_shot`,
  later `fireball`). An L2 action declares `base: <archetype>` and overrides/adds fields.
- **L3 — skills / traits:** data overlays that *grant* actions and *modify* them (modifier sources
  targeting `action.<id>`). Judo lives here.

Entities (creature/character/npc) reference L2 actions; skills/traits attach at L3.

---

## 2. L0 — resolution primitives

Three kinds. Everything the brief floated as a possible extra kind is a composition:
magic/psionics = a kind + an activation cost; summon = an effect verb; over-time = status+trigger;
hazard = a contest whose source is an environmental entity; movement = an `apply` (or a `check`)
with a reposition effect.

### 2.1 `contest`

One resolution structure produces a margin against a target number. Parameterized, it subsumes
check, opposed, attack, and save:

| Want | `roller` | `tn_source` |
|---|---|---|
| skill check | actor | `difficulty <value>` |
| opposed | actor | `opposed <target roll>` |
| attack | actor | `defense <defense-id of target>` |
| save | defender | `effect_dc <value>` |

### 2.2 `reaction`

Not a standalone resolution — a **trigger** that fires inside another resolution's reaction window
and can inject a modifier, run a nested contest, or cancel/alter the in-flight result. (Parry, dodge,
block, riposte, GURPS active defenses.) See §6 for the phased flow that makes this possible.

### 2.3 `apply`

No roll. Directly produces effects: stances, buffs, environment, repositioning.

### 2.4 The margin currency

Every contest yields one structure, which every outcome spec consumes. This is what lets one action
definition work across dice systems:

```
ResolutionResult {
  success:    bool
  raw_margin: int     # signed: total − TN (or skill − roll for roll-under systems)
  degree:     int     # banded by the mechanic: 0 = bare success, +1/+2 …, negatives = worse
  crit:       bool    # mechanic-defined
  fumble:     bool
  rolled: int, total: int, tn: int
}
```

Both points and degrees are first-class (per your call): outcome specs may key on `raw_margin`
thresholds **or** `degree` bands.

---

## 3. L0 — roll-mechanic abstraction

A `RollMechanic` is a registered resolver implementing
`resolve(roll_spec, modifiers, tn) -> ResolutionResult`. It owns the dice procedure (including
exploding/wild dice, advantage), the comparison direction (roll-over vs roll-under), crit/fumble
rules, and the `degree` banding. **Which** mechanic an action or world uses is data (`mechanic:` on
the roll spec, defaulting to the world's mechanic); the **set** of mechanics is code.

Shipped now (XWN):

- `xwn_2d6` — `2d6 + skill + attr_mod + mods ≥ difficulty`; `raw_margin = total − difficulty`.
- `xwn_d20_attack` — `d20 + attack_bonus + skill + attr_mod ≥ defense`; nat 1 = fumble, nat 20 = crit.
- `xwn_d20_save` — `d20 + bonus ≥ target`.

Designed-for (added with their packs): `gurps_3d6_under` (roll-under, margin = skill − roll),
`savage_trait` (exploding trait die + wild die, take higher; `degree = 1 + floor((total−TN)/4)` →
raises). These are not built until those packs are real.

---

## 4. L0 — damage & health pipeline

A `DamagePacket { amount, types: [tags], tier, source }` flows through stages. **Every stage is
modifier-targetable** — that's what lets environment transform damage (e.g. submersion zeroes fire),
not just penalize the roll.

1. **Avoidance** — resolved at the attack contest (target's defense TN). A miss produces no packet.
   *AC lives only here.*
2. **Amount** — roll packet amount + attribute/ability mods.
3. **Wounding** — per-type multiplier (default ×1; GURPS cutting ×1.5, impaling ×2).
4. **Mitigation** — any of: subtractive `damage_resistance`; threshold `armor_rating` (amount ≤
   threshold → reduced/zero, Savage Toughness-style); `absorbing_pool` (armor SDC/MDC soaks first).
5. **Routing** — packet `types`/`tier` select target pool(s) and bypass rules (Rifts MD bypasses SD;
   tier mismatch → ÷100 or 0).
6. **Pool application** — subtract from pool(s); depletion fires effects (0 hp → dying).
7. **Degree → wounds** — for margin-driven systems, the contest `degree` converts to wound levels
   (Savage raises).

Entities carry: `pools: [{id, tags, current, max, can_go_negative}]`,
`defenses: {<id>: <value>}`, and optional `mitigations: [{kind, value, applies_to_types}]`.
XWN entities default to `pools: [hp]` and `defenses: {ac}`.

### 4.1 Cross-system mapping — AC is one configuration, not the base

| System | Avoidance (attack TN) | Amount | Wounding | Mitigation | Routing / pools | Degree→wounds |
|---|---|---|---|---|---|---|
| XWN / D&D | `ac` = 10+DEX+armor | die + STR (+Warrior ½ lvl) | — | optional DR | `hp`; 0 = dying | — |
| Rifts SDC | dodge/parry or `ac` | die | — | armor as absorbing SDC pool | `sdc` → `hp` | — |
| Rifts MDC | dodge/parry or `ac` | MD die | — | armor absorbing MDC pool; MD tier bypasses SD pools | `mdc`; SD vs MDC ≈ 0 | — |
| GURPS | active defense (a `reaction`) | die | ×type (cut 1.5 / imp 2 / cr 1) | DR subtractive | `HP`; death checks at −×HP | — |
| Savage | vs Parry (static TN) | trait die exploding | — | Toughness threshold + Soak roll | — | raises (margin/4) → wounds |

Only the XWN row is implemented now; the others are the design target proving the pipeline
generalizes.

### 4.2 Coarse vs fine resolution (Godbound)

Granularity is just how much an entity declares. A mob is an entity with one coarse pool, a flat
defense, and a `mob` tag; a boss declares multiple typed pools, named defenses, and reactions.
Actions/effects can key on the `mob` tag (a Fray-die effect auto-damages `mob`-tagged foes). Full
Godbound mob/Fray rules are deferred to that pack; the model already supports the split.

---

## 5. Record shapes

### 5.1 `action`

```yaml
action:
  id: melee_attack
  base: null                 # L2 actions set base: <archetype-id> and override/add
  kind: contest              # contest | apply  (reaction is a trigger — §6)
  resolution:
    roller: actor            # actor | defender
    tn_source: { defense: ac }            # static <v> | difficulty <key> | defense <id> | opposed <skill>
    roll_spec:
      mechanic: xwn_d20_attack            # defaults to world mechanic
      skill: melee            # the SKILL THIS ACTION DRAWS ON; entity supplies the level
      attribute: STR
  targeting:  { shape: single, range_band: melee, requires_los: true }
  activation: { economy: action, costs: [], cooldown: null, uses: null }
  prerequisites: []          # {kind,arg,value} or condition-string
  outcome:
    - when: "crit"
      do: [ { emit_damage: { amount: "weapon+STR", types: [physical], tier: sd }, to: target },
            { log: "A brutal hit!" } ]
    - when: "success"
      do: [ { emit_damage: { amount: "weapon+STR", types: [physical], tier: sd }, to: target } ]
    - when: "fumble"
      do: [ { log: "The blow goes wide and you stumble." } ]
  tags: [attack, melee]
```

**Governing skill (your decision 4):** the action *names* the skill (`roll_spec.skill: melee`); the
entity supplies its level in that skill and the named attribute. Any entity may attempt any action;
lacking the skill applies the untrained penalty (a default modifier). The skill is not looked up from
the weapon — the action you invoke (because you're wielding that weapon or using that ability)
already names it.

### 5.2 `skill` (L3 overlay)

```yaml
skill:
  id: stab
  name: Stab
  governs:                                   # actions it applies to + per-level modifiers
    - action: melee_attack
      requires_tag: bladed                   # only when the wielded action carries this tag
      per_level: [ { target: "action.melee_attack.roll", value: 1, stacking: additive } ]
  grants: [ parry ]                          # actions/reactions this skill makes available
  triggers: []                               # special reactions
  tags: []
```

Skills are modifier sources targeting `action.<id>` and its `roll_spec`/stages. "Untrained" = no
level in a named skill → the default unskilled-penalty modifier applies.

### 5.3 How both map onto the existing engine

| Concept | Existing mechanism |
|---|---|
| outcome `do:` effects | effect verbs (`apply_status`, `change_resource`, `emit_event`, `log`) + new verbs `emit_damage`, `spawn_entity`, `reposition` |
| `when:`, prerequisites, modifier gates, reaction triggers | the condition DSL |
| skills/traits/statuses/environment modifying actions | the modifier pipeline, now also targeting `action.<id>` and pipeline stages; these are modifier *sources* |
| stances, buffs, prone, submerged | status effects (carry their own modifiers) |
| costs (PP, strain, slots, ammo, charges) and pools | resources |
| reactions | triggers on resolution-stage events (§6) |

---

## 6. Phased, interruptible resolution (the refactor)

Reactions, environment, and "chains that can be modified over time" all require resolution to stop
being one-shot. The resolver emits a fixed sequence of stage events; modifiers and reactions attach
at the relevant stage. The stage set is **fixed** (pack-extensible only via modifiers/reactions/
effects at existing stages — a brand-new stage is engine work):

```
resolution.started
resolution.modifiers_assembled   ← environment/stance/buff modifiers land here (condition-gated sources)
resolution.rolled                ← ResolutionResult produced
resolution.reaction_window       ← reactions fire: alter TN, add a nested contest, or cancel
resolution.outcome               ← success/margin/degree selects the outcome branch
damage.packet_created            ← amount + wounding
damage.mitigated                 ← DR / armor / absorbing pool
damage.applied                   ← pool routing + application  (this is the pre-write _requested window)
resolution.completed
```

- **Environment** (underwater/cold/hot) = modifier sources gated by conditions, landing at
  `modifiers_assembled` (numeric penalty), or a prerequisite that *gates* the action (can't fire a
  bow submerged), or a conditional ×0 multiplier at `damage.packet_created`/`mitigated` (submersion
  zeroes fire). All three reduce to existing machinery because every stage is modifier-targetable.
- **Reactions** are triggers on `resolution.reaction_window`:

```yaml
trigger:
  on: resolution.reaction_window
  when: "event.target eq self.id and event.action_archetype eq 'melee_attack' and has_action(self,'parry')"
  do:
    - run_action: parry           # a nested contest whose result can raise event.tn or cancel the hit
```

### 6.1 Action chains (your "chains of actions")

Two senses, both supported:

- **A single resolution as a modifiable pipeline** — the stage list above; modifiers/reactions
  attach and can be added over time as game state evolves.
- **Action sequences / combos** — reuse the procedure machinery: a chained ability is a `procedure`
  with a new `run_action` step kind (charge = `reposition` then `melee_attack`). No second sequencing
  concept.

---

## 7. Action-economy, cost, targeting

- **Economy:** `free | action | reaction`. Per-round budgets (default 1 action; reaction budget
  configurable, default 1/round) enforced by the engine. Cooldowns are statuses with durations;
  `uses: {scope: round|scene|day, n}` caps repeats.
- **Cost:** a resource list (`[{resource, amount}]`) — PP/strain/slots/ammo/charges are all resources;
  paid at `resolution.started`, refunded if the action is canceled before commit.
- **Targeting:** `shape ∈ self | single | area | line`, plus `range_band`, `reach`, `requires_los`.
  `area`/`line` require the engine to gather affected entities (a targeting primitive). Range bands
  are data-defined per world; mapping bands to square-grid distance is deferred (see open questions).

---

## 8. Primitive vs data — the line

**Code (primitives):** the three resolution kinds and the phased stage orchestration; each registered
roll mechanic; the damage-pipeline stages; the targeting primitives; action-economy enforcement; the
effect verbs (including new `emit_damage`, `spawn_entity`, `reposition`); and the existing modifier
pipeline / condition DSL / status / resource / trigger systems.

**Data (records & config):** which mechanic a world uses; all action archetypes (L1), concrete
actions (L2), skills/traits (L3); entity pools/defenses/mitigations/actions; range-band definitions,
difficulty values, outcome specs, costs; wounding multipliers, pool routing/bypass rules, mitigation
configs. The engine never names `melee_attack`, `Stab`, `fireball`, `AC`, `MDC`, or a wounding
multiplier.

---

## 9. Entity field-set changes (signed off)

- `creature`: add `actions: [action-id]`; add `pools: [...]` (default `[hp]`) and
  `defenses: {ac}`; retire `attack_skill` (subsumed by the named skill on each action) and fold
  `innate_attacks` into `actions`.
- `character`: add `actions: [action-id]`, `pools`, `defenses`; `ac`/save fields become entries
  under `defenses`.
- The compiler validates `action`/`skill` references the same way it validates ability references;
  unknown ones surface as unimplemented primitives.

---

## 10. Open questions (residual)

- **Range model.** Abstract range bands now, mapping to square-grid distance later — confirm, or wire
  grid distance from the start.
- **Degree banding for XWN checks.** XWN skill checks are largely binary; do you want margin-banded
  outcomes for ordinary checks, or reserve `degree` for attacks/saves and keep checks pass/fail?
- **Passive vs active perception.** Lean: passive = a stored `perceive` defense value used as the
  `tn_source` for opposed stealth; active = a `perceive` contest. Confirm.
- **Reaction budget default.** 1 reaction/round, overridable per reaction — or unlimited but
  resource-gated?
- **Unskilled penalty.** Confirm the XWN untrained value to bake into the default modifier.
