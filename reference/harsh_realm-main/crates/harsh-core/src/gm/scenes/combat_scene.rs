//! Combat scene handler for Harsh Realm.
//!
//! Ported from `src/harsh_realm/gm/scenes/combat.py` and the five mixin modules
//! (`combat_core.py`, `combat_actions.py`, `combat_enemy.py`,
//! `combat_special.py`, `combat_support.py`).

use rand::rngs::ThreadRng;
use rand::Rng;
use serde_json::{Map, Value as JsonValue};

use crate::character::Character;
use crate::combat::action_resolver::resolve_action;
use crate::combat::flee::resolve_flee;
use crate::combat::resolvers::{resolve_attack, resolve_shock, AttackParams};
use crate::combat_runtime::{Combatant, CombatState, FleeOpponent, GroundLoot};
use crate::creature::CreatureData;
use crate::db::WorldDatabase;
use crate::enemy_ai::choose_action;
use crate::engine_runtime::PendingVeteranLuckRecord;
use crate::events::GameEvent;
use crate::gm::scenes::base::{SceneHandler, SceneState};
use crate::intent::Intent;
use crate::item::ItemData;
use crate::item_registry::ItemRegistry;
use crate::items::ItemSystem;
use crate::difficulty::DifficultyProfile;
use crate::loot_gen::LootGenerator;
use crate::resolution::damage::apply_damage;
use crate::resolution::economy::{can_afford, Cost};
use crate::resolution::targeting::TargetShape;
use crate::runtime_content::{
    EntityResolver, EntitySnapshot, EvalContextBuilder, RuntimeContentStore, TriggerRuntime,
};
use crate::status_effects::repository::StatusEffectRepository;
use crate::status_effects::service::{StatusEffectService, WorldClock};
use crate::combat::positioning::{assign_positions, positions_notice};
use crate::payloads::notices_combat::{
    CharacterDeathNotice, CharacterSnapshot, CombatActionButton, CombatActionsNotice,
    CombatAttackNotice, CombatEnemyDefeatedNotice, CombatFledNotice, CombatPlayerHitNotice,
    CombatStartNotice, EnemyCombatantState, NarrationNotice,
};
use crate::payloads::requests::{
    CombatConsumeAmmoRequested, CombatFleeRequested, CombatHarvestRequested,
    CombatTakeDamageRequested, CombatUpdateCharacterRequested, CombatUseItemRequested,
    CombatVictoryRequested,
};
use crate::command::ParsedCommand;
use crate::runtime::{InventoryItemRecord, JsonObject};

// ---------------------------------------------------------------------------
// Simple flee-difficulty wrapper (CreatureData doesn't impl FleeOpponent yet)
// ---------------------------------------------------------------------------

#[allow(dead_code)]
struct CreatureFlee(i32);
impl FleeOpponent for CreatureFlee {
    fn flee_difficulty(&self) -> i32 {
        self.0
    }
}

/// Bounded cascade depth for IR-driven trigger chains spawned by enemy actions.
const IR_TRIGGER_CASCADE_DEPTH: usize = 8;

/// Fixed world-clock view for the action resolver's status service.
struct CombatClock(i32);
impl WorldClock for CombatClock {
    fn tick(&self) -> i32 {
        self.0
    }
}

/// A combatant's spendable resources for action-cost affordability checks: each
/// typed pool by id, plus `hp`.
fn resource_map(c: &Combatant) -> std::collections::BTreeMap<String, i64> {
    let mut map = std::collections::BTreeMap::new();
    map.insert("hp".to_string(), c.hp as i64);
    for pool in &c.pools {
        map.insert(pool.id.clone(), pool.current);
    }
    map
}

// ---------------------------------------------------------------------------
// CombatScene
// ---------------------------------------------------------------------------

/// Full combat scene assembled from all five Python mixin modules.
pub struct CombatScene {
    character: Character,
    state: CombatState,
    rng: ThreadRng,
    creatures: Vec<CreatureData>,
    tick: i64,
    pending_transition: Option<SceneState>,
    item_system: ItemSystem,
    item_registry: Option<ItemRegistry>,
    enemy_surprise_pending: bool,
    /// World IR content, so an enemy with authored `actions` resolves them through
    /// the IR pipeline on its turn (`None` → legacy attacks only).
    runtime_content: Option<RuntimeContentStore>,
    /// Difficulty profile loaded at combat start; all scalars for this encounter.
    difficulty_profile: DifficultyProfile,
}

impl CombatScene {
    /// Create a new `CombatScene`.
    pub fn new(
        character: Character,
        combat_state: CombatState,
        creatures: Vec<CreatureData>,
        item_registry: Option<ItemRegistry>,
        runtime_content: Option<RuntimeContentStore>,
        difficulty_profile: DifficultyProfile,
    ) -> Self {
        let enemy_surprise_pending = combat_state.enemy_surprise;
        Self {
            character,
            state: combat_state,
            rng: rand::thread_rng(),
            creatures,
            tick: 0,
            pending_transition: None,
            item_system: ItemSystem,
            item_registry,
            enemy_surprise_pending,
            runtime_content,
            difficulty_profile,
        }
    }

    // -----------------------------------------------------------------------
    // SceneHandler helpers
    // -----------------------------------------------------------------------

    fn narrate(&self, text: impl Into<String>) -> GameEvent {
        let notice = NarrationNotice { text: text.into() };
        let data: JsonObject =
            serde_json::from_value(serde_json::to_value(&notice).unwrap_or_default())
                .unwrap_or_default();
        GameEvent::new(self.tick, "gm.narrate", data).with_source("gm")
    }

    fn char_snapshot(&self) -> CharacterSnapshot {
        character_snapshot(&self.character)
    }

    fn update_character_event(&self) -> GameEvent {
        let req = CombatUpdateCharacterRequested {
            character_id: self.character.id.clone(),
            character_data: self.char_snapshot(),
        };
        payload_event(self.tick, "combat.update_character_requested", &req)
    }

    /// Build the `combat.start` event with current enemy roster and HP for the
    /// frontend panel. Called by the controller immediately after wiring.
    pub fn make_start_event(&self, tick: i64) -> GameEvent {
        let enemy_states: Vec<EnemyCombatantState> = self
            .state
            .combatants
            .iter()
            .filter(|c| !c.is_player)
            .map(|c| EnemyCombatantState {
                entity_id: c.entity_id.clone(),
                name: c.display_name.clone(),
                hp: c.hp,
                max_hp: c.max_hp,
            })
            .collect();
        let awareness = if self.state.player_surprise {
            "player_surprise"
        } else if self.state.enemy_surprise {
            "enemy_surprise"
        } else {
            "mutual"
        };
        let notice = CombatStartNotice {
            awareness: awareness.to_string(),
            enemies: enemy_states.iter().map(|e| e.name.clone()).collect(),
            enemy_states,
        };
        payload_event(tick, "combat.start", &notice)
    }

    /// Build the `combat.positions` event from the current state.
    ///
    /// Called at combat start (right after `make_start_event`) and after every
    /// band change (`advance` / `withdraw`). The event carries the full 81-cell
    /// grid with terrain backdrop and the positions of all living combatants.
    pub fn make_positions_event(&self, tick: i64) -> GameEvent {
        let notice = positions_notice(&self.state);
        payload_event(tick, "combat.positions", &notice)
    }

    // -----------------------------------------------------------------------
    // Combatant helpers
    // -----------------------------------------------------------------------

    fn get_player_combatant(&self) -> Option<&Combatant> {
        self.state.combatants.iter().find(|c| c.is_player)
    }

    #[allow(dead_code)]
    fn get_player_combatant_mut(&mut self) -> Option<&mut Combatant> {
        self.state.combatants.iter_mut().find(|c| c.is_player)
    }

    pub(crate) fn find_combatant(&self, entity_id: &str) -> Option<&Combatant> {
        self.state
            .combatants
            .iter()
            .find(|c| c.entity_id == entity_id)
    }

    fn find_combatant_mut(&mut self, entity_id: &str) -> Option<&mut Combatant> {
        self.state
            .combatants
            .iter_mut()
            .find(|c| c.entity_id == entity_id)
    }

    fn update_hp(&mut self, entity_id: &str, new_hp: i32) {
        let clamped = new_hp.max(0);
        if let Some(c) = self.find_combatant_mut(entity_id) {
            c.hp = clamped;
            c.alive = clamped > 0;
        }
        // Keep the persisted character's HP in lockstep with the player's combatant.
        // `update_hp` is the single mutation point for combatant HP, so every
        // player-damage path funnels through it: the legacy weapon attack, IR actions
        // (via `apply_resource_delta`), Last Stand, and in-combat healing. Previously
        // only the legacy path re-synced `self.character.hp` afterward, so IR-dealt
        // damage (and combat heals) left `self.character.hp` stale — and because the
        // flee/victory snapshots are built from `self.character`, combat persisted the
        // player's *pre-combat* HP. Post-combat `rest` then saw an uninjured character
        // and healed nothing. Syncing here fixes all paths uniformly.
        if entity_id == self.character.id {
            self.character.hp = clamped;
        }
    }

    fn apply_damage(&mut self, entity_id: &str, damage: i32) {
        let current_hp = self
            .find_combatant(entity_id)
            .map(|c| c.hp)
            .unwrap_or(0);
        self.update_hp(entity_id, current_hp - damage);
    }

    fn get_enemy_status_line(&self, enemy: &Combatant) -> String {
        if self.state.enemy_detail_revealed {
            return format!(
                "  {}: {}/{} HP, AC {}",
                enemy.display_name, enemy.hp, enemy.max_hp, enemy.ac
            );
        }
        let hp_pct = enemy.hp as f32 / enemy.max_hp.max(1) as f32;
        let desc = if hp_pct >= 0.75 {
            "healthy"
        } else if hp_pct >= 0.50 {
            "wounded"
        } else if hp_pct >= 0.25 {
            "badly wounded"
        } else {
            "near death"
        };
        format!("  {} looks {}.", enemy.display_name, desc)
    }

    fn get_attack_skill_level(&self) -> i32 {
        let stab = self.character.skills.get("stab").copied().unwrap_or(-1);
        let shoot = self.character.skills.get("shoot").copied().unwrap_or(-1);
        let punch = self.character.skills.get("punch").copied().unwrap_or(-1);
        stab.max(shoot).max(punch)
    }

    fn get_alive_flee_difficulties(&self) -> Vec<i32> {
        let living_ids: std::collections::HashSet<String> = self
            .state
            .combatants
            .iter()
            .filter(|c| !c.is_player && c.alive)
            .map(|c| {
                let s = &c.entity_id;
                match s.rfind('_') {
                    Some(idx) => s[..idx].to_string(),
                    None => s.clone(),
                }
            })
            .collect();

        if !self.creatures.is_empty() {
            self.creatures
                .iter()
                .filter(|cr| living_ids.contains(&cr.id))
                .map(|cr| cr.flee_difficulty)
                .collect()
        } else {
            // Default difficulty when no creature data available
            self.state
                .combatants
                .iter()
                .filter(|c| !c.is_player && c.alive)
                .map(|_| 8)
                .collect()
        }
    }

    fn lookup_weapon(&self, player: &Combatant) -> Option<&ItemData> {
        let registry = self.item_registry.as_ref()?;
        let weapon_id = player.weapon_id.as_deref()?;
        registry.get(weapon_id)
    }

    fn get_ammo_count(&self, ammo_type: &str) -> i32 {
        let mut count = 0;
        for raw in &self.character.equipment {
            if let Ok(item) = serde_json::from_value::<InventoryItemRecord>(
                serde_json::Value::Object(raw.clone()),
            ) {
                if item.item_id.as_deref() == Some(ammo_type)
                    || item.name.to_lowercase() == ammo_type.to_lowercase()
                {
                    count += item.quantity;
                }
            }
        }
        count
    }

    fn consume_ammo_event(&self, ammo_type: &str) -> GameEvent {
        let req = CombatConsumeAmmoRequested {
            character_id: self.character.id.clone(),
            character_data: self.char_snapshot(),
            ammo_type: ammo_type.to_string(),
        };
        payload_event(self.tick, "combat.consume_ammo_requested", &req)
    }

    fn auto_switch_to_melee(&mut self, player_entity_id: String) -> String {
        // Find a melee fallback weapon
        let fallback: Option<(String, String)> =
            if let Some(registry) = &self.item_registry {
                self.character.equipment.iter().find_map(|raw| {
                    let item_id = raw.get("item_id")?.as_str()?;
                    let item_data = registry.get(item_id)?;
                    if item_data.is_melee() {
                        Some((item_id.to_string(), item_data.name.clone()))
                    } else {
                        None
                    }
                })
            } else {
                None
            };

        if let Some((id, name)) = fallback {
            if let Some(p) = self.find_combatant_mut(&player_entity_id) {
                p.weapon_id = Some(id);
                p.range_band = "melee".to_string();
            }
            format!("[No ammo — switched to {name}]")
        } else {
            if let Some(p) = self.find_combatant_mut(&player_entity_id) {
                p.weapon_id = Some("weapon.unarmed".to_string());
                p.damage_expr = "1d2".to_string();
                p.range_band = "melee".to_string();
            }
            "[No ammo — fighting unarmed]".to_string()
        }
    }

    fn parse_target<'a>(
        &self,
        cmd: &ParsedCommand,
        enemies: &'a [&'a Combatant],
    ) -> Option<&'a Combatant> {
        if enemies.is_empty() {
            return None;
        }
        if cmd.args.is_empty() {
            return Some(enemies[0]);
        }
        let target_str = cmd.args.join(" ").to_lowercase();
        for &enemy in enemies {
            if enemy.display_name.to_lowercase() == target_str {
                return Some(enemy);
            }
        }
        for &enemy in enemies {
            if enemy.display_name.to_lowercase().contains(&target_str) {
                return Some(enemy);
            }
        }
        Some(enemies[0])
    }

    fn advance_round(&mut self) {
        self.state.current_turn_index += 1;
        if self.state.current_turn_index >= self.state.initiative_order.len() as i32 {
            self.state.current_turn_index = 0;
            self.state.round_number += 1;
        }
    }

    // -----------------------------------------------------------------------
    // Combat action bar
    // -----------------------------------------------------------------------

    /// Build the structured action button list for the current combat phase.
    ///
    /// Mirrors the `get_suggestions` / `get_valid_commands` logic but returns
    /// typed [`CombatActionsNotice`] instead of plain strings.
    fn actions_notice(&self) -> CombatActionsNotice {
        if self.state.pending_veteran_luck.is_some() {
            return CombatActionsNotice {
                phase: "prompt".to_string(),
                actions: vec![
                    CombatActionButton {
                        command: "yes".to_string(),
                        label: "Use Veteran's Luck".to_string(),
                        style: "primary".to_string(),
                    },
                    CombatActionButton {
                        command: "no".to_string(),
                        label: "Take the hit".to_string(),
                        style: "default".to_string(),
                    },
                ],
            };
        }

        if self.state.last_stand_active {
            return CombatActionsNotice {
                phase: "last_stand".to_string(),
                actions: vec![
                    CombatActionButton {
                        command: "attack".to_string(),
                        label: "Last Stand: Attack".to_string(),
                        style: "danger".to_string(),
                    },
                    CombatActionButton {
                        command: "use".to_string(),
                        label: "Use Item".to_string(),
                        style: "default".to_string(),
                    },
                    CombatActionButton {
                        command: "flee".to_string(),
                        label: "Flee".to_string(),
                        style: "default".to_string(),
                    },
                ],
            };
        }

        if self.state.combat_over {
            let mut actions: Vec<CombatActionButton> = self
                .state
                .pending_harvest
                .iter()
                .map(|h| CombatActionButton {
                    command: format!("harvest {}", h.material),
                    label: format!("Harvest {}", capitalize(&h.material)),
                    style: "default".to_string(),
                })
                .collect();
            actions.push(CombatActionButton {
                command: "leave".to_string(),
                label: "Leave".to_string(),
                style: "primary".to_string(),
            });
            return CombatActionsNotice {
                phase: "over".to_string(),
                actions,
            };
        }

        // Normal turn: check for alive enemies.
        let has_alive_enemies = self
            .state
            .combatants
            .iter()
            .any(|c| !c.is_player && c.alive);

        if !has_alive_enemies {
            return CombatActionsNotice {
                phase: "over".to_string(),
                actions: vec![CombatActionButton {
                    command: "leave".to_string(),
                    label: "Leave".to_string(),
                    style: "primary".to_string(),
                }],
            };
        }

        CombatActionsNotice {
            phase: "active".to_string(),
            actions: vec![
                CombatActionButton {
                    command: "attack".to_string(),
                    label: "Attack".to_string(),
                    style: "primary".to_string(),
                },
                CombatActionButton {
                    command: "flee".to_string(),
                    label: "Flee".to_string(),
                    style: "danger".to_string(),
                },
                CombatActionButton {
                    command: "use".to_string(),
                    label: "Use Item".to_string(),
                    style: "default".to_string(),
                },
                CombatActionButton {
                    command: "advance".to_string(),
                    label: "Advance".to_string(),
                    style: "default".to_string(),
                },
                CombatActionButton {
                    command: "withdraw".to_string(),
                    label: "Withdraw".to_string(),
                    style: "default".to_string(),
                },
            ],
        }
    }

    /// Build a `combat.actions` event wrapping the current action bar state.
    pub fn actions_event(&self, tick: i64) -> GameEvent {
        payload_event(tick, "combat.actions", &self.actions_notice())
    }

    // -----------------------------------------------------------------------
    // Command dispatch
    // -----------------------------------------------------------------------

    fn dispatch_command(
        &mut self,
        cmd: &ParsedCommand,
        db: &WorldDatabase,
    ) -> Result<Vec<GameEvent>, String> {
        let verb = cmd.verb.as_str();

        if self.state.pending_veteran_luck.is_some() {
            let raw = cmd.raw.trim().to_lowercase();
            if verb == "yes" || raw.starts_with("yes") {
                return Ok(self.handle_veteran_luck_yes());
            }
            if verb == "no" || raw.starts_with("no") {
                return Ok(self.handle_veteran_luck_no());
            }
            return Ok(vec![
                self.narrate("Please answer yes or no for Veteran's Luck.")
            ]);
        }

        if self.state.last_stand_active {
            return match verb {
                "attack" => Ok(self.handle_last_stand_attack(cmd)),
                "use" => Ok(self.handle_last_stand_use(cmd)),
                "flee" | "move" => Ok(self.handle_last_stand_flee()),
                "help" => Ok(self.handle_help()),
                "status" => Ok(self.handle_status()),
                _ => Ok(vec![self.narrate(
                    "Last stand — attack, use <item>, or flee.",
                )]),
            };
        }

        if self.state.combat_over {
            return match verb {
                "harvest" => Ok(self.handle_harvest(cmd)),
                "look" | "leave" => {
                    self.state.pending_harvest.clear();
                    Ok(vec![self.narrate("You leave the battlefield.")])
                }
                "move" if cmd.direction.is_some() => {
                    self.state.pending_harvest.clear();
                    Ok(vec![self.narrate("You leave the battlefield.")])
                }
                "status" => Ok(self.handle_status()),
                _ => {
                    if !self.state.pending_harvest.is_empty() {
                        let mats: Vec<String> = self
                            .state
                            .pending_harvest
                            .iter()
                            .map(|h| h.material.clone())
                            .collect();
                        Ok(vec![self.narrate(format!(
                            "The fight is over. You can harvest: {}.\nUse 'harvest <material>' or 'leave' to move on.",
                            mats.join(", ")
                        ))])
                    } else {
                        Ok(vec![self.narrate(
                            "The fight is over. Type 'look' to survey your surroundings.",
                        )])
                    }
                }
            };
        }

        match verb {
            "attack" => Ok(self.handle_attack(cmd, db)),
            "flee" => Ok(self.handle_flee()),
            "use" => Ok(self.handle_use(cmd, db)),
            "advance" => Ok(self.handle_advance()),
            "withdraw" => Ok(self.handle_withdraw()),
            "harvest" => Ok(self.handle_harvest(cmd)),
            "status" => Ok(self.handle_status()),
            "help" => Ok(self.handle_help()),
            _ => Ok(vec![self.narrate(format!(
                "Unknown combat command \"{}\". Try: attack, flee, use <item>, status, help.",
                cmd.raw
            ))]),
        }
    }

    // -----------------------------------------------------------------------
    // Action handlers
    // -----------------------------------------------------------------------

    fn handle_attack(&mut self, cmd: &ParsedCommand, db: &WorldDatabase) -> Vec<GameEvent> {
        let round = self.state.round_number;
        let mut events = vec![self.narrate(format!(
            "--- Round {round} — Your turn ---"
        ))];

        let player_id = match self.get_player_combatant() {
            Some(p) => p.entity_id.clone(),
            None => return vec![self.narrate("[ERROR] Player combatant not found.")],
        };

        let alive_enemy_ids: Vec<String> = self
            .state
            .combatants
            .iter()
            .filter(|c| !c.is_player && c.alive)
            .map(|c| c.entity_id.clone())
            .collect();

        if alive_enemy_ids.is_empty() {
            self.state.combat_over = true;
            events.push(self.narrate("All enemies are defeated. Victory!"));
            return events;
        }

        // Resolve target from command args
        let target_id: String = {
            let alive_refs: Vec<&Combatant> = self
                .state
                .combatants
                .iter()
                .filter(|c| !c.is_player && c.alive)
                .collect();
            match self.parse_target(cmd, &alive_refs) {
                Some(t) => t.entity_id.clone(),
                None => {
                    let names: Vec<String> = alive_refs
                        .iter()
                        .map(|c| c.display_name.clone())
                        .collect();
                    return vec![self.narrate(format!(
                        "Attack whom? Enemies: {}",
                        names.join(", ")
                    ))];
                }
            }
        };

        // Check range
        let (player_range_band, _weapon_id_opt, weapon_is_melee, weapon_is_ranged, weapon_ammo_type) = {
            let player = self.get_player_combatant().unwrap();
            let wid = player.weapon_id.clone();
            let range_band = player.range_band.clone();
            if let Some(w) = self.lookup_weapon(player) {
                (
                    range_band,
                    wid,
                    w.is_melee(),
                    w.is_ranged(),
                    w.ammo_type.clone(),
                )
            } else {
                (range_band, wid, false, false, None)
            }
        };

        if weapon_is_melee && player_range_band != "melee" {
            return vec![self.narrate(
                "You need to advance to melee range first. (advance)",
            )];
        }

        // Auto-switch if out of ammo
        let mut ammo_switch_msg: Option<String> = None;
        let (weapon_is_ranged, weapon_ammo_type, weapon_is_melee) =
            if weapon_is_ranged {
                if let Some(ref ammo_type) = weapon_ammo_type {
                    if self.get_ammo_count(ammo_type) <= 0 {
                        let pid = player_id.clone();
                        ammo_switch_msg = Some(self.auto_switch_to_melee(pid));
                        // Re-read weapon after switch
                        let player = self.get_player_combatant().unwrap();
                        if let Some(w) = self.lookup_weapon(player) {
                            (w.is_ranged(), w.ammo_type.clone(), w.is_melee())
                        } else {
                            (false, None, false)
                        }
                    } else {
                        (weapon_is_ranged, weapon_ammo_type, weapon_is_melee)
                    }
                } else {
                    (weapon_is_ranged, weapon_ammo_type, weapon_is_melee)
                }
            } else {
                (weapon_is_ranged, weapon_ammo_type, weapon_is_melee)
            };

        let (attr_mod, _current_range_band, final_weapon_id) = {
            let player = self.get_player_combatant().unwrap();
            let w = self.lookup_weapon(player);
            let attr_key = w.and_then(|wd| wd.attribute.as_deref()).unwrap_or("str");
            let mut am = self.character.attr_mods.get(attr_key).copied().unwrap_or(0);
            if weapon_is_ranged && player.range_band == "melee" {
                am -= 2;
            }
            (am, player.range_band.clone(), player.weapon_id.clone())
        };

        let (attack_result, player_display_name, target_display_name, target_ac) = {
            let player = self
                .state
                .combatants
                .iter()
                .find(|c| c.entity_id == player_id)
                .unwrap()
                .clone();
            let target = self
                .state
                .combatants
                .iter()
                .find(|c| c.entity_id == target_id)
                .unwrap()
                .clone();
            let ac = target.ac;
            let pdn = player.display_name.clone();
            let tdn = target.display_name.clone();
            let is_warrior = self.character.character_class == "warrior";
            let skill_level = self.get_attack_skill_level();
            let result = resolve_attack(
                &player,
                &target,
                AttackParams {
                    is_warrior,
                    warrior_level: self.character.level,
                    skill_level,
                    attr_mod,
                    attack_modifier: self.difficulty_profile.player_to_hit_mod,
                    ..Default::default()
                },
                &mut self.rng,
            );
            (result, pdn, tdn, ac)
        };

        if weapon_is_ranged {
            if let Some(ref ammo_type) = weapon_ammo_type {
                events.push(self.consume_ammo_event(ammo_type));
            }
        }
        if let Some(msg) = ammo_switch_msg {
            events.push(self.narrate(msg));
        }
        events.push(self.narrate(&attack_result.narration));

        let damage_dealt = if attack_result.hit {
            attack_result.damage.as_ref().map(|d| d.total)
        } else {
            None
        };

        let mut shock_dealt = 0;
        if !attack_result.hit && weapon_is_melee {
            if let Some(player) = self.get_player_combatant() {
                if let Some(weapon) = self.lookup_weapon(player) {
                    shock_dealt = resolve_shock(
                        self.character.attr_mods.get("str").copied().unwrap_or(0),
                        weapon,
                        target_ac,
                    );
                }
            }
            if shock_dealt > 0 {
                self.apply_damage(&target_id, shock_dealt);
                let weapon_name = self
                    .get_player_combatant()
                    .and_then(|p| self.lookup_weapon(p))
                    .map(|w| w.name.clone())
                    .unwrap_or_default();
                events.push(self.narrate(format!(
                    "Shock damage: {shock_dealt} ({weapon_name} vs AC {target_ac})"
                )));
            }
        }

        if attack_result.hit {
            if let Some(ref dmg) = attack_result.damage {
                self.apply_damage(&target_id, dmg.total);
            }
        }

        let target_hp_after = self
            .state
            .combatants
            .iter()
            .find(|c| c.entity_id == target_id)
            .map(|c| (c.hp, c.max_hp));

        let weapon_id_str = final_weapon_id
            .as_deref()
            .unwrap_or("weapon.unarmed")
            .to_string();

        let notice = CombatAttackNotice {
            attacker: player_display_name.clone(),
            target: target_display_name.clone(),
            attacker_id: player_id.clone(),
            target_id: target_id.clone(),
            weapon: weapon_id_str,
            roll: attack_result.roll,
            modifier: attack_result.total - attack_result.roll,
            total: attack_result.total,
            target_ac,
            hit: attack_result.hit,
            damage: damage_dealt,
            shock: shock_dealt,
            critical: attack_result.natural_20,
            target_hp_remaining: target_hp_after.map(|(hp, _)| hp),
            target_max_hp: target_hp_after.map(|(_, max)| max),
        };
        events.push(payload_event(self.tick, "combat.attack", &notice));

        let target_alive = self
            .find_combatant(&target_id)
            .map(|c| c.alive)
            .unwrap_or(false);

        if !target_alive {
            events.push(self.narrate(format!("{target_display_name} is defeated!")));
            let defeated = CombatEnemyDefeatedNotice {
                entity_id: target_id.clone(),
                name: target_display_name.clone(),
            };
            events.push(payload_event(self.tick, "combat.enemy_defeated", &defeated));
            let base_id = match target_id.rfind('_') {
                Some(idx) => target_id[..idx].to_string(),
                None => target_id.clone(),
            };
            self.state.defeated_enemies.push(base_id);
        }

        // HR-787: drop the newly-defeated enemy's loot onto its grid tile and
        // re-emit positions so the encounter view shows the pile as it falls.
        if self.reconcile_ground_loot() {
            events.push(self.make_positions_event(self.tick));
        }

        let still_alive: bool = self
            .state
            .combatants
            .iter()
            .any(|c| !c.is_player && c.alive);

        if !still_alive {
            self.state.combat_over = true;
            events.extend(self.handle_victory());
            return events;
        }

        events.extend(self.run_enemy_turns(db));
        self.advance_round();
        events
    }

    fn handle_advance(&mut self) -> Vec<GameEvent> {
        let player_id = match self.get_player_combatant() {
            Some(p) => p.entity_id.clone(),
            None => return vec![self.narrate("[ERROR] Player combatant not found.")],
        };
        let already_melee = self
            .find_combatant(&player_id)
            .map(|c| c.range_band == "melee")
            .unwrap_or(false);
        if already_melee {
            return vec![self.narrate("You are already at melee range.")];
        }
        if let Some(p) = self.find_combatant_mut(&player_id) {
            p.range_band = "melee".to_string();
        }
        // Recompute grid positions to reflect the new range band, then emit.
        assign_positions(&mut self.state);
        let pos_ev = self.make_positions_event(self.tick);
        vec![self.narrate("You advance to melee range."), pos_ev]
    }

    fn handle_withdraw(&mut self) -> Vec<GameEvent> {
        let player_id = match self.get_player_combatant() {
            Some(p) => p.entity_id.clone(),
            None => return vec![self.narrate("[ERROR] Player combatant not found.")],
        };
        let already_near = self
            .find_combatant(&player_id)
            .map(|c| c.range_band == "near")
            .unwrap_or(false);
        if already_near {
            return vec![self.narrate("You are already at near range.")];
        }
        if let Some(p) = self.find_combatant_mut(&player_id) {
            p.range_band = "near".to_string();
        }
        // Recompute grid positions to reflect the new range band, then emit.
        assign_positions(&mut self.state);
        let pos_ev = self.make_positions_event(self.tick);
        vec![self.narrate("You withdraw to near range."), pos_ev]
    }

    fn handle_flee(&mut self) -> Vec<GameEvent> {
        let difficulties = self.get_alive_flee_difficulties();
        let flee_result = resolve_flee(
            &self.character,
            &difficulties,
            self.character.position_q,
            self.character.position_r,
            false,
            None,
            &mut self.rng,
        );

        let mut events = vec![self.narrate(&flee_result.narration)];

        self.state.fled = true;
        self.state.combat_over = true;

        let fled_notice = CombatFledNotice {
            clean: flee_result.clean,
            consequence: flee_result.consequence.clone(),
            destination_q: flee_result.destination_q,
            destination_r: flee_result.destination_r,
        };
        events.push(payload_event(self.tick, "combat.fled", &fled_notice));

        let req = CombatFleeRequested {
            character_id: self.character.id.clone(),
            character_data: self.char_snapshot(),
            damage_taken: flee_result.damage_taken,
            item_lost: flee_result.item_lost.clone(),
            destination_q: flee_result.destination_q,
            destination_r: flee_result.destination_r,
        };
        events.push(payload_event(self.tick, "combat.flee_requested", &req));
        events
    }

    fn handle_use(&mut self, cmd: &ParsedCommand, db: &WorldDatabase) -> Vec<GameEvent> {
        if cmd.args.is_empty() {
            return vec![self.narrate("Use what? Specify an item name.")];
        }
        let item_name = cmd.args.join(" ");
        let use_result = self
            .item_system
            .use_item(&mut self.character, &item_name, &mut self.rng);

        if !use_result.success {
            return vec![self.narrate(&use_result.narration)];
        }

        let hp_restored: i32 = if use_result.effect.contains("healed") {
            use_result
                .effect
                .split_whitespace()
                .nth(1)
                .and_then(|s| s.parse().ok())
                .unwrap_or(0)
        } else {
            0
        };

        let mut events = vec![self.narrate(&use_result.narration)];

        let req = CombatUseItemRequested {
            character_id: self.character.id.clone(),
            character_data: self.char_snapshot(),
            item_name: use_result.item_name.clone(),
            hp_restored,
            narration: use_result.narration.clone(),
        };
        events.push(payload_event(self.tick, "combat.use_item_requested", &req));

        if hp_restored > 0 {
            let player_id = self.get_player_combatant().map(|p| p.entity_id.clone());
            if let Some(pid) = player_id {
                let new_hp = self
                    .find_combatant(&pid)
                    .map(|p| (p.hp + hp_restored).min(p.max_hp))
                    .unwrap_or(0);
                self.update_hp(&pid, new_hp);
            }
        }

        events.extend(self.run_enemy_turns(db));
        self.advance_round();
        events
    }

    fn handle_status(&self) -> Vec<GameEvent> {
        let player = self.get_player_combatant();
        let hp = player.map(|p| p.hp).unwrap_or(self.character.hp);
        let mut lines = vec![
            format!("--- {} (Combat) ---", self.character.name),
            format!(
                "HP: {}/{}  AC: {}  Attack: {:+}",
                hp,
                self.character.max_hp,
                self.character.ac,
                self.character.attack_bonus,
            ),
            format!("Round: {}", self.state.round_number),
        ];
        for enemy in self.state.combatants.iter().filter(|c| !c.is_player) {
            if enemy.alive {
                lines.push(self.get_enemy_status_line(enemy));
            } else {
                lines.push(format!("  {}: [DEFEATED]", enemy.display_name));
            }
        }
        vec![self.narrate(lines.join("\n"))]
    }

    fn handle_help(&self) -> Vec<GameEvent> {
        vec![self.narrate(
            "Combat commands:\n\
              attack [target]   Attack an enemy (defaults to first enemy)\n\
              flee              Attempt to escape combat\n\
              use <item>        Use an item from your inventory\n\
              status            Show your current HP and enemy status\n\
              help              Show this message",
        )]
    }

    fn handle_harvest(&mut self, cmd: &ParsedCommand) -> Vec<GameEvent> {
        if self.state.pending_harvest.is_empty() {
            return vec![self.narrate("There's nothing to harvest.")];
        }

        let target_name = cmd.args.join(" ").to_lowercase();
        if target_name.is_empty() {
            let materials: Vec<String> = self
                .state
                .pending_harvest
                .iter()
                .map(|h| h.material.clone())
                .collect();
            return vec![self.narrate(format!(
                "Available to harvest: {}. Use 'harvest <material>'.",
                materials.join(", ")
            ))];
        }

        let found_idx = self
            .state
            .pending_harvest
            .iter()
            .position(|h| h.material.to_lowercase().contains(&target_name));

        let found_idx = match found_idx {
            Some(i) => i,
            None => {
                return vec![self.narrate(format!(
                    "No harvestable material matching '{target_name}'."
                ))]
            }
        };

        let harvest = self.state.pending_harvest[found_idx].clone();
        let material = harvest.material.clone();
        let difficulty = harvest.difficulty;
        let skill = harvest.skill.clone().unwrap_or_else(|| "survive".to_string());
        let roll = self.rng.gen_range(1..=6) + self.rng.gen_range(1..=6);
        let skill_level = self.character.skills.get(&skill).copied().unwrap_or(-1);
        let wis_mod = self.character.attr_mods.get("wis").copied().unwrap_or(0);
        let total = roll + skill_level + wis_mod;
        let success = total >= difficulty;

        let narration = if success {
            format!(
                "You carefully harvest the {material}. \
                ({} check: {}+{}={} vs {} — success!)",
                capitalize(&skill),
                roll,
                skill_level + wis_mod,
                total,
                difficulty
            )
        } else {
            format!(
                "You attempt to harvest the {material} but botch the job — the \
                material is ruined. ({} check: {}+{}={} vs {} — failed)",
                capitalize(&skill),
                roll,
                skill_level + wis_mod,
                total,
                difficulty
            )
        };

        let mut events = vec![self.narrate(&narration)];

        let req = CombatHarvestRequested {
            character_id: self.character.id.clone(),
            character_data: self.char_snapshot(),
            material: material.clone(),
            success,
            narration,
        };
        events.push(payload_event(self.tick, "combat.harvest_requested", &req));
        self.state.pending_harvest.remove(found_idx);
        events
    }

    // -----------------------------------------------------------------------
    // Enemy turn processing
    // -----------------------------------------------------------------------

    fn run_enemy_turns(&mut self, db: &WorldDatabase) -> Vec<GameEvent> {
        let mut events: Vec<GameEvent> = Vec::new();

        let order: Vec<String> = self.state.initiative_order.clone();
        let player_id = match self.get_player_combatant() {
            Some(p) => p.entity_id.clone(),
            None => return events,
        };

        for entity_id in &order {
            let combatant_opt = self.find_combatant(entity_id).cloned();
            let combatant = match combatant_opt {
                Some(c) if !c.is_player && c.alive => c,
                _ => continue,
            };

            if self.state.player_surprise && self.state.round_number == 1 {
                events.push(self.narrate(format!(
                    "{} is caught off guard and cannot act!",
                    combatant.display_name
                )));
                continue;
            }

            events.push(self.narrate(format!("--- {}'s turn ---", combatant.display_name)));

            // An enemy with any authored action (present in the world's IR content)
            // spends its turn on actions instead of legacy weapon attacks. Action
            // economy: up to `num_attacks` main-action activations, each one
            // selecting the first affordable action and spending its costs.
            let has_actions = self.runtime_content.as_ref().is_some_and(|store| {
                combatant.actions.iter().any(|a| store.get_action(a).is_some())
            });
            if has_actions {
                let budget = combatant.num_attacks.max(1);
                for _ in 0..budget {
                    let player_alive = self
                        .find_combatant(&player_id)
                        .map(|p| p.alive)
                        .unwrap_or(false);
                    if !player_alive || self.state.combat_over {
                        break;
                    }
                    // Re-read the actor each slot: costs spent on prior actions
                    // deplete its pools and gate what it can still afford.
                    let actor = match self.find_combatant(entity_id).cloned() {
                        Some(c) if c.alive => c,
                        _ => break,
                    };
                    let available = resource_map(&actor);
                    let Some(action_id) = self.select_action(&actor.actions, &available) else {
                        break;
                    };
                    let action_events =
                        self.perform_enemy_action(db, &actor, &action_id, &player_id);
                    events.extend(action_events);
                    if self.state.pending_veteran_luck.is_some() {
                        // Veteran's Luck offered — pause for the player's yes/no.
                        return events;
                    }
                }
                continue;
            }

            let num_attacks = combatant.num_attacks;
            for _ in 0..num_attacks {
                let player_alive = self
                    .find_combatant(&player_id)
                    .map(|p| p.alive)
                    .unwrap_or(false);
                if !player_alive || self.state.combat_over {
                    break;
                }

                let action = choose_action(&combatant, &self.state);
                let action_type = action
                    .as_ref()
                    .map(|a| a.action_type.as_str())
                    .unwrap_or("wait");
                if action_type != "attack" {
                    continue;
                }

                let player_snapshot = match self.find_combatant(&player_id).cloned() {
                    Some(p) => p,
                    None => break,
                };

                let attack_result = resolve_attack(
                    &combatant,
                    &player_snapshot,
                    AttackParams {
                        attack_modifier: self.difficulty_profile.enemy_to_hit_mod,
                        ..Default::default()
                    },
                    &mut self.rng,
                );
                events.push(self.narrate(&attack_result.narration));

                if attack_result.hit {
                    if let Some(ref dmg) = attack_result.damage {
                        let damage = dmg.total;

                        // Route through the unified player-damage handler (the
                        // same path enemy IR actions use).
                        let (dmg_events, paused) =
                            self.apply_player_damage(&combatant.display_name, damage);
                        events.extend(dmg_events);
                        if paused {
                            // Veteran's Luck was offered — combat pauses for the yes/no.
                            return events;
                        }
                        let player_down = self
                            .find_combatant(&player_id)
                            .map(|p| p.hp <= 0)
                            .unwrap_or(true);
                        if player_down {
                            break;
                        }
                    }
                }
            }
        }

        self.advance_round();
        events
    }

    /// Resolve and apply one authored enemy `action` through the IR pipeline.
    ///
    /// The target is chosen from the action's `targeting.shape` (`self` → the
    /// actor, anything else → the player, the sole hostile target in single-player
    /// combat) and bound as `target` in the eval context. The action's activation
    /// `costs` are spent from the actor's resources.
    ///
    /// Non-player effects (status, logs, damage to other entities) are applied
    /// via [`TriggerRuntime::apply_intents`]; HP damage dealt to the player is
    /// split out and routed through [`apply_player_damage`](Self::apply_player_damage),
    /// the *same* handler the legacy weapon attack uses — so action damage offers
    /// Veteran's Luck, emits `combat.take_damage_requested`, fires the
    /// `combat.player_hit` notice, and triggers Last Stand uniformly.
    fn perform_enemy_action(
        &mut self,
        db: &WorldDatabase,
        combatant: &Combatant,
        action_id: &str,
        player_id: &str,
    ) -> Vec<GameEvent> {
        let mut events: Vec<GameEvent> = Vec::new();

        // Resolve the action into intents, then partition the player's HP damage
        // out of the intent list. The store is taken out so `apply_intents(self)`
        // can borrow `&mut self` without aliasing the field; restored after.
        let store = self.runtime_content.take();
        let mut player_damage = 0i32;
        let mut costs: Vec<Cost> = Vec::new();
        if let Some(ref store) = store {
            if let Some(action) = store.get_action(action_id).cloned() {
                // Target: `self` shape → the actor; everything else → the player.
                let target_id = match action.targeting.as_ref().map(|t| t.shape) {
                    Some(TargetShape::SelfTarget) => combatant.entity_id.clone(),
                    _ => player_id.to_string(),
                };
                let target = self
                    .find_combatant(&target_id)
                    .cloned()
                    .unwrap_or_else(|| combatant.clone());
                if let Some(activation) = &action.activation {
                    costs = activation.costs.clone();
                }
                let tick = self.tick;
                let status = StatusEffectService::new(
                    StatusEffectRepository::new(db),
                    store,
                    Some(CombatClock(tick as i32)),
                );
                let runtime = TriggerRuntime::new(store, &status, db, tick, IR_TRIGGER_CASCADE_DEPTH);
                let builder = EvalContextBuilder::new(store, &status);
                let self_snap = EntitySnapshot::from_combatant(combatant);
                let target_snap = EntitySnapshot::from_combatant(&target);
                match builder.build(&self_snap, Some(&target_snap), "action.resolve", &Map::new()) {
                    Ok(ctx) => {
                        match resolve_action(
                            &action,
                            combatant.attack_bonus as i64
                                + self.difficulty_profile.enemy_to_hit_mod as i64,
                            &target,
                            &ctx,
                            &mut self.rng,
                        ) {
                            Ok(out) => {
                                let (dmg, rest) = self.split_player_damage(out.intents, player_id);
                                player_damage = dmg;
                                let applied = runtime.apply_intents(self, &rest);
                                events.push(self.narrate(format!(
                                    "{} performs {} ({}).",
                                    combatant.display_name, action.id, out.outcome_key
                                )));
                                for ev in &applied.emitted {
                                    if let Some(msg) =
                                        ev.data.get("message").and_then(JsonValue::as_str)
                                    {
                                        events.push(self.narrate(msg.to_string()));
                                    }
                                }
                                for e in &applied.errors {
                                    eprintln!("enemy action {}: {e}", action.id);
                                }
                            }
                            Err(e) => eprintln!("enemy action {}: {e}", action.id),
                        }
                    }
                    Err(e) => eprintln!("enemy action context: {e}"),
                }
            }
        }
        self.runtime_content = store;

        // Spend the action's costs from the actor's resources (so ammo/charges
        // deplete and gate later activations this turn).
        for cost in &costs {
            self.apply_resource_delta(&combatant.entity_id, &cost.resource, -cost.amount);
        }

        // Route the player's HP damage through the unified handler (Veteran's
        // Luck / persistence / player_hit / last stand). `paused` (Veteran's Luck
        // pending) is observed by the caller via `pending_veteran_luck`.
        let (dmg_events, _paused) =
            self.apply_player_damage(&combatant.display_name, player_damage);
        events.extend(dmg_events);
        events
    }

    /// Select the next action an actor will use: the first of its `action_ids`
    /// that exists in the IR content and whose activation `costs` are affordable
    /// given `available` resources. Returns `None` when nothing is usable.
    fn select_action(
        &self,
        action_ids: &[String],
        available: &std::collections::BTreeMap<String, i64>,
    ) -> Option<String> {
        let store = self.runtime_content.as_ref()?;
        for id in action_ids {
            let Some(action) = store.get_action(id) else {
                continue;
            };
            let costs = action
                .activation
                .as_ref()
                .map(|a| a.costs.as_slice())
                .unwrap_or(&[]);
            if can_afford(costs, available) {
                return Some(id.clone());
            }
        }
        None
    }

    /// Partition `intents`, summing HP damage that an enemy action would deal to
    /// the player (`emit_damage` packets resolved through the damage pipeline
    /// against the player's live pools/mitigations) and returning that total plus
    /// the remaining intents to apply normally. The player's HP damage is handled
    /// by [`apply_player_damage`](Self::apply_player_damage) instead, so it shares
    /// the legacy attack's consequence path.
    fn split_player_damage(
        &self,
        intents: Vec<Intent>,
        player_id: &str,
    ) -> (i32, Vec<Intent>) {
        let snap = self.snapshot(player_id);
        let mut total = 0i64;
        let mut rest = Vec::with_capacity(intents.len());
        for intent in intents {
            match &intent {
                Intent::EmitDamage { entity_id, packet } if entity_id == player_id => {
                    if let Some(ref s) = snap {
                        let application = apply_damage(
                            packet,
                            &std::collections::BTreeMap::new(),
                            &s.mitigations,
                            &s.pools,
                        );
                        // Damage is negative pool deltas; sum their magnitude.
                        total += application.deltas.iter().map(|d| -d.delta).sum::<i64>();
                    }
                }
                _ => rest.push(intent),
            }
        }
        (total.max(0) as i32, rest)
    }

    /// The single entry point for HP damage dealt **to the player** in combat,
    /// used by both the legacy weapon attack and enemy IR actions.
    ///
    /// Offers Veteran's Luck (warrior, once per combat) — when offered, the damage
    /// is recorded as pending and **not** yet applied, and the returned `paused`
    /// flag is `true` (the caller stops the turn and waits for the yes/no). When
    /// not paused it emits `combat.take_damage_requested`, applies the damage,
    /// syncs `character.hp`, emits the `combat.player_hit` notice, and triggers
    /// Last Stand. A non-positive `damage` is a no-op.
    fn apply_player_damage(&mut self, attacker_name: &str, damage: i32) -> (Vec<GameEvent>, bool) {
        let mut events: Vec<GameEvent> = Vec::new();
        if damage <= 0 {
            return (events, false);
        }
        let player_id = match self.get_player_combatant().map(|p| p.entity_id.clone()) {
            Some(p) => p,
            None => return (events, false),
        };
        let player_max_hp = self.find_combatant(&player_id).map(|p| p.max_hp).unwrap_or(0);

        let is_warrior = self.character.character_class == "warrior";
        let veteran_luck_available = is_warrior
            && !self.state.veteran_luck_used
            && !self
                .character
                .class_abilities
                .get("veteran_luck_used")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
        if veteran_luck_available {
            self.state.pending_veteran_luck = Some(PendingVeteranLuckRecord {
                attacker_name: attacker_name.to_string(),
                damage,
            });
            events.push(self.narrate(format!(
                "{attacker_name} hits you for {damage} damage! Use Veteran's Luck to negate? (yes/no)"
            )));
            return (events, true);
        }

        let req = CombatTakeDamageRequested {
            character_id: self.character.id.clone(),
            character_data: self.char_snapshot(),
            damage,
            source: "combat".to_string(),
        };
        events.push(payload_event(self.tick, "combat.take_damage_requested", &req));

        self.apply_damage(&player_id, damage);
        let player_hp = self.find_combatant(&player_id).map(|p| p.hp).unwrap_or(0);
        self.character.hp = player_hp;

        let hit_notice = CombatPlayerHitNotice {
            attacker: attacker_name.to_string(),
            damage,
            player_hp,
            player_max_hp,
            player_id: Some(player_id.clone()),
            player_name: Some(self.character.name.clone()),
            player_alive: player_hp > 0,
        };
        events.push(payload_event(self.tick, "combat.player_hit", &hit_notice));

        if player_hp <= 0 && !self.state.last_stand_active {
            self.state.last_stand_active = true;
            events.push(self.narrate("You are gravely wounded! Make your Last Stand!"));
        }
        (events, false)
    }

    /// The `CreatureData` for an enemy combatant, resolved from its
    /// `"{creature.id}_{roster_index}"` entity id (HR-787).
    fn creature_for_combatant(&self, entity_id: &str) -> Option<&CreatureData> {
        let idx: usize = entity_id.rsplit('_').next()?.parse().ok()?;
        self.creatures.get(idx)
    }

    /// Roll and record ground loot for any newly-defeated enemy that has not yet
    /// dropped (HR-787). Each dead enemy drops exactly once — an entry is
    /// recorded even when the roll yields nothing, so it is never re-rolled.
    /// Loot rolls reuse the same per-enemy `generate_combat_loot` path (and the
    /// HR-107 difficulty multipliers) as the victory flow, so the economy is
    /// unchanged; only the timing moves to the moment of death. Returns whether
    /// any *visible* loot (items or currency) newly appeared, so callers can
    /// decide to re-emit `combat.positions`.
    fn reconcile_ground_loot(&mut self) -> bool {
        let pending: Vec<(String, i32, i32)> = self
            .state
            .combatants
            .iter()
            .filter(|c| !c.is_player && !c.alive)
            .filter(|c| {
                !self
                    .state
                    .ground_loot
                    .iter()
                    .any(|g| g.entity_id == c.entity_id)
            })
            .map(|c| (c.entity_id.clone(), c.q, c.r))
            .collect();
        if pending.is_empty() {
            return false;
        }
        let mut loot_gen = LootGenerator::with_difficulty(
            None,
            self.difficulty_profile.loot_amount_mult,
            self.difficulty_profile.loot_probability_mult,
        );
        let mut visible = false;
        for (entity_id, q, r) in pending {
            let (items, currency) = match self.creature_for_combatant(&entity_id) {
                Some(creature) => {
                    let creature = creature.clone();
                    let result =
                        loot_gen.generate_combat_loot(std::slice::from_ref(&creature), &mut self.rng);
                    self.state
                        .pending_harvest
                        .extend(result.harvestable.iter().cloned());
                    let items: Vec<JsonObject> = result
                        .items
                        .iter()
                        .map(|item| {
                            let mut obj = JsonObject::new();
                            obj.insert("name".into(), JsonValue::String(item.name.clone()));
                            obj.insert("type".into(), JsonValue::String(item.item_type.clone()));
                            obj.insert("value".into(), JsonValue::Number(item.value.into()));
                            obj
                        })
                        .collect();
                    (items, result.currency)
                }
                None => (Vec::new(), 0),
            };
            if !items.is_empty() || currency > 0 {
                visible = true;
            }
            self.state.ground_loot.push(GroundLoot {
                entity_id,
                q,
                r,
                items,
                currency,
            });
        }
        visible
    }

    fn handle_victory(&mut self) -> Vec<GameEvent> {
        let mut events = vec![self.narrate("All enemies defeated! Victory!")];

        let mut xp_gained = 0i32;
        // HR-786: combat item drops are left on a searchable corpse (see the
        // victory handler); only gold auto-collects via `currency_gained`.
        let mut corpse_items: Vec<JsonObject> = Vec::new();
        let mut currency_gained = 0i32;

        if !self.creatures.is_empty() {
            let defeated_enemies = self.state.defeated_enemies.clone();
            let defeated_creatures: Vec<&CreatureData> = self
                .creatures
                .iter()
                .filter(|cr| {
                    defeated_enemies.contains(&cr.id)
                        || defeated_enemies
                            .iter()
                            .any(|eid| eid.starts_with(&cr.id))
                })
                .collect();

            let src: &[&CreatureData] = if defeated_creatures.is_empty() {
                &[]
            } else {
                &defeated_creatures
            };

            let base_xp: i32 = src.iter().map(|cr| cr.xp_value).sum();
            xp_gained = ((base_xp as f64 * self.difficulty_profile.xp_mult).round() as i32)
                .max(0);
            if xp_gained > 0 {
                events.push(self.narrate(format!("You gain {xp_gained} XP.")));
            }
        }

        if !self.creatures.is_empty() {
            // HR-787: loot was rolled per enemy as it fell (reconcile_ground_loot)
            // and shown on the grid. Sweep every dropped pile into the player's
            // inventory. A final reconcile catches the killing blow (which may
            // reach victory before its own reconcile emit). Harvestables were
            // accumulated into pending_harvest during the same rolls.
            self.reconcile_ground_loot();
            for pile in &self.state.ground_loot {
                corpse_items.extend(pile.items.iter().cloned());
                currency_gained += pile.currency;
            }
            if currency_gained > 0 {
                events.push(self.narrate("You pocket the coin from the fallen."));
            }
            if !corpse_items.is_empty() {
                events.push(
                    self.narrate("The fallen leave something worth searching. (search)"),
                );
            }
        }

        let req = CombatVictoryRequested {
            character_id: self.character.id.clone(),
            character_data: self.char_snapshot(),
            xp_gained,
            items_gained: Vec::new(),
            currency_gained,
            corpse_items,
            harvestable: self
                .state
                .pending_harvest
                .iter()
                .map(|h| {
                    serde_json::from_value(serde_json::to_value(h).unwrap_or_default())
                        .unwrap_or_default()
                })
                .collect(),
        };
        events.push(payload_event(self.tick, "combat.victory_requested", &req));
        events
    }

    // -----------------------------------------------------------------------
    // Last stand + Veteran's Luck
    // -----------------------------------------------------------------------

    fn handle_last_stand_attack(&mut self, _cmd: &ParsedCommand) -> Vec<GameEvent> {
        let alive_enemy_ids: Vec<String> = self
            .state
            .combatants
            .iter()
            .filter(|c| !c.is_player && c.alive)
            .map(|c| c.entity_id.clone())
            .collect();

        if alive_enemy_ids.is_empty() {
            let player_id = self.get_player_combatant().map(|p| p.entity_id.clone());
            if let Some(pid) = player_id {
                self.update_hp(&pid, 1);
            }
            self.character.hp = 1;
            self.state.last_stand_active = false;
            self.state.combat_over = true;
            let mut events = vec![self.narrate(
                "By some miracle, the last enemy falls as you collapse. \
                 You survive at 1 HP, barely clinging to life.",
            )];
            events.push(self.update_character_event());
            return events;
        }

        let target_id = alive_enemy_ids[0].clone();
        let player_id = match self.get_player_combatant() {
            Some(p) => p.entity_id.clone(),
            None => return vec![self.narrate("[ERROR] Player combatant not found.")],
        };

        let (attack_result, target_display_name) = {
            let player = self
                .state
                .combatants
                .iter()
                .find(|c| c.entity_id == player_id)
                .unwrap()
                .clone();
            let target = self
                .state
                .combatants
                .iter()
                .find(|c| c.entity_id == target_id)
                .unwrap()
                .clone();
            let tdn = target.display_name.clone();
            let is_warrior = self.character.character_class == "warrior";
            let result = resolve_attack(
                &player,
                &target,
                AttackParams {
                    is_warrior,
                    warrior_level: self.character.level,
                    skill_level: self.get_attack_skill_level() - 2,
                    attr_mod: self.character.attr_mods.get("str").copied().unwrap_or(0),
                    attack_modifier: self.difficulty_profile.player_to_hit_mod,
                    ..Default::default()
                },
                &mut self.rng,
            );
            (result, tdn)
        };

        let mut events = vec![self.narrate(format!("[LAST STAND] {}", attack_result.narration))];
        let mut survived = false;

        if attack_result.hit {
            if let Some(ref dmg) = attack_result.damage {
                self.apply_damage(&target_id, dmg.total);
                let target_alive = self
                    .find_combatant(&target_id)
                    .map(|c| c.alive)
                    .unwrap_or(false);

                if !target_alive {
                    let remaining_alive = self
                        .state
                        .combatants
                        .iter()
                        .any(|c| !c.is_player && c.alive);

                    if !remaining_alive {
                        self.update_hp(&player_id, 1);
                        self.character.hp = 1;
                        self.state.last_stand_active = false;
                        self.state.combat_over = true;
                        survived = true;
                        events.push(self.narrate(format!(
                            "{target_display_name} falls! Against all odds, you survive at 1 HP!"
                        )));
                        events.push(self.update_character_event());
                    } else {
                        events.push(
                            self.narrate(format!("{target_display_name} is defeated!"))
                        );
                    }
                }
            }
        }

        if !survived && self.state.last_stand_active {
            self.state.last_stand_active = false;
            self.state.combat_over = true;
            self.pending_transition = Some(SceneState::Respawn);
            events.push(self.narrate(
                "Your last strike fails to end the fight. You collapse, \
                 darkness closing in...",
            ));
            let death_notice = CharacterDeathNotice {
                position_q: self.character.position_q,
                position_r: self.character.position_r,
            };
            events.push(payload_event(self.tick, "character.death", &death_notice));
        }

        events
    }

    fn handle_last_stand_use(&mut self, cmd: &ParsedCommand) -> Vec<GameEvent> {
        if cmd.args.is_empty() {
            return vec![self.narrate("[LAST STAND] Use what? Specify an item name.")];
        }
        let item_name = cmd.args.join(" ");
        let result = self
            .item_system
            .use_item(&mut self.character, &item_name, &mut self.rng);

        let mut events = vec![self.narrate(format!("[LAST STAND] {}", result.narration))];

        if result.success && self.character.hp > 0 {
            let new_hp = self.character.hp;
            let player_id = self.get_player_combatant().map(|p| p.entity_id.clone());
            if let Some(pid) = player_id {
                self.update_hp(&pid, new_hp);
            }
            self.state.last_stand_active = false;
            events.push(self.narrate(
                "You stabilize! The healing is just enough to keep you fighting.",
            ));
            events.push(self.update_character_event());
        } else if self.character.hp <= 0 {
            self.state.last_stand_active = false;
            self.state.combat_over = true;
            self.pending_transition = Some(SceneState::Respawn);
            events.push(self.narrate(
                "The healing wasn't enough. You collapse into darkness...",
            ));
            let death_notice = CharacterDeathNotice {
                position_q: self.character.position_q,
                position_r: self.character.position_r,
            };
            events.push(payload_event(self.tick, "character.death", &death_notice));
        }

        events
    }

    fn handle_last_stand_flee(&mut self) -> Vec<GameEvent> {
        let difficulties = self.get_alive_flee_difficulties();
        let flee_result = resolve_flee(
            &self.character,
            &difficulties,
            self.character.position_q,
            self.character.position_r,
            true,
            None,
            &mut self.rng,
        );

        self.state.last_stand_active = false;
        self.state.fled = true;
        self.state.combat_over = true;

        let player_id = self.get_player_combatant().map(|p| p.entity_id.clone());
        if let Some(pid) = player_id {
            self.update_hp(&pid, 1);
        }
        self.character.hp = 1;

        vec![
            self.narrate(format!(
                "[LAST STAND] With your last ounce of strength, you flee! {}",
                flee_result.narration
            )),
            self.update_character_event(),
        ]
    }

    fn handle_veteran_luck_yes(&mut self) -> Vec<GameEvent> {
        self.state.pending_veteran_luck = None;
        self.state.veteran_luck_used = true;
        self.character
            .class_abilities
            .insert("veteran_luck_used".into(), JsonValue::Bool(true));
        vec![
            self.narrate(
                "Veteran's Luck activates! The hit that should have landed somehow misses.",
            ),
            self.update_character_event(),
        ]
    }

    fn handle_veteran_luck_no(&mut self) -> Vec<GameEvent> {
        let damage = self
            .state
            .pending_veteran_luck
            .as_ref()
            .map(|r| r.damage)
            .unwrap_or(0);
        self.state.pending_veteran_luck = None;

        let player_id = self.get_player_combatant().map(|p| p.entity_id.clone());
        if let Some(pid) = &player_id {
            self.apply_damage(pid, damage);
            self.character.hp = self.find_combatant(pid).map(|p| p.hp).unwrap_or(0);
        }

        let mut events = vec![self.narrate(format!(
            "You take the hit for {damage} damage. HP: {}/{}.",
            self.character.hp, self.character.max_hp
        ))];

        if self.character.hp <= 0 && !self.state.last_stand_active {
            self.state.last_stand_active = true;
            events.push(self.narrate(
                "You are gravely wounded! Make your Last Stand!",
            ));
        }

        events.push(self.update_character_event());
        events
    }
}

// ---------------------------------------------------------------------------
// SceneHandler impl
// ---------------------------------------------------------------------------

impl SceneHandler for CombatScene {
    fn get_valid_commands(&self) -> Vec<String> {
        if self.state.last_stand_active {
            return vec![
                "attack".into(),
                "use".into(),
                "flee".into(),
                "status".into(),
                "help".into(),
            ];
        }
        if self.state.pending_veteran_luck.is_some() {
            return vec!["yes".into(), "no".into()];
        }
        if self.state.combat_over {
            let mut cmds = vec!["status".into(), "help".into(), "look".into(), "leave".into()];
            if !self.state.pending_harvest.is_empty() {
                cmds.insert(0, "harvest".into());
            }
            return cmds;
        }
        vec![
            "attack".into(),
            "flee".into(),
            "use".into(),
            "advance".into(),
            "withdraw".into(),
            "status".into(),
            "help".into(),
        ]
    }

    fn get_prompt(&self, _db: &WorldDatabase) -> String {
        let mut lines: Vec<String> = Vec::new();

        if self.state.last_stand_active {
            lines.push("[LAST STAND] You are gravely wounded. One final action:".into());
            lines.push("  attack — One last strike (-2 penalty)".into());
            lines.push("  use <item> — Use a healing item".into());
            lines.push("  flee — Desperate escape (always succeeds)".into());
            return lines.join("\n");
        }

        if let Some(ref hit_data) = self.state.pending_veteran_luck {
            lines.push(format!(
                "Veteran's Luck — {} just hit you for {} damage. Use Veteran's Luck to negate? (yes/no)",
                hit_data.attacker_name, hit_data.damage
            ));
            return lines.join("\n");
        }

        lines.push(format!("--- Combat: Round {} ---", self.state.round_number));
        for enemy in self.state.combatants.iter().filter(|c| !c.is_player) {
            if enemy.alive {
                lines.push(self.get_enemy_status_line(enemy));
            } else {
                lines.push(format!("  {}: [DEFEATED]", enemy.display_name));
            }
        }
        if let Some(player) = self.get_player_combatant() {
            lines.push(format!("You: {}/{} HP", player.hp, player.max_hp));
        }
        lines.push("Actions: attack | flee | use <item> | status | help".into());
        lines.join("\n")
    }

    fn handle_command(
        &mut self,
        cmd: &ParsedCommand,
        db: &WorldDatabase,
    ) -> Result<Vec<GameEvent>, String> {
        let mut events = if self.enemy_surprise_pending {
            self.enemy_surprise_pending = false;
            let mut surprise_events = vec![self.narrate(
                "The enemy caught you off guard! They strike before you can react!",
            )];
            surprise_events.extend(self.run_enemy_turns(db));
            self.advance_round();
            if !self.state.last_stand_active && !self.state.combat_over {
                let extra = self.dispatch_command(cmd, db)?;
                surprise_events.extend(extra);
            }
            surprise_events
        } else {
            self.dispatch_command(cmd, db)?
        };
        // Append the current action bar state so the frontend refreshes buttons
        // after every command (including mid-turn state changes such as
        // last-stand activation or combat ending).
        events.push(self.actions_event(self.tick));
        Ok(events)
    }

    fn check_transitions(&self, _events: &[GameEvent]) -> Option<SceneState> {
        if let Some(ref state) = self.pending_transition {
            return Some(*state);
        }

        if self.state.combat_over || self.state.fled {
            let player = self.get_player_combatant();
            if let Some(p) = player {
                if p.hp <= 0 && !self.state.fled {
                    return Some(SceneState::Respawn);
                }
            }
            if !self.state.pending_harvest.is_empty() {
                return None;
            }
            return Some(SceneState::Exploration);
        }

        let enemies_alive = self.state.combatants.iter().any(|c| !c.is_player && c.alive);
        if !enemies_alive {
            if !self.state.pending_harvest.is_empty() {
                return None;
            }
            return Some(SceneState::Exploration);
        }

        None
    }

    fn get_suggestions(&self) -> Vec<String> {
        if self.state.pending_veteran_luck.is_some() {
            return vec!["yes".into(), "no".into()];
        }
        if self.state.last_stand_active {
            return vec!["attack".into(), "use <item>".into(), "flee".into()];
        }
        if self.state.combat_over {
            if !self.state.pending_harvest.is_empty() {
                let mut cmds: Vec<String> = self
                    .state
                    .pending_harvest
                    .iter()
                    .map(|h| format!("harvest {}", h.material))
                    .collect();
                cmds.extend_from_slice(&["leave".into(), "status".into()]);
                return cmds;
            }
            return vec!["leave".into(), "look".into(), "status".into()];
        }

        let alive_enemies: Vec<&Combatant> = self
            .state
            .combatants
            .iter()
            .filter(|c| !c.is_player && c.alive)
            .collect();

        if alive_enemies.is_empty() {
            return vec!["look".into(), "status".into()];
        }
        if alive_enemies.len() == 1 {
            return vec![
                format!("attack {}", alive_enemies[0].display_name.to_lowercase()),
                "flee".into(),
                "use <item>".into(),
                "status".into(),
            ];
        }
        let targets: Vec<String> = alive_enemies[..3.min(alive_enemies.len())]
            .iter()
            .map(|c| c.display_name.to_lowercase())
            .collect();
        vec![
            format!("attack [{}]", targets.join(", ")),
            "flee".into(),
            "use <item>".into(),
            "status".into(),
        ]
    }
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

fn character_snapshot(c: &Character) -> CharacterSnapshot {
    CharacterSnapshot {
        id: c.id.clone(),
        name: c.name.clone(),
        character_class: c.character_class.clone(),
        level: c.level,
        xp: c.xp,
        xp_next: c.xp_next,
        attributes: c.attributes.clone(),
        attr_mods: c.attr_mods.clone(),
        skills: c.skills.clone(),
        hp: c.hp,
        max_hp: c.max_hp,
        ac: c.ac,
        attack_bonus: c.attack_bonus,
        physical_save: c.physical_save,
        evasion_save: c.evasion_save,
        mental_save: c.mental_save,
        equipment: c.equipment.clone(),
        class_abilities: c.class_abilities.clone(),
        position_q: c.position_q,
        position_r: c.position_r,
    }
}

/// Lets the IR [`TriggerRuntime`](crate::runtime_content::TriggerRuntime) read and
/// mutate combatant state when triggers fire during live combat.
impl EntityResolver for CombatScene {
    fn snapshot(&self, entity_id: &str) -> Option<EntitySnapshot> {
        self.find_combatant(entity_id).map(EntitySnapshot::from_combatant)
    }

    fn apply_resource_delta(&mut self, entity_id: &str, resource: &str, delta: i64) {
        if resource == "hp" {
            let current = self.find_combatant(entity_id).map(|c| c.hp).unwrap_or(0);
            let new_hp = (current as i64 + delta).clamp(0, i32::MAX as i64) as i32;
            self.update_hp(entity_id, new_hp);
            return;
        }
        // Extra typed pool (shield/MDC): the damage pipeline already clamped the
        // delta to the pool's config, so apply it directly.
        if let Some(c) = self.find_combatant_mut(entity_id) {
            if let Some(pool) = c.pools.iter_mut().find(|p| p.id == resource) {
                pool.current += delta;
            }
        }
    }
}

fn payload_event<T: serde::Serialize>(tick: i64, event_type: &str, payload: &T) -> GameEvent {
    let data: JsonObject =
        serde_json::from_value(serde_json::to_value(payload).unwrap_or_default())
            .unwrap_or_default();
    GameEvent::new(tick, event_type, data).with_source("combat")
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combat_runtime::CombatState;

    fn make_combatant(id: &str, is_player: bool, hp: i32) -> Combatant {
        Combatant {
            entity_id: id.to_string(),
            name: id.to_string(),
            display_name: id.to_string(),
            is_player,
            initiative: 10,
            hp,
            max_hp: hp,
            ac: 12,
            attack_bonus: 0,
            damage_expr: "1d6".to_string(),
            attack_description: "attacks".to_string(),
            behavior: "melee".to_string(),
            num_attacks: 1,
            alive: hp > 0,
            weapon_id: None,
            range_band: "melee".to_string(),
            traits: vec![],
            pools: vec![],
            defenses: Default::default(),
            actions: vec![],
            inventory: vec![],
            gold: 0,
            q: 0,
            r: 0,
        }
    }

    fn make_state(player_hp: i32, enemy_hp: i32) -> CombatState {
        let player = make_combatant("player_1", true, player_hp);
        let enemy = make_combatant("goblin_1", false, enemy_hp);
        CombatState {
            combatants: vec![player.clone(), enemy.clone()],
            initiative_order: vec!["player_1".to_string(), "goblin_1".to_string()],
            current_turn_index: 0,
            round_number: 1,
            terrain: String::new(),
            features: vec![],
            player_surprise: false,
            enemy_surprise: false,
            veteran_luck_used: false,
            first_aid_used: false,
            enemy_detail_revealed: false,
            fled: false,
            combat_over: false,
            last_stand_active: false,
            pending_veteran_luck: None,
            pending_harvest: vec![],
            defeated_enemies: vec![],
            ground_loot: vec![],
        }
    }

    #[test]
    fn player_damage_syncs_character_hp_for_persistence() {
        // Regression (rest-after-flee bug): every player-HP change in combat funnels
        // through `update_hp`. The IR-action path (`apply_resource_delta`) and combat
        // heals previously updated only the combatant, leaving `self.character.hp`
        // stale. Because the flee/victory snapshots are built from `self.character`,
        // combat then persisted the player's PRE-combat HP, so post-combat `rest` saw
        // an uninjured character and healed nothing.
        use crate::runtime_content::EntityResolver;
        let mut character = make_character();
        character.hp = 10;
        character.max_hp = 10;
        let mut scene = CombatScene::new(character, make_state(10, 10), vec![], None, None, DifficultyProfile::default());

        // An IR action deals 4 damage to the player's HP pool (the path HR-767 unified).
        scene.apply_resource_delta("player_1", "hp", -4);

        assert_eq!(scene.get_player_combatant().unwrap().hp, 6, "combatant HP updated");
        assert_eq!(
            scene.character.hp, 6,
            "self.character.hp must sync so flee/victory snapshots persist the injured HP"
        );
        assert_eq!(
            scene.char_snapshot().hp, 6,
            "the snapshot combat persists must carry the injured HP, not the pre-combat HP"
        );
    }

    #[test]
    fn enemy_performs_authored_action_in_live_combat() {
        use crate::db_schema::SCHEMA_SQL;
        use crate::ir::ComponentRecord;

        let db = WorldDatabase::open_in_memory().unwrap();
        for stmt in SCHEMA_SQL.split(';') {
            let s = stmt.trim();
            if !s.is_empty() {
                let _ = db.execute(s, &[]);
            }
        }

        // A creature action: contest vs the target's AC, success → emit damage.
        let records: Vec<ComponentRecord> = vec![serde_json::from_value(serde_json::json!({
            "component_type": "action", "id": "rend", "kind": "contest",
            "resolution": {"tn_source": {"defense": "ac"},
                           "roll_spec": {"mechanic": "xwn_d20_attack"}},
            "outcome": [{"when": "success", "do": [
                {"kind": "emit_damage",
                 "params": {"entity_id": "target", "packet": {"amount": 3, "types": ["physical"]}}},
                {"kind": "log", "params": {"message": "rend tears flesh"}}]}]
        }))
        .unwrap()];
        let store = RuntimeContentStore::from_records(records);

        // A sturdy player plus an enemy carrying the `rend` action.
        let mut player = make_combatant("player_1", true, 30);
        player.ac = 5;
        let mut enemy = make_combatant("brute", false, 10);
        enemy.attack_bonus = 6;
        enemy.actions = vec!["rend".into()];
        let state = CombatState {
            combatants: vec![player, enemy],
            initiative_order: vec!["player_1".to_string(), "brute".to_string()],
            ..make_state(30, 10)
        };
        let mut character = make_character();
        // Non-warrior so action damage applies directly (no Veteran's Luck offer).
        character.character_class = "expert".to_string();
        character.hp = 30;
        character.max_hp = 30;

        let mut scene = CombatScene::new(character, state, vec![], None, Some(store), DifficultyProfile::default());
        let events = scene.run_enemy_turns(&db);

        // The enemy resolved its authored action (not a legacy attack) on its turn.
        let narration = |e: &GameEvent| {
            e.data.get("text").and_then(JsonValue::as_str).unwrap_or("").to_string()
        };
        let performed = events.iter().any(|e| narration(e).contains("performs rend"));
        assert!(
            performed,
            "enemy should perform its `rend` action: {:?}",
            events.iter().map(narration).collect::<Vec<_>>()
        );
        // On a hit the action's HP damage routes through the unified player-damage
        // handler, so a `combat.take_damage_requested` event accompanies any HP loss.
        let player_hp = scene
            .state
            .combatants
            .iter()
            .find(|c| c.is_player)
            .unwrap()
            .hp;
        let took_damage_event = events
            .iter()
            .any(|e| e.event_type == "combat.take_damage_requested");
        assert_eq!(
            player_hp < 30,
            took_damage_event,
            "HP loss and the unified take_damage event must agree (hp={player_hp})"
        );
        assert!(player_hp >= 27, "rend deals 3; hp {player_hp}");
    }

    #[test]
    fn apply_player_damage_applies_and_notifies_for_non_warrior() {
        let mut character = make_character();
        character.character_class = "expert".to_string();
        let mut scene = CombatScene::new(character, make_state(20, 10), vec![], None, None, DifficultyProfile::default());

        let (events, paused) = scene.apply_player_damage("ogre", 5);

        assert!(!paused, "non-warrior gets no Veteran's Luck offer");
        let player_hp = scene.get_player_combatant().unwrap().hp;
        assert_eq!(player_hp, 15, "5 damage applied");
        assert_eq!(scene.character.hp, 15, "character.hp synced");
        assert!(events.iter().any(|e| e.event_type == "combat.take_damage_requested"));
        assert!(events.iter().any(|e| e.event_type == "combat.player_hit"));
    }

    #[test]
    fn apply_player_damage_offers_veteran_luck_for_warrior() {
        let scene_char = make_character(); // warrior by default
        let mut scene = CombatScene::new(scene_char, make_state(20, 10), vec![], None, None, DifficultyProfile::default());

        let (events, paused) = scene.apply_player_damage("ogre", 8);

        assert!(paused, "warrior is offered Veteran's Luck before damage lands");
        assert_eq!(
            scene.state.pending_veteran_luck.as_ref().map(|r| r.damage),
            Some(8),
            "pending damage recorded for the yes/no"
        );
        assert_eq!(scene.get_player_combatant().unwrap().hp, 20, "damage not yet applied");
        assert!(events
            .iter()
            .any(|e| e.data.get("text").and_then(JsonValue::as_str).unwrap_or("").contains("Veteran's Luck")));
    }

    #[test]
    fn apply_player_damage_triggers_last_stand_when_lethal() {
        let mut character = make_character();
        character.character_class = "expert".to_string();
        let mut scene = CombatScene::new(character, make_state(6, 10), vec![], None, None, DifficultyProfile::default());

        let (_events, paused) = scene.apply_player_damage("ogre", 10);

        assert!(!paused);
        assert!(scene.state.last_stand_active, "lethal damage triggers Last Stand");
        assert_eq!(scene.get_player_combatant().unwrap().hp, 0);
    }

    // --- action economy / targeting / selection -----------------------------

    fn contest_action(id: &str, extra: serde_json::Value) -> serde_json::Value {
        let mut base = serde_json::json!({
            "component_type": "action", "id": id, "kind": "contest",
            "resolution": {"tn_source": {"defense": "ac"},
                           "roll_spec": {"mechanic": "xwn_d20_attack"}},
            "outcome": [{"when": "success", "do": [
                {"kind": "emit_damage",
                 "params": {"entity_id": "target", "packet": {"amount": 3, "types": ["physical"]}}}]}]
        });
        let obj = base.as_object_mut().unwrap();
        for (k, v) in extra.as_object().unwrap() {
            obj.insert(k.clone(), v.clone());
        }
        base
    }

    fn store_with(actions: Vec<serde_json::Value>) -> RuntimeContentStore {
        use crate::ir::ComponentRecord;
        let records: Vec<ComponentRecord> = actions
            .into_iter()
            .map(|a| serde_json::from_value(a).unwrap())
            .collect();
        RuntimeContentStore::from_records(records)
    }

    #[test]
    fn select_action_picks_first_affordable() {
        let store = store_with(vec![
            contest_action("blast", serde_json::json!({"activation": {"costs": [{"resource": "ammo", "amount": 1}]}})),
            contest_action("rend", serde_json::json!({})),
        ]);
        let scene = CombatScene::new(make_character(), make_state(10, 10), vec![], None, Some(store), DifficultyProfile::default());
        let ids = vec!["blast".to_string(), "rend".to_string()];

        let mut available = std::collections::BTreeMap::new();
        // No ammo → blast unaffordable, falls through to the free `rend`.
        assert_eq!(scene.select_action(&ids, &available).as_deref(), Some("rend"));
        // With ammo → the first action (blast) is affordable and chosen.
        available.insert("ammo".to_string(), 1);
        assert_eq!(scene.select_action(&ids, &available).as_deref(), Some("blast"));
        // Unknown ids are skipped.
        assert_eq!(scene.select_action(&["nope".to_string()], &available), None);
    }

    #[test]
    fn enemy_action_economy_spends_budget_and_costs() {
        use crate::db_schema::SCHEMA_SQL;
        use crate::resolution::damage::Pool;

        let db = WorldDatabase::open_in_memory().unwrap();
        for stmt in SCHEMA_SQL.split(';') {
            let s = stmt.trim();
            if !s.is_empty() {
                let _ = db.execute(s, &[]);
            }
        }

        let store = store_with(vec![contest_action(
            "blast",
            serde_json::json!({"activation": {"costs": [{"resource": "ammo", "amount": 1}]}}),
        )]);

        let mut player = make_combatant("player_1", true, 30);
        player.ac = 5;
        // Two attacks of budget, but only one charge of ammo.
        let mut enemy = make_combatant("brute", false, 10);
        enemy.attack_bonus = 6;
        enemy.num_attacks = 2;
        enemy.actions = vec!["blast".into()];
        enemy.pools = vec![Pool {
            id: "ammo".into(),
            tags: vec![],
            current: 1,
            max: 1,
            can_go_negative: false,
        }];
        let state = CombatState {
            combatants: vec![player, enemy],
            initiative_order: vec!["player_1".to_string(), "brute".to_string()],
            ..make_state(30, 10)
        };
        let mut character = make_character();
        character.character_class = "expert".to_string();
        character.hp = 30;
        character.max_hp = 30;

        let mut scene = CombatScene::new(character, state, vec![], None, Some(store), DifficultyProfile::default());
        let events = scene.run_enemy_turns(&db);

        let performs = events
            .iter()
            .filter(|e| {
                e.data.get("text").and_then(JsonValue::as_str).unwrap_or("").contains("performs blast")
            })
            .count();
        // Budget is 2, but the second slot can't afford ammo, so only one fires.
        assert_eq!(performs, 1, "ammo gates the second activation");
        let ammo = scene
            .state
            .combatants
            .iter()
            .find(|c| c.entity_id == "brute")
            .and_then(|c| c.pools.iter().find(|p| p.id == "ammo"))
            .map(|p| p.current);
        assert_eq!(ammo, Some(0), "the action's cost was spent");
    }

    #[test]
    fn enemy_self_targeted_action_spares_the_player() {
        use crate::db_schema::SCHEMA_SQL;

        let db = WorldDatabase::open_in_memory().unwrap();
        for stmt in SCHEMA_SQL.split(';') {
            let s = stmt.trim();
            if !s.is_empty() {
                let _ = db.execute(s, &[]);
            }
        }

        // Same contest action but targeting `self`: the `target` role resolves to
        // the actor, so the player is never hit.
        let store = store_with(vec![contest_action(
            "selfrend",
            serde_json::json!({"targeting": {"shape": "self"}}),
        )]);

        let mut player = make_combatant("player_1", true, 30);
        player.ac = 5;
        let mut enemy = make_combatant("brute", false, 20);
        enemy.attack_bonus = 6;
        enemy.actions = vec!["selfrend".into()];
        let state = CombatState {
            combatants: vec![player, enemy],
            initiative_order: vec!["player_1".to_string(), "brute".to_string()],
            ..make_state(30, 20)
        };
        let mut character = make_character();
        character.character_class = "expert".to_string();
        character.hp = 30;
        character.max_hp = 30;

        let mut scene = CombatScene::new(character, state, vec![], None, Some(store), DifficultyProfile::default());
        let events = scene.run_enemy_turns(&db);

        assert!(events
            .iter()
            .any(|e| e.data.get("text").and_then(JsonValue::as_str).unwrap_or("").contains("performs selfrend")));
        let player_hp = scene.state.combatants.iter().find(|c| c.is_player).unwrap().hp;
        assert_eq!(player_hp, 30, "a self-targeted action must not damage the player");
    }

    fn make_character() -> Character {
        let mut attrs = std::collections::BTreeMap::new();
        attrs.insert("str".to_string(), 10);
        attrs.insert("dex".to_string(), 10);
        attrs.insert("con".to_string(), 10);
        attrs.insert("int".to_string(), 10);
        attrs.insert("wis".to_string(), 10);
        attrs.insert("cha".to_string(), 10);
        let attr_mods: std::collections::BTreeMap<String, i32> = attrs
            .keys()
            .map(|k| (k.clone(), 0))
            .collect();
        Character {
            id: "player_1".to_string(),
            name: "Hero".to_string(),
            character_class: "warrior".to_string(),
            level: 1,
            xp: 0,
            xp_next: 1500,
            attributes: attrs,
            attr_mods,
            skills: std::collections::BTreeMap::new(),
            hp: 10,
            max_hp: 10,
            ac: 12,
            attack_bonus: 1,
            physical_save: 15,
            evasion_save: 15,
            mental_save: 15,
            equipment: vec![],
            class_abilities: serde_json::Map::new(),
            position_q: 0,
            position_r: 0,
            ..Default::default()
        }
    }

    // --- combat.actions notice tests ----------------------------------------

    #[test]
    fn actions_notice_active_lists_core_actions() {
        let state = make_state(10, 10);
        let scene = CombatScene::new(make_character(), state, vec![], None, None, DifficultyProfile::default());
        let notice = scene.actions_notice();
        assert_eq!(notice.phase, "active");
        let commands: Vec<&str> = notice.actions.iter().map(|a| a.command.as_str()).collect();
        assert!(commands.contains(&"attack"), "attack button missing: {commands:?}");
        assert!(commands.contains(&"flee"), "flee button missing: {commands:?}");
        assert!(commands.contains(&"use"), "use button missing: {commands:?}");
        assert!(commands.contains(&"advance"), "advance button missing: {commands:?}");
        assert!(commands.contains(&"withdraw"), "withdraw button missing: {commands:?}");
    }

    #[test]
    fn actions_notice_last_stand_swaps_actions() {
        let mut state = make_state(1, 10);
        state.last_stand_active = true;
        let scene = CombatScene::new(make_character(), state, vec![], None, None, DifficultyProfile::default());
        let notice = scene.actions_notice();
        assert_eq!(notice.phase, "last_stand");
        let attack_btn = notice.actions.iter().find(|a| a.command == "attack");
        assert!(attack_btn.is_some(), "attack button missing in last_stand");
        assert_eq!(
            attack_btn.unwrap().style, "danger",
            "last_stand attack must be styled 'danger'"
        );
        let commands: Vec<&str> = notice.actions.iter().map(|a| a.command.as_str()).collect();
        assert!(!commands.contains(&"advance"), "advance must be absent in last_stand");
        assert!(!commands.contains(&"withdraw"), "withdraw must be absent in last_stand");
    }

    #[test]
    fn handle_command_appends_actions_event() {
        use crate::db_schema::SCHEMA_SQL;
        let db = WorldDatabase::open_in_memory().unwrap();
        for stmt in SCHEMA_SQL.split(';') {
            let s = stmt.trim();
            if !s.is_empty() {
                let _ = db.execute(s, &[]);
            }
        }
        let state = make_state(10, 10);
        let mut scene = CombatScene::new(make_character(), state, vec![], None, None, DifficultyProfile::default());
        let cmd = ParsedCommand {
            raw: "status".to_string(),
            verb: "status".to_string(),
            args: vec![],
            direction: None,
        };
        let events = scene.handle_command(&cmd, &db).expect("handle_command failed");
        let has_actions = events.iter().any(|e| e.event_type == "combat.actions");
        assert!(
            has_actions,
            "handle_command must append a combat.actions event; got: {:?}",
            events.iter().map(|e| &e.event_type).collect::<Vec<_>>()
        );
    }

    // ------------------------------------------------------------------------

    #[test]
    fn valid_commands_normal_combat() {
        let state = make_state(10, 10);
        let scene = CombatScene::new(make_character(), state, vec![], None, None, DifficultyProfile::default());
        let cmds = scene.get_valid_commands();
        assert!(cmds.contains(&"attack".to_string()));
        assert!(cmds.contains(&"flee".to_string()));
    }

    #[test]
    fn combatant_resolver_reads_and_mutates_hp() {
        let state = make_state(10, 6);
        let mut scene = CombatScene::new(make_character(), state, vec![], None, None, DifficultyProfile::default());
        // Snapshot reads a combatant's hp via the resolver.
        assert_eq!(scene.snapshot("goblin_1").map(|s| s.scalar_fields["hp"].clone()), Some(serde_json::json!(6)));
        // Applies an hp delta; non-hp resources are ignored.
        scene.apply_resource_delta("goblin_1", "mana", -3);
        assert_eq!(scene.find_combatant("goblin_1").map(|c| c.hp), Some(6));
        scene.apply_resource_delta("goblin_1", "hp", -2);
        assert_eq!(scene.find_combatant("goblin_1").map(|c| c.hp), Some(4));
        // Lethal delta clamps to 0 and marks dead.
        scene.apply_resource_delta("goblin_1", "hp", -10);
        let g = scene.find_combatant("goblin_1").unwrap();
        assert_eq!(g.hp, 0);
        assert!(!g.alive);
    }

    #[test]
    fn valid_commands_last_stand() {
        let mut state = make_state(1, 10);
        state.last_stand_active = true;
        let scene = CombatScene::new(make_character(), state, vec![], None, None, DifficultyProfile::default());
        let cmds = scene.get_valid_commands();
        assert!(cmds.contains(&"attack".to_string()));
        assert!(cmds.contains(&"flee".to_string()));
        assert!(!cmds.contains(&"advance".to_string()));
    }

    #[test]
    fn valid_commands_veteran_luck() {
        let mut state = make_state(10, 10);
        state.pending_veteran_luck = Some(PendingVeteranLuckRecord {
            attacker_name: "Goblin".to_string(),
            damage: 3,
        });
        let scene = CombatScene::new(make_character(), state, vec![], None, None, DifficultyProfile::default());
        let cmds = scene.get_valid_commands();
        assert_eq!(cmds, vec!["yes", "no"]);
    }

    #[test]
    fn handle_status_returns_event() {
        let state = make_state(10, 10);
        let scene = CombatScene::new(make_character(), state, vec![], None, None, DifficultyProfile::default());
        let events = scene.handle_status();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, "gm.narrate");
    }

    #[test]
    fn handle_advance_sets_range() {
        let mut state = make_state(10, 10);
        // Put player at near range
        state.combatants[0].range_band = "near".to_string();
        let mut scene = CombatScene::new(make_character(), state, vec![], None, None, DifficultyProfile::default());
        let events = scene.handle_advance();
        assert!(!events.is_empty());
        let player = scene.get_player_combatant().unwrap();
        assert_eq!(player.range_band, "melee");
    }

    #[test]
    fn handle_withdraw_sets_range() {
        let mut scene = CombatScene::new(make_character(), make_state(10, 10), vec![], None, None, DifficultyProfile::default());
        let events = scene.handle_withdraw();
        assert!(!events.is_empty());
        let player = scene.get_player_combatant().unwrap();
        assert_eq!(player.range_band, "near");
    }

    #[test]
    fn check_transitions_combat_over() {
        let mut state = make_state(10, 0);
        state.combat_over = true;
        let scene = CombatScene::new(make_character(), state, vec![], None, None, DifficultyProfile::default());
        let transition = scene.check_transitions(&[]);
        assert_eq!(transition, Some(SceneState::Exploration));
    }

    #[test]
    fn check_transitions_none_during_combat() {
        let state = make_state(10, 10);
        let scene = CombatScene::new(make_character(), state, vec![], None, None, DifficultyProfile::default());
        assert_eq!(scene.check_transitions(&[]), None);
    }

    #[test]
    fn get_prompt_last_stand() {
        let db = WorldDatabase::open_in_memory().unwrap();
        let mut state = make_state(1, 10);
        state.last_stand_active = true;
        let scene = CombatScene::new(make_character(), state, vec![], None, None, DifficultyProfile::default());
        let prompt = scene.get_prompt(&db);
        assert!(prompt.contains("LAST STAND"));
    }

    #[test]
    fn veteran_luck_yes_clears_pending() {
        let mut state = make_state(10, 10);
        state.pending_veteran_luck = Some(PendingVeteranLuckRecord {
            attacker_name: "Goblin".to_string(),
            damage: 5,
        });
        let mut scene = CombatScene::new(make_character(), state, vec![], None, None, DifficultyProfile::default());
        let events = scene.handle_veteran_luck_yes();
        assert!(scene.state.pending_veteran_luck.is_none());
        assert!(scene.state.veteran_luck_used);
        assert!(!events.is_empty());
    }

    #[test]
    fn veteran_luck_no_applies_damage() {
        let mut state = make_state(10, 10);
        state.pending_veteran_luck = Some(PendingVeteranLuckRecord {
            attacker_name: "Goblin".to_string(),
            damage: 5,
        });
        let mut scene = CombatScene::new(make_character(), state, vec![], None, None, DifficultyProfile::default());
        let events = scene.handle_veteran_luck_no();
        assert!(scene.state.pending_veteran_luck.is_none());
        // Player combatant should have taken 5 damage
        let player_hp = scene.get_player_combatant().map(|p| p.hp).unwrap_or(99);
        assert_eq!(player_hp, 5);
        assert!(!events.is_empty());
    }

    #[test]
    fn reconcile_ground_loot_records_each_dead_enemy_once() {
        // HR-787: a dead enemy drops exactly once, at its grid tile. With no
        // matching creature the pile is empty (not "visible") but still
        // recorded, so a second reconcile is a no-op.
        let mut state = make_state(10, 0);
        for c in state.combatants.iter_mut() {
            if !c.is_player {
                c.q = 3;
                c.r = 7;
                c.alive = false;
            }
        }
        let mut scene = CombatScene::new(
            make_character(),
            state,
            vec![],
            None,
            None,
            DifficultyProfile::default(),
        );

        let visible = scene.reconcile_ground_loot();
        assert!(!visible, "an empty pile is not a visible drop");
        assert_eq!(scene.state.ground_loot.len(), 1, "one entry per dead enemy");
        assert_eq!(
            (scene.state.ground_loot[0].q, scene.state.ground_loot[0].r),
            (3, 7),
            "the pile sits on the fallen enemy's tile"
        );

        scene.reconcile_ground_loot();
        assert_eq!(
            scene.state.ground_loot.len(),
            1,
            "reconcile is idempotent — no re-roll"
        );
    }

    #[test]
    fn victory_sweeps_ground_loot_into_inventory() {
        // HR-787: piles dropped as enemies fell are collected into the victory
        // payload (items_gained + currency_gained). Pre-recording the pile means
        // the final reconcile is a no-op, so the assertion is RNG-free.
        let mut state = make_state(10, 0);
        let enemy_id = state
            .combatants
            .iter()
            .find(|c| !c.is_player)
            .map(|c| c.entity_id.clone())
            .unwrap();
        for c in state.combatants.iter_mut() {
            if !c.is_player {
                c.alive = false;
            }
        }
        let relic: JsonObject = serde_json::from_value(
            serde_json::json!({ "name": "Pretech Relic", "type": "relic", "value": 150 }),
        )
        .unwrap();
        state.ground_loot = vec![GroundLoot {
            entity_id: enemy_id,
            q: 4,
            r: 6,
            items: vec![relic],
            currency: 25,
        }];
        let creatures = vec![crate::creature::generate_dragon("small", "fire").unwrap()];
        let mut scene = CombatScene::new(
            make_character(),
            state,
            creatures,
            None,
            None,
            DifficultyProfile::default(),
        );

        let events = scene.handle_victory();
        let vic = events
            .iter()
            .find(|e| e.event_type == "combat.victory_requested")
            .expect("victory event");
        // HR-786: item drops go onto a searchable corpse (corpse_items), NOT
        // auto-collected into inventory (items_gained). Gold still auto-collects.
        assert_eq!(
            vic.data.get("items_gained").and_then(|v| v.as_array()).map(|a| a.len()),
            Some(0),
            "items are not auto-collected — they wait on the corpse"
        );
        let corpse = vic
            .data
            .get("corpse_items")
            .and_then(|v| v.as_array())
            .expect("corpse_items array");
        assert_eq!(corpse.len(), 1, "the dropped relic is left on the corpse");
        assert_eq!(
            corpse[0].get("name").and_then(|v| v.as_str()),
            Some("Pretech Relic")
        );
        assert_eq!(
            vic.data.get("currency_gained").and_then(|v| v.as_i64()),
            Some(25),
            "gold still auto-collects"
        );
    }
}
