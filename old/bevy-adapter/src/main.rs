use bevy::prelude::*;
use bevy_egui::{EguiContexts, EguiPlugin, EguiPrimaryContextPass, egui};
use outpost_3_core::{Command, *};

#[derive(Resource)]
struct StateStore {
    state: GameState,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(EguiPlugin::default())
        .insert_resource(StateStore {
            state: GameState::new(),
        })
        .add_systems(Startup, setup_camera_system)
        .add_systems(EguiPrimaryContextPass, ui_system)
        .run();
}

fn setup_camera_system(mut commands: Commands) {
    commands.spawn(Camera2d::default());
}

fn ui_system(mut contexts: EguiContexts, mut store: ResMut<StateStore>) -> Result {
    egui::Window::new("Bevy + Rust Core").show(contexts.ctx_mut()?, |ui| {
        ui.heading("Colony Simulation");
        ui.label(format!("Game Time: {:.1}h", store.state.game_time));

        if ui.button("Advance 10h").clicked() {
            let ctx = systems::ReducerContext {
                current_offset: 0,
                game_time: store.state.game_time,
            };
            let (new_state, _events) =
                systems::reduce(store.state.clone(), Command::AdvanceTime { dt: 10.0 }, ctx);
            store.state = new_state;
        }
    });

    Ok(())
}
