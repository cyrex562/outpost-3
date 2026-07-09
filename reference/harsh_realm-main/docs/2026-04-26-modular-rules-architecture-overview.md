# Modular Rules Architecture — Cycle Overview

**Date:** 2026-04-26
**Status:** Draft
**Phase specs:**

- `2026-04-26-modular-rules-phase-0-foundation.md` (to be written)
- `2026-04-26-modular-rules-phase-1-procedures.md` (to be written)
- `2026-04-26-modular-rules-phase-2-frameworks.md` (to be written)
- `2026-04-26-modular-rules-phase-3-trigger-effect.md` (to be written)
- `2026-04-26-modular-rules-phase-4-npc-routines.md` (to be written, **not implemented this cycle**)

**Reading order for agents:** This document first (read once), then the phase spec for the task you're picking up.

---

## 1. What this cycle is

This cycle establishes the architectural seam between **engine** and **rules content** so that future content — random tables, mechanics, traits, monsters, spells, oracle tools, alternate combat algorithms — can be added largely as data (with code-bearing escape hatches), without modifying the engine itself.

The current codebase is shaped as "an XWN-flavored game engine." This cycle reshapes it into "a generic TTRPG engine that ships with an XWN content pack as its default." The XWN ruleset doesn't go away; it becomes the first content pack rather than baked-in defaults.

The cycle deliberately does not implement most of the *content* this enables. It builds the frameworks; future cycles import GURPS bestiaries, Godbound Words and Dominion, WWN Legates, Wickham generators, and so on. The success criterion for this cycle is that the *next* cycle's content imports can largely be data work rather than engine work.

## 2. Mental model: kernel, frameworks, content packs, worlds

The architecture this cycle establishes has four layers:

- **Engine kernel.** Always present. EventBus, resolver pipelines, persistence, scenes/GM controller, parser, grid, world clock, event log. No game rules. No dice math. No hit points.
- **Mechanic frameworks.** Pluggable, framework-level subsystems that define *what kinds of things exist*. "There is a thing called HP" (resource service), "there is a thing called a status effect" (status effect service), "there is a thing called a trait" (trait/feature framework). Each framework defines the shape and lifecycle but not the specific values, formulas, or content.
- **Content packs.** Data, mostly YAML. A pack bundles content records (creatures, items, traits, tables, procedures, status effect definitions) plus optional Python modules that register resolvers, handlers, or system-specific logic. Examples: `xwn-core` (current XWN content), `gurps-bestiary` (creatures only), `godbound-base` (Words, Gifts, Dominion), `wickham-tables` (oracle and generator tables), `mythic-gme-extensions` (procedures).
- **World instances.** Per-world SQLite databases. A world is created with a list of packs in a specific load order. The world reads through to pack data for unmodified records and writes per-world overrides when the user edits content in the admin UI.

The engine kernel is small and stable. The mechanic frameworks are added one by one across this cycle's phases. Content packs (other than `xwn-core`) are mostly future-cycle work.

## 3. Why this is a multi-phase cycle, not one spec

The work splits cleanly into phases that can be implemented independently. Each phase has its own spec with task breakdown, story points, acceptance criteria, and explicit test layers. Phases can be implemented across multiple sessions; phase boundaries are commit-friendly natural seams.

Phase ordering reflects dependencies, not equal weight. Phase 0 is foundational and must complete first. Phase 1 is parallel-safe but small. Phase 2 is the largest piece of work and depends on Phase 0. Phase 3 depends on Phase 2. Phase 4 is documented this cycle but implemented in a future cycle.

| Phase | Scope | Depends on | Implemented this cycle? |
|---|---|---|---|
| 0 | Pack format, loader, world-pack binding, override layer, `xwn-core` refactor | — | Yes |
| 1 | Generator/procedure framework, status effect service | Phase 0 | Yes |
| 2 | Modifier framework, trait/feature framework, resource service | Phase 0 | Yes |
| 3 | Trigger/effect engine (small declarative DSL) | Phase 2 | Yes |
| 4 | NPC scheduled routines with interrupts | — (parallelizable) | **No — documented for future cycle** |

Phase 4's spec is written this cycle so that the framework decisions in Phases 0–3 are made knowing where NPC behavior is going, but the implementation work is deferred to a separate cycle.

## 4. Source-system test cases

Each phase has at least one concrete test case drawn from a real source system. These are not aspirational; they are the proofs that the framework actually works.

- **Phase 0:** Existing XWN content fully migrated into `packs/xwn-core/`. World creation lets the user pick packs. Editing a record in admin creates a per-world override; "revert to pack default" restores it.
- **Phase 1:** UNE personality generation re-implemented as a procedure (currently hardcoded). One Wickham town generator imported as a procedure. One status effect (e.g., "Poisoned") imported as a content record and applied/expired correctly.
- **Phase 2:** A representative slice of GURPS Advantages and Disadvantages imported as trait records (mechanically functional in their non-world-mutation parts). A Godbound Gift imported as a trait. Bennies defined as a content-only resource with no Python.
- **Phase 3:** A non-trivial Godbound Gift expressed entirely declaratively in the trigger/effect DSL.
- **Phase 4 (future cycle):** A shopkeeper NPC follows a daily schedule (open shop → tend shop → close shop → tavern → home → sleep), interrupts to player interaction, returns to schedule afterward.

The Godbound test cases this cycle deliberately exclude Words of Creation and Dominion mechanics that mutate world structure. Those require subsystems (weather, dynamic settlement creation, faction creation API) that are deferred to future cycles. The frameworks in this cycle are designed *knowing* those verbs will land later, but ship without them.

## 5. Cross-cutting principles

These apply across every phase. They live here in the overview so they're a single contract; phase specs implement against them.

### 5.1 Pack identity and namespacing

Every content record has a namespaced ID of the form `<pack-id>:<category>.<slug>`. Examples: `xwn-core:weapon.short_sword`, `gurps-bestiary:creature.dire_wolf`, `godbound-base:gift.sky.command_winds`. Pack IDs are lowercase kebab-case. Slugs are lowercase snake_case.

A pack's ID is immutable across versions. A pack with a new ID is a new pack, not an update. Worlds record the exact pack IDs and versions they were created with.

### 5.2 Pack load order and conflict resolution

A world specifies its pack list as an ordered sequence at creation time. Earlier packs in the list are foundational; later packs override or extend them. The default convention: `xwn-core` first, then themed packs (e.g., `godbound-base`), then content packs (e.g., `gurps-bestiary`).

When two packs define a record with the same fully-qualified ID (which can happen only if a later pack explicitly *targets* an earlier pack's namespace, since packs normally use their own namespace), the later pack wins. When two packs define records in different namespaces with conflicting *behavior* (e.g., two packs both register a status effect resolver for "burning"), the loader detects this and reports an error at world creation time. Conflicts are not silently resolved.

Pack dependencies are declared explicitly. A pack manifest lists `depends: [xwn-core@>=1.0]` for hard requirements. The loader fails fast if dependencies are missing.

### 5.3 World override layer

Pack data is read-only. When the user edits a record through the admin UI, the edit is stored in a per-world `pack_overrides` table keyed by pack ID + record ID. Reads check overrides first, fall back to pack data. The admin UI displays clearly which records are overridden and offers a "revert to pack default" action that deletes the override row.

This means pack data and world state remain cleanly separated: a pack update doesn't lose user edits, and reverting an edit doesn't require the user to remember the original value.

### 5.4 Pack frozen at world creation

A world's pack list is recorded at creation and not normally changed. Mid-game pack swapping is out of scope. A world can be migrated to a new pack list as an explicit operation that requires user confirmation and, potentially, data migration steps. The default user experience: pick packs, create world, play; if you want a different pack mix, create a new world.

This matches Factorio's model and avoids a class of subtle bugs around mid-game state that depends on now-removed pack content.

### 5.5 Migrations

Pack version updates can require two kinds of migration:

- **Data migrations** are the common case. The pack format change is rewriting existing rows in place — renamed fields, new defaults, restructured nested data. The pack ships migration scripts that transform records of the old version into the new version. These run when a world is loaded against an updated pack version.
- **Schema migrations** are required when a pack update touches durable subsystem tables (e.g., adding a column to `entity_status_effects`). These are SQLite DDL changes that ship alongside the pack. The engine's existing migration system (or a new one if needed; Phase 0 will determine) runs these before the world is loaded.

A pack manifest declares which kinds of migrations its current version requires from each previous version. A pack that introduces no schema changes only ships data migrations.

### 5.6 Code-bearing pack API surface

Code-bearing packs can register against a narrow set of API surfaces. The Phase-0 surface is intentionally minimal; subsequent phases add to it.

- **Phase 0:** register pack with manifest, declare dependencies, contribute YAML data files.
- **Phase 1:** register procedure step types, register status effect resolvers.
- **Phase 2:** register modifier source types, register trait implementations, register resource definitions.
- **Phase 3:** register trigger condition operators, register effect verbs.
- **Future cycles (not in scope):** register scene types, register subsystem-level frameworks, register grid topologies.

Code-bearing packs are *trusted*. There is no sandbox. They run as ordinary Python modules in the engine process. This is acceptable because Harsh Realm is single-user; pack distribution and trust models are deferred.

### 5.7 House rules become a pack

The existing `house_rules/` extension point is preserved as the canonical location for code-bearing pack code. Existing entries (the practice-skills resolver) become part of `xwn-core`'s code surface. New code-bearing pack content lives in `house_rules/` directories named for their pack, e.g., `house_rules/godbound_base/` for a hypothetical Godbound code-bearing pack. The `house_rules/` directory is the Python escape hatch; the trigger/effect DSL is the declarative path; most pack content uses neither.

### 5.8 Test layers per task

Every task in every phase spec lists which of the four test layers apply (pytest unit, Hypothesis property, mutmut mutation, Playwright E2E). Frontend tasks additionally list Vitest unit, fast-check property, Stryker mutation. The default for any task is "all applicable layers." The phase spec may explicitly note that a layer is N/A (e.g., a backend-only task has no Playwright requirement) but never silently skip.

## 6. Pack format at a glance

Phase 0 specifies this in detail. A high-level shape for orientation:

```
packs/
  xwn-core/
    pack.yaml                 # Manifest: id, version, name, description, depends, conflicts
    content/
      weapons/                # Categorized YAML content files
        short_sword.yaml
        ...
      armor/
      creatures/
      tables/
      classes/
      skills.yaml
    code/                     # Optional Python module(s) for code-bearing packs
      __init__.py
      house_rules/
        practice_skills.py
    migrations/
      data/                   # Data migration scripts per version
        v1_to_v2.py
      schema/                 # Schema migration scripts per version
        v1_to_v2.sql
```

A pack can be a directory or a `.zip` archive treated as a directory. The loader handles both transparently.

## 7. Things this cycle does NOT do

These are explicitly out of scope. Most are documented elsewhere or get their own cycle:

- **Words of Creation, Dominion, and Legate domain mechanics.** The frameworks built this cycle support the modifier-and-trait-shaped portions, but world-mutation verbs (create_settlement, shift_weather_pattern, transform_terrain, establish_dominion) and the dependent subsystems (weather, dynamic settlement creation, faction creation API) are future-cycle work.
- **Behavior trees, utility AI, full drive simulation for NPCs.** Phase 4 documents the FSM-with-scheduled-routines pattern; behavior trees are a future evolution.
- **Quest/plotline service.** Future.
- **Weather, season, economy subsystems.** Future. The trigger/effect engine ships without their verbs.
- **Resolver pipeline formalization for skill checks.** Already deferred from the rules-architecture spec; remains deferred.
- **Sandboxing or trust models for code-bearing packs.** Single-user deployment makes this unnecessary now.
- **Mid-game pack swapping.** Out of scope.
- **Pack distribution, registry, marketplace.** Out of scope.
- **Visual node-graph or behavior-tree authoring UIs.** The admin UI for this cycle continues to be form-based.
- **Combat replacement (GURPS-style hit locations, Savage-style action cards, etc.).** The existing XWN combat stays. Combat plug-points are not designed this cycle.
- **Magic / psionics systems.** Already deferred in the project roadmap; nothing in this cycle adds them.

## 8. Success criteria for the cycle

The cycle is complete when *all* of the following hold:

1. `packs/xwn-core/` exists and contains all current XWN content. `data/` no longer has hardcoded engine defaults outside the pack.
2. World creation accepts a pack list. Loading a world reconstitutes pack data plus world overrides correctly. Editing a record creates an override; reverting clears it.
3. Pack manifest, loader, conflict detection, dependency resolution, and migration scaffolding are implemented and tested.
4. The generator/procedure framework exists. UNE personality generation runs as a procedure. At least one Wickham generator is importable as data.
5. The status effect service exists. Status effects can be defined as content records and applied to entities with a duration.
6. The modifier framework, trait/feature framework, and resource service exist. HP and gold are refactored as resource instances. A representative GURPS Advantage and a representative Godbound Gift work as trait content (where their effects don't require deferred world-mutation verbs).
7. The trigger/effect engine exists. The trigger DSL has a documented condition language and effect verb list. A non-trivial Godbound Gift is expressed entirely declaratively.
8. Phase 4 spec is written and committed; implementation is not started.
9. Test counts have not decreased. All existing tests pass. New code carries the four-layer test rule per AGENTS.md.
10. `CLAUDE.md` and `AGENTS.md` are updated to reflect the new architecture. The pack model is documented in `AGENTS.md` as a canonical reference.

## 9. Reading order for agents

Working on a task in this cycle:

1. Read this overview once. (You're doing it now.)
2. Read the phase spec for the phase containing your task.
3. Read the relevant existing files in the codebase before editing. The rules-architecture spec at `docs/superpowers/specs/2026-04-22-rules-architecture-design.md` defines Rule 1–4; this cycle's frameworks comply with those rules.
4. Read `AGENTS.md` for coding standards and the four-layer test policy.
5. Implement, test, commit. One task per session is the target; tasks are sized for 1–3 story points so this is realistic.

When in doubt about a low-stakes detail (manifest field name, error message wording, file layout), make a best-guess decision and flag it in the commit message as `Decision: <description>` so it can be reviewed. When in doubt about a high-stakes detail (DSL verb list, condition language semantics, override resolution algorithm), stop and ask before implementing.

## 10. Open questions for the human (Josh)

These are flagged for resolution before the relevant phase starts. Best-guess defaults are noted; phase specs will use them unless overridden.

- **Pack archive format.** Default: directory at dev time, optional `.zip` archive at distribution. Confirm.
- **Manifest filename and root key conventions.** Default: `pack.yaml` at pack root, top-level fields `id`, `version`, `name`, `description`, `authors`, `depends`, `conflicts`, `provides`. Confirm before Phase 0.
- **Pack version semantics.** Default: SemVer-like `MAJOR.MINOR.PATCH` strings. Major version bumps allow schema migrations; minor/patch bumps do not. Confirm.
- **Override storage.** Default: per-world SQLite table `pack_overrides(pack_id, record_id, data_json, updated_at)`. Confirm before Phase 0.
- **Trigger DSL condition syntax.** Default: a small expression language with `==`, `!=`, `<`, `<=`, `>`, `>=`, `and`, `or`, `not`, plus dotted path access into entity/event payload. **High-stakes — flagged for explicit review before Phase 3.**
- **Trigger DSL effect verb list.** Default: minimal initial list (apply_modifier, remove_modifier, change_resource, apply_status, remove_status, emit_event). **High-stakes — flagged for explicit review before Phase 3.**
- **`xwn-core` pack version.** Default: 1.0.0 at refactor completion. Confirm.

---

## 11. Deferred items (master list)

Items deferred from this cycle to future cycles. As each phase spec is written, additional deferrals get appended here. This list is the authoritative deferred-work record for the cycle.

### Deferred to future cycles

- **Words of Creation, Dominion, Legate domain powers as content.** Frameworks support the shape; content lands when world-mutation verbs and their subsystems land.
- **World-mutation verbs in the trigger/effect engine.** `create_settlement`, `shift_weather_pattern`, `transform_terrain`, `establish_dominion`, etc. Each verb depends on its target subsystem existing.
- **Weather subsystem.** Required by some Words of Creation content.
- **Season and time-of-day simulation subsystems.** Required by NPC scheduled routines (Phase 4) and some Words content.
- **Economy subsystem.** Required for Bennies-style narrative currencies that interact with world simulation.
- **Reputation and faction-affiliation subsystems.** Documented in the rules-arch spec as Rule 1 carried-forward violations; replace `Character.faction_id`/`NPC.faction_id` scalars when these land.
- **Quest/plotline service.** Future cycle.
- **Behavior trees and utility AI for NPCs.** Phase 4 ships FSM-with-routines; BT/utility AI is a future evolution.
- **NPC scheduled routines implementation.** Spec written this cycle (Phase 4); implementation is a separate future cycle.
- **Resolver pipeline formalization for skill checks.** Inherited from rules-arch spec; still deferred.
- **GURPS combat replacement.** Hit locations, active defenses, posture, range/speed — all future-cycle work.
- **Magic and psionics systems.** Inherited deferral from project roadmap.
- **Code-bearing pack sandboxing.** Out of scope while Harsh Realm is single-user.
- **Pack distribution, registry, marketplace.** Out of scope.
- **Visual authoring UIs (node graphs, behavior trees, dialogue editors).** Out of scope.
- **Mid-game pack swapping.** Out of scope.

Phase-specific deferrals will be appended to this list as phase specs are written.
