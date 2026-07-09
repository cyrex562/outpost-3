# Phase 2 Codebase Audit: Modifiers, Traits, and Resources

Date: 2026-05-01

This audit grounds Phase 2 of the modular rules architecture. It inventories
current HP/gold state, trait-shaped state, modifier-shaped logic, and resource
regeneration hooks before introducing modifier, trait, and resource services.

## 1. HP and Gold Reads

### Combat and Damage

- `src/harsh_realm/engine/combat.py:create_combat` reads
  `character.hp` and `character.max_hp` to create the player `Combatant`.
- `src/harsh_realm/gm/scenes/combat_core.py` reads player combatant HP for
  status narration and combat-over checks.
- `src/harsh_realm/gm/scenes/combat_support.py` reads enemy HP/max HP for enemy
  visibility descriptions, and saves the character with `sync_hp=True`.
- `src/harsh_realm/gm/scenes/combat_actions.py` reads player and target HP in
  attack, flee, use-item, and status flows.
- `src/harsh_realm/gm/scenes/combat_special.py` reads player/character HP for
  last-stand survival, healing, and death transition decisions.
- `src/harsh_realm/engine/low_health_narration.py` reads HP/max HP from
  combatants and event payloads to produce low-health notices.

### Healing and Rest

- `src/harsh_realm/engine/healing.py` reads `character.hp` and
  `character.max_hp` for first aid, rest, healing item use, and town healer
  calculations.
- `src/harsh_realm/gm/scenes/exploration_movement.py` reads HP/max HP for
  `status`, `rest`, and `rest until healed`.
- `src/harsh_realm/gm/scenes/respawn.py` reads `max_hp` to restore the
  character to half health after death.
- `src/harsh_realm/engine/items.py` delegates healing item behavior to
  `HealingSystem`, which reads HP/max HP.

### Shopping, Currency, and Sidebar/API Display

- `src/harsh_realm/gm/scenes/shopping.py` reads gold from
  `Character.class_abilities["gold"]` for affordability checks, purchase/sale
  narration, and shopping status output. It also reads HP/max HP for shopping
  status output.
- `src/harsh_realm/engine/healing.py:town_healer` reads coin from inventory
  currency items and gold from `class_abilities["gold"]`.
- `src/harsh_realm/api/routes.py:get_character` reads HP/max HP and
  `class_abilities` for character summary responses consumed by the frontend.
- `src/harsh_realm/models/entity_state.py` mirrors HP/max HP and
  `class_abilities` between `Character` and `CharacterState`.
- `src/harsh_realm/gm/entity_state_repository.py` reads HP/max HP and
  `class_abilities_json` from the typed character table.

### Save Throws and Encumbrance

- Save checks do not read HP or gold. `src/harsh_realm/engine/saves.py` reads
  `character.save_bonuses`, attributes, and save targets.
- Encumbrance does not currently read HP or gold. Inventory load remains
  item/equipment shaped and should not migrate in Phase 2.

## 2. HP and Gold Writes

### Combat Damage and Death-Adjacent Writes

- `src/harsh_realm/gm/scenes/combat_actions.py:_handle_attack` subtracts shock
  and normal damage from enemy combatants and clamps them to zero.
- `src/harsh_realm/gm/scenes/combat_actions.py:_handle_flee` subtracts parting
  blow damage from the player combatant and mirrors it to `self._character.hp`.
- `src/harsh_realm/gm/scenes/combat_actions.py:_handle_use` delegates healing
  item writes through `ItemSystem`/`HealingSystem`, then syncs combatant HP.
- `src/harsh_realm/gm/scenes/combat_special.py` writes HP for last-stand
  outcomes: survive at 1 HP, healing during last stand, and forced bad flee.
- Enemy HP in `CombatState` is transient combat state; Phase 2 HP resource
  migration should target character HP first, not generated enemy combatants.

### Healing Writes

- `src/harsh_realm/engine/healing.py:first_aid` writes `character.hp` after a
  successful heal or no-op full-health result.
- `src/harsh_realm/engine/healing.py:rest` writes `character.hp`.
- `src/harsh_realm/engine/healing.py:use_healing_item` writes `character.hp`.
- `src/harsh_realm/engine/healing.py:town_healer` writes `character.hp` to full.
- `src/harsh_realm/gm/scenes/exploration_movement.py` persists rest outcomes by
  emitting `exploration.rest_requested`; the handler writes the updated
  character snapshot.
- `src/harsh_realm/gm/scenes/respawn.py` writes `self._character.hp` to half of
  max HP.

### Level-Up and Recalculation Writes

- `src/harsh_realm/engine/advancement.py:apply_level_up` writes `level`,
  `max_hp`, `hp`, `attack_bonus`, `xp_next`, and saves.
- `src/harsh_realm/engine/character_recalc.py` computes `max_hp` for editor
  previews. It does not persist by itself.
- `src/harsh_realm/api/editor/characters.py` applies recalculation output to
  editor-created or editor-updated characters.

### Shopping and Admin Writes

- `src/harsh_realm/gm/scenes/shopping.py:_set_gold` writes
  `class_abilities["gold"]` for purchases and sales, then emits
  `shopping.purchase_requested` or `shopping.sale_requested`.
- `src/harsh_realm/gm/shopping_event_handlers.py` persists the character
  snapshot from shopping request events.
- `src/harsh_realm/engine/healing.py:town_healer` writes
  `class_abilities["gold"]` and may deduct currency item amounts.
- `src/harsh_realm/api/gm_command_handlers.py:handle_set_hp_requested` writes
  HP from the GM command event payload.
- `src/harsh_realm/api/gm_command_handlers.py:handle_set_gold_requested` writes
  `class_abilities["gold"]` from the GM command event payload.

## 3. Modifier-Shaped Logic

These are the current ad-hoc "source contributes a bonus/penalty to a target"
patterns. They are candidates for the Phase 2 modifier resolver or later
resolver-pipeline migrations.

- `src/harsh_realm/engine/skill_checks.py` computes
  `skill_level + attr_mod`, applies opposed NPC WIS as a difficulty modifier,
  and contains an explicit TODO for resolver-pipeline formalization.
- `src/harsh_realm/engine/skill_checks.py:resolve_with_reroll` implements the
  Expert reroll via `character_class` and
  `class_abilities["expert_reroll_available"]`. This is trait-shaped, but the
  reroll is triggered behavior and should wait for Phase 3.
- `src/harsh_realm/engine/combat.py:AttackResolver.resolve_attack` combines
  d20 roll, attack bonus, combat skill level, and attribute modifier.
- `src/harsh_realm/engine/combat.py:DamageResolver.resolve_damage` adds the
  Warrior killing-blow bonus through `is_warrior` and `level`.
- `src/harsh_realm/engine/combat.py:resolve_shock` adds STR modifier to weapon
  shock damage and clamps at zero.
- `src/harsh_realm/gm/scenes/combat_actions.py` applies an ad-hoc `-2`
  ranged-in-melee penalty.
- `src/harsh_realm/gm/scenes/combat_special.py` applies an ad-hoc `-2`
  last-stand attack skill penalty.
- `src/harsh_realm/engine/saves.py` combines stat modifier and
  `SaveBonusProfile` for physical, evasion, mental, and luck saves.
- `src/harsh_realm/engine/healing.py:first_aid` adds Heal skill level to the
  healing roll, treating untrained skill as zero for the healing amount.
- `src/harsh_realm/engine/encounters.py` applies terrain modifiers to encounter
  probability.
- `src/harsh_realm/engine/npc_personality.py` applies chaos-factor disposition
  shifts and bearing roll selection adjustments.
- `src/harsh_realm/models/item.py` and `src/harsh_realm/models/runtime.py`
  contain item `ac_bonus`, weapon damage, and save-bonus structures. Items are
  explicitly deferred from Phase 2 modifier-source integration.

## 4. Feature, Ability, and Talent Structures

Current feature-like records and state that should become trait records or feed
the trait framework:

- `packs/xwn-core/content/classes.yaml` defines class abilities:
  `veterans_luck`, `killing_blow`, `masterful_expertise`, `quick_learner`,
  `partial_veterans_luck`, and `partial_expertise`.
- `Character.class_abilities` is a JSON object used for several unrelated
  ability/resource flags, including `gold`, `ammo`, and
  `expert_reroll_available`.
- `src/harsh_realm/models/combat_runtime.py:PendingVeteranLuckRecord` stores a
  pending Veteran's Luck decision in combat state.
- `src/harsh_realm/models/gm_runtime.py:ExpertRerollState` stores pending
  social-scene Expert reroll state.
- `src/harsh_realm/models/creature.py:CreatureData.special_abilities` is a
  future-use list of special ability strings. It is trait-shaped but should not
  be migrated in Phase 2 unless creature traits become necessary for tests.
- `SaveBonusProfile` can represent item or temporary-effect bonuses to saves.
  In Phase 2, traits can express the same kind of save modifier. Existing item
  save bonuses should remain on the legacy path until item integration is
  explicitly scheduled.

Recommended Phase 2 trait candidates:

- `xwn-core:traits.class_feature.killing_blow`: passive damage modifier for
  Warriors. Current implementation remains in combat until resolver integration.
- `xwn-core:traits.class_feature.masterful_expertise`: triggered reroll marker;
  define as a trait only if needed for display, execute in Phase 3.
- GURPS/Godbound imported records from the Phase 2 spec should be the first
  end-to-end trait content because they prove passive modifier resolution
  without disrupting XWN class behavior.

## 5. Resource-Shaped State and Migration Decision

Phase 2 should migrate only HP and gold.

- HP: migrate now. It is high-traffic, bounded, emits threshold events, and has
  clear current/max semantics.
- Gold: migrate now. It is currency-shaped, currently smuggled through
  `class_abilities`, and shopping/admin writes can route through a single
  resource service.
- XP: defer. XP has level-threshold behavior owned by advancement, not a simple
  pool. It can become a resource later if advancement is redesigned around
  resource events.
- Encumbrance/load: defer. It is derivable from inventory/equipment and should
  remain an inventory calculation until inventory is refactored.
- Ammo: defer. Current ammo lives in `class_abilities["ammo"]` and is coupled to
  item/equipment behavior. The Phase 2 spec explicitly defers item-as-modifier
  and item-resource integration.
- System strain: not found as active runtime state in the current codebase.
- Faction HP/wealth: defer. Faction HP and wealth live in the faction subsystem
  and are not character resources. Migrating them would cross subsystem
  ownership boundaries and is outside Phase 2.
- Currency inventory items: defer. Town healer currently supports coin items in
  equipment. Phase 2 gold migration should preserve this compatibility path or
  convert only `class_abilities["gold"]` while leaving currency items as items.

## 6. Regeneration and World-Clock Hooks

Current rest-based HP recovery is explicit command behavior, not passive
tick-based regeneration:

- `src/harsh_realm/gm/scenes/exploration_movement.py:_handle_rest` runs
  `HealingSystem.rest(char, ticks=10)`, updates cached current HP fields, and
  emits `exploration.rest_requested`.
- `src/harsh_realm/gm/scenes/exploration_movement.py:_handle_rest_until_healed`
  loops `HealingSystem.rest(..., ticks=50)` until HP is full or a random
  encounter interrupts rest, then emits one `exploration.rest_requested` event
  with the final character snapshot.
- `src/harsh_realm/gm/exploration_event_handlers.py` handles
  `exploration.rest_requested`, persists the updated character, and emits
  `character.hp_changed`.
- `src/harsh_realm/engine/healing.py:rest` implements the mechanics:
  short rest restores 1 HP; full rest restores `max(1, level + CON modifier)`.
- Phase 1 status effects already have tick expiration:
  `StatusEffectService.expire_due(current_tick)` and status-effect handlers for
  world tick events. This is the closest existing model for future
  resource-regeneration subscription behavior.

Phase 2 resource regeneration should preserve current rest semantics. HP should
not gain passive regeneration merely because the resource framework supports
regeneration; `packs/xwn-core/content/resources/hp.yaml` should set
`regeneration: null`, and rest should later call `ResourceService.change` or
`set_current` explicitly.

