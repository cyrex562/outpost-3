pub mod exploration;

use crate::*;

pub struct ReducerContext {
    pub current_offset: u64,
    pub game_time: f64,
}

pub type ReducerResult = (GameState, Vec<EventPayload>);

pub fn reduce(state: GameState, cmd: Command, ctx: ReducerContext) -> ReducerResult {
    match cmd {
        Command::AdvanceTime { dt } => reduce_advance_time(state, dt, ctx),
        Command::LaunchProbe { target_system_id } => {
            reduce_launch_probe(state, target_system_id, ctx)
        }
    }
}

fn reduce_advance_time(state: GameState, dt: f64, _ctx: ReducerContext) -> ReducerResult {
    let new_time = state.game_time + dt;
    let mut events = vec![EventPayload::TimeAdvanced { dt, new_time }];

    // Find probes that have arrived
    let mut arrived_probe_ids = Vec::new();
    let mut new_state = state.clone();

    for probe in &state.probes_in_flight {
        if probe.arrival_time <= new_time {
            // Probe has arrived!
            arrived_probe_ids.push(probe.id);

            events.push(EventPayload::ProbeArrived {
                probe_id: probe.id,
                system_id: probe.target_system_id,
            });

            // Generate the discovered system
            let system = exploration::generate_system(probe.target_system_id);

            events.push(EventPayload::SystemDiscovered {
                system: system.clone(),
            });

            // Add system to state
            new_state = new_state.with_system_discovered(system);
        }
    }

    // Remove arrived probes from state
    new_state = new_state.with_probes_removed(&arrived_probe_ids);

    // Update game time
    new_state.game_time = new_time;

    (new_state, events)
}

fn reduce_launch_probe(
    state: GameState,
    target_system_id: StarSystemId,
    ctx: ReducerContext,
) -> ReducerResult {
    // Mock distance calculation - in real game, this would come from system coords
    let distance_ly = 4.37; // Alpha Centauri distance
    let travel_time = exploration::calculate_travel_time(distance_ly);
    let arrival_time = ctx.game_time + travel_time;

    // Add probe to state
    let (new_state, probe_id) = state.with_probe_launched(target_system_id, arrival_time);

    let events = vec![EventPayload::ProbeLaunched {
        probe_id,
        target_system_id,
        eta: arrival_time,
    }];

    (new_state, events)
}
