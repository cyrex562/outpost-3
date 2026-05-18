pub mod turn;
pub mod game_clock;
pub mod time_command;
pub mod tick_processor;
pub mod construction_processor;
pub mod production;
pub mod storage_helpers;
pub mod power_processor;
pub mod life_support_processor;
pub mod event_engine;

pub use game_clock::{ClockConfig, GameClock};
pub use time_command::TimeCommand;
pub use tick_processor::{process_tick, process_ticks};
pub use construction_processor::process_construction_tick;
pub use production::{process_building_production, ProductionResult, ProductionError};
pub use storage_helpers::{
    compute_site_storage, compute_resource_rates, active_production_chains,
    list_resource_producers, list_resource_consumers, ResourceRate, ActiveChain,
    compute_storage_utilisation,
};
pub use power_processor::{compute_generation, compute_consumption, process_power_tick};
pub use life_support_processor::{compute_life_support, process_life_support_tick};
pub use event_engine::{evaluate_trigger, should_fire, apply_outcome, apply_choice, process_events_tick, resolve_choice};


