# Harsh Realm — XWN Skill & Action Decomposition (Catalog)

**Date:** 2026-06-19
**Audience:** the agent authoring the core XWN content pack (and you)
**Companion:** `2026-06-19-action-resolution-primitives-design.md` (the model: layers, primitives,
damage pipeline, record shapes)

This is the content catalog that sits on the primitives: the consolidated **L1 action archetypes**,
seven worked examples exercising them, and all **19 XWN skills re-expressed as L3 overlays**. The 19
skills stay the canonical set — they just become data. A new skill (Judo) is shown as nothing but
additional data.

---

## 1. Consolidated L1 action vocabulary

Decomposing the 19 skills yields this small archetype set. The rule (your decision): one archetype
per distinct *mechanical* structure; flavor differences (thrust vs slash) are data variants, not new
archetypes. Each archetype is a configured `contest` (or `apply`/reaction).

| Archetype | Kind / config | Governing draw | Notes |
|---|---|---|---|
| `melee_attack` | contest · actor vs `defense:ac` | a melee skill + STR/DEX | armed & unarmed via tags/variants |
| `ranged_attack` | contest · actor vs `defense:ac` | Shoot + DEX | aimed/snap/suppress as variants |
| `grapple` | contest · `opposed` (Exert/Punch) | Punch/Exert + STR | on success → `grappled` status |
| `defend` (parry / dodge / block) | **reaction** on `reaction_window` | varies | alters TN / cancels / nests a contest |
| `athletics_check` | contest · `difficulty` | Exert + STR/DEX | climb, jump, force, swim, endure |
| `perceive` | contest · `difficulty` (active) or stored defense (passive) | Notice + WIS | spot, listen, search |
| `stealth` | contest · `opposed` vs `perceive` | Sneak + DEX | hide, move-silent, tail |
| `social_contest` | contest · `opposed` vs target resist | Talk/Connect + CHA | persuade, intimidate, deceive |
| `knowledge_check` | contest · `difficulty` | Know + INT | recall, identify, research |
| `tech_check` | contest · `difficulty` | Fix/Program + INT | repair, hack, disable, operate |
| `heal_check` | contest · `difficulty` | Heal + INT/WIS | first aid, treat, surgery |
| `pilot_check` | contest · `difficulty`/opposed | Pilot + DEX | drive, fly, evade |
| `perform_check` | contest · `difficulty`/opposed | Perform + CHA | entertain, distract |
| `survive_check` | contest · `difficulty` | Survive + CON/WIS | forage, navigate, track, endure |
| `work_check` (craft/labor) | contest · `difficulty` | Work + STR/INT | build, craft, labor |
| `trade_check` | contest · `difficulty`/opposed | Trade + CHA | haggle, appraise, bribe |
| `lead_check` | contest · `difficulty`/opposed | Lead + CHA | command, rally; feeds faction turns |
| `administer_check` | contest · `difficulty` | Administer + INT/CHA | organize, bureaucracy |
| `cast` / `activate` | wrapper: any kind + `costs` | varies | cost-gated host for abilities/magic (later) |

`save_vs_effect` is the same `contest` with `roller: defender` and `tn_source: effect_dc` — used by
anything that forces a save, so it isn't a separate archetype.

---

## 2. Seven worked examples

### 2.1 Unarmed strike (L2 over `melee_attack`)

```yaml
action:
  id: unarmed_strike
  base: melee_attack
  resolution: { roll_spec: { skill: punch, attribute: STR } }
  targeting: { shape: single, range_band: melee }
  outcome:
    - when: "success"
      do: [ { emit_damage: { amount: "1d2+STR", types: [physical, blunt], tier: sd }, to: target } ]
  tags: [attack, melee, unarmed]
```

### 2.2 Parry + riposte (reactions)

```yaml
action:
  id: parry
  kind: contest
  resolution: { roller: defender, tn_source: { opposed: incoming_attack }, roll_spec: { skill: stab, attribute: DEX } }
  activation: { economy: reaction }
  outcome:
    - when: "success"
      do: [ { set_event: { field: hit, value: false } } ]      # cancels the in-flight hit
  tags: [defense, reaction]
```

```yaml
skill:
  id: riposte_training
  name: Riposte Training
  grants: [ riposte ]
  triggers:
    - on: resolution.reaction_window
      when: "event.target eq self.id and event.action_archetype eq 'melee_attack' and event.outcome eq 'parried'"
      do: [ { run_action: unarmed_strike, target: event.attacker } ]   # free counter on a successful parry
```

### 2.3 Grapple (opposed contest → status)

```yaml
action:
  id: grapple
  kind: contest
  resolution: { roller: actor, tn_source: { opposed: "max(skill.exert, skill.punch)" }, roll_spec: { skill: punch, attribute: STR } }
  targeting: { shape: single, range_band: melee }
  outcome:
    - when: "success"
      do: [ { apply: grappled, to: target, for: 0 } ]
  tags: [melee, control]
```

### 2.4 Aimed shot vs snap shot (two variants of `ranged_attack`)

```yaml
action: { id: aimed_shot, base: ranged_attack, activation: { economy: action },
          resolution: { roll_spec: { skill: shoot, attribute: DEX } },
          modifiers: [ { target: "action.aimed_shot.roll", value: 1, stacking: additive } ],
          outcome: [ { when: "success", do: [ { emit_damage: { amount: "weapon+DEX", types: [physical], tier: sd }, to: target } ] } ],
          tags: [attack, ranged, aimed] }

action: { id: snap_shot, base: ranged_attack, activation: { economy: free, uses: { scope: round, n: 1 } },
          resolution: { roll_spec: { skill: shoot, attribute: DEX } },
          modifiers: [ { target: "action.snap_shot.roll", value: -2, stacking: additive } ],
          outcome: [ { when: "success", do: [ { emit_damage: { amount: "weapon", types: [physical], tier: sd }, to: target } ] } ],
          tags: [attack, ranged, snap] }
```

### 2.5 Fireball-style cost-gated AoE save (the magic shape, deferred content but valid now)

```yaml
action:
  id: fireball
  kind: contest
  resolution: { roller: defender, tn_source: { effect_dc: 13 }, roll_spec: { mechanic: xwn_d20_save, skill: null, attribute: null } }
  targeting: { shape: area, range_band: long, requires_los: true }
  activation: { economy: action, costs: [ { resource: spell_slots, amount: 1 } ] }
  prerequisites: [ "has_trait(self, 'arcane_caster')" ]
  outcome:
    - when: "success"            # target saved
      do: [ { emit_damage: { amount: "3d6", types: [fire], tier: sd, half: true }, to: target } ]
    - when: "miss"               # target failed the save
      do: [ { emit_damage: { amount: "3d6", types: [fire], tier: sd }, to: target } ]
  tags: [magic, aoe, fire]
```

Note: `fireball` is "an `apply`/contest + a cost + AoE targeting + a fire-typed damage packet." The
engine knows none of those four things are "magic." The submersion-zeroes-fire transform is a
conditional ×0 multiplier on the `fire`-typed packet — pure data.

### 2.6 Notice (perceive check)

```yaml
action:
  id: spot
  base: perceive
  resolution: { tn_source: { difficulty: 8 }, roll_spec: { skill: notice, attribute: WIS } }
  targeting: { shape: self }
  outcome:
    - when: "success"
      do: [ { emit_event: { event_type: perception.revealed, payload: { detail: high } } } ]
  tags: [perception]
```

### 2.7 Social deceive (opposed)

```yaml
action:
  id: deceive
  base: social_contest
  resolution: { tn_source: { opposed: "skill.notice" }, roll_spec: { skill: talk, attribute: CHA } }
  targeting: { shape: single, range_band: conversation }
  outcome:
    - when: "success"
      do: [ { apply: deceived, to: target, for: 6 } ]
    - when: "fumble"
      do: [ { emit_event: { event_type: social.disposition_change_update_requested, payload: { delta: -1 } } } ]
  tags: [social]
```

---

## 3. The 19 XWN skills as L3 overlays

The 19 stay canonical. Each becomes a `skill` record that governs/modifies archetypes and grants
actions. "Governs" applies per-level modifiers to the named action's roll; "grants" exposes actions.

| Skill | Governs (archetypes) | Concrete actions / variants | Grants / special |
|---|---|---|---|
| Administer | `administer_check` | organize, requisition, bureaucracy | — |
| Connect | `social_contest`, `knowledge_check` | make-contact, gather-rumor | grants `call_in_favor` |
| Exert | `athletics_check`, `grapple` | climb, jump, force, swim, endure | — |
| Fix | `tech_check` | repair, jury-rig, disable | — |
| Heal | `heal_check` | first_aid, treat, surgery | — |
| Know | `knowledge_check` | recall, identify, research | — |
| Lead | `lead_check` | command, rally | feeds `faction.turn` |
| Notice | `perceive` | spot, listen, search | sets passive `perceive` defense |
| Perform | `perform_check`, `social_contest` | entertain, distract | — |
| Pilot | `pilot_check` | drive, fly, evade | grants `evasive_maneuver` |
| Program | `tech_check` | hack, program, operate-computer | — |
| Punch | `melee_attack` (unarmed), `grapple` | unarmed_strike, grapple | grants `brawl_block` |
| Shoot | `ranged_attack` | aimed_shot, snap_shot, suppress | — |
| Sneak | `stealth` | hide, move_silent, tail | grants `ambush` |
| Stab | `melee_attack` (bladed) | sword_thrust, slash | grants `parry` |
| Survive | `survive_check` | forage, navigate, track, endure | — |
| Talk | `social_contest` | persuade, intimidate, deceive | maps to skill-verbs convince/intimidate/deceive |
| Trade | `trade_check`, `social_contest` | haggle, appraise, bribe | maps to skill-verb bribe |
| Work | `work_check` | labor, craft, build | — |

Example skill record (Shoot):

```yaml
skill:
  id: shoot
  name: Shoot
  governs:
    - action: ranged_attack
      per_level: [ { target: "action.ranged_attack.roll", value: 1, stacking: additive } ]
  grants: [ aimed_shot, snap_shot ]
  tags: [combat, ranged]
```

The Notice passive-perception note: the skill writes a stored `perceive` defense on the entity
(`defenses.perceive = 8 + Notice + WIS`), which `stealth` contests target as `tn_source: opposed`.

---

## 4. Judo — a new skill is purely additional data

No engine change. Judo grants/modifies grapple, a throw action, and dodge; adds a riposte-style
reaction; and on a clean throw applies `prone` (an outcome modifier the skill injects onto `throw`).

```yaml
action:                                   # the throw archetype-variant Judo relies on
  id: throw
  base: grapple
  outcome:
    - when: "degree >= 1"                  # a clean throw (margin band)
      do: [ { apply: prone, to: target, for: 0 },
            { emit_damage: { amount: "1d6", types: [physical, blunt], tier: sd }, to: target } ]
    - when: "success"
      do: [ { apply: prone, to: target, for: 0 } ]
  tags: [melee, control, throw]
```

```yaml
skill:
  id: judo
  name: Judo
  governs:
    - { action: grapple, per_level: [ { target: "action.grapple.roll", value: 1, stacking: additive } ] }
    - { action: throw,   per_level: [ { target: "action.throw.roll",   value: 1, stacking: additive } ] }
    - { action: dodge,   per_level: [ { target: "action.dodge.roll",   value: 1, stacking: additive } ] }
  grants: [ throw ]
  triggers:
    - on: resolution.reaction_window
      when: "event.target eq self.id and event.action_archetype eq 'melee_attack' and event.outcome eq 'dodged' and has_skill(self,'judo')"
      do: [ { run_action: throw, target: event.attacker } ]      # riposte: dodge → throw
  tags: [martial]
```

Everything Judo does — bonuses to existing actions, a granted action, a reaction, a margin-keyed
prone — is composition over primitives the engine already has. That is the test the whole system is
built to pass: new content is data until it names a primitive that doesn't exist, and then the
compiler tells you exactly which one to build.
