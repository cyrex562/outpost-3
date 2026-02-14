use crate::storage::Storage; // bring trait into scope for save/load methods
use bevy::input::keyboard::KeyCode;
use bevy::input::mouse::{MouseButton, MouseMotion, MouseScrollUnit, MouseWheel};
use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts, EguiPlugin};
use egui_plot::{Line, Plot, PlotPoints};
use outpost_core as core;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

mod camera;
use camera::{CameraLimits, CameraState};
mod hex;
pub mod storage;
use hex::{axial_to_world, world_to_axial_round};
mod scenes;
use scenes::{
    about_ui_system, cleanup_scene_entities_system, gameplay_scene_init_system,
    render_resource_categories, settings_ui_system, start_menu_ui_system, AboutSceneDisplayed,
    AppSettings, GamePlaySceneLoaded, MenuButtonClicked, PreviousScene, Scene, SceneChanged,
    SceneEntitiesCleaned, SettingsChanged, StartMenuState, RESOURCE_PANEL_MAX_WIDTH,
};

// --- UI Debug / Verification ---
#[derive(Event, Clone)]
struct UiRectLogged {
    name: String,
    min: [f32; 2],
    max: [f32; 2],
}

#[derive(Resource, Default)]
struct UiDebug {
    enabled: bool,
    // Keep a small rolling buffer of last-reported rects
    last_rects: Vec<(String, [f32; 2], [f32; 2])>,
}

/// Enhanced debug metrics resource for FPS, entity count, system times, memory
#[derive(Resource)]
struct DebugMetrics {
    fps: f64,
    frame_time_ms: f64,
    entity_count: u64,
    visible_entities: u64,
    memory_mb: f64,
    last_update: std::time::Instant,
}

impl Default for DebugMetrics {
    fn default() -> Self {
        Self {
            fps: 0.0,
            frame_time_ms: 0.0,
            entity_count: 0,
            visible_entities: 0,
            memory_mb: 0.0,
            last_update: std::time::Instant::now(),
        }
    }
}

/// Log filtering configuration for debug overlay
/// Task 7.6.6: Filter logs by level and category keywords
#[derive(Resource, Clone)]
struct DebugLogFilter {
    pub show_debug: bool,
    pub show_info: bool,
    pub show_warn: bool,
    pub show_error: bool,
    pub category_filter: String,
}

impl Default for DebugLogFilter {
    fn default() -> Self {
        Self {
            show_debug: true,
            show_info: true,
            show_warn: true,
            show_error: true,
            category_filter: String::new(),
        }
    }
}

#[derive(Resource, Default)]
struct RailInspector {
    enabled: bool,
    selected_segment: Option<usize>,
    selected_train: Option<usize>,
}

#[derive(Resource, Default)]
struct SandboxMode {
    enabled: bool,
}

// Optional client asset handles (fonts/sprites). These are best-effort: if files
// are missing, the app continues with defaults.
#[derive(Resource, Default)]
struct ClientAssets {
    font_applied: bool,
    terrain_sprite: Option<Handle<Image>>,
    buildings_sprite: Option<Handle<Image>>,
    status: String,
}

fn setup_client_assets(
    mut contexts: EguiContexts,
    asset_server: Res<AssetServer>,
    mut commands: Commands,
) {
    let mut assets = ClientAssets::default();

    // Try to load and apply a custom egui font from assets/fonts on desktop builds
    // WASM will skip direct filesystem access.
    #[cfg(feature = "desktop")]
    {
        let font_path = std::path::Path::new("assets/fonts/Inter-Regular.ttf");
        if let Ok(bytes) = std::fs::read(font_path) {
            let mut fonts = egui::FontDefinitions::default();
            use egui::FontFamily;
            // Register the custom font
            fonts.font_data.insert(
                "inter-regular".to_owned(),
                egui::FontData::from_owned(bytes),
            );
            // Put it higher priority for proportional and monospace
            fonts
                .families
                .entry(FontFamily::Proportional)
                .or_default()
                .insert(0, "inter-regular".to_owned());
            fonts
                .families
                .entry(FontFamily::Monospace)
                .or_default()
                .insert(0, "inter-regular".to_owned());
            contexts.ctx_mut().set_fonts(fonts);
            assets.font_applied = true;
            assets.status.push_str("font Inter-Regular applied; ");
        } else {
            assets.status.push_str("default font; ");
        }
    }

    // Load placeholder sprites via AssetServer (will be a pending handle if missing)
    let terrain: Handle<Image> = asset_server.load("sprites/terrain_placeholder.png");
    let buildings: Handle<Image> = asset_server.load("sprites/buildings_placeholder.png");
    assets.terrain_sprite = Some(terrain);
    assets.buildings_sprite = Some(buildings);
    assets.status.push_str("placeholder sprites queued");

    commands.insert_resource(assets);
}

#[derive(Component)]
struct MainCamera;

#[derive(Resource, Default)]
struct Dragging(bool);

// --- Build Mode / Placement ---
#[derive(Resource, Default)]
struct BuildMode {
    active: bool,
    building_type: String, // simple string for now (e.g., "Solar")
}

// Tracks client-side placed buildings until core modeling is expanded
#[derive(Clone)]
struct ClientBuilding {
    kind: String,
    paused: bool,
}

impl ClientBuilding {
    fn new(kind: String) -> Self {
        Self {
            kind,
            paused: false,
        }
    }
}

#[derive(Resource, Default)]
struct PlacedBuildings(pub HashMap<(i32, i32), ClientBuilding>);

// Simple modal state controller
#[derive(Resource, Default)]
struct ModalState {
    show_construction: bool,
    show_building_detail: bool,
    show_power_chart: bool,
    show_resources_chart: bool,
    show_population_chart: bool,
    selected_recipe_index: u32, // Track selected recipe (0-based index)
}

// Simple visibility culling configuration
#[derive(Resource)]
struct CullingConfig {
    enabled: bool,
    margin_world: f32, // expand the view rect by this many world units to avoid popping
}

impl Default for CullingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            margin_world: 200.0,
        }
    }
}

// Timer resource that triggers process exit after a duration (for CI smoke)
#[derive(Resource)]
struct SmokeExitTimer(Timer);

fn smoke_exit_system(time: Res<Time>, mut timer: Option<ResMut<SmokeExitTimer>>) {
    if let Some(mut t) = timer {
        t.0.tick(time.delta());
        if t.0.finished() {
            info!("SMOKE: exiting app after timer elapsed");
            // On native targets, exit process. On wasm, request app exit.
            #[cfg(not(target_arch = "wasm32"))]
            {
                std::process::exit(0);
            }
            #[cfg(target_arch = "wasm32")]
            {
                // For wasm, we can't exit the process, but we can panic to signal completion
                // or simply log. We'll just panic to break the loop in tests.
                // Note: this code path shouldn't be used by CI wasm step.
                panic!("SMOKE EXIT");
            }
        }
    }
}

// Save/Load manager (desktop JSON now; wasm IndexedDB later)
#[derive(Resource)]
struct StorageManager {
    autosave_every: u64,
    last_saved_turn: u64,
    profile_id: String,
    status: String,
    store: storage::SelectedStorage,
}

impl StorageManager {
    fn new_default() -> Self {
        let store = storage::new_default_storage();
        match store {
            Ok(store) => Self {
                autosave_every: 5,
                last_saved_turn: 0,
                profile_id: "default".into(),
                status: String::from("storage ok"),
                store,
            },
            Err(e) => {
                // Fallback: this will keep autosave disabled effectively
                // by leaving a dummy store unreachable on wasm (but SelectedStorage exists).
                // However, in desktop this should not fail under normal circumstances.
                let store = storage::new_default_storage()
                    .ok()
                    .expect("storage backend must be constructible");
                Self {
                    autosave_every: 0,
                    last_saved_turn: 0,
                    profile_id: "default".into(),
                    status: format!("storage error: {}", e),
                    store,
                }
            }
        }
    }

    fn try_autosave(&mut self, state: &core::GameState) {
        if self.autosave_every == 0 {
            return;
        }
        if state.turn == 0 {
            return;
        }
        if state.turn % self.autosave_every != 0 {
            return;
        }
        // Avoid double-saving the same turn
        if self.last_saved_turn == state.turn {
            return;
        }
        match self.store.save(&self.profile_id, state) {
            Ok(_) => {
                self.last_saved_turn = state.turn;
                self.status = format!(
                    "autosaved at turn {} (profile '{}')",
                    state.turn, self.profile_id
                );
            }
            Err(e) => {
                self.status = format!("autosave failed: {}", e);
            }
        }
    }
}

fn main() {
    // Build a single app, with window configuration depending on target.
    let mut app = App::new();

    // Common resources
    app.insert_resource(ClearColor(Color::rgb(0.05, 0.05, 0.08)))
        .insert_resource(Dragging::default())
        .insert_resource(CameraState::new(CameraLimits::new(
            -2000.0..=2000.0,
            -2000.0..=2000.0,
            0.25..=4.0,
        )));

    // Configure window differently for wasm vs desktop
    #[cfg(target_arch = "wasm32")]
    let primary = Window {
        title: "Outpost 3 (Desktop/WASM)".into(),
        canvas: Some("#outpost-canvas".into()),
        fit_canvas_to_parent: true,
        ..Default::default()
    };
    #[cfg(not(target_arch = "wasm32"))]
    let primary = Window {
        title: "Outpost 3 (Desktop/WASM)".into(),
        resolution: bevy::window::WindowResolution::new(1920.0, 1080.0),
        ..Default::default()
    };

    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(primary),
        ..Default::default()
    }))
    .add_plugins(EguiPlugin)
    .add_event::<UiRectLogged>()
    .add_event::<SceneChanged>()
    .add_event::<MenuButtonClicked>()
    .add_event::<SceneEntitiesCleaned>()
    .add_event::<GamePlaySceneLoaded>()
    .add_event::<SettingsChanged>()
    .add_event::<AboutSceneDisplayed>()
    .insert_resource(Scene::default())
    .insert_resource(StartMenuState::default())
    .insert_resource(PreviousScene::default())
    .insert_resource(AppSettings::default())
    .insert_resource(Game::default())
    .insert_resource(HexHover::default())
    .insert_resource(HexSelected::default())
    .insert_resource(GridConfig::default())
    .insert_resource(CullingConfig::default())
    .insert_resource(BuildMode {
        active: false,
        building_type: "Solar".into(),
    })
    .insert_resource(PlacedBuildings::default())
    .insert_resource(ModalState::default())
    .insert_resource(StorageManager::new_default())
    .insert_resource(ClientAssets::default())
    .insert_resource(UiDebug::default())
    .insert_resource(DebugMetrics::default())
    .insert_resource(DebugLogFilter::default())
    .insert_resource(RailInspector::default())
    .insert_resource(SandboxMode::default())
    .insert_resource(SegmentBottlenecks::default())
    .add_systems(Startup, setup_camera)
    .add_systems(Startup, setup_hex_grid)
    .add_systems(Startup, setup_rail_segments)
    .add_systems(Startup, setup_trains)
    .add_systems(Startup, setup_client_assets)
    .add_systems(
        Update,
        (
            scenes::handle_scene_transitions,
            cleanup_scene_entities_system,
            gameplay_scene_init_system,
            start_menu_ui_system,
            settings_ui_system,
            about_ui_system,
            ui_panel,
            update_debug_metrics_system,
            ui_debug_overlay_system,
            ui_debug_toggle_system,
            rail_inspector_toggle_system,
            rail_inspector_ui_system,
            sandbox_toggle_system,
            ui_rect_event_logger_system,
            advance_turn_shortcut_system,
            build_shortcuts_system,
        ),
    )
    .add_systems(
        Update,
        (
            power_chart_window_system,
            resources_chart_window_system,
            population_chart_window_system,
            keyboard_pan_system,
            mouse_pan_system,
            mouse_wheel_zoom_system,
            apply_camera_state_system,
        ),
    )
    .add_systems(
        Update,
        (
            visibility_culling_system,
            rail_label_ui_system,
            train_badge_ui_system,
            cargo_animation_system,
            train_motion_system,
            update_hex_hover_system,
            handle_click_select_or_place_system,
            update_hex_highlight_colors_system,
            construction_modal_window_system,
            building_detail_modal_window_system,
            building_detail_post_ui_apply_intents_system,
        ),
    );

    // Optional CI smoke: exit automatically after N seconds if env var is set
    if let Ok(secs_str) = std::env::var("OUTPOST_SMOKE_SECONDS") {
        if let Ok(secs) = secs_str.parse::<f32>() {
            app.insert_resource(SmokeExitTimer(Timer::from_seconds(
                secs.max(0.01),
                TimerMode::Once,
            )))
            .add_systems(Update, smoke_exit_system);
            info!("OUTPOST_SMOKE_ENABLED secs={}", secs);
        }
    }

    app.run();
}

#[derive(Resource)]
pub struct Game {
    pub state: core::GameState,
    // keep a short rolling history for charts
    pub history: std::collections::VecDeque<core::GameState>,
    pub history_max: usize,
}

impl Default for Game {
    fn default() -> Self {
        let mut history = std::collections::VecDeque::with_capacity(128);
        let s = core::GameState::default();
        history.push_back(s.clone());
        Self {
            state: s,
            history,
            history_max: 128,
        }
    }
}

fn setup_camera(mut commands: Commands) {
    commands.spawn((Camera2dBundle::default(), MainCamera));
}

fn ui_panel(
    mut contexts: EguiContexts,
    scene: Res<Scene>,
    mut game: ResMut<Game>,
    hover: Res<HexHover>,
    mut build: ResMut<BuildMode>,
    mut modals: ResMut<ModalState>,
    mut storage_mgr: ResMut<StorageManager>,
    client_assets: Res<ClientAssets>,
    mut culling: ResMut<CullingConfig>,
    mut ui_debug: ResMut<UiDebug>,
    mut ui_rect_ev: EventWriter<UiRectLogged>,
    mut sandbox: ResMut<SandboxMode>,
) {
    // Only show gameplay UI when in GamePlay scene
    if *scene != Scene::GamePlay {
        return;
    }

    let ctx = contexts.ctx_mut();

    // Top resource bar
    let top_resp = egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.label("Outpost 3 — Desktop/WASM");
            ui.separator();
            if client_assets.font_applied {
                ui.small("Font: Inter");
            } else {
                ui.small("Font: default");
            }
            ui.separator();
            ui.monospace(format!("Turn: {}", game.state.turn));
            let power = core::get_power_snapshot(&game.state);
            ui.separator();
            ui.label(format!(
                "Power gen:{} cons:{}",
                power.generation, power.consumption
            ));
            let pop = core::get_population_snapshot(&game.state);
            ui.separator();
            ui.label(format!("Pop total:{} employed:{}", pop.total, pop.employed));
            ui.separator();
            ui.label(format!("Resources:{}", game.state.resources_stock));
            ui.separator();
            if ui.button("Power Chart").clicked() {
                modals.show_power_chart = true;
            }
            if ui.button("Resources Chart").clicked() {
                modals.show_resources_chart = true;
            }
            if ui.button("Population Chart").clicked() {
                modals.show_population_chart = true;
            }
            ui.separator();
            ui.label("Layers:");
            let power_on = core::is_layer_enabled(&game.state, core::Layer::Power);
            let res_on = core::is_layer_enabled(&game.state, core::Layer::Resources);
            let pol_on = core::is_layer_enabled(&game.state, core::Layer::Pollution);
            let power_txt = if power_on { "Power:on" } else { "Power:off" };
            if ui.button(power_txt).clicked() {
                if let Ok(new_state) = core::exec_and_apply(
                    &game.state,
                    core::Command::ToggleLayer {
                        layer: core::Layer::Power,
                    },
                ) {
                    game.state = new_state;
                }
            }
            let res_txt = if res_on {
                "Resources:on"
            } else {
                "Resources:off"
            };
            if ui.button(res_txt).clicked() {
                if let Ok(new_state) = core::exec_and_apply(
                    &game.state,
                    core::Command::ToggleLayer {
                        layer: core::Layer::Resources,
                    },
                ) {
                    game.state = new_state;
                }
            }
            let pol_txt = if pol_on {
                "Pollution:on"
            } else {
                "Pollution:off"
            };
            if ui.button(pol_txt).clicked() {
                if let Ok(new_state) = core::exec_and_apply(
                    &game.state,
                    core::Command::ToggleLayer {
                        layer: core::Layer::Pollution,
                    },
                ) {
                    game.state = new_state;
                }
            }
            ui.separator();
            ui.label("Profile:");
            let mut pid = storage_mgr.profile_id.clone();
            ui.text_edit_singleline(&mut pid);
            if pid != storage_mgr.profile_id {
                storage_mgr.profile_id = pid;
            }
            if ui.button("Save").clicked() {
                match storage_mgr.store.save(&storage_mgr.profile_id, &game.state) {
                    Ok(_) => {
                        storage_mgr.last_saved_turn = game.state.turn;
                        storage_mgr.status = format!(
                            "saved '{}' at turn {}",
                            storage_mgr.profile_id, game.state.turn
                        );
                    }
                    Err(e) => storage_mgr.status = format!("save failed: {}", e),
                }
            }
            if ui.button("Load").clicked() {
                match storage_mgr.store.load(&storage_mgr.profile_id) {
                    Ok(loaded) => {
                        game.state = loaded.clone();
                        game.history.push_back(loaded);
                        while game.history.len() > game.history_max {
                            game.history.pop_front();
                        }
                        storage_mgr.status = format!(
                            "loaded '{}' (turn {})",
                            storage_mgr.profile_id, game.state.turn
                        );
                    }
                    Err(e) => storage_mgr.status = format!("load failed: {}", e),
                }
            }
            ui.separator();
            ui.small(format!(
                "{} | autosave:{}T",
                storage_mgr.status, storage_mgr.autosave_every
            ));
            ui.separator();
            // Simple toggle for visibility culling and a compact margin control
            let cull_txt = if culling.enabled {
                "Culling:on"
            } else {
                "Culling:off"
            };
            if ui
                .button(cull_txt)
                .on_hover_text("Toggle visibility culling")
                .clicked()
            {
                culling.enabled = !culling.enabled;
            }
            let mut margin = culling.margin_world;
            ui.add(
                egui::DragValue::new(&mut margin)
                    .speed(5.0)
                    .clamp_range(0.0..=1000.0)
                    .prefix("margin:"),
            );
            if (margin - culling.margin_world).abs() > f32::EPSILON {
                culling.margin_world = margin.max(0.0);
            }
        });
    });
    if ui_debug.enabled {
        let rect = top_resp.response.rect;
        ui_rect_ev.send(UiRectLogged {
            name: "top_bar".into(),
            min: [rect.min.x, rect.min.y],
            max: [rect.max.x, rect.max.y],
        });
        push_last_rect(&mut ui_debug, "top_bar", rect);
    }

    // Left sidebar: buildings/selection placeholder and build controls
    let left_resp = egui::SidePanel::left("left_sidebar")
        .resizable(true)
        .default_width(260.0)
        .show(ctx, |ui| {
            ui.heading("Sidebar");
            if let Some((q, r)) = hover.0 {
                ui.label(format!("Hover: q={}, r={}", q, r));
                if build.active {
                    let valid = core::is_hex_in_bounds(q, r);
                    ui.label(if valid {
                        "Placement: valid"
                    } else {
                        "Placement: out of bounds"
                    });
                }
            } else {
                ui.label("Hover a hex to see details");
            }
            ui.separator();
            ui.label("Buildings (placeholder):");
            ui.small("No buildings stored yet — scaffolding phase");
            ui.separator();
            ui.horizontal(|ui| {
                let toggle_txt = if build.active {
                    "Exit Build (B/Esc)"
                } else {
                    "Enter Build (B)"
                };
                if ui.button(toggle_txt).clicked() {
                    build.active = !build.active;
                    if build.active {
                        modals.show_construction = true;
                    }
                }
                ui.label("Type:");
                let mut bt = build.building_type.clone();
                egui::ComboBox::from_label("")
                    .selected_text(&bt)
                    .show_ui(ui, |ui| {
                        for choice in ["Solar", "Wind", "Hab"].iter() {
                            ui.selectable_value(&mut bt, (*choice).to_string(), *choice);
                        }
                    });
                build.building_type = bt;
            });
            if ui.button("Open Construction (B)").clicked() {
                build.active = true;
                modals.show_construction = true;
            }

            // Layer toggles
            ui.separator();
            ui.label("Layers:");
            let mut power = core::is_layer_enabled(&game.state, core::Layer::Power);
            let mut resources = core::is_layer_enabled(&game.state, core::Layer::Resources);
            let mut pollution = core::is_layer_enabled(&game.state, core::Layer::Pollution);
            if ui.checkbox(&mut power, "Power").changed() {
                if let Ok(s) = core::exec_and_apply(
                    &game.state,
                    core::Command::ToggleLayer {
                        layer: core::Layer::Power,
                    },
                ) {
                    game.state = s;
                }
            }
            if ui.checkbox(&mut resources, "Resources").changed() {
                if let Ok(s) = core::exec_and_apply(
                    &game.state,
                    core::Command::ToggleLayer {
                        layer: core::Layer::Resources,
                    },
                ) {
                    game.state = s;
                }
            }
            if ui.checkbox(&mut pollution, "Pollution").changed() {
                if let Ok(s) = core::exec_and_apply(
                    &game.state,
                    core::Command::ToggleLayer {
                        layer: core::Layer::Pollution,
                    },
                ) {
                    game.state = s;
                }
            }
            ui.separator();
            ui.heading("Resources");
            let max_width = ui.available_width().min(RESOURCE_PANEL_MAX_WIDTH);
            render_resource_categories(ui, game.state.resources_stock, max_width);
        });
    if ui_debug.enabled {
        let rect = left_resp.response.rect;
        ui_rect_ev.send(UiRectLogged {
            name: "left_sidebar".into(),
            min: [rect.min.x, rect.min.y],
            max: [rect.max.x, rect.max.y],
        });
        push_last_rect(&mut ui_debug, "left_sidebar", rect);
    }

    // Bottom bar: notifications and advance turn
    let bottom_resp = egui::TopBottomPanel::bottom("bottom_bar").show(ctx, |ui| {
        ui.horizontal(|ui| {
            if ui.button("Advance Turn (Space)").clicked() {
                let new_state = core::exec_and_apply(&game.state, core::Command::AdvanceTurn)
                    .expect("advance turn should succeed");
                let snapshot = new_state.clone();
                game.state = new_state;
                // push new snapshot to history
                game.history.push_back(snapshot);
                while game.history.len() > game.history_max {
                    game.history.pop_front();
                }
                // attempt autosave
                storage_mgr.try_autosave(&game.state);
            }
            ui.separator();
            ui.label("Notifications:");
            ui.small("(placeholder)");
            ui.separator();
            let button_label = if sandbox.enabled {
                "🔧 Sandbox (Active)"
            } else {
                "Sandbox"
            };
            if ui.button(button_label).clicked() {
                sandbox.enabled = !sandbox.enabled;
                if sandbox.enabled {
                    game.state.resources_stock = 10000;
                    info!("Sandbox mode enabled - resources set to 10000");
                } else {
                    info!("Sandbox mode disabled");
                }
            }
            if sandbox.enabled && ui.button("Reset Sandbox").clicked() {
                // Reset to initial state
                game.state = core::GameState::default();
                game.history.clear();
                info!("Sandbox reset to initial state");
            }
        });
    });
    if ui_debug.enabled {
        let rect = bottom_resp.response.rect;
        ui_rect_ev.send(UiRectLogged {
            name: "bottom_bar".into(),
            min: [rect.min.x, rect.min.y],
            max: [rect.max.x, rect.max.y],
        });
        push_last_rect(&mut ui_debug, "bottom_bar", rect);
    }
}

fn push_last_rect(ui_debug: &mut UiDebug, name: &str, rect: egui::Rect) {
    // Keep up to 16 entries; replace existing if same name
    if let Some(slot) = ui_debug.last_rects.iter_mut().find(|(n, _, _)| n == name) {
        *slot = (
            name.to_string(),
            [rect.min.x, rect.min.y],
            [rect.max.x, rect.max.y],
        );
        return;
    }
    if ui_debug.last_rects.len() >= 16 {
        ui_debug.last_rects.remove(0);
    }
    ui_debug.last_rects.push((
        name.to_string(),
        [rect.min.x, rect.min.y],
        [rect.max.x, rect.max.y],
    ));
}

/// Update debug metrics (FPS, entity count, memory, etc.)
/// Task 7.6.4: Enhanced debug metrics collection with < 1ms overhead
fn update_debug_metrics_system(
    mut metrics: ResMut<DebugMetrics>,
    time: Res<Time>,
    entities: Query<Entity>,
    visible_entities: Query<&Visibility>,
) {
    // Update at most once per 100ms to minimize overhead
    let now = std::time::Instant::now();
    if now.duration_since(metrics.last_update).as_millis() < 100 {
        return;
    }
    metrics.last_update = now;

    // Calculate FPS from delta time
    let delta = time.delta_seconds();
    if delta > 0.0 {
        metrics.fps = 1.0 / delta as f64;
        metrics.frame_time_ms = delta as f64 * 1000.0;
    }

    // Count entities
    metrics.entity_count = entities.iter().count() as u64;

    // Count visible entities
    metrics.visible_entities = visible_entities
        .iter()
        .filter(|v| **v == Visibility::Visible)
        .count() as u64;

    // Memory estimation (platform-specific, fallback to 0.0)
    #[cfg(target_os = "windows")]
    {
        use windows::Win32::System::ProcessStatus::*;
        use windows::Win32::System::Threading::GetCurrentProcess;
        unsafe {
            let mut pmc: PROCESS_MEMORY_COUNTERS = std::mem::zeroed();
            pmc.cb = std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32;
            if GetProcessMemoryInfo(
                GetCurrentProcess(),
                &mut pmc,
                std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32,
            )
            .is_ok()
            {
                metrics.memory_mb = pmc.WorkingSetSize as f64 / 1024.0 / 1024.0;
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        // For non-Windows platforms, use a placeholder or system-specific API
        // This keeps overhead minimal by not attempting cross-platform memory tracking
        metrics.memory_mb = 0.0;
    }
}

fn ui_debug_toggle_system(keys: Res<ButtonInput<KeyCode>>, mut ui_debug: ResMut<UiDebug>) {
    if keys.just_pressed(KeyCode::F10) {
        ui_debug.enabled = !ui_debug.enabled;
        // Task 7.6.4: Structured logging for debug overlay toggle
        info!(enabled = ui_debug.enabled, "Debug overlay toggled");
    }
}

fn rail_inspector_toggle_system(
    keys: Res<ButtonInput<KeyCode>>,
    mut inspector: ResMut<RailInspector>,
) {
    if keys.just_pressed(KeyCode::F11) {
        inspector.enabled = !inspector.enabled;
        info!(enabled = inspector.enabled, "Rail inspector toggled");
    }
}

fn rail_inspector_ui_system(
    mut contexts: EguiContexts,
    inspector: Res<RailInspector>,
    network: Option<Res<RailNetworkVis>>,
    trains: Query<&TrainProgress>,
) {
    if !inspector.enabled {
        return;
    }

    let ctx = contexts.ctx_mut();
    let mut open = inspector.enabled;
    egui::Window::new("Rail Inspector (F11)")
        .open(&mut open)
        .default_width(400.0)
        .show(ctx, |ui| {
            ui.heading("Rail Network");
            if let Some(net) = network.as_ref() {
                ui.label(format!("Total segments: {}", net.segments.len()));
                ui.label(format!("Connected hubs: {}", net.adjacency.len()));
                ui.separator();

                ui.heading("Segments");
                for (idx, seg) in net.segments.iter().enumerate() {
                    let selected = inspector.selected_segment == Some(idx);
                    if ui
                        .selectable_label(
                            selected,
                            format!(
                                "Seg {}: ({}, {}) → ({}, {}) [{:?}×{}]",
                                idx,
                                seg.from.0,
                                seg.from.1,
                                seg.to.0,
                                seg.to.1,
                                seg.rail_type,
                                seg.track_count
                            ),
                        )
                        .clicked()
                    {
                        // Note: selection is read-only for now; future: click to inspect
                    }
                }

                ui.separator();
                ui.heading("Trains");
                for (idx, prog) in trains.iter().enumerate() {
                    let selected = inspector.selected_train == Some(idx);
                    let status_str = match prog.status {
                        TrainStatusVis::Moving => "Moving",
                        TrainStatusVis::Loading => "Loading",
                    };
                    if ui
                        .selectable_label(
                            selected,
                            format!(
                                "Train {}: segment {} at t={:.2} [{}]",
                                idx, prog.segment_idx, prog.t, status_str
                            ),
                        )
                        .clicked()
                    {
                        // Note: selection is read-only for now; future: click to inspect
                    }
                }

                ui.separator();
                ui.heading("Adjacency Graph");
                ui.label(format!("Hubs with connections: {}", net.adjacency.len()));
                for ((q, r), seg_indices) in net.adjacency.iter().take(5) {
                    ui.small(format!("({}, {}): {} segments", q, r, seg_indices.len()));
                }
                if net.adjacency.len() > 5 {
                    ui.small(format!("... and {} more hubs", net.adjacency.len() - 5));
                }
            } else {
                ui.label("No rail network loaded");
            }
        });
}

fn sandbox_toggle_system(
    keys: Res<ButtonInput<KeyCode>>,
    mut sandbox: ResMut<SandboxMode>,
    mut game: ResMut<Game>,
) {
    // Toggle sandbox mode if S key is pressed (alternative to button)
    if keys.just_pressed(KeyCode::KeyS) {
        sandbox.enabled = !sandbox.enabled;
        if sandbox.enabled {
            // Grant free resources when entering sandbox
            game.state.resources_stock = 10000;
            info!("Sandbox mode enabled - resources set to 10000");
        } else {
            info!("Sandbox mode disabled");
        }
    }
}

fn ui_rect_event_logger_system(mut ev: EventReader<UiRectLogged>) {
    for e in ev.read() {
        info!(
            "UI rect: {} min=({:.1},{:.1}) max=({:.1},{:.1})",
            e.name, e.min[0], e.min[1], e.max[0], e.max[1]
        );
    }
}

fn ui_debug_overlay_system(
    mut contexts: EguiContexts,
    ui_debug: Res<UiDebug>,
    metrics: Res<DebugMetrics>,
    mut log_filter: ResMut<DebugLogFilter>,
    time: Res<Time>,
) {
    if !ui_debug.enabled {
        return;
    }
    let ctx = contexts.ctx_mut();

    // Main debug window with comprehensive metrics
    egui::Window::new("Debug Overlay")
        .anchor(egui::Align2::RIGHT_TOP, egui::vec2(-10.0, 10.0))
        .default_width(380.0)
        .collapsible(true)
        .show(ctx, |ui| {
            ui.label("Press F10 to toggle debug overlay");
            ui.separator();

            // Performance metrics
            ui.heading("Performance");
            ui.monospace(format!("FPS: {:.1}", metrics.fps));
            ui.monospace(format!("Frame Time: {:.2} ms", metrics.frame_time_ms));
            ui.monospace(format!("Delta Time: {:.4} s", time.delta_seconds()));
            ui.separator();

            // Entity counts
            ui.heading("Entities");
            ui.monospace(format!("Total: {}", metrics.entity_count));
            ui.monospace(format!("Visible: {}", metrics.visible_entities));
            ui.separator();

            // Memory usage
            ui.heading("Memory");
            ui.monospace(format!("Usage: {:.1} MB", metrics.memory_mb));
            ui.separator();

            // Log Filtering UI
            // Task 7.6.6: Filter UI for log levels and categories
            ui.heading("Log Filters");
            ui.label("Filter logs by level:");
            ui.horizontal(|ui| {
                ui.checkbox(&mut log_filter.show_debug, "Debug");
                ui.checkbox(&mut log_filter.show_info, "Info");
                ui.checkbox(&mut log_filter.show_warn, "Warn");
                ui.checkbox(&mut log_filter.show_error, "Error");
            });

            ui.label("Filter by category (text search):");
            ui.text_edit_singleline(&mut log_filter.category_filter);
            if !log_filter.category_filter.is_empty() {
                ui.small(format!(
                    "Showing logs containing: \"{}\"",
                    log_filter.category_filter
                ));
            }
            ui.separator();

            // UI Rects tracking
            ui.heading("UI Rects");
            ui.small("Last known screen rects of key UI panels:");
            for (name, min, max) in ui_debug.last_rects.iter() {
                ui.monospace(format!(
                    "{}: min=({:.0},{:.0}) max=({:.0},{:.0})",
                    name, min[0], min[1], max[0], max[1]
                ));
            }
        });
}

fn keyboard_pan_system(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<CameraState>,
) {
    let mut ax = 0.0f32;
    let mut ay = 0.0f32;
    if keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::ArrowLeft) {
        ax -= 1.0;
    }
    if keys.pressed(KeyCode::KeyD) || keys.pressed(KeyCode::ArrowRight) {
        ax += 1.0;
    }
    if keys.pressed(KeyCode::KeyW) || keys.pressed(KeyCode::ArrowUp) {
        ay += 1.0;
    }
    if keys.pressed(KeyCode::KeyS) || keys.pressed(KeyCode::ArrowDown) {
        ay -= 1.0;
    }
    if ax != 0.0 || ay != 0.0 {
        *state = state.apply_keyboard(ax, ay, time.delta_seconds());
    }
    // Shortcuts
    if keys.just_pressed(KeyCode::Space) {
        // Advance turn via command
        // Note: We cannot access Game here; handled in a separate system for simplicity
    }
}

fn mouse_pan_system(
    mut drag: ResMut<Dragging>,
    buttons: Res<ButtonInput<MouseButton>>,
    mut motions: EventReader<MouseMotion>,
    mut state: ResMut<CameraState>,
    q_win: Query<&Window, With<bevy::window::PrimaryWindow>>,
) {
    drag.0 = buttons.pressed(MouseButton::Right) || buttons.pressed(MouseButton::Middle);
    if !drag.0 {
        return;
    }
    let mut dx = 0.0f32;
    let mut dy = 0.0f32;
    for ev in motions.read() {
        dx += ev.delta.x;
        dy += ev.delta.y;
    }
    // Normalize by DPI/scale factor so web canvas and desktop feel similar
    if let Ok(win) = q_win.get_single() {
        let scale = win.scale_factor().max(1.0);
        dx /= scale as f32;
        dy /= scale as f32;
    }
    if dx != 0.0 || dy != 0.0 {
        *state = state.apply_mouse_drag(dx, dy, 1.5);
    }
}

fn mouse_wheel_zoom_system(mut wheel: EventReader<MouseWheel>, mut state: ResMut<CameraState>) {
    let mut total = 0.0f32;
    for ev in wheel.read() {
        // Normalize units: treat 1 line as baseline, approximate pixels as lines
        let mut delta = ev.y as f32;
        match ev.unit {
            MouseScrollUnit::Line => { /* use as-is */ }
            MouseScrollUnit::Pixel => {
                // Heuristic: ~100 px per line for consistent feel across platforms
                delta /= 100.0;
            }
        }
        total += delta;
    }
    if total != 0.0 {
        *state = state.apply_wheel(total);
    }
}

fn apply_camera_state_system(
    state: Res<CameraState>,
    mut q_cam: Query<(&mut Transform, &mut OrthographicProjection), With<MainCamera>>,
) {
    if let Ok((mut transform, mut ortho)) = q_cam.get_single_mut() {
        transform.translation.x = state.x;
        transform.translation.y = state.y;
        ortho.scale = state.zoom.max(0.0001);
    }
}

// --- Hex Grid Rendering & Interaction ---

#[derive(Component, Copy, Clone)]
struct HexTile {
    q: i32,
    r: i32,
}

#[derive(Component)]
struct BuildingSprite;

#[derive(Resource, Default)]
struct HexHover(pub Option<(i32, i32)>);

#[derive(Resource, Default)]
struct HexSelected(pub Option<(i32, i32)>);

#[derive(Resource)]
struct GridConfig {
    radius: f32, // hex radius in world units
    q_range: std::ops::Range<i32>,
    r_range: std::ops::Range<i32>,
}

impl Default for GridConfig {
    fn default() -> Self {
        Self {
            radius: 20.0,
            q_range: 0..50,
            r_range: 0..50,
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum RailTypeVis {
    Standard,
    Advanced,
    Monorail,
    Maglev,
}

#[derive(Debug, Clone)]
struct RailSegmentDef {
    from: (i32, i32),
    to: (i32, i32),
    rail_type: RailTypeVis,
    track_count: u8,
}

#[derive(Component, Debug, Clone, Copy)]
struct RailSegmentVisual {
    midpoint: Vec2,
    rail_type: RailTypeVis,
    track_count: u8,
}

#[derive(Resource, Default)]
struct RailNetworkVis {
    segments: Vec<RailSegmentDef>,
    adjacency: HashMap<(i32, i32), Vec<usize>>, // tile -> segment indices
}

#[derive(Debug, Clone, Deserialize)]
struct PersistedRailSegment {
    from: [i32; 2],
    to: [i32; 2],
    rail_type: String,
    track_count: u8,
}

#[derive(Debug, Clone, Deserialize)]
struct PersistedRailNetwork {
    segments: Vec<PersistedRailSegment>,
}

#[derive(Debug, Clone, Copy)]
enum TrainStatusVis {
    Moving,
    Loading,
}

#[derive(Component)]
struct TrainIndicator;

#[derive(Component)]
struct TrainProgress {
    segment_idx: usize,
    t: f32,
    speed: f32,
    status: TrainStatusVis,
}

#[derive(Component)]
struct CargoParticle {
    segment_idx: usize,
    t: f32,
    speed: f32,
    resource_type: String,
}

#[derive(Resource, Default)]
struct SegmentBottlenecks {
    congestion: HashMap<usize, f32>, // segment_idx -> congestion 0..1
}

fn rail_type_from_str(s: &str) -> Option<RailTypeVis> {
    match s.to_ascii_lowercase().as_str() {
        "standard" => Some(RailTypeVis::Standard),
        "advanced" => Some(RailTypeVis::Advanced),
        "monorail" => Some(RailTypeVis::Monorail),
        "maglev" => Some(RailTypeVis::Maglev),
        _ => None,
    }
}

fn build_adjacent_segments(segments: &[RailSegmentDef]) -> HashMap<(i32, i32), Vec<usize>> {
    let mut adjacency: HashMap<(i32, i32), Vec<usize>> = HashMap::new();
    for (idx, seg) in segments.iter().enumerate() {
        adjacency.entry(seg.from).or_default().push(idx);
        adjacency.entry(seg.to).or_default().push(idx);
    }
    adjacency
}

fn load_persisted_rail_network() -> Option<RailNetworkVis> {
    #[cfg(target_arch = "wasm32")]
    {
        return None;
    }

    let path = Path::new("data/rail_network.json");
    let bytes = std::fs::read(path).ok()?;
    let parsed: PersistedRailNetwork = serde_json::from_slice(&bytes).ok()?;

    let mut segments = Vec::new();
    for s in parsed.segments.into_iter() {
        let rail_type = rail_type_from_str(&s.rail_type)?;
        segments.push(RailSegmentDef {
            from: (s.from[0], s.from[1]),
            to: (s.to[0], s.to[1]),
            rail_type,
            track_count: s.track_count.max(1),
        });
    }

    let adjacency = build_adjacent_segments(&segments);
    Some(RailNetworkVis {
        segments,
        adjacency,
    })
}

fn rail_type_color(rail_type: RailTypeVis) -> Color {
    match rail_type {
        RailTypeVis::Standard => Color::rgb(0.65, 0.75, 0.95),
        RailTypeVis::Advanced => Color::rgb(0.60, 0.90, 0.70),
        RailTypeVis::Monorail => Color::rgb(0.95, 0.80, 0.45),
        RailTypeVis::Maglev => Color::rgb(0.90, 0.55, 0.95),
    }
}

fn rail_type_for_lane(idx: usize) -> RailTypeVis {
    match idx % 4 {
        0 => RailTypeVis::Standard,
        1 => RailTypeVis::Advanced,
        2 => RailTypeVis::Monorail,
        _ => RailTypeVis::Maglev,
    }
}

fn track_count_for_lane(idx: usize) -> u8 {
    // Spread counts 1-4 across bands, keeping within 1..=4
    (1 + (idx % 4)) as u8
}

fn build_rail_network(cfg: &GridConfig) -> RailNetworkVis {
    // Deterministic lattice: two east-west trunks and two diagonals to form a graph.
    let mut segments = Vec::new();
    let mut adjacency: HashMap<(i32, i32), Vec<usize>> = HashMap::new();

    let mut push_segment =
        |from: (i32, i32),
         to: (i32, i32),
         rail_type: RailTypeVis,
         track_count: u8,
         segments: &mut Vec<RailSegmentDef>,
         adjacency: &mut HashMap<(i32, i32), Vec<usize>>| {
            let idx = segments.len();
            segments.push(RailSegmentDef {
                from,
                to,
                rail_type,
                track_count,
            });
            adjacency.entry(from).or_default().push(idx);
            adjacency.entry(to).or_default().push(idx);
        };

    // Primary east-west trunks across the visible band
    let q_min = cfg.q_range.start.max(5);
    let q_max = (cfg.q_range.end - 5).max(q_min + 2);
    let r_base = cfg.r_range.start.max(10);
    let rows = [r_base, r_base + 2];

    for (row_idx, r) in rows.into_iter().enumerate() {
        for q in q_min..q_max {
            let rail_type = rail_type_for_lane(row_idx);
            let track_count = track_count_for_lane(row_idx);
            push_segment(
                (q, r),
                (q + 1, r),
                rail_type,
                track_count,
                &mut segments,
                &mut adjacency,
            );
        }
    }

    // Diagonal connectors to create branching graph and cycles
    let diag_r = r_base + 1;
    for (i, q) in (q_min..q_max - 1).enumerate() {
        let rail_type = rail_type_for_lane(i + 2);
        let track_count = track_count_for_lane(i + 2);
        // NE-SW diagonal: (q, diag_r) -> (q + 1, diag_r - 1)
        push_segment(
            (q, diag_r),
            (q + 1, diag_r - 1),
            rail_type,
            track_count,
            &mut segments,
            &mut adjacency,
        );
        // NW-SE diagonal: (q, diag_r) -> (q + 1, diag_r)
        push_segment(
            (q, diag_r),
            (q + 1, diag_r),
            rail_type,
            track_count,
            &mut segments,
            &mut adjacency,
        );
    }

    RailNetworkVis {
        segments,
        adjacency,
    }
}

fn setup_rail_segments(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    cfg: Res<GridConfig>,
) {
    use bevy::math::primitives::Rectangle;
    use bevy::sprite::{MaterialMesh2dBundle, Mesh2dHandle};

    let thickness = cfg.radius * 0.35;
    let network = load_persisted_rail_network().unwrap_or_else(|| build_rail_network(&cfg));
    let defs = network.segments.clone();
    commands.insert_resource(network);

    for def in defs.into_iter() {
        let from_world = axial_to_world(def.from.0, def.from.1, cfg.radius);
        let to_world = axial_to_world(def.to.0, def.to.1, cfg.radius);
        let delta = to_world - from_world;
        let length = delta.length();
        let angle = delta.y.atan2(delta.x);
        let midpoint = (from_world + to_world) * 0.5;

        let mesh: Mesh2dHandle = meshes
            .add(Mesh::from(Rectangle::new(length, thickness)))
            .into();
        let material = materials.add(rail_type_color(def.rail_type));

        commands.spawn((
            MaterialMesh2dBundle {
                mesh,
                material,
                transform: Transform::from_xyz(midpoint.x, midpoint.y, 0.4)
                    .with_rotation(Quat::from_rotation_z(angle)),
                ..Default::default()
            },
            RailSegmentVisual {
                midpoint,
                rail_type: def.rail_type,
                track_count: def.track_count,
            },
        ));
    }
}

fn setup_trains(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    cfg: Res<GridConfig>,
    network: Option<Res<RailNetworkVis>>,
) {
    use bevy::math::primitives::RegularPolygon;
    use bevy::sprite::{MaterialMesh2dBundle, Mesh2dHandle};

    let Some(net) = network else {
        return;
    };
    if net.segments.is_empty() {
        return;
    }

    let train_mesh: Mesh2dHandle = meshes
        .add(Mesh::from(RegularPolygon::new(cfg.radius * 0.35, 12)))
        .into();
    let train_material = materials.add(Color::rgb(1.0, 0.85, 0.35));

    let train_count = net.segments.len().min(3).max(1);
    for i in 0..train_count {
        let seg_idx = i % net.segments.len();
        let def = &net.segments[seg_idx];
        let from = axial_to_world(def.from.0, def.from.1, cfg.radius);
        let to = axial_to_world(def.to.0, def.to.1, cfg.radius);
        let t0 = (i as f32) / (train_count as f32);
        let pos = from + (to - from) * t0;
        commands.spawn((
            MaterialMesh2dBundle {
                mesh: train_mesh.clone(),
                material: train_material.clone(),
                transform: Transform::from_xyz(pos.x, pos.y, 0.6),
                ..Default::default()
            },
            TrainIndicator,
            TrainProgress {
                segment_idx: seg_idx,
                t: t0,
                speed: 0.35,
                status: TrainStatusVis::Moving,
            },
        ));
    }

    // Initialize bottleneck tracking
    commands.insert_resource(SegmentBottlenecks::default());

    // Spawn initial cargo particles on segments
    let cargo_mesh: Mesh2dHandle = meshes
        .add(Mesh::from(RegularPolygon::new(cfg.radius * 0.15, 8)))
        .into();
    let resource_colors = [
        ("Steel", Color::rgb(0.7, 0.7, 0.7)),
        ("Electronics", Color::rgb(0.2, 0.8, 1.0)),
        ("Energy", Color::rgb(1.0, 0.8, 0.2)),
    ];

    for (seg_idx, seg) in net.segments.iter().enumerate() {
        let resource = resource_colors[seg_idx % resource_colors.len()];
        let material = materials.add(resource.1);
        let from = axial_to_world(seg.from.0, seg.from.1, cfg.radius);
        let to = axial_to_world(seg.to.0, seg.to.1, cfg.radius);
        let t0 = 0.5;
        let pos = from + (to - from) * t0;
        commands.spawn((
            MaterialMesh2dBundle {
                mesh: cargo_mesh.clone(),
                material,
                transform: Transform::from_xyz(pos.x, pos.y, 0.5),
                ..Default::default()
            },
            CargoParticle {
                segment_idx: seg_idx,
                t: t0,
                speed: 0.25,
                resource_type: resource.0.to_string(),
            },
        ));
    }
}

fn train_motion_system(
    time: Res<Time>,
    cfg: Res<GridConfig>,
    network: Option<Res<RailNetworkVis>>,
    mut trains: Query<(&mut TrainProgress, &mut Transform)>,
) {
    let Some(net) = network else {
        return;
    };
    if net.segments.is_empty() {
        return;
    }

    for (mut prog, mut tf) in trains.iter_mut() {
        let seg = &net.segments[prog.segment_idx % net.segments.len()];
        let from = axial_to_world(seg.from.0, seg.from.1, cfg.radius);
        let to = axial_to_world(seg.to.0, seg.to.1, cfg.radius);
        let delta = time.delta_seconds() * prog.speed;
        let mut t = prog.t + delta;
        if t > 1.0 {
            t -= 1.0;
            prog.segment_idx = (prog.segment_idx + 1) % net.segments.len();
            prog.status = TrainStatusVis::Moving; // placeholder; future: loading dwell
        }
        prog.t = t;
        let seg2 = &net.segments[prog.segment_idx % net.segments.len()];
        let from2 = axial_to_world(seg2.from.0, seg2.from.1, cfg.radius);
        let to2 = axial_to_world(seg2.to.0, seg2.to.1, cfg.radius);
        let pos = from2 + (to2 - from2) * prog.t;
        tf.translation.x = pos.x;
        tf.translation.y = pos.y;
        let dir = to2 - from2;
        let angle = dir.y.atan2(dir.x);
        tf.rotation = Quat::from_rotation_z(angle);
    }
}

fn cargo_animation_system(
    time: Res<Time>,
    cfg: Res<GridConfig>,
    network: Option<Res<RailNetworkVis>>,
    mut cargos: Query<(&mut CargoParticle, &mut Transform)>,
    mut bottlenecks: ResMut<SegmentBottlenecks>,
) {
    let Some(net) = network else {
        return;
    };
    if net.segments.is_empty() {
        return;
    }

    bottlenecks.congestion.clear();
    let mut segment_cargo_count: HashMap<usize, usize> = HashMap::new();

    for (mut cargo, mut tf) in cargos.iter_mut() {
        let delta = time.delta_seconds() * cargo.speed;
        let mut t = cargo.t + delta;
        if t > 1.0 {
            t -= 1.0;
            cargo.segment_idx = (cargo.segment_idx + 1) % net.segments.len();
        }
        cargo.t = t;
        *segment_cargo_count.entry(cargo.segment_idx).or_insert(0) += 1;

        let seg = &net.segments[cargo.segment_idx % net.segments.len()];
        let from = axial_to_world(seg.from.0, seg.from.1, cfg.radius);
        let to = axial_to_world(seg.to.0, seg.to.1, cfg.radius);
        let pos = from + (to - from) * cargo.t;
        tf.translation.x = pos.x;
        tf.translation.y = pos.y;
    }

    // Calculate congestion: 0 = empty, 1 = fully congested (>3 particles)
    for (seg_idx, count) in segment_cargo_count.iter() {
        let congestion = (*count as f32 / 4.0).min(1.0);
        bottlenecks.congestion.insert(*seg_idx, congestion);
    }
}

fn train_badge_ui_system(
    mut contexts: EguiContexts,
    windows: Query<&Window>,
    q_cam: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    trains: Query<(&TrainProgress, &Visibility, &Transform)>,
) {
    let window = if let Ok(w) = windows.get_single() {
        w
    } else {
        return;
    };
    let (camera, cam_tf) = if let Ok(c) = q_cam.get_single() {
        c
    } else {
        return;
    };

    let ctx = contexts.ctx_mut();
    let layer = egui::LayerId::new(egui::Order::Foreground, egui::Id::new("train_badges"));
    let painter = ctx.layer_painter(layer);

    for (prog, visibility, tf) in trains.iter() {
        if *visibility != Visibility::Visible {
            continue;
        }
        let world = tf.translation;
        let Some(screen_pos) = camera.world_to_viewport(cam_tf, Vec3::new(world.x, world.y, 0.0))
        else {
            continue;
        };
        let pos = egui::pos2(screen_pos.x, window.height() - screen_pos.y);
        let text = match prog.status {
            TrainStatusVis::Moving => "Moving",
            TrainStatusVis::Loading => "Loading",
        };
        painter.text(
            pos,
            egui::Align2::CENTER_CENTER,
            text,
            egui::FontId::proportional(13.0),
            egui::Color32::WHITE,
        );
    }
}

fn setup_hex_grid(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    cfg: Res<GridConfig>,
    assets: Res<ClientAssets>,
) {
    use bevy::math::primitives::RegularPolygon;
    use bevy::sprite::{MaterialMesh2dBundle, Mesh2dHandle};

    // One shared mesh; materials per tile for unique colors
    let hex_mesh: Mesh2dHandle = meshes
        .add(Mesh::from(RegularPolygon::new(cfg.radius, 6)))
        .into();

    for q in cfg.q_range.clone() {
        for r in cfg.r_range.clone() {
            let pos = axial_to_world(q, r, cfg.radius);
            let color = if (q + r) % 2 == 0 {
                Color::rgb(0.08, 0.12, 0.18)
            } else {
                Color::rgb(0.10, 0.14, 0.22)
            };
            let material = materials.add(color);
            // Base hex colored mesh (kept for grid highlight and tints)
            let mut e = commands.spawn((
                MaterialMesh2dBundle {
                    mesh: hex_mesh.clone(),
                    material,
                    transform: Transform::from_xyz(pos.x, pos.y, 0.0),
                    ..Default::default()
                },
                HexTile { q, r },
            ));

            // Add a terrain sprite underlay as a child if available
            if let Some(handle) = assets.terrain_sprite.clone() {
                e.with_children(|child| {
                    child.spawn(SpriteBundle {
                        texture: handle,
                        transform: Transform::from_xyz(0.0, 0.0, -0.5)
                            .with_scale(Vec3::splat((cfg.radius * 2.0) / 64.0)), // assume 64px placeholder
                        sprite: Sprite {
                            // Opaque underlay to reduce overdraw from blending
                            color: Color::rgba(1.0, 1.0, 1.0, 1.0),
                            ..Default::default()
                        },
                        ..Default::default()
                    });
                });
            }
        }
    }
}

fn update_hex_hover_system(
    windows: Query<&Window>,
    q_cam: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    mut hover: ResMut<HexHover>,
    cfg: Res<GridConfig>,
) {
    let window = if let Ok(w) = windows.get_single() {
        w
    } else {
        return;
    };
    let (camera, cam_transform) = if let Ok(c) = q_cam.get_single() {
        c
    } else {
        return;
    };
    if let Some(cursor_pos) = window.cursor_position() {
        if let Some(world_pos) = camera.viewport_to_world_2d(cam_transform, cursor_pos) {
            let (q, r) = world_to_axial_round(world_pos, cfg.radius);
            hover.0 = Some((q, r));
            return;
        }
    }
    hover.0 = None;
}

fn handle_click_select_or_place_system(
    buttons: Res<ButtonInput<MouseButton>>,
    hover: Res<HexHover>,
    mut selected: ResMut<HexSelected>,
    mut game: ResMut<Game>,
    build: Res<BuildMode>,
    mut placed: ResMut<PlacedBuildings>,
    mut modals: ResMut<ModalState>,
    assets: Res<ClientAssets>,
    cfg: Res<GridConfig>,
    mut commands: Commands,
) {
    if buttons.just_pressed(MouseButton::Left) {
        if build.active {
            if let Some((q, r)) = hover.0 {
                if core::is_hex_in_bounds(q, r) {
                    let cmd = core::Command::PlaceBuilding {
                        q,
                        r,
                        building_type: build.building_type.clone(),
                    };
                    if let Ok(new_state) = core::exec_and_apply(&game.state, cmd) {
                        game.state = new_state;
                        selected.0 = Some((q, r));
                        placed
                            .0
                            .insert((q, r), ClientBuilding::new(build.building_type.clone()));
                        // Spawn a building sprite at this hex (above the grid)
                        if let Some(tex) = assets.buildings_sprite.clone() {
                            let pos = axial_to_world(q, r, cfg.radius);
                            commands.spawn((
                                SpriteBundle {
                                    texture: tex,
                                    transform: Transform::from_xyz(pos.x, pos.y, 1.0)
                                        .with_scale(Vec3::splat((cfg.radius * 1.2) / 64.0)),
                                    sprite: Sprite {
                                        color: Color::rgba(1.0, 1.0, 1.0, 1.0),
                                        ..Default::default()
                                    },
                                    ..Default::default()
                                },
                                BuildingSprite,
                            ));
                        }
                        modals.show_building_detail = true; // jump to detail after placement
                        modals.show_construction = false;
                    }
                }
            }
        } else {
            selected.0 = hover.0;
            if let Some(coord) = selected.0 {
                // if a building exists here, open detail modal
                if placed.0.contains_key(&coord) {
                    modals.show_building_detail = true;
                }
            }
        }
    }
    if buttons.just_pressed(MouseButton::Right) {
        // clear selection on right click
        selected.0 = None;
        // also close detail modal on right click
        modals.show_building_detail = false;
    }
}

fn update_hex_highlight_colors_system(
    hover: Res<HexHover>,
    selected: Res<HexSelected>,
    build: Res<BuildMode>,
    game: Res<Game>,
    mut tiles: Query<(&HexTile, &Handle<ColorMaterial>)>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let hovered = hover.0;
    let selected = selected.0;
    // Determine a combined layer tint based on active layers
    let mut tint = Vec3::ZERO; // accumulate RGB
    let mut tint_count = 0.0f32;
    if core::is_layer_enabled(&game.state, core::Layer::Power) {
        tint += Vec3::new(0.95, 0.85, 0.30); // warm/yellow
        tint_count += 1.0;
    }
    if core::is_layer_enabled(&game.state, core::Layer::Resources) {
        tint += Vec3::new(0.30, 0.85, 0.40); // green
        tint_count += 1.0;
    }
    if core::is_layer_enabled(&game.state, core::Layer::Pollution) {
        tint += Vec3::new(0.80, 0.30, 0.80); // magenta
        tint_count += 1.0;
    }
    if tint_count > 0.0 {
        tint /= tint_count;
    }
    let has_tint = tint_count > 0.0;
    for (tile, mat_handle) in tiles.iter_mut() {
        let base = if (tile.q + tile.r) % 2 == 0 {
            Color::rgb(0.08, 0.12, 0.18)
        } else {
            Color::rgb(0.10, 0.14, 0.22)
        };
        let mut color = base;
        // Apply layer tint to base color when not hovered/selected
        if has_tint {
            let rgba = base.as_rgba_f32();
            let br = rgba[0];
            let bg = rgba[1];
            let bb = rgba[2];
            let ba = rgba[3];
            let tr = tint.x;
            let tg = tint.y;
            let tb = tint.z;
            // simple lerp 30%
            let mix = 0.3f32;
            let rr = br * (1.0 - mix) + tr * mix;
            let rg = bg * (1.0 - mix) + tg * mix;
            let rb = bb * (1.0 - mix) + tb * mix;
            color = Color::rgba(rr, rg, rb, ba);
        }
        if Some((tile.q, tile.r)) == hovered {
            if build.active {
                // ghost preview: green if valid, red if invalid
                let valid = core::is_hex_in_bounds(tile.q, tile.r);
                color = if valid {
                    Color::rgba(0.20, 0.50, 0.30, 1.0)
                } else {
                    Color::rgba(0.50, 0.20, 0.20, 1.0)
                };
            } else {
                color = Color::rgb(0.20, 0.25, 0.35);
            }
        }
        if Some((tile.q, tile.r)) == selected {
            color = Color::rgb(0.35, 0.25, 0.20);
        }
        if let Some(mat) = materials.get_mut(&*mat_handle) {
            mat.color = color;
        }
    }
}

// Hide grid tiles and building sprites that are outside the camera's view rectangle
fn visibility_culling_system(
    windows: Query<&Window>,
    q_cam: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    cfg: Res<GridConfig>,
    culling: Res<CullingConfig>,
    mut param_set: ParamSet<(
        Query<(&HexTile, &mut Visibility)>,
        Query<(&GlobalTransform, &mut Visibility), With<BuildingSprite>>,
        Query<(&RailSegmentVisual, &mut Visibility)>,
        Query<(&Transform, &mut Visibility), With<TrainIndicator>>,
        Query<(&Transform, &mut Visibility), With<CargoParticle>>,
    )>,
) {
    // If culling disabled, ensure everything is visible and exit
    if !culling.enabled {
        for (_, mut vis) in param_set.p0().iter_mut() {
            *vis = Visibility::Visible;
        }
        for (_, mut vis) in param_set.p1().iter_mut() {
            *vis = Visibility::Visible;
        }
        for (_, mut vis) in param_set.p2().iter_mut() {
            *vis = Visibility::Visible;
        }
        for (_, mut vis) in param_set.p3().iter_mut() {
            *vis = Visibility::Visible;
        }
        for (_, mut vis) in param_set.p4().iter_mut() {
            *vis = Visibility::Visible;
        }
        return;
    }

    let window = if let Ok(w) = windows.get_single() {
        w
    } else {
        return;
    };
    let (camera, cam_tf) = if let Ok(c) = q_cam.get_single() {
        c
    } else {
        return;
    };

    // Map viewport corners (0,0) and (w,h) to world coords
    let size = Vec2::new(window.width(), window.height());
    let Some(min_world) = camera.viewport_to_world_2d(cam_tf, Vec2::ZERO) else {
        return;
    };
    let Some(max_world) = camera.viewport_to_world_2d(cam_tf, size) else {
        return;
    };
    let mut min_x = min_world.x.min(max_world.x);
    let mut max_x = min_world.x.max(max_world.x);
    let mut min_y = min_world.y.min(max_world.y);
    let mut max_y = min_world.y.max(max_world.y);

    // Expand by margin to avoid popping on edges
    min_x -= culling.margin_world;
    max_x += culling.margin_world;
    min_y -= culling.margin_world;
    max_y += culling.margin_world;

    // Cull tiles by computing their world positions from axial coords
    for (tile, mut vis) in param_set.p0().iter_mut() {
        let pos = axial_to_world(tile.q, tile.r, cfg.radius);
        let inside = pos.x >= min_x && pos.x <= max_x && pos.y >= min_y && pos.y <= max_y;
        *vis = if inside {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }

    // Cull standalone building sprites using their world transform
    for (gt, mut vis) in param_set.p1().iter_mut() {
        let p = gt.translation();
        let inside = p.x >= min_x && p.x <= max_x && p.y >= min_y && p.y <= max_y;
        *vis = if inside {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }

    // Cull rail segments using their midpoints
    for (seg, mut vis) in param_set.p2().iter_mut() {
        let p = seg.midpoint;
        let inside = p.x >= min_x && p.x <= max_x && p.y >= min_y && p.y <= max_y;
        *vis = if inside {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }

    // Cull trains by their world position
    for (tf, mut vis) in param_set.p3().iter_mut() {
        let p = tf.translation;
        let inside = p.x >= min_x && p.x <= max_x && p.y >= min_y && p.y <= max_y;
        *vis = if inside {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }

    // Cull cargo particles by their world position
    for (tf, mut vis) in param_set.p4().iter_mut() {
        let p = tf.translation;
        let inside = p.x >= min_x && p.x <= max_x && p.y >= min_y && p.y <= max_y;
        *vis = if inside {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

fn rail_label_ui_system(
    mut contexts: EguiContexts,
    windows: Query<&Window>,
    q_cam: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    rails: Query<(&RailSegmentVisual, &Visibility)>,
    bottlenecks: Res<SegmentBottlenecks>,
) {
    let window = if let Ok(w) = windows.get_single() {
        w
    } else {
        return;
    };
    let (camera, cam_tf) = if let Ok(c) = q_cam.get_single() {
        c
    } else {
        return;
    };
    let ctx = contexts.ctx_mut();
    let layer = egui::LayerId::new(egui::Order::Foreground, egui::Id::new("rail_labels"));
    let painter = ctx.layer_painter(layer);

    for (idx, (seg, visibility)) in rails.iter().enumerate() {
        if *visibility != Visibility::Visible {
            continue;
        }
        let world = Vec3::new(seg.midpoint.x, seg.midpoint.y, 0.0);
        let Some(screen_pos) = camera.world_to_viewport(cam_tf, world) else {
            continue;
        };
        // Bevy's viewport origin is bottom-left; egui is top-left
        let pos = egui::pos2(screen_pos.x, window.height() - screen_pos.y);
        let color = match seg.rail_type {
            RailTypeVis::Standard => egui::Color32::from_rgb(180, 210, 255),
            RailTypeVis::Advanced => egui::Color32::from_rgb(170, 235, 190),
            RailTypeVis::Monorail => egui::Color32::from_rgb(240, 210, 140),
            RailTypeVis::Maglev => egui::Color32::from_rgb(230, 140, 240),
        };

        // Highlight bottlenecked segments with bright red background
        if let Some(congestion) = bottlenecks.congestion.get(&idx) {
            if *congestion > 0.7 {
                painter.circle(
                    pos,
                    20.0,
                    egui::Color32::from_rgba_unmultiplied(255, 100, 100, 80),
                    egui::Stroke::new(2.0, egui::Color32::RED),
                );
            }
        }

        let text = format!("×{}", seg.track_count);
        painter.text(
            pos,
            egui::Align2::CENTER_CENTER,
            text,
            egui::FontId::proportional(14.0),
            color,
        );
    }
}

// Separate system to handle Space shortcut for advancing turn
fn advance_turn_shortcut_system(
    keys: Res<ButtonInput<KeyCode>>,
    mut game: ResMut<Game>,
    mut storage_mgr: ResMut<StorageManager>,
) {
    if keys.just_pressed(KeyCode::Space) {
        let new_state = core::exec_and_apply(&game.state, core::Command::AdvanceTurn)
            .expect("advance turn should succeed");
        let snapshot = new_state.clone();
        game.state = new_state;
        // push new snapshot to history
        game.history.push_back(snapshot);
        while game.history.len() > game.history_max {
            game.history.pop_front();
        }
        // attempt autosave
        storage_mgr.try_autosave(&game.state);
    }
}

// Keyboard shortcuts for build mode: B toggles build; Esc cancels
fn build_shortcuts_system(
    keys: Res<ButtonInput<KeyCode>>,
    mut build: ResMut<BuildMode>,
    mut modals: ResMut<ModalState>,
) {
    if keys.just_pressed(KeyCode::KeyB) {
        build.active = !build.active;
        if build.active {
            modals.show_construction = true;
        }
    }
    if keys.just_pressed(KeyCode::Escape) {
        build.active = false;
        // Close any open modal on Esc
        modals.show_construction = false;
        modals.show_building_detail = false;
    }
}

// --- Construction Modal ---
fn construction_modal_window_system(
    mut contexts: EguiContexts,
    mut build: ResMut<BuildMode>,
    mut modals: ResMut<ModalState>,
) {
    if !modals.show_construction {
        return;
    }
    let ctx = contexts.ctx_mut();
    // Avoid borrow issues by splitting open() and show(), and deferring state mutation
    let mut open = modals.show_construction;
    let mut window = egui::Window::new("Construction");
    window = window.open(&mut open);
    let mut clicked_close = false;
    window.show(ctx, |ui| {
        ui.label("Select a building to place. Left-click on the map to confirm.");
        ui.separator();
        ui.columns(2, |cols| {
            // Column 1: grid of buildings
            cols[0].vertical(|ui| {
                ui.heading("Buildings");
                ui.horizontal_wrapped(|ui| {
                    for (name, cost) in [("Solar", 10), ("Wind", 12), ("Hab", 8)] {
                        let mut btn = egui::Button::new(format!("{}\nCost:{}", name, cost));
                        if build.building_type == name {
                            btn = btn.fill(egui::Color32::from_rgb(40, 80, 120));
                        }
                        if ui.add(btn).clicked() {
                            build.building_type = name.to_string();
                        }
                    }
                });
            });
            // Column 2: details for selected
            cols[1].vertical(|ui| {
                ui.heading("Details");
                match build.building_type.as_str() {
                    "Solar" => {
                        ui.label("Generates a small amount of power. No workers.");
                    }
                    "Wind" => {
                        ui.label("Generates power depending on wind (placeholder).");
                    }
                    "Hab" => {
                        ui.label("Provides housing for population. Consumes some resources.");
                    }
                    _ => {
                        ui.label("Select a building to see details.");
                    }
                }
                ui.separator();
                ui.small("Tip: Hover a hex to see if placement is valid. Click to place.");
            });
        });
        ui.separator();
        // Close handled by window close button too; explicit button toggles local flag
        if ui.button("Close").clicked() {
            clicked_close = true;
        }
    });
    if clicked_close {
        open = false;
    }
    modals.show_construction = open;
}

// --- Building Detail Modal ---
/// Available recipe options for production buildings
const AVAILABLE_RECIPES: &[&str] = &[
    "Iron → Steel",
    "Coal + Iron → Steel",
    "Nickel + Iron → Steel",
    "Copper → Electronics",
    "Gold + Graphite → Electronics",
    "Lithium + Cobalt → Battery",
    "Solar + Lithium → Battery",
    "Battery → Energy",
    "Silicon + Titanium → Solar Power",
    "Uranium → Nuclear Fuel",
    "Gas + Coal → Hydrogen",
    "Food Production",
];

fn building_detail_modal_window_system(
    mut contexts: EguiContexts,
    selected: Res<HexSelected>,
    placed: Res<PlacedBuildings>,
    mut modals: ResMut<ModalState>,
) {
    if !modals.show_building_detail {
        return;
    }
    let Some(coord) = selected.0 else {
        modals.show_building_detail = false;
        return;
    };
    let Some(building) = placed.0.get(&coord) else {
        modals.show_building_detail = false;
        return;
    };
    let ctx = contexts.ctx_mut();
    let mut open = modals.show_building_detail;
    let mut window = egui::Window::new("Building Details");
    window = window.open(&mut open);
    let mut clicked_close = false;
    window.show(ctx, |ui| {
        ui.label(format!("Hex q={}, r={}", coord.0, coord.1));
        if building.paused {
            ui.heading(format!("{} (Paused)", building.kind));
        } else {
            ui.heading(&building.kind);
        }
        ui.separator();
        ui.label("Stats (placeholder):");
        match building.kind.as_str() {
            "Solar" => {
                if building.paused {
                    ui.monospace("Power +0 (paused), Workers 0, Upkeep 0");
                } else {
                    ui.monospace("Power +2, Workers 0, Upkeep 0");
                }
            }
            "Wind" => {
                if building.paused {
                    ui.monospace("Power +0 (paused), Workers 0, Upkeep 0");
                } else {
                    ui.monospace("Power +3 (avg), Workers 0, Upkeep 0");
                }
            }
            "Hab" => {
                if building.paused {
                    ui.monospace("Housing +0 (paused), Upkeep 0");
                } else {
                    ui.monospace("Housing +4, Upkeep -1 resources/turn");
                }
            }
            _ => {
                ui.monospace("Unknown building");
            }
        }
        ui.separator();

        // Recipe selection UI (new for 7.4.6)
        ui.label("Production Recipe:");
        let current_recipe = modals
            .selected_recipe_index
            .min((AVAILABLE_RECIPES.len() - 1) as u32);
        let mut new_recipe = current_recipe as usize;
        let mut recipe_changed = false;

        ui.horizontal(|ui| {
            for (idx, recipe_label) in AVAILABLE_RECIPES.iter().enumerate() {
                if ui
                    .selectable_value(&mut new_recipe, idx, *recipe_label)
                    .clicked()
                {
                    recipe_changed = idx != current_recipe as usize;
                }
            }
        });

        if recipe_changed {
            modals.selected_recipe_index = new_recipe as u32;
            let selected_recipe = AVAILABLE_RECIPES[new_recipe];
            info!("Recipe UI: user selected {}", selected_recipe);
        }

        ui.separator();
        ui.horizontal(|ui| {
            // Note: these mutate via commands at end to avoid borrow conflicts
            let pause_label = if building.paused { "Resume" } else { "Pause" };
            if ui.button(pause_label).clicked() {
                // Use a repaint signal; actual state change will happen after the window closes
                // We can't mutate here due to `placed` being an immutable borrow in this scope.
                // Workaround: store intent in Response id, we'll handle below.
                ui.data_mut(|d| d.insert_temp(egui::Id::new("detail_intent"), 1_i32));
            }
            if ui.button("Demolish").clicked() {
                ui.data_mut(|d| d.insert_temp(egui::Id::new("detail_intent"), 2_i32));
            }
        });
        ui.separator();
        if ui.button("Close").clicked() {
            clicked_close = true;
        }
    });
    if clicked_close {
        open = false;
    }
    modals.show_building_detail = open;
}

// Post-UI mutator: applies intents from building detail modal safely
fn building_detail_post_ui_apply_intents_system(
    mut contexts: EguiContexts,
    selected: Res<HexSelected>,
    mut placed: ResMut<PlacedBuildings>,
    mut modals: ResMut<ModalState>,
) {
    if !modals.show_building_detail {
        return;
    }
    let Some(coord) = selected.0 else {
        return;
    };
    let ctx = contexts.ctx_mut();
    if let Some(intent) = ctx.data_mut(|d| d.remove_temp::<i32>(egui::Id::new("detail_intent"))) {
        match intent {
            1 => {
                if let Some(b) = placed.0.get_mut(&coord) {
                    b.paused = !b.paused;
                }
            }
            2 => {
                placed.0.remove(&coord);
                modals.show_building_detail = false;
            }
            _ => {}
        }
    }
}

// --- Power Chart Window (egui_plot) ---
fn power_chart_window_system(
    mut contexts: EguiContexts,
    game: Res<Game>,
    mut modals: ResMut<ModalState>,
) {
    if !modals.show_power_chart {
        return;
    }
    let ctx = contexts.ctx_mut();
    let mut open = modals.show_power_chart;
    let mut window = egui::Window::new("Power Chart");
    window = window
        .open(&mut open)
        .default_width(520.0)
        .default_height(320.0);
    window.show(ctx, |ui| {
        ui.label("Generation vs Consumption over recent turns");
        ui.separator();
        // Prepare datasets
        let (gen, cons) = prepare_power_series(&game.history);
        let line_gen = Line::new(PlotPoints::from_iter(gen.into_iter().map(|(x, y)| [x, y])))
            .name("Generation");
        let line_cons = Line::new(PlotPoints::from_iter(cons.into_iter().map(|(x, y)| [x, y])))
            .name("Consumption");
        Plot::new("power_plot")
            .legend(egui_plot::Legend::default())
            .show(ui, |plot_ui| {
                plot_ui.line(line_gen);
                plot_ui.line(line_cons);
            });
        ui.separator();
        if ui.button("Refresh").clicked() {
            // No-op: data is from current history; button exists to satisfy on-demand update affordance.
        }
    });
    modals.show_power_chart = open;
}

// Pure helper to transform states to series; x uses GameState.turn when nonzero, else index.
fn prepare_power_series(
    states: &std::collections::VecDeque<core::GameState>,
) -> (Vec<(f64, f64)>, Vec<(f64, f64)>) {
    let mut gen = Vec::with_capacity(states.len());
    let mut cons = Vec::with_capacity(states.len());
    for (i, s) in states.iter().enumerate() {
        let x = if s.turn > 0 { s.turn as f64 } else { i as f64 };
        let snap = core::get_power_snapshot(s);
        gen.push((x, snap.generation as f64));
        cons.push((x, snap.consumption as f64));
    }
    (gen, cons)
}

// --- Resources Chart Window (egui_plot) ---
fn resources_chart_window_system(
    mut contexts: EguiContexts,
    game: Res<Game>,
    mut modals: ResMut<ModalState>,
) {
    if !modals.show_resources_chart {
        return;
    }
    let ctx = contexts.ctx_mut();
    let mut open = modals.show_resources_chart;
    let mut window = egui::Window::new("Resources Chart");
    window = window
        .open(&mut open)
        .default_width(520.0)
        .default_height(320.0);
    window.show(ctx, |ui| {
        ui.label("Resource stock, production, and consumption over recent turns");
        ui.separator();
        // Prepare datasets
        let (stock, prod, cons) = prepare_resource_series(&game.history);
        let line_stock = Line::new(PlotPoints::from_iter(
            stock.into_iter().map(|(x, y)| [x, y]),
        ))
        .name("Stock");
        let line_prod = Line::new(PlotPoints::from_iter(prod.into_iter().map(|(x, y)| [x, y])))
            .name("Production");
        let line_cons = Line::new(PlotPoints::from_iter(cons.into_iter().map(|(x, y)| [x, y])))
            .name("Consumption");
        Plot::new("resources_plot")
            .legend(egui_plot::Legend::default())
            .show(ui, |plot_ui| {
                plot_ui.line(line_stock);
                plot_ui.line(line_prod);
                plot_ui.line(line_cons);
            });
        ui.separator();
        if ui.button("Refresh").clicked() {
            // No-op: data is derived from current history; button keeps the UI consistent with on-demand updates.
        }
    });
    modals.show_resources_chart = open;
}

// Pure helper for resources: returns (stock, production, consumption) series.
// Production/consumption are derived from per-turn delta of resources_stock (>=0 to production, <0 to consumption as positive value).
fn prepare_resource_series(
    states: &std::collections::VecDeque<core::GameState>,
) -> (Vec<(f64, f64)>, Vec<(f64, f64)>, Vec<(f64, f64)>) {
    let mut stock = Vec::with_capacity(states.len());
    let mut prod = Vec::with_capacity(states.len());
    let mut cons = Vec::with_capacity(states.len());
    let mut prev: Option<&core::GameState> = None;
    for (i, s) in states.iter().enumerate() {
        let x = if s.turn > 0 { s.turn as f64 } else { i as f64 };
        stock.push((x, s.resources_stock as f64));
        if let Some(p) = prev {
            let delta = s.resources_stock - p.resources_stock;
            if delta >= 0 {
                prod.push((x, delta as f64));
                cons.push((x, 0.0));
            } else {
                prod.push((x, 0.0));
                cons.push((x, (-delta) as f64));
            }
        } else {
            // First point: no previous delta
            prod.push((x, 0.0));
            cons.push((x, 0.0));
        }
        prev = Some(s);
    }
    (stock, prod, cons)
}

// --- Population Chart Window (egui_plot) ---
fn population_chart_window_system(
    mut contexts: EguiContexts,
    game: Res<Game>,
    mut modals: ResMut<ModalState>,
) {
    if !modals.show_population_chart {
        return;
    }
    let ctx = contexts.ctx_mut();
    let mut open = modals.show_population_chart;
    let mut window = egui::Window::new("Population Chart");
    window = window
        .open(&mut open)
        .default_width(520.0)
        .default_height(320.0);
    window.show(ctx, |ui| {
        ui.label("Population totals over recent turns");
        ui.separator();
        // Prepare datasets
        let (total, employed, unemployed) = prepare_population_series(&game.history);
        let line_total = Line::new(PlotPoints::from_iter(
            total.into_iter().map(|(x, y)| [x, y]),
        ))
        .name("Total");
        let line_employed = Line::new(PlotPoints::from_iter(
            employed.into_iter().map(|(x, y)| [x, y]),
        ))
        .name("Employed");
        let line_unemployed = Line::new(PlotPoints::from_iter(
            unemployed.into_iter().map(|(x, y)| [x, y]),
        ))
        .name("Unemployed");
        Plot::new("population_plot")
            .legend(egui_plot::Legend::default())
            .show(ui, |plot_ui| {
                plot_ui.line(line_total);
                plot_ui.line(line_employed);
                plot_ui.line(line_unemployed);
            });
        ui.separator();
        if ui.button("Refresh").clicked() {
            // No-op: derived from current history
        }
    });
    modals.show_population_chart = open;
}

// Pure helper for population: returns (total, employed, unemployed) series.
fn prepare_population_series(
    states: &std::collections::VecDeque<core::GameState>,
) -> (Vec<(f64, f64)>, Vec<(f64, f64)>, Vec<(f64, f64)>) {
    let mut total = Vec::with_capacity(states.len());
    let mut employed = Vec::with_capacity(states.len());
    let mut unemployed = Vec::with_capacity(states.len());
    for (i, s) in states.iter().enumerate() {
        let x = if s.turn > 0 { s.turn as f64 } else { i as f64 };
        let snap = core::get_population_snapshot(s);
        let unemp = (snap.total - snap.employed).max(0);
        total.push((x, snap.total as f64));
        employed.push((x, snap.employed as f64));
        unemployed.push((x, unemp as f64));
    }
    (total, employed, unemployed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prepare_power_series() {
        let mut dq = std::collections::VecDeque::new();
        dq.push_back(core::GameState {
            turn: 0,
            power_generation: 10,
            power_consumption: 6,
            ..Default::default()
        });
        dq.push_back(core::GameState {
            turn: 1,
            power_generation: 12,
            power_consumption: 7,
            ..Default::default()
        });
        dq.push_back(core::GameState {
            turn: 2,
            power_generation: 15,
            power_consumption: 9,
            ..Default::default()
        });
        let (g, c) = prepare_power_series(&dq);
        assert_eq!(g.len(), 3);
        assert_eq!(c.len(), 3);
        assert_eq!(g[0], (0.0, 10.0)); // first uses index since turn=0
        assert_eq!(g[1], (1.0, 12.0));
        assert_eq!(c[2], (2.0, 9.0));
    }

    #[test]
    fn test_prepare_resource_series() {
        let mut dq = std::collections::VecDeque::new();
        dq.push_back(core::GameState {
            turn: 0,
            resources_stock: 10,
            ..Default::default()
        });
        dq.push_back(core::GameState {
            turn: 1,
            resources_stock: 13,
            ..Default::default()
        });
        dq.push_back(core::GameState {
            turn: 2,
            resources_stock: 11,
            ..Default::default()
        });
        let (stock, prod, cons) = prepare_resource_series(&dq);
        assert_eq!(stock.len(), 3);
        assert_eq!(prod.len(), 3);
        assert_eq!(cons.len(), 3);
        assert_eq!(stock[0], (0.0, 10.0));
        assert_eq!(prod[1], (1.0, 3.0)); // +3 production
        assert_eq!(cons[2], (2.0, 2.0)); // -2 consumption shown as positive
    }

    #[test]
    fn test_prepare_population_series() {
        let mut dq = std::collections::VecDeque::new();
        dq.push_back(core::GameState {
            turn: 0,
            population_total: 10,
            population_employed: 6,
            ..Default::default()
        });
        dq.push_back(core::GameState {
            turn: 1,
            population_total: 12,
            population_employed: 7,
            ..Default::default()
        });
        dq.push_back(core::GameState {
            turn: 2,
            population_total: 11,
            population_employed: 9,
            ..Default::default()
        });
        let (tot, emp, unemp) = prepare_population_series(&dq);
        assert_eq!(tot.len(), 3);
        assert_eq!(emp.len(), 3);
        assert_eq!(unemp.len(), 3);
        assert_eq!(tot[0], (0.0, 10.0));
        assert_eq!(emp[1], (1.0, 7.0));
        assert_eq!(unemp[2], (2.0, 2.0));
    }

    #[test]
    fn test_debug_metrics_default() {
        let metrics = DebugMetrics::default();
        assert_eq!(metrics.fps, 0.0);
        assert_eq!(metrics.frame_time_ms, 0.0);
        assert_eq!(metrics.entity_count, 0);
        assert_eq!(metrics.visible_entities, 0);
        assert_eq!(metrics.memory_mb, 0.0);
    }

    #[test]
    fn test_ui_debug_default() {
        let debug = UiDebug::default();
        assert!(!debug.enabled);
        assert_eq!(debug.last_rects.len(), 0);
    }

    #[test]
    fn test_debug_overlay_performance() {
        // Task 7.6.4: Verify debug overlay overhead is < 1ms
        // This is a smoke test to ensure update_debug_metrics_system can run quickly
        use bevy::app::App;
        use bevy::MinimalPlugins;

        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(DebugMetrics::default())
            .add_systems(Update, update_debug_metrics_system);

        // Measure time for 100 updates
        let start = std::time::Instant::now();
        for _ in 0..100 {
            app.update();
        }
        let elapsed = start.elapsed();
        let avg_per_update = elapsed.as_micros() / 100;

        // Should be well under 1ms (1000 microseconds) per update
        assert!(
            avg_per_update < 1000,
            "Debug overlay overhead too high: {} µs per update (target: < 1000 µs)",
            avg_per_update
        );
    }

    #[test]
    fn test_debug_log_filter_default() {
        // Task 7.6.6: Verify log filter defaults show all levels
        let filter = DebugLogFilter::default();
        assert!(filter.show_debug);
        assert!(filter.show_info);
        assert!(filter.show_warn);
        assert!(filter.show_error);
        assert_eq!(filter.category_filter, "");
    }

    #[test]
    fn test_debug_log_filter_level_toggles() {
        // Test that individual log level filters can be toggled
        let mut filter = DebugLogFilter::default();
        assert!(filter.show_debug);

        filter.show_debug = false;
        assert!(!filter.show_debug);
        assert!(filter.show_info);
        assert!(filter.show_warn);
        assert!(filter.show_error);

        filter.show_error = false;
        assert!(!filter.show_debug);
        assert!(filter.show_info);
        assert!(filter.show_warn);
        assert!(!filter.show_error);
    }

    #[test]
    fn test_debug_log_filter_category_text() {
        // Test category filter text can be set and retrieved
        let mut filter = DebugLogFilter::default();
        assert_eq!(filter.category_filter, "");

        filter.category_filter = "building".to_string();
        assert_eq!(filter.category_filter, "building");

        filter.category_filter = "resource:production".to_string();
        assert_eq!(filter.category_filter, "resource:production");
        assert!(filter.category_filter.contains("resource"));
        assert!(filter.category_filter.contains("production"));
    }

    #[test]
    fn test_debug_log_filter_cloneable() {
        // Verify DebugLogFilter can be cloned (important for resource mutation)
        let filter = DebugLogFilter {
            show_debug: false,
            show_info: true,
            show_warn: false,
            show_error: true,
            category_filter: "test_category".to_string(),
        };

        let cloned = filter.clone();
        assert_eq!(cloned.show_debug, false);
        assert_eq!(cloned.show_info, true);
        assert_eq!(cloned.show_warn, false);
        assert_eq!(cloned.show_error, true);
        assert_eq!(cloned.category_filter, "test_category");
    }

    #[test]
    fn test_debug_log_filter_all_levels_disabled() {
        // Test edge case where all levels are disabled
        let mut filter = DebugLogFilter::default();
        filter.show_debug = false;
        filter.show_info = false;
        filter.show_warn = false;
        filter.show_error = false;

        // UI should still work, just display nothing
        assert!(!filter.show_debug && !filter.show_info && !filter.show_warn && !filter.show_error);
    }
}
