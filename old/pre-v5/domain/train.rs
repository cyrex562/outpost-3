use serde::{Deserialize, Serialize};
use super::PlanetId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TrainId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RouteId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ConsistId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EngineId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CarId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TrainRouteId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StationId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrainType {
    Freight,
    Passenger,
    Mixed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrainSize {
    Small,
    Medium,
    Large,
    Massive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrainState {
    Idle,
    InTransit,
    Loading,
    Unloading,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Train {
    pub id: TrainId,
    pub name: String,
    pub train_type: TrainType,
    pub size: TrainSize,
    pub speed: u32,
    pub capacity: u32,
    pub current_planet_id: Option<PlanetId>,
    pub state: TrainState,
}

impl Train {
    pub fn new(id: TrainId, name: String, train_type: TrainType, size: TrainSize) -> Self {
        let (speed, capacity) = match size {
            TrainSize::Small => (100, 50),
            TrainSize::Medium => (80, 200),
            TrainSize::Large => (60, 500),
            TrainSize::Massive => (40, 1000),
        };

        Self {
            id,
            name,
            train_type,
            size,
            speed,
            capacity,
            current_planet_id: None,
            state: TrainState::Idle,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Route {
    pub id: RouteId,
    pub name: String,
    pub origin_planet_id: PlanetId,
    pub destination_planet_id: PlanetId,
    pub active: bool,
}

impl Route {
    pub fn new(id: RouteId, name: String, origin: PlanetId, destination: PlanetId) -> Self {
        Self {
            id,
            name,
            origin_planet_id: origin,
            destination_planet_id: destination,
            active: true,
        }
    }
}

/// Cargo class for different types of freight cars
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CargoClass {
    Flatbed,    // Solids (ore, steel, etc.)
    Tanker,     // Liquids (water, fuel)
    Reefer,     // Perishables (food)
    Boxcar,     // General goods
    Hopper,     // Bulk (grain, coal)
}

impl CargoClass {
    pub fn name(&self) -> &'static str {
        match self {
            CargoClass::Flatbed => "Flatbed",
            CargoClass::Tanker => "Tanker",
            CargoClass::Reefer => "Reefer",
            CargoClass::Boxcar => "Boxcar",
            CargoClass::Hopper => "Hopper",
        }
    }
}

/// Engine type with power and speed characteristics
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EngineType {
    pub name: String,
    pub power: u32,           // Power rating (affects speed under load)
    pub speed_factor: f32,    // Multiplier on rail_type speed
    pub capacity: u32,        // Pulling capacity (max total car mass)
    pub cost_credits: i64,
    pub maintenance_per_turn: i64,
}

impl EngineType {
    pub fn diesel() -> Self {
        EngineType {
            name: "Diesel".to_string(),
            power: 1000,
            speed_factor: 1.0,
            capacity: 5000,
            cost_credits: 10000,
            maintenance_per_turn: 50,
        }
    }

    pub fn electric() -> Self {
        EngineType {
            name: "Electric".to_string(),
            power: 1200,
            speed_factor: 1.2,
            capacity: 6000,
            cost_credits: 15000,
            maintenance_per_turn: 40,
        }
    }

    pub fn hyperion() -> Self {
        EngineType {
            name: "Hyperion Express".to_string(),
            power: 1500,
            speed_factor: 1.5,
            capacity: 4000,
            cost_credits: 25000,
            maintenance_per_turn: 80,
        }
    }
}

/// Freight car type with capacity and cargo class
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CarType {
    pub name: String,
    pub capacity: u32,         // Units of cargo
    pub cargo_class: CargoClass,
    pub mass: u32,             // For power calculation
    pub cost_credits: i64,
}

impl CarType {
    pub fn flatbed() -> Self {
        CarType {
            name: "Standard Flatbed".to_string(),
            capacity: 100,
            cargo_class: CargoClass::Flatbed,
            mass: 50,
            cost_credits: 2000,
        }
    }

    pub fn tank_car() -> Self {
        CarType {
            name: "Tank Car".to_string(),
            capacity: 120,
            cargo_class: CargoClass::Tanker,
            mass: 60,
            cost_credits: 2500,
        }
    }

    pub fn reefer_car() -> Self {
        CarType {
            name: "Reefer Car".to_string(),
            capacity: 80,
            cargo_class: CargoClass::Reefer,
            mass: 55,
            cost_credits: 3000,
        }
    }

    pub fn box_car() -> Self {
        CarType {
            name: "Box Car".to_string(),
            capacity: 90,
            cargo_class: CargoClass::Boxcar,
            mass: 50,
            cost_credits: 2200,
        }
    }

    pub fn hopper_car() -> Self {
        CarType {
            name: "Hopper Car".to_string(),
            capacity: 150,
            cargo_class: CargoClass::Hopper,
            mass: 70,
            cost_credits: 2800,
        }
    }
}

/// Engine instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Engine {
    pub id: EngineId,
    pub engine_type: EngineType,
    pub condition: f32,  // 0.0-1.0 (maintenance degrades this)
}

impl Engine {
    pub fn new(id: EngineId, engine_type: EngineType) -> Self {
        Self {
            id,
            engine_type,
            condition: 1.0,
        }
    }

    /// Get effective power based on condition
    pub fn effective_power(&self) -> u32 {
        (self.engine_type.power as f32 * self.condition) as u32
    }

    /// Get effective speed factor based on condition
    pub fn effective_speed_factor(&self) -> f32 {
        self.engine_type.speed_factor * self.condition
    }
}

/// Freight car instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FreightCar {
    pub id: CarId,
    pub car_type: CarType,
    pub cargo: Option<(super::ResourceType, u32)>,  // (resource, quantity)
}

impl FreightCar {
    pub fn new(id: CarId, car_type: CarType) -> Self {
        Self {
            id,
            car_type,
            cargo: None,
        }
    }

    /// Get current cargo weight (approximation)
    pub fn current_weight(&self) -> u32 {
        let base = self.car_type.mass;
        if let Some((_, quantity)) = self.cargo {
            base + quantity
        } else {
            base
        }
    }

    /// Check if car is empty
    pub fn is_empty(&self) -> bool {
        self.cargo.is_none()
    }

    /// Get available capacity
    pub fn available_capacity(&self) -> u32 {
        if let Some((_, quantity)) = self.cargo {
            self.car_type.capacity.saturating_sub(quantity)
        } else {
            self.car_type.capacity
        }
    }

    /// Load cargo into car
    pub fn load_cargo(
        &mut self,
        resource: super::ResourceType,
        quantity: u32,
    ) -> Result<(), String> {
        if quantity > self.available_capacity() {
            return Err("Insufficient capacity".to_string());
        }

        if let Some((current_resource, current_quantity)) = &mut self.cargo {
            if *current_resource != resource {
                return Err("Car already contains different resource".to_string());
            }
            *current_quantity += quantity;
        } else {
            self.cargo = Some((resource, quantity));
        }

        Ok(())
    }

    /// Unload all cargo
    pub fn unload_cargo(&mut self) -> Option<(super::ResourceType, u32)> {
        self.cargo.take()
    }
}

/// Train consist (engine + cars)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Consist {
    pub id: ConsistId,
    pub engine: Engine,
    pub cars: Vec<FreightCar>,
}

impl Consist {
    pub const MAX_CARS: usize = 16;

    pub fn new(id: ConsistId, engine: Engine) -> Self {
        Self {
            id,
            engine,
            cars: Vec::new(),
        }
    }

    /// Add a car to the consist
    pub fn add_car(&mut self, car: FreightCar) -> Result<(), String> {
        if self.cars.len() >= Self::MAX_CARS {
            return Err(format!("Maximum {} cars per consist", Self::MAX_CARS));
        }

        if self.total_mass() + car.current_weight() > self.engine.engine_type.capacity {
            return Err("Exceeds engine capacity".to_string());
        }

        self.cars.push(car);
        Ok(())
    }

    /// Get total cargo capacity across all cars
    pub fn total_capacity(&self) -> u32 {
        self.cars.iter().map(|c| c.car_type.capacity).sum()
    }

    /// Get available capacity
    pub fn available_capacity(&self) -> u32 {
        self.cars.iter().map(|c| c.available_capacity()).sum()
    }

    /// Get total mass (engine + cars + cargo)
    pub fn total_mass(&self) -> u32 {
        self.cars.iter().map(|c| c.current_weight()).sum()
    }

    /// Get number of cars
    pub fn car_count(&self) -> usize {
        self.cars.len()
    }

    /// Calculate cost to purchase this consist
    pub fn total_cost(&self) -> i64 {
        self.engine.engine_type.cost_credits
            + self.cars.iter().map(|c| c.car_type.cost_credits).sum::<i64>()
    }

    /// Check if consist can carry a specific resource type
    pub fn can_carry_resource(&self, resource: super::ResourceType) -> bool {
        // For now, simplified: any car can carry any resource
        // In full implementation, would match resource to cargo class
        self.available_capacity() > 0
    }

    /// Calculate maximum speed on a rail segment
    pub fn max_speed_on_rail(&self, rail_segment: &super::rail::RailSegment) -> f32 {
        let base_speed = 100.0; // Base speed in km/h or similar
        let rail_speed = base_speed * rail_segment.speed_factor();
        let engine_speed = rail_speed * self.engine.effective_speed_factor();

        // Reduce speed based on load
        let load_factor = if self.total_mass() > 0 {
            let capacity = self.engine.engine_type.capacity as f32;
            let mass = self.total_mass() as f32;
            1.0 - (mass / capacity * 0.3) // Max 30% speed reduction at full load
        } else {
            1.0
        };

        engine_speed * load_factor
    }
}

#[cfg(test)]
mod train_tests {
    use super::*;

    #[test]
    fn test_engine_creation() {
        let diesel = EngineType::diesel();
        let engine = Engine::new(EngineId(1), diesel.clone());
        assert_eq!(engine.id, EngineId(1));
        assert_eq!(engine.condition, 1.0);
        assert_eq!(engine.effective_power(), diesel.power);
    }

    #[test]
    fn test_engine_degraded_condition() {
        let diesel = EngineType::diesel();
        let mut engine = Engine::new(EngineId(1), diesel.clone());
        engine.condition = 0.5;

        assert_eq!(engine.effective_power(), diesel.power / 2);
    }

    #[test]
    fn test_freight_car_creation() {
        let flatbed = CarType::flatbed();
        let car = FreightCar::new(CarId(1), flatbed.clone());
        assert!(car.is_empty());
        assert_eq!(car.available_capacity(), flatbed.capacity);
    }

    #[test]
    fn test_freight_car_load_cargo() {
        let mut car = FreightCar::new(CarId(1), CarType::flatbed());
        let result = car.load_cargo(super::super::ResourceType::Steel, 50);
        assert!(result.is_ok());
        assert_eq!(car.available_capacity(), 50);
        assert!(!car.is_empty());
    }

    #[test]
    fn test_freight_car_capacity_exceeded() {
        let mut car = FreightCar::new(CarId(1), CarType::flatbed());
        let result = car.load_cargo(super::super::ResourceType::Steel, 200);
        assert!(result.is_err());
    }

    #[test]
    fn test_consist_creation() {
        let engine = Engine::new(EngineId(1), EngineType::diesel());
        let consist = Consist::new(ConsistId(1), engine);
        assert_eq!(consist.car_count(), 0);
        assert_eq!(consist.total_capacity(), 0);
    }

    #[test]
    fn test_consist_add_car() {
        let engine = Engine::new(EngineId(1), EngineType::diesel());
        let mut consist = Consist::new(ConsistId(1), engine);

        let flatbed = CarType::flatbed();
        let car = FreightCar::new(CarId(1), flatbed.clone());
        assert!(consist.add_car(car).is_ok());
        assert_eq!(consist.car_count(), 1);
        assert_eq!(consist.total_capacity(), flatbed.capacity);
    }

    #[test]
    fn test_consist_max_cars() {
        let engine = Engine::new(EngineId(1), EngineType::diesel());
        let mut consist = Consist::new(ConsistId(1), engine);

        // Add maximum cars
        for i in 0..Consist::MAX_CARS {
            let car = FreightCar::new(CarId(i as u64), CarType::flatbed());
            assert!(consist.add_car(car).is_ok());
        }

        // Try to add one more
        let extra_car = FreightCar::new(CarId(999), CarType::flatbed());
        assert!(consist.add_car(extra_car).is_err());
    }

    #[test]
    fn test_consist_total_cost() {
        let diesel = EngineType::diesel();
        let flatbed = CarType::flatbed();
        let engine = Engine::new(EngineId(1), diesel.clone());
        let mut consist = Consist::new(ConsistId(1), engine);

        let car = FreightCar::new(CarId(1), flatbed.clone());
        consist.add_car(car).unwrap();

        let expected_cost = diesel.cost_credits + flatbed.cost_credits;
        assert_eq!(consist.total_cost(), expected_cost);
    }

    #[test]
    fn test_consist_speed_calculation() {
        let engine = Engine::new(EngineId(1), EngineType::diesel());
        let consist = Consist::new(ConsistId(1), engine);

        let segment = super::super::rail::RailSegment::new(
            super::super::SegmentId(1),
            super::super::HexCoord::new(0, 0),
            super::super::HexCoord::new(1, 0),
            super::super::RailType::Standard,
        );

        let speed = consist.max_speed_on_rail(&segment);
        assert!(speed > 0.0);
    }
}
