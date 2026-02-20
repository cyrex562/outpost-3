// V5 entity hierarchy modules
pub mod ids;
pub mod celestial_body;
pub mod site;
pub mod star_system;
pub mod galaxy;
pub mod game_state;
pub mod building_instance;
pub mod building_queries;
pub mod construction_queue;
pub mod resource_deposit;
pub mod deposit_generator;
pub mod recipe;
pub mod resource_mapping;
pub mod power_grid;
pub mod life_support;
pub mod representative_character;

// Legacy modules (pre-V5, kept for migration)
pub mod banking;
pub mod building;
pub mod colony;
pub mod economy;
pub mod market;
pub mod planet;
pub mod population;
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
pub use game_state::{GameState, EventEngineState, PendingEventChoice};
pub use building_instance::{BuildingInstance, BuildingState as BuildingStateV5};
pub use construction_queue::{ConstructionQueue, ConstructionJob};
pub use resource_deposit::{ResourceDeposit, ExtractionDifficulty};
pub use recipe::{Recipe, RecipeError};
pub use resource_mapping::{
    string_to_resource_type, resource_type_to_string, is_virtual_resource, MappingError,
};
pub use representative_character::{
    CharacterId, RepresentativeCharacter, generate_characters,
};

// Legacy exports (pre-V5)
pub use banking::{amortized_payment, Loan, LoanId};
pub use building::{Building, BuildingState, BuildingType};
pub use colony::{Colony, ColonyId};
pub use economy::{calculate_snapshot, EconomySnapshot};
pub use market::{market_price_update, MarketPrice};
pub use planet::{Planet, PlanetId};
pub use population::{Population, ColonistNeeds, SkillCategory, AgeBucket};
pub use power_grid::PowerGrid;
pub use resource::{ResourceType, Resources};
pub use wormhole::WormholeId;
