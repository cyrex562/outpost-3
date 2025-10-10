mod systems;
mod components;
mod resources;
mod plugins;

use bevy::prelude::*;
use bevy::log::LogPlugin;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(LogPlugin {
                level: bevy::log::Level::INFO,
                filter: "wgpu=warn,bevy_render=warn".into(),
                update_subscriber: None,
            }),
            plugins::GamePlugin,
        ))
        .run();
}
