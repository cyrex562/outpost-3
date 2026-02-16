// V5 entity hierarchy modules
pub mod ids;
pub mod celestial_body;
pub mod site;
pub mod star_system;
pub mod galaxy;
pub mod game_state;
pub mod building_instance;
pub mod building_queries;

// Legacy modules (pre-V5, kept for migration)
pub mod banking;
pub mod building;
pub mod colony;
pub mod economy;
pub mod market;
pub mod planet;
pub mod population;
pub mod power_grid;
pub mod production_chain;
pub mod resource;
pub mod wormhole;

// V5 entity hierarchy exports
pub use ids::{
    BuildingId, CelestialBodyId, GalaxyId, SiteId, StarSystemId,
};
pub use celestial_body::{
    Atmosphere, BodyType, CelestialBody, HazardLevel, Temperature,
};
pub use site::{Site, SiteType};
pub use star_system::StarSystem;
pub use galaxy::Galaxy;
pub use game_state::GameState;
pub use building_instance::{BuildingInstance, BuildingState as BuildingStateV5};

// Legacy exports (pre-V5)
pub use banking::{amortized_payment, Loan, LoanId};
pub use building::{Building, BuildingState, BuildingType};
pub use colony::{Colony, ColonyId};
pub use economy::{calculate_snapshot, EconomySnapshot};
pub use market::{market_price_update, MarketPrice};
pub use planet::{Planet, PlanetId};
pub use population::Population;
pub use power_grid::PowerGrid;
pub use resource::{ResourceType, Resources};
pub use wormhole::WormholeId;
