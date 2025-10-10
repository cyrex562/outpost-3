pub enum CelestialBodyType {
    Star,
    Planet,
    Moon,
    Asteroid,
    Comet,
    DwarfPlanet,
}

pub struct CelestialBody {
    pub name: String,
    pub id: Uuid,
    pub body_type: CelestialBodyType,
    pub mass: f64,
    pub diameter: f64
}