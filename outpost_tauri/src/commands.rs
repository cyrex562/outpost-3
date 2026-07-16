//! Tauri command handlers.
//!
//! Uses shim `ClientCommand` / `ClientQuery` / `ServerEvent` types that mirror
//! the wire format the Vue frontend already speaks. Each handler translates
//! the shim type into a core [`Command`] / [`Query`], applies it, and returns
//! the results in the same wire shape.

use include_dir::{include_dir, Dir};
use outpost_core::colony::ColonyId;
use outpost_core::content::loader::PackLoader;
use outpost_core::difficulty::DifficultyPreset;
use outpost_core::modifier::{DifficultyScalar, ModifiableQuantity};
use outpost_core::needs::NeedsConfig;
use outpost_core::snapshot::Snapshot as SnapshotDb;
use outpost_core::system::{BodyKind, SystemCommand, SystemRole};
use outpost_core::tech::load_tech_registry;
use outpost_core::trade::SiteId;
use outpost_core::{Command, Event, GameEngine, Query, QueryResult};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use tauri::{AppHandle, Manager, State};
use uuid::Uuid;

use crate::state::EngineState;

/// Embedded content pack shipped inside the binary. Path is relative to this crate's Cargo.toml.
static EMBEDDED_PACK: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/../content/base");

// ── Error type ────────────────────────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum CmdError {
    #[error("engine not initialised — call bootstrap first")]
    NotInitialised,
    #[error("engine error: {0}")]
    Engine(String),
    #[error("content load error: {0}")]
    Content(String),
    #[error("snapshot error: {0}")]
    Snapshot(String),
    #[error("invalid argument: {0}")]
    InvalidArg(String),
}

// Serialise as the Display string. Deriving `Serialize` on the enum makes
// tauri reject `invoke` with a structured object (e.g. `{"Snapshot": "..."}`)
// which the frontend then stringifies to `[object Object]`. Emitting the
// human-readable form keeps every reject path legible in the UI.
impl Serialize for CmdError {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl From<outpost_core::EngineError> for CmdError {
    fn from(e: outpost_core::EngineError) -> Self {
        Self::Engine(e.to_string())
    }
}

type CmdResult<T> = Result<T, CmdError>;

// ── Wire types (mirror the frontend's `Command` / `GameEvent`) ────────────────

/// Client → engine command. Matches the frontend's `Command` discriminated union.
#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ClientCommand {
    AdvanceSol,
    FoundColony {
        name: String,
        starting_population: u64,
    },
    QueueConstruction {
        colony_id: String,
        building_type: String,
        slot_cost: u32,
        labor_per_turn: u32,
        construction_cost: Vec<(String, f64)>,
        construction_turns: u32,
    },
    AssignLabour {
        colony_id: String,
        slot: String,
        labour: u64,
    },
    /// Select which recipe a building type runs in this colony (issue #166).
    SetActiveRecipe {
        colony_id: String,
        building_type: String,
        recipe_id: String,
    },
    /// Cancel a queued construction project and receive a 50% partial refund.
    CancelConstruction {
        colony_id: String,
        project_id: String,
    },
    ResearchTech {
        tech_id: String,
    },
    EnqueueResearch {
        tech_id: String,
    },
    FoundColonyAtSite {
        name: String,
        starting_population: u64,
        site_id: String,
        focus: Option<String>,
        #[serde(default)]
        supplies_id: Option<String>,
        /// Star-system body this site belongs to (issue #183). `None` skips
        /// the habitability gate, preserving pre-#183 client behaviour.
        #[serde(default)]
        body_id: Option<String>,
    },
    /// Set an active preset (built-in) mid-game.
    SetDifficulty {
        preset: String,
    },
    /// Install a custom scalar map + toggles atomically (issue #161, #180).
    SetCustomDifficulty {
        /// Slider values keyed by knob id (see [`get_difficulty_knobs`]).
        scalars: std::collections::HashMap<String, f32>,
        menace_enabled: bool,
        hazards_enabled: bool,
        /// Master maintenance toggle (issue #180). Pre-#180 clients that omit
        /// this default to `true` so authored building upkeep still applies.
        #[serde(default)]
        maintenance_enabled: Option<bool>,
    },
    /// Master hazards toggle (issue #161).
    SetHazardsEnabled {
        enabled: bool,
    },
    /// Master maintenance toggle (issue #180).
    SetMaintenanceEnabled {
        enabled: bool,
    },
    /// Link a colony to its home star-system body (issue #163).
    AssignColonyHomeBody {
        colony_id: String,
        body_id: String,
    },
}

/// Read-only query. Matches the frontend's query message.
#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ClientQuery {
    CurrentSol,
    CurrentMonth,
    ListColonies,
    ColonyStatus {
        colony_id: String,
    },
    ColonyScreen {
        colony_id: String,
    },
    SystemResearchTotal,
    /// Full detail for one building type within a colony (issue #182).
    BuildingDetail {
        colony_id: String,
        building_type: String,
    },
}

/// Wire event returned to the frontend. Matches `GameEvent` (snake_case, `kind`-tagged).
#[derive(Debug, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ServerEvent {
    ColonySolAdvanced {
        sol: u64,
    },
    StrategicMonthAdvanced {
        month: u64,
    },
    ColonyFounded {
        colony_id: String,
        name: String,
        starting_population: u64,
    },
    ConstructionQueued {
        colony_id: String,
        building_type: String,
        project_id: String,
    },
    ConstructionCancelled {
        colony_id: String,
        project_id: String,
        refund: Vec<(String, f64)>,
    },
    BuildingConstructed {
        colony_id: String,
        building_type: String,
    },
    LabourAssigned {
        colony_id: String,
        slot: String,
        labour: u64,
    },
    /// A building type's active recipe was set (issue #166).
    ActiveRecipeSet {
        colony_id: String,
        building_type: String,
        recipe_id: String,
    },
    NeedsResolved {
        colony_id: String,
        composite_satisfaction: f32,
        stability_delta: f32,
        population_delta: f32,
    },
    ResearchProduced {
        colony_id: String,
        amount: f32,
    },
    ProductionShortfall {
        colony_id: String,
        building_type: String,
        scale: f64,
        reason: String,
    },
    /// The active difficulty preset changed (issue #161).
    DifficultyChanged {
        preset: String,
    },
    /// A colony's home body was recorded (issue #163).
    ColonyHomeBodySet {
        colony_id: String,
        body_id: String,
        habitability_modifier: f32,
    },
    /// Fallback for events we don't have a typed variant for yet.
    Unknown {
        core_kind: String,
    },
}

impl ServerEvent {
    fn from_core(event: &Event) -> Self {
        match event {
            Event::ColonySolAdvanced { sol } => Self::ColonySolAdvanced { sol: *sol },
            Event::StrategicMonthAdvanced { month } => {
                Self::StrategicMonthAdvanced { month: *month }
            }
            Event::ColonyFounded {
                colony_id,
                name,
                starting_population,
            } => Self::ColonyFounded {
                colony_id: colony_id.to_string(),
                name: name.clone(),
                starting_population: *starting_population,
            },
            // Semantically equivalent for UI purposes — surface the same
            // `colony_founded` wire shape so the frontend reducer + wizard
            // handler don't need a parallel code path.
            Event::ColonyFoundedAtSite {
                colony_id,
                name,
                starting_population,
                ..
            } => Self::ColonyFounded {
                colony_id: colony_id.to_string(),
                name: name.clone(),
                starting_population: *starting_population,
            },
            Event::ConstructionQueued {
                colony_id,
                building_type,
                project_id,
            } => Self::ConstructionQueued {
                colony_id: colony_id.to_string(),
                building_type: building_type.clone(),
                project_id: project_id.to_string(),
            },
            Event::ConstructionCancelled {
                colony_id,
                project_id,
                refund,
            } => Self::ConstructionCancelled {
                colony_id: colony_id.to_string(),
                project_id: project_id.to_string(),
                refund: refund.clone(),
            },
            Event::BuildingConstructed {
                colony_id,
                building_type,
            } => Self::BuildingConstructed {
                colony_id: colony_id.to_string(),
                building_type: building_type.clone(),
            },
            Event::LabourAssigned {
                colony_id,
                slot,
                labour,
            } => Self::LabourAssigned {
                colony_id: colony_id.to_string(),
                slot: slot.clone(),
                labour: *labour,
            },
            Event::ActiveRecipeSet {
                colony_id,
                building_type,
                recipe_id,
            } => Self::ActiveRecipeSet {
                colony_id: colony_id.to_string(),
                building_type: building_type.clone(),
                recipe_id: recipe_id.clone(),
            },
            Event::NeedsResolved {
                colony_id,
                composite_satisfaction,
                stability_delta,
                population_delta,
            } => Self::NeedsResolved {
                colony_id: colony_id.to_string(),
                composite_satisfaction: *composite_satisfaction,
                stability_delta: *stability_delta,
                population_delta: *population_delta,
            },
            Event::ResearchProduced { colony_id, amount } => Self::ResearchProduced {
                colony_id: colony_id.to_string(),
                amount: *amount,
            },
            Event::ProductionShortfall {
                colony_id,
                building_type,
                scale,
                reason,
            } => Self::ProductionShortfall {
                colony_id: colony_id.to_string(),
                building_type: building_type.clone(),
                scale: *scale,
                reason: format!("{reason:?}"),
            },
            Event::DifficultyChanged { preset } => Self::DifficultyChanged {
                preset: preset.label().to_owned(),
            },
            Event::ColonyHomeBodySet {
                colony_id,
                body_id,
                habitability_modifier,
            } => Self::ColonyHomeBodySet {
                colony_id: colony_id.to_string(),
                body_id: body_id.0.to_string(),
                habitability_modifier: *habitability_modifier,
            },
            other => Self::Unknown {
                core_kind: format!("{other:?}")
                    .split_whitespace()
                    .next()
                    .unwrap_or("event")
                    .to_owned(),
            },
        }
    }
}

/// Initial snapshot returned by `bootstrap` or `snapshot`.
#[derive(Debug, Serialize)]
pub struct SnapshotPayload {
    pub sol: u64,
    pub month: u64,
    pub colonies: Vec<ColonyWire>,
    pub research_total: f32,
}

#[derive(Debug, Serialize)]
pub struct ColonyWire {
    pub id: String,
    pub name: String,
    pub population: f32,
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn parse_colony(id: &str) -> Result<ColonyId, CmdError> {
    ColonyId::from_str(id).map_err(|_| CmdError::InvalidArg(format!("bad colony id: {id}")))
}

fn build_snapshot(engine: &GameEngine) -> SnapshotPayload {
    let state = &engine.state;
    let colonies = state
        .colonies
        .iter()
        .zip(state.populations.iter())
        .map(|(c, p)| ColonyWire {
            id: c.id.to_string(),
            name: c.name.clone(),
            population: p.count,
        })
        .collect();
    SnapshotPayload {
        sol: state.sol,
        month: state.month,
        colonies,
        research_total: state.research_pool.total,
    }
}

fn load_embedded_content() -> Result<outpost_core::content::registry::ContentRegistry, CmdError> {
    let mut raw: Vec<(&str, &str)> = Vec::new();
    for file in EMBEDDED_PACK.files() {
        if file.path().extension().and_then(|e| e.to_str()) != Some("yaml") {
            continue;
        }
        let name = file
            .path()
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");
        let text = file
            .contents_utf8()
            .ok_or_else(|| CmdError::Content(format!("non-utf8 embedded file: {name}")))?;
        raw.push((name, text));
    }
    PackLoader::load(&raw).map_err(|e| CmdError::Content(e.to_string()))
}

fn load_content(
    content_dir: &Path,
) -> Result<outpost_core::content::registry::ContentRegistry, CmdError> {
    let mut raw_owned: Vec<(String, String)> = Vec::new();
    let entries = std::fs::read_dir(content_dir).map_err(|e| CmdError::Content(e.to_string()))?;
    for entry in entries {
        let entry = entry.map_err(|e| CmdError::Content(e.to_string()))?;
        let path = entry.path();
        if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("yaml") {
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_owned();
            let text =
                std::fs::read_to_string(&path).map_err(|e| CmdError::Content(e.to_string()))?;
            raw_owned.push((name, text));
        }
    }
    let raw: Vec<(&str, &str)> = raw_owned
        .iter()
        .map(|(n, t)| (n.as_str(), t.as_str()))
        .collect();
    PackLoader::load(&raw).map_err(|e| CmdError::Content(e.to_string()))
}

fn parse_preset(difficulty: &str) -> DifficultyPreset {
    match difficulty {
        "Sandbox" | "sandbox" => DifficultyPreset::Sandbox,
        "Easy" | "easy" => DifficultyPreset::Easy,
        "Hard" | "hard" => DifficultyPreset::Hard,
        "Brutal" | "brutal" => DifficultyPreset::Brutal,
        "Custom" | "custom" => DifficultyPreset::Custom,
        _ => DifficultyPreset::Normal,
    }
}

// ── #161: knob spec + custom-preset persistence ─────────────────────────────

/// One row in the custom-difficulty panel. Sliders and toggles are unified so
/// the frontend can render every knob from a single list.
///
/// `preset_default_at_current_preset` seeds the panel from the currently
/// selected built-in preset so "Custom" starts as a copy of that preset.
#[derive(Debug, Serialize)]
pub struct KnobSpec {
    pub id: String,
    pub label: String,
    /// `"slider"` or `"toggle"`.
    pub kind: &'static str,
    /// Slider step values (empty for toggles).
    pub step: Vec<f32>,
    pub min: f32,
    pub max: f32,
    /// Value at the currently active preset (baseline reference).
    pub preset_default_at_current_preset: f32,
    /// The engine's current effective value (post-Custom / post-toggle).
    pub current_value: f32,
}

/// Canonical order of the 11 slider knobs + 2 toggles surfaced to the UI.
///
/// The `.0` is the wire id used both in [`KnobSpec::id`] and in the
/// [`ClientCommand::SetCustomDifficulty::scalars`] map keys.
const SLIDER_KNOBS: &[(&str, &str, ModifiableQuantityKey)] = &[
    (
        "production_rate",
        "Production Rate",
        ModifiableQuantityKey::ProductionRateAll,
    ),
    (
        "labor_efficiency",
        "Labor Efficiency",
        ModifiableQuantityKey::LaborEfficiency,
    ),
    (
        "research_rate",
        "Research Rate",
        ModifiableQuantityKey::ResearchRate,
    ),
    (
        "storage_capacity",
        "Storage Capacity",
        ModifiableQuantityKey::StorageCapacity,
    ),
    (
        "population_growth",
        "Population Growth",
        ModifiableQuantityKey::PopulationGrowth,
    ),
    (
        "stability_decay",
        "Stability Decay",
        ModifiableQuantityKey::StabilityRate,
    ),
    (
        "hazard_probability",
        "Hazard Probability",
        ModifiableQuantityKey::HazardProbability,
    ),
    (
        "slot_capacity",
        "Slot Capacity",
        ModifiableQuantityKey::SlotCapacity,
    ),
    (
        "resource_consumption",
        "Resource Consumption",
        ModifiableQuantityKey::ResourceConsumption,
    ),
    (
        "research_cost",
        "Research Cost",
        ModifiableQuantityKey::ResearchCost,
    ),
    (
        "power_requirement",
        "Power Requirement",
        ModifiableQuantityKey::PowerRequirement,
    ),
    (
        "maintenance_consumption",
        "Maintenance Consumption",
        ModifiableQuantityKey::MaintenanceConsumption,
    ),
];

/// Fixed anchor slider steps shared by every knob (issue #161 design decision).
const SLIDER_STEPS: [f32; 6] = [0.5, 0.75, 1.0, 1.25, 1.5, 2.0];

#[derive(Debug, Clone, Copy)]
enum ModifiableQuantityKey {
    ProductionRateAll,
    LaborEfficiency,
    ResearchRate,
    StorageCapacity,
    PopulationGrowth,
    StabilityRate,
    HazardProbability,
    SlotCapacity,
    ResourceConsumption,
    ResearchCost,
    PowerRequirement,
    MaintenanceConsumption,
}

impl ModifiableQuantityKey {
    fn to_quantity(self) -> ModifiableQuantity {
        match self {
            Self::ProductionRateAll => ModifiableQuantity::ProductionRate("*".into()),
            Self::LaborEfficiency => ModifiableQuantity::LaborEfficiency,
            Self::ResearchRate => ModifiableQuantity::ResearchRate,
            Self::StorageCapacity => ModifiableQuantity::StorageCapacity,
            Self::PopulationGrowth => ModifiableQuantity::PopulationGrowth,
            Self::StabilityRate => ModifiableQuantity::StabilityRate,
            Self::HazardProbability => ModifiableQuantity::HazardProbability,
            Self::SlotCapacity => ModifiableQuantity::SlotCapacity,
            Self::ResourceConsumption => ModifiableQuantity::ResourceConsumption,
            Self::ResearchCost => ModifiableQuantity::ResearchCost,
            Self::PowerRequirement => ModifiableQuantity::PowerRequirement,
            Self::MaintenanceConsumption => ModifiableQuantity::MaintenanceConsumption,
        }
    }
}

/// Assemble a [`DifficultyScalar`] from a knob-id → value map submitted by the UI.
fn scalars_from_map(map: &std::collections::HashMap<String, f32>) -> DifficultyScalar {
    let mut out = DifficultyScalar::new();
    for (id, _, key) in SLIDER_KNOBS {
        if let Some(v) = map.get(*id) {
            out.set(key.to_quantity(), *v);
        }
    }
    out
}

/// Wire form of a saved custom-difficulty preset.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomPreset {
    pub name: String,
    /// Knob-id → value.
    pub scalars: std::collections::HashMap<String, f32>,
    pub menace_enabled: bool,
    pub hazards_enabled: bool,
    /// Master maintenance toggle (issue #180). Pre-#180 preset YAMLs default
    /// to `true` so authored building upkeep still applies after upgrade.
    #[serde(default = "yes")]
    pub maintenance_enabled: bool,
}

fn yes() -> bool {
    true
}

/// Directory containing one .yaml per saved preset.
fn custom_preset_dir(app: &AppHandle) -> Result<PathBuf, CmdError> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| CmdError::Snapshot(format!("no app data dir: {e}")))?
        .join("custom_difficulty");
    if !dir.exists() {
        std::fs::create_dir_all(&dir).map_err(|e| CmdError::Snapshot(e.to_string()))?;
    }
    Ok(dir)
}

fn sanitize_preset_name(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || matches!(c, '-' | '_' | ' ') {
                c
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim()
        .to_owned()
}

// ── Commands ──────────────────────────────────────────────────────────────────

/// Boot a fresh engine: load content, set difficulty, seed the planet map.
///
/// If `content_dir` is empty (or literally `"embedded"`), the pack embedded
/// in the binary at build time is used. Otherwise `content_dir` is read
/// from the filesystem.
#[tauri::command]
pub fn bootstrap(
    content_dir: String,
    planet_seed: u64,
    difficulty: String,
    #[allow(non_snake_case)] custom_scalars: Option<std::collections::HashMap<String, f32>>,
    custom_menace_enabled: Option<bool>,
    custom_hazards_enabled: Option<bool>,
    custom_maintenance_enabled: Option<bool>,
    engine_state: State<'_, EngineState>,
) -> CmdResult<SnapshotPayload> {
    let registry = if content_dir.is_empty() || content_dir == "embedded" {
        load_embedded_content()?
    } else {
        load_content(Path::new(&content_dir))?
    };

    let mut engine = GameEngine::new();
    engine.state.registry = Some(registry);
    engine.state.needs_config = Some(NeedsConfig::default_survival());

    let _ = engine.apply(&Command::SetDifficulty {
        preset: parse_preset(&difficulty),
    });

    // Overlay a custom-difficulty payload if one was supplied (issue #161).
    if let Some(map) = custom_scalars {
        let _ = engine.apply(&Command::SetCustomDifficulty {
            scalars: scalars_from_map(&map),
            menace_enabled: custom_menace_enabled.unwrap_or(true),
            hazards_enabled: custom_hazards_enabled.unwrap_or(true),
            maintenance_enabled: custom_maintenance_enabled.unwrap_or(true),
        });
    }

    let _ = engine.apply(&Command::SeedPlanet {
        seed: planet_seed,
        radius: 12,
    });

    // Seed the star system from the loaded content pack. Falls back to
    // an empty system if the pack ships no `systems.yaml`.
    seed_system_from_content(&mut engine);
    load_embedded_tech(&mut engine);

    let snap = build_snapshot(&engine);
    *engine_state.engine.lock().unwrap() = Some(engine);
    Ok(snap)
}

fn load_embedded_tech(engine: &mut GameEngine) {
    let Some(tech_file) = EMBEDDED_PACK.get_file("tech.yaml") else {
        return;
    };
    let Some(yaml) = tech_file.contents_utf8() else {
        return;
    };
    if let Ok(registry) = load_tech_registry(yaml) {
        engine.state.tech_registry = Some(registry);
    }
}

/// Seed `system_state.node_map` from the loaded content pack.
///
/// Picks the first `StarSystemDef` in the registry and applies one
/// [`SystemCommand::AddBody`] per authored body. No-op when the pack ships
/// no systems. Selection UI (letting the player pick which scenario)
/// can layer on later without changing this seeding path.
///
/// Assigned roles land on the body via a follow-up
/// [`SystemCommand::AssignRole`] since `AddBody` sets `Unassigned` by default.
fn seed_system_from_content(engine: &mut GameEngine) {
    let Some(system) = engine
        .state
        .registry
        .as_ref()
        .and_then(|r| r.star_systems().next().cloned())
    else {
        return;
    };

    // Pass 1: add every body and its authored attributes/role. `parent_body`
    // is deferred to pass 2 (below) since it may name a body that hasn't
    // been added yet — content validation (issue #196) only guarantees the
    // name resolves *somewhere* in this system, not that it comes earlier.
    let mut pending_parents: Vec<(outpost_core::system::BodyId, String)> = Vec::new();
    for body in &system.bodies {
        let _ = engine.apply(&Command::System(SystemCommand::AddBody {
            name: body.name.clone(),
            kind: body.kind.clone(),
            distance_au: body.distance_au,
        }));
        // `AddBody` doesn't carry the resulting body id back; look it up by name.
        let body_id = engine
            .state
            .system_state
            .node_map
            .bodies
            .iter()
            .find(|(_, b)| b.name == body.name)
            .map(|(id, _)| id.clone());
        let Some(id) = body_id else { continue };
        let _ = engine.apply(&Command::System(SystemCommand::SetBodyAttributes {
            body_id: id.clone(),
            atmosphere_density: body.atmosphere_density,
            atmosphere_hazard: body.atmosphere_hazard,
            temperature: body.temperature,
            gravity_g: body.gravity_g,
            radiation: body.radiation,
            subtype: body.subtype,
            tidally_locked: body.tidally_locked,
            axial_tilt_deg: body.axial_tilt_deg,
            rotation_period_hours: body.rotation_period_hours,
            moon_count: body.moon_count,
        }));
        if !matches!(body.role, SystemRole::Unassigned) {
            let _ = engine.apply(&Command::System(SystemCommand::AssignRole {
                body_id: id.clone(),
                role: body.role.clone(),
            }));
        }
        if let Some(parent_name) = &body.parent_body {
            pending_parents.push((id, parent_name.clone()));
        }
    }

    // Pass 2: resolve authored parent-body names now that every body in the
    // system has a live BodyId.
    for (body_id, parent_name) in pending_parents {
        let parent_id = engine
            .state
            .system_state
            .node_map
            .bodies
            .iter()
            .find(|(_, b)| b.name == parent_name)
            .map(|(id, _)| id.clone());
        let Some(parent_body) = parent_id else {
            continue;
        };
        let _ = engine.apply(&Command::System(SystemCommand::SetBodyParent {
            body_id,
            parent_body,
        }));
    }
}

/// Return whether an engine has been bootstrapped.
#[tauri::command]
pub fn is_ready(engine_state: State<'_, EngineState>) -> bool {
    engine_state.engine.lock().unwrap().is_some()
}

/// Terminate the app process cleanly.
///
/// Using `AppHandle::exit(0)` instead of `WebviewWindow::close()` avoids the
/// v2 quirk where closing the main window doesn't necessarily reap the
/// process, and doesn't depend on the `core:window:close` capability.
#[tauri::command]
pub fn exit_app(app: tauri::AppHandle) {
    app.exit(0);
}

/// Return a fresh full snapshot of engine state.
#[tauri::command]
pub fn snapshot(engine_state: State<'_, EngineState>) -> CmdResult<SnapshotPayload> {
    let guard = engine_state.engine.lock().unwrap();
    let engine = guard.as_ref().ok_or(CmdError::NotInitialised)?;
    Ok(build_snapshot(engine))
}

/// Apply a single command (mirrors the WebSocket "command" message).
#[tauri::command]
pub fn apply_command(
    command: ClientCommand,
    engine_state: State<'_, EngineState>,
) -> CmdResult<Vec<ServerEvent>> {
    let mut guard = engine_state.engine.lock().unwrap();
    let engine = guard.as_mut().ok_or(CmdError::NotInitialised)?;

    let core_cmd = match command {
        ClientCommand::AdvanceSol => Command::AdvanceColonySol,
        ClientCommand::FoundColony {
            name,
            starting_population,
        } => Command::FoundColony {
            name,
            starting_population,
        },
        ClientCommand::QueueConstruction {
            colony_id,
            building_type,
            slot_cost,
            labor_per_turn,
            construction_cost,
            construction_turns,
        } => Command::QueueConstruction {
            colony_id: parse_colony(&colony_id)?,
            building_type,
            slot_cost,
            labor_per_turn,
            construction_cost,
            construction_turns,
        },
        ClientCommand::AssignLabour {
            colony_id,
            slot,
            labour,
        } => Command::AssignLabour {
            colony_id: parse_colony(&colony_id)?,
            slot,
            labour,
        },
        ClientCommand::SetActiveRecipe {
            colony_id,
            building_type,
            recipe_id,
        } => Command::SetActiveRecipe {
            colony_id: parse_colony(&colony_id)?,
            building_type,
            recipe_id,
        },
        ClientCommand::CancelConstruction {
            colony_id,
            project_id,
        } => Command::CancelConstruction {
            colony_id: parse_colony(&colony_id)?,
            project_id: Uuid::parse_str(&project_id)
                .map_err(|_| CmdError::InvalidArg(format!("bad project_id: {project_id}")))?,
        },
        ClientCommand::ResearchTech { tech_id } => Command::ResearchTech { tech_id },
        ClientCommand::EnqueueResearch { tech_id } => Command::EnqueueResearch { tech_id },
        ClientCommand::FoundColonyAtSite {
            name,
            starting_population,
            site_id,
            focus,
            supplies_id,
            body_id,
        } => {
            let uuid = Uuid::parse_str(&site_id)
                .map_err(|_| CmdError::InvalidArg(format!("bad site_id: {site_id}")))?;
            let body_id = body_id
                .map(|b| {
                    Uuid::parse_str(&b)
                        .map(outpost_core::system::BodyId)
                        .map_err(|_| CmdError::InvalidArg(format!("bad body_id: {b}")))
                })
                .transpose()?;
            Command::FoundColonyAtSite {
                name,
                starting_population,
                site_id: SiteId(uuid),
                focus,
                supplies_id,
                body_id,
            }
        }
        ClientCommand::SetDifficulty { preset } => Command::SetDifficulty {
            preset: parse_preset(&preset),
        },
        ClientCommand::SetCustomDifficulty {
            scalars,
            menace_enabled,
            hazards_enabled,
            maintenance_enabled,
        } => Command::SetCustomDifficulty {
            scalars: scalars_from_map(&scalars),
            menace_enabled,
            hazards_enabled,
            maintenance_enabled: maintenance_enabled.unwrap_or(true),
        },
        ClientCommand::SetHazardsEnabled { enabled } => Command::SetHazardsEnabled { enabled },
        ClientCommand::SetMaintenanceEnabled { enabled } => {
            Command::SetMaintenanceEnabled { enabled }
        }
        ClientCommand::AssignColonyHomeBody { colony_id, body_id } => {
            let cid = parse_colony(&colony_id)?;
            let bid = Uuid::parse_str(&body_id)
                .map_err(|_| CmdError::InvalidArg(format!("bad body_id: {body_id}")))?;
            Command::AssignColonyHomeBody {
                colony_id: cid,
                body_id: outpost_core::system::BodyId(bid),
            }
        }
    };

    let events = engine.apply(&core_cmd).map_err(CmdError::from)?;
    Ok(events.iter().map(ServerEvent::from_core).collect())
}

/// Run a read-only query. Returns raw JSON to accommodate heterogeneous result shapes.
#[tauri::command]
pub fn run_query(
    query: ClientQuery,
    engine_state: State<'_, EngineState>,
) -> CmdResult<serde_json::Value> {
    let guard = engine_state.engine.lock().unwrap();
    let engine = guard.as_ref().ok_or(CmdError::NotInitialised)?;

    let core_query = match query {
        ClientQuery::CurrentSol => Query::CurrentSol,
        ClientQuery::CurrentMonth => Query::CurrentMonth,
        ClientQuery::ListColonies => Query::ListColonies,
        ClientQuery::ColonyStatus { colony_id } => Query::ColonyStatus {
            colony_id: parse_colony(&colony_id)?,
        },
        ClientQuery::ColonyScreen { colony_id } => Query::ColonyScreen {
            colony_id: parse_colony(&colony_id)?,
        },
        ClientQuery::SystemResearchTotal => Query::SystemResearchTotal,
        ClientQuery::BuildingDetail {
            colony_id,
            building_type,
        } => Query::BuildingDetail {
            colony_id: parse_colony(&colony_id)?,
            building_type,
        },
    };

    let result = engine.query(&core_query).map_err(CmdError::from)?;
    let value = match result {
        QueryResult::Counter(v) => serde_json::json!({ "kind": "counter", "value": v }),
        QueryResult::Colonies(list) => serde_json::json!({ "kind": "colonies", "colonies": list }),
        QueryResult::ColonyStatus(s) => serde_json::json!({ "kind": "colony_status", "status": s }),
        QueryResult::Labour(l) => serde_json::json!({ "kind": "labour", "labour": l }),
        QueryResult::ResearchTotal(t) => {
            serde_json::json!({ "kind": "research_total", "total": t })
        }
        QueryResult::ColonyScreen(d) => serde_json::json!({ "kind": "colony_screen", "data": d }),
        QueryResult::BuildingDetail(d) => {
            serde_json::json!({ "kind": "building_detail", "data": d })
        }
        other => serde_json::json!({ "kind": "other", "debug": format!("{other:?}") }),
    };
    Ok(value)
}

/// Discard the current engine and return to a pre-bootstrap state.
#[tauri::command]
pub fn reset_engine(engine_state: State<'_, EngineState>) {
    *engine_state.engine.lock().unwrap() = None;
}

/// Persist the current game state to a SQLite file.
#[tauri::command]
pub fn save_game(path: String, engine_state: State<'_, EngineState>) -> CmdResult<()> {
    let guard = engine_state.engine.lock().unwrap();
    let engine = guard.as_ref().ok_or(CmdError::NotInitialised)?;
    let mut db =
        SnapshotDb::open(Path::new(&path)).map_err(|e| CmdError::Snapshot(e.to_string()))?;
    db.save(&engine.state)
        .map_err(|e| CmdError::Snapshot(e.to_string()))
}

/// Load a previously saved game from a SQLite file.
#[tauri::command]
pub fn load_game(path: String, engine_state: State<'_, EngineState>) -> CmdResult<SnapshotPayload> {
    let db = SnapshotDb::open(Path::new(&path)).map_err(|e| CmdError::Snapshot(e.to_string()))?;
    let game_state = db.load().map_err(|e| CmdError::Snapshot(e.to_string()))?;
    let mut engine = GameEngine::new();
    engine.state = game_state;
    let snap = build_snapshot(&engine);
    *engine_state.engine.lock().unwrap() = Some(engine);
    Ok(snap)
}

// ── Custom, high-level queries used by dedicated UI views ────────────────────

/// A body on the system map, positioned for rendering.
#[derive(Debug, Serialize)]
pub struct SystemBodyWire {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub role: String,
    pub distance_au: f32,
    pub colonizable: bool,
    /// Atmospheric thickness/density band (issue #197).
    pub atmosphere_density: String,
    /// Atmospheric chemical hazard band (issue #197).
    pub atmosphere_hazard: String,
    pub temperature: String,
    pub gravity_g: f32,
    pub radiation: String,
    pub habitability: u8,
    pub habitability_modifier: f32,
    /// Habitability score after tech-driven mitigations are applied (issue
    /// #185). Equal to `habitability` when no mitigation applies.
    pub habitability_effective: u8,
    /// Habitability modifier after tech-driven mitigations are applied
    /// (issue #185). Equal to `habitability_modifier` when no mitigation applies.
    pub habitability_modifier_effective: f32,
    /// Surface/composition archetype (issue #196). Flavor/authoring
    /// guidance — not a habitability input.
    pub subtype: String,
    pub tidally_locked: bool,
    pub axial_tilt_deg: f32,
    pub rotation_period_hours: f32,
    pub moon_count: u32,
    /// Display name of the body this one orbits, if any.
    pub parent_body_name: Option<String>,
}

/// Return the current list of system bodies with rendering hints.
#[tauri::command]
pub fn get_system_bodies(engine_state: State<'_, EngineState>) -> CmdResult<Vec<SystemBodyWire>> {
    let guard = engine_state.engine.lock().unwrap();
    let engine = guard.as_ref().ok_or(CmdError::NotInitialised)?;
    let node_bodies = &engine.state.system_state.node_map.bodies;
    let bodies: Vec<SystemBodyWire> = node_bodies
        .values()
        .map(|b| SystemBodyWire {
            id: b.id.0.to_string(),
            name: b.name.clone(),
            kind: format!("{:?}", b.kind),
            role: format!("{:?}", b.role),
            distance_au: b.distance_au,
            colonizable: matches!(
                b.kind,
                BodyKind::InnerPlanet | BodyKind::Moon | BodyKind::AsteroidBelt
            ),
            atmosphere_density: format!("{:?}", b.atmosphere_density),
            atmosphere_hazard: format!("{:?}", b.atmosphere_hazard),
            temperature: format!("{:?}", b.temperature),
            gravity_g: b.gravity_g,
            radiation: format!("{:?}", b.radiation),
            habitability: b.habitability(),
            habitability_modifier: b.habitability_modifier(),
            habitability_effective: b
                .habitability_with_mitigations(&engine.state.habitability_mitigations),
            habitability_modifier_effective: b
                .habitability_modifier_with_mitigations(&engine.state.habitability_mitigations),
            subtype: format!("{:?}", b.subtype),
            tidally_locked: b.tidally_locked,
            axial_tilt_deg: b.axial_tilt_deg,
            rotation_period_hours: b.rotation_period_hours,
            moon_count: b.moon_count,
            parent_body_name: b
                .parent_body
                .as_ref()
                .and_then(|pid| node_bodies.get(pid))
                .map(|p| p.name.clone()),
        })
        .collect();
    Ok(bodies)
}

/// Tech definition + player state (researched / in_progress / available / locked).
#[derive(Debug, Serialize)]
pub struct TechNodeWire {
    pub id: String,
    pub name: String,
    pub category: String,
    pub description: String,
    pub tier: u32,
    pub cost: f32,
    pub prerequisites: Vec<String>,
    pub state: String, // researched | in_progress | available | locked
    pub progress: f32,
}

/// Return the full tech tree with per-node state.
#[tauri::command]
pub fn get_tech_tree(engine_state: State<'_, EngineState>) -> CmdResult<Vec<TechNodeWire>> {
    let guard = engine_state.engine.lock().unwrap();
    let engine = guard.as_ref().ok_or(CmdError::NotInitialised)?;

    let tech_registry = engine
        .state
        .tech_registry
        .as_ref()
        .ok_or_else(|| CmdError::Content("no tech registry loaded".into()))?;

    let tech_state = &engine.state.tech_state;
    let current = tech_state.current_project.as_deref();

    let mut nodes = Vec::new();
    for tech in tech_registry.all() {
        let is_done = tech_state.is_researched(&tech.id);
        let is_active = current == Some(tech.id.as_str());
        let is_queued = tech_state.research_queue.iter().any(|t| t == &tech.id);
        let prereqs_met = tech_state.prerequisites_met(tech);

        let state = if is_done {
            "researched"
        } else if is_active || is_queued {
            "in_progress"
        } else if prereqs_met {
            "available"
        } else {
            "locked"
        };
        let progress = if is_active {
            tech_state.progress / tech.research_cost.max(0.001)
        } else {
            0.0
        };
        nodes.push(TechNodeWire {
            id: tech.id.clone(),
            name: tech.display_name.clone(),
            category: tech.category.clone(),
            description: tech.description.clone(),
            tier: tech.tier,
            cost: tech.research_cost,
            prerequisites: tech.prerequisites.clone(),
            state: state.to_owned(),
            progress,
        });
    }
    nodes.sort_by(|a, b| {
        a.tier
            .cmp(&b.tier)
            .then_with(|| a.category.cmp(&b.category))
            .then_with(|| a.name.cmp(&b.name))
    });
    Ok(nodes)
}

/// Colonizable target for the founding wizard.
#[derive(Debug, Serialize)]
pub struct ColonizeTargetWire {
    pub body_id: String,
    pub body_name: String,
    pub kind: String,
    pub distance_au: f32,
    /// Body habitability score (0-100), issue #183.
    pub habitability: u8,
    /// Whether founding on this body is currently allowed — either the
    /// habitability score clears `HABITABILITY_FOUNDING_THRESHOLD`, or the
    /// player has unlocked `HARSH_WORLD_CAPABILITY_ID`. The wizard still
    /// shows bodies below this so the player understands what's blocking
    /// them (issue #183 UX decision), rather than hiding them outright.
    pub can_found: bool,
}

#[tauri::command]
pub fn get_colonize_targets(
    engine_state: State<'_, EngineState>,
) -> CmdResult<Vec<ColonizeTargetWire>> {
    let guard = engine_state.engine.lock().unwrap();
    let engine = guard.as_ref().ok_or(CmdError::NotInitialised)?;
    let harsh_world_unlocked = engine
        .state
        .unlocked_capabilities
        .contains(outpost_core::system::HARSH_WORLD_CAPABILITY_ID);
    let list = engine
        .state
        .system_state
        .node_map
        .bodies
        .values()
        .filter(|b| {
            matches!(
                b.kind,
                BodyKind::InnerPlanet | BodyKind::Moon | BodyKind::AsteroidBelt
            )
        })
        .map(|b| ColonizeTargetWire {
            body_id: b.id.0.to_string(),
            body_name: b.name.clone(),
            kind: format!("{:?}", b.kind),
            distance_au: b.distance_au,
            habitability: b.habitability(),
            can_found: b.meets_founding_threshold() || harsh_world_unlocked,
        })
        .collect();
    Ok(list)
}

/// A building option in the founding wizard.
///
/// Carries the canonical construction data from `BuildingDef` so the UI
/// never has to invent slot cost / labor / cost / turns.
#[derive(Debug, Serialize)]
pub struct BuildingOptionWire {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub slot_cost: u32,
    pub labor_per_turn: u32,
    pub construction_turns: u32,
    pub construction_cost: Vec<(String, f64)>,
    pub tech_prerequisite: Option<String>,
}

#[tauri::command]
pub fn list_buildings(engine_state: State<'_, EngineState>) -> CmdResult<Vec<BuildingOptionWire>> {
    let guard = engine_state.engine.lock().unwrap();
    let engine = guard.as_ref().ok_or(CmdError::NotInitialised)?;
    let registry = engine
        .state
        .registry
        .as_ref()
        .ok_or_else(|| CmdError::Content("no content registry loaded".into()))?;
    let mut out: Vec<BuildingOptionWire> = registry
        .buildings()
        .map(|b| BuildingOptionWire {
            id: b.id.clone(),
            name: b.name.clone(),
            description: b.description.clone(),
            category: format!("{:?}", b.category),
            slot_cost: b.slot_cost,
            labor_per_turn: b.labor_required,
            construction_turns: b.construction_turns,
            construction_cost: b
                .construction_cost
                .iter()
                .map(|i| (i.id.clone(), i.quantity))
                .collect(),
            tech_prerequisite: b.tech_prerequisite.clone(),
        })
        .collect();
    out.sort_by(|a, b| {
        a.category
            .cmp(&b.category)
            .then_with(|| a.name.cmp(&b.name))
    });
    Ok(out)
}

/// One hex on the planet surface, enriched with UI-relevant fields.
#[derive(Debug, Serialize)]
pub struct PlanetHexWire {
    pub q: i32,
    pub r: i32,
    pub site_id: String,
    pub terrain: String,
    pub biome: String,
    pub elevation: f32,
    pub temperature: String,
    pub deposits: Vec<DepositWire>,
    pub habitable: bool,
    pub suitability: f32,
    pub occupied_by: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct DepositWire {
    pub commodity_id: String,
    pub richness: f32,
}

#[derive(Debug, Serialize)]
pub struct PlanetMapWire {
    pub seed: u64,
    pub radius: u32,
    pub hexes: Vec<PlanetHexWire>,
}

/// A starter supply package option surfaced in the founding wizard.
#[derive(Debug, Serialize)]
pub struct SupplyPackageWire {
    pub id: String,
    pub name: String,
    pub description: String,
    pub commodities: Vec<(String, f64)>,
}

/// Return all `SupplyPackage` records from the loaded content pack, sorted by name.
#[tauri::command]
pub fn list_supply_packages(
    engine_state: State<'_, EngineState>,
) -> CmdResult<Vec<SupplyPackageWire>> {
    let guard = engine_state.engine.lock().unwrap();
    let engine = guard.as_ref().ok_or(CmdError::NotInitialised)?;
    let registry = engine
        .state
        .registry
        .as_ref()
        .ok_or_else(|| CmdError::Content("no content registry loaded".into()))?;
    let mut out: Vec<SupplyPackageWire> = registry
        .supply_packages()
        .map(|p| SupplyPackageWire {
            id: p.id.clone(),
            name: p.name.clone(),
            description: p.description.clone(),
            commodities: p
                .commodities
                .iter()
                .map(|i| (i.id.clone(), i.quantity))
                .collect(),
        })
        .collect();
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

/// Return the current planet hex map with per-cell metadata.
#[tauri::command]
pub fn get_planet_map(engine_state: State<'_, EngineState>) -> CmdResult<PlanetMapWire> {
    let guard = engine_state.engine.lock().unwrap();
    let engine = guard.as_ref().ok_or(CmdError::NotInitialised)?;

    let pm = engine
        .state
        .planet_map
        .as_ref()
        .ok_or_else(|| CmdError::Engine("no planet map — bootstrap first".into()))?;

    // Build reverse coord → site_id lookup so hex rows carry a site_id.
    let coord_to_site: std::collections::HashMap<_, _> =
        pm.sites.iter().map(|(sid, coord)| (*coord, *sid)).collect();

    // Reverse coord → colony_name lookup for the `occupied_by` field.
    let colony_names: std::collections::HashMap<_, _> = engine
        .state
        .colonies
        .iter()
        .map(|c| (c.id, c.name.clone()))
        .collect();
    let coord_to_colony: std::collections::HashMap<_, _> = pm
        .colonies
        .iter()
        .filter_map(|node| {
            colony_names
                .get(&node.colony_id)
                .map(|name| (node.coord, name.clone()))
        })
        .collect();

    let mut hexes: Vec<PlanetHexWire> = pm
        .cells
        .values()
        .map(|cell| {
            let site_id = coord_to_site
                .get(&cell.coord)
                .map(|sid| sid.0.to_string())
                .unwrap_or_default();
            PlanetHexWire {
                q: cell.coord.q,
                r: cell.coord.r,
                site_id,
                terrain: format!("{:?}", cell.terrain),
                biome: format!("{:?}", cell.biome),
                elevation: cell.elevation,
                temperature: format!("{:?}", cell.temperature),
                deposits: cell
                    .deposits
                    .iter()
                    .map(|d| DepositWire {
                        commodity_id: d.commodity_id.clone(),
                        richness: d.richness,
                    })
                    .collect(),
                habitable: cell.is_habitable(),
                suitability: cell.suitability(),
                occupied_by: coord_to_colony.get(&cell.coord).cloned(),
            }
        })
        .collect();

    // Stable ordering so the frontend doesn't churn cell z-order between calls.
    hexes.sort_by_key(|h| (h.r, h.q));

    Ok(PlanetMapWire {
        seed: pm.seed,
        radius: pm.radius,
        hexes,
    })
}

/// List `*.o3save` files in the given directory. Returns filenames only.
#[tauri::command]
pub fn list_saves(dir: String) -> CmdResult<Vec<String>> {
    let path = Path::new(&dir);
    if !path.exists() {
        return Ok(vec![]);
    }
    let mut saves = Vec::new();
    for entry in std::fs::read_dir(path).map_err(|e| CmdError::Snapshot(e.to_string()))? {
        let entry = entry.map_err(|e| CmdError::Snapshot(e.to_string()))?;
        let name = entry.file_name();
        let s = name.to_string_lossy();
        if s.ends_with(".o3save") || s.ends_with(".sqlite") {
            saves.push(s.into_owned());
        }
    }
    saves.sort();
    Ok(saves)
}

// ── #161: difficulty knob discovery + preset persistence ────────────────────

/// Return one [`KnobSpec`] per slider / toggle so the panel is data-driven.
///
/// Slider defaults + current values are read from the engine's current
/// difficulty scalar; toggles read `hazards_enabled` and the presence of an
/// active `menace_state`.
#[tauri::command]
pub fn get_difficulty_knobs(engine_state: State<'_, EngineState>) -> CmdResult<Vec<KnobSpec>> {
    let guard = engine_state.engine.lock().unwrap();
    let engine = guard.as_ref().ok_or(CmdError::NotInitialised)?;
    let state = &engine.state;

    // Reference row from the current preset (or Normal for Custom).
    let ref_preset = if matches!(state.difficulty_preset, DifficultyPreset::Custom) {
        DifficultyPreset::Normal
    } else {
        state.difficulty_preset
    };
    let ref_scalar = state.difficulty_grade_table.build_scalar(ref_preset);

    let mut specs: Vec<KnobSpec> = Vec::new();
    for (id, label, key) in SLIDER_KNOBS {
        let q = key.to_quantity();
        specs.push(KnobSpec {
            id: (*id).to_owned(),
            label: (*label).to_owned(),
            kind: "slider",
            step: SLIDER_STEPS.to_vec(),
            min: SLIDER_STEPS[0],
            max: SLIDER_STEPS[SLIDER_STEPS.len() - 1],
            preset_default_at_current_preset: ref_scalar.scalar_for(&q),
            current_value: state.difficulty_scalar.scalar_for(&q),
        });
    }

    specs.push(KnobSpec {
        id: "menace_enabled".into(),
        label: "Menace Enabled".into(),
        kind: "toggle",
        step: vec![],
        min: 0.0,
        max: 1.0,
        preset_default_at_current_preset: 1.0,
        current_value: if state.menace_state.is_some() {
            1.0
        } else {
            0.0
        },
    });
    specs.push(KnobSpec {
        id: "hazards_enabled".into(),
        label: "Hazards Enabled".into(),
        kind: "toggle",
        step: vec![],
        min: 0.0,
        max: 1.0,
        preset_default_at_current_preset: 1.0,
        current_value: if state.hazards_enabled { 1.0 } else { 0.0 },
    });
    specs.push(KnobSpec {
        id: "maintenance_enabled".into(),
        label: "Maintenance Enabled".into(),
        kind: "toggle",
        step: vec![],
        min: 0.0,
        max: 1.0,
        preset_default_at_current_preset: 1.0,
        current_value: if state.maintenance_enabled { 1.0 } else { 0.0 },
    });
    Ok(specs)
}

/// Enumerate every saved preset in the app-data directory.
#[tauri::command]
pub fn list_custom_presets(app: AppHandle) -> CmdResult<Vec<CustomPreset>> {
    let dir = custom_preset_dir(&app)?;
    let mut out: Vec<CustomPreset> = Vec::new();
    let entries = std::fs::read_dir(&dir).map_err(|e| CmdError::Snapshot(e.to_string()))?;
    for entry in entries {
        let entry = entry.map_err(|e| CmdError::Snapshot(e.to_string()))?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("yaml") {
            continue;
        }
        let text = match std::fs::read_to_string(&path) {
            Ok(t) => t,
            Err(_) => continue,
        };
        if let Ok(preset) = serde_yaml::from_str::<CustomPreset>(&text) {
            out.push(preset);
        }
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

/// Persist a named preset (replaces on collision).
#[tauri::command]
pub fn save_custom_preset(
    app: AppHandle,
    name: String,
    scalars: std::collections::HashMap<String, f32>,
    menace_enabled: bool,
    hazards_enabled: bool,
    maintenance_enabled: Option<bool>,
) -> CmdResult<()> {
    let sanitized = sanitize_preset_name(&name);
    if sanitized.is_empty() {
        return Err(CmdError::InvalidArg("preset name is empty".into()));
    }
    let dir = custom_preset_dir(&app)?;
    let preset = CustomPreset {
        name: sanitized.clone(),
        scalars,
        menace_enabled,
        hazards_enabled,
        maintenance_enabled: maintenance_enabled.unwrap_or(true),
    };
    let text =
        serde_yaml::to_string(&preset).map_err(|e| CmdError::Snapshot(format!("yaml: {e}")))?;
    let path = dir.join(format!("{sanitized}.yaml"));
    std::fs::write(&path, text).map_err(|e| CmdError::Snapshot(e.to_string()))
}

/// Delete a named preset. Missing files are treated as success.
#[tauri::command]
pub fn delete_custom_preset(app: AppHandle, name: String) -> CmdResult<()> {
    let sanitized = sanitize_preset_name(&name);
    let dir = custom_preset_dir(&app)?;
    let path = dir.join(format!("{sanitized}.yaml"));
    if path.exists() {
        std::fs::remove_file(&path).map_err(|e| CmdError::Snapshot(e.to_string()))?;
    }
    Ok(())
}
