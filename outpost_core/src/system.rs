//! System zoom — world specialization, inter-body logistics, and megaprojects.
//!
//! Covers DESIGN.md §8.3: the system scope's distinct responsibilities:
//! * World-scale specialization: which body plays which system role.
//! * Inter-body logistics: cargo ships with hauler capacity as a managed resource.
//! * Megaprojects: multi-milestone construction pooled from the entire system.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

// ─── Body & Node Map ─────────────────────────────────────────────────────────

/// Stable identifier for a celestial body (planet, moon, belt, station).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BodyId(pub Uuid);

impl BodyId {
    /// Stand-in id for a founding map that has no real body behind it.
    ///
    /// Two cases produce one (issue #300): `SeedPlanet` applied with no
    /// generated system (most engine tests, and the balance harness), and a
    /// pre-#300 save whose founding map was stored unkeyed. Both still need a
    /// key for [`crate::turn::GameState::planet_maps`] so that map stays the
    /// single source of truth.
    ///
    /// A body under this id is deliberately *not* in
    /// [`NodeMap::bodies`] — anything resolving a body from the system will
    /// miss, which is the intended signal that the association is absent
    /// rather than merely unread.
    #[must_use]
    pub fn placeholder_home() -> Self {
        Self(Uuid::nil())
    }

    /// Whether this is [`Self::placeholder_home`] rather than a real body.
    #[must_use]
    pub fn is_placeholder_home(&self) -> bool {
        self.0.is_nil()
    }

    /// Stable surface-generation seed for this body (issue #300).
    ///
    /// Hashes the body's UUID down to a `u64` so the same body always
    /// generates the same surface, and different bodies differ. A
    /// wrapping-multiply combine (not XOR) keeps the two halves
    /// order-sensitive, so swapped-half UUIDs don't collide on one seed.
    ///
    /// The single source of this derivation: both
    /// `GameEngine::body_surface_preview` and the persisted map a body gains
    /// when it is first settled go through here, which is what guarantees the
    /// surface a player previewed is the surface they land on.
    #[must_use]
    pub fn surface_seed(&self) -> u64 {
        let (hi, lo) = self.0.as_u64_pair();
        hi.wrapping_mul(0x9E37_79B9_7F4A_7C15).wrapping_add(lo)
    }
}

impl BodyId {
    /// Create a new random [`BodyId`].
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for BodyId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for BodyId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Category of a celestial body in the system node map.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BodyKind {
    /// Rocky inner planet.
    InnerPlanet,
    /// Gas or ice giant.
    GasGiant,
    /// Moon orbiting a planet or giant.
    Moon,
    /// Asteroid belt aggregate node.
    AsteroidBelt,
    /// Cometary belt — icy volatiles at the cold outer edge (Kuiper-like).
    CometaryBelt,
    /// Orbital station (not a natural body).
    OrbitalStation,
}

impl BodyKind {
    /// Whether this kind is a belt (asteroid or cometary) — belts carry a
    /// [`BeltProfile`] and render as a density-zoned annulus rather than a
    /// point body (system-screen fix B2).
    #[must_use]
    pub fn is_belt(&self) -> bool {
        matches!(self, BodyKind::AsteroidBelt | BodyKind::CometaryBelt)
    }

    /// Whether this kind has a solid surface worth previewing as a hex map
    /// (planet, giant, or moon). Belts are diffuse aggregates and an
    /// [`OrbitalStation`](BodyKind::OrbitalStation) is a built structure, not a
    /// world — neither has a surface, so the "view surface" action is scoped
    /// out for them (map/nav plan).
    #[must_use]
    pub fn has_surface(&self) -> bool {
        matches!(
            self,
            BodyKind::InnerPlanet | BodyKind::GasGiant | BodyKind::Moon
        )
    }
}

/// One angular zone of a belt's annulus, carrying its own density
/// (system-screen fix B2). Zones subdivide the ring so a belt reads as
/// clumpy — denser and sparser arcs — rather than a uniform band.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BeltZone {
    /// Angular start of the zone, in degrees `[0, 360)`.
    pub start_deg: f32,
    /// Angular sweep of the zone, in degrees.
    pub sweep_deg: f32,
    /// Fill density `[0, 1]` — drives the annulus fill opacity for this zone.
    pub density: f32,
}

/// Radial + angular profile of an asteroid or cometary belt (system-screen
/// fix B2). Drawn as an annulus between `inner_au` and `outer_au`, subdivided
/// into `zones` whose densities vary the fill opacity around the ring. `None`
/// on non-belt bodies and on authored belts that don't specify one.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BeltProfile {
    /// Inner radius of the annulus, in AU.
    pub inner_au: f32,
    /// Outer radius of the annulus, in AU.
    pub outer_au: f32,
    /// Angular zones subdividing the annulus. Contiguous and covering the
    /// full ring when procedurally generated.
    pub zones: Vec<BeltZone>,
}

/// Assigned system-role for a celestial body.
///
/// The player assigns a role to guide automated resource flow and bonuses.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SystemRole {
    /// Heavy industry, manufacturing, and processing.
    Industry,
    /// Raw resource extraction (mining, harvesting).
    RawExtraction,
    /// Scientific research and development.
    Science,
    /// Volatile and fuel production (e.g. gas giant skimmers).
    FuelProduction,
    /// Population hub and administrative centre.
    PopulationHub,
    /// Unassigned; no specialization bonus applied.
    Unassigned,
}

/// Atmospheric thickness/density band for a body (issue #197).
///
/// Orthogonal to [`AtmosphereHazard`] — a body's atmosphere is fully
/// described by the pair, e.g. `Dense` + `Corrosive` (a Venus-like world),
/// which a single flat enum couldn't represent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AtmosphereDensity {
    /// No meaningful atmosphere (vacuum / trace).
    Vacuum,
    /// Very thin — supports rudimentary EVA operations only.
    Thin,
    /// Human-breathable pressure without heavy equipment.
    Breathable,
    /// Thick / high-pressure.
    Dense,
}

impl AtmosphereDensity {
    /// Whether this density supports unaided human breathing. Does not
    /// account for [`AtmosphereHazard`] — a `Breathable`-density atmosphere
    /// with a `Toxic` hazard is still lethal to breathe.
    #[must_use]
    pub fn is_breathable(self) -> bool {
        matches!(self, Self::Breathable)
    }
}

/// Chemical hazard band for a body's atmosphere (issue #197).
///
/// Orthogonal to [`AtmosphereDensity`]. See that type's doc comment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AtmosphereHazard {
    /// Chemically inert — no additional hazard beyond density.
    None,
    /// Corrosive to unshielded equipment and tissue.
    Corrosive,
    /// Poisonous — lethal without a sealed suit or habitat.
    Toxic,
    /// Oxidizing and/or combustible — fire and explosion risk.
    OxidizingCombustible,
}

/// Surface temperature band for a body.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TemperatureBand {
    /// Well below survivable liquid-water range.
    Frozen,
    /// Cold but manageable with insulation.
    Cold,
    /// Human-comfortable band.
    Temperate,
    /// Consistently hot — cooling infrastructure required.
    Hot,
    /// Lethal without heavy shielding.
    Extreme,
}

/// Ambient radiation exposure level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RadiationLevel {
    /// Negligible — near-Earth-baseline exposure.
    Low,
    /// Elevated — shielding recommended.
    Moderate,
    /// Dangerous — heavy shielding mandatory.
    High,
}

/// A single habitability-scoring input that a tech effect can mitigate
/// (issue #185).
///
/// Mitigating an attribute treats it as its best band for
/// [`Body::habitability_with_mitigations`] purposes only — the body's
/// underlying attribute value is never changed. Gravity is intentionally
/// excluded: it is a continuous `f32`, not one of the small fixed enums the
/// other three inputs use, so "cancel the penalty" isn't well-defined for it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HabitabilityAttribute {
    /// Treats [`AtmosphereDensity`] as [`AtmosphereDensity::Breathable`].
    AtmosphereDensity,
    /// Treats [`AtmosphereHazard`] as [`AtmosphereHazard::None`].
    AtmosphereHazard,
    /// Treats [`TemperatureBand`] as [`TemperatureBand::Temperate`].
    Temperature,
    /// Treats [`RadiationLevel`] as [`RadiationLevel::Low`].
    Radiation,
}

fn default_atmosphere_density() -> AtmosphereDensity {
    AtmosphereDensity::Vacuum
}
fn default_atmosphere_hazard() -> AtmosphereHazard {
    AtmosphereHazard::None
}
fn default_temperature() -> TemperatureBand {
    TemperatureBand::Temperate
}
fn default_gravity_g() -> f32 {
    1.0
}
fn default_radiation() -> RadiationLevel {
    RadiationLevel::Low
}
fn default_abundance_scalar() -> f32 {
    1.0
}
fn default_habitable_zone_center_au() -> f32 {
    crate::system_gen::SystemGenParams::default().habitable_zone_center_au
}
fn default_min_inner_planets() -> u32 {
    crate::system_gen::SystemGenParams::default().min_inner_planets
}
fn default_max_inner_planets() -> u32 {
    crate::system_gen::SystemGenParams::default().max_inner_planets
}

/// Surface/composition archetype for a body, orthogonal to [`BodyKind`]
/// (issue #196).
///
/// `BodyKind` says what a body broadly *is* (an inner planet, a gas giant,
/// a moon, ...); `PlanetarySubtype` says what it's *made of* — content
/// authoring guidance rather than a habitability input (see
/// [`Body::habitability`], which is unchanged by this field). Not every
/// variant is valid on every `BodyKind`; see [`PlanetarySubtype::compatible_with`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanetarySubtype {
    /// No archetype authored yet — the neutral default. Never biases
    /// downstream systems (e.g. [`crate::map::PlanetMap`] deposit
    /// generation) any differently than [`PlanetarySubtype::EarthLike`].
    Unclassified,
    /// Rocky and airless-to-thin, baked by proximity to the primary.
    RockyBarrenHot,
    /// Rocky and airless-to-thin, chilled by distance from the primary.
    RockyBarrenCold,
    /// Active volcanism; typically hot, toxic, and dense-atmosphered.
    Molten,
    /// Breathable, temperate, broadly Earth-comparable.
    EarthLike,
    /// Water-dominant surface.
    Ocean,
    /// Bare rock, no strong thermal extreme — valid on inner planets,
    /// moons, and asteroid-belt aggregates alike.
    Rocky,
    /// Ice-dominant composition — valid on inner planets, moons, and
    /// asteroid-belt aggregates alike.
    Icy,
    /// A hydrogen/helium-dominated giant. Only valid on [`BodyKind::GasGiant`].
    GasGiant,
    /// A volatile-ice-dominated giant (methane/ammonia/water ice). Only
    /// valid on [`BodyKind::GasGiant`].
    IceGiant,
}

impl PlanetarySubtype {
    /// Whether this subtype is a sensible archetype for a body of the
    /// given `kind`. `Unclassified` is always compatible; `OrbitalStation`
    /// (not a natural body) accepts only `Unclassified`.
    #[must_use]
    pub fn compatible_with(self, kind: &BodyKind) -> bool {
        match self {
            Self::Unclassified => true,
            Self::RockyBarrenHot
            | Self::RockyBarrenCold
            | Self::Molten
            | Self::EarthLike
            | Self::Ocean => {
                matches!(kind, BodyKind::InnerPlanet)
            }
            Self::Rocky | Self::Icy => {
                matches!(
                    kind,
                    BodyKind::InnerPlanet
                        | BodyKind::Moon
                        | BodyKind::AsteroidBelt
                        | BodyKind::CometaryBelt
                )
            }
            Self::GasGiant | Self::IceGiant => matches!(kind, BodyKind::GasGiant),
        }
    }
}

fn default_subtype() -> PlanetarySubtype {
    PlanetarySubtype::Unclassified
}
fn default_tidally_locked() -> bool {
    false
}
fn default_axial_tilt_deg() -> f32 {
    23.5
}
fn default_rotation_period_hours() -> f32 {
    24.0
}
fn default_moon_count() -> u32 {
    0
}

/// Per-category production yield a body's [`BodyModifier`]s can affect
/// (issue #184).
///
/// Distinct from the flat [`Body::habitability_modifier`], which applies
/// uniformly to every recipe output regardless of what it produces — a
/// `YieldCategory` modifier targets one slice of production instead.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum YieldCategory {
    /// Recipes producing a `consumable`-category commodity (food, water).
    FoodYield,
    /// The catch-all for extraction/processing recipes that aren't food —
    /// ore, refined metals, components, end-items, and similar chains.
    IndustryYield,
    /// Recipes run by a `BuildingCategory::Research` building.
    ScienceYield,
    /// Recipes run by a `BuildingCategory::Power` building.
    PowerOutput,
}

/// An authored per-category production modifier on a [`Body`] (issue #184).
///
/// Stacks *multiplicatively* with [`Body::habitability_modifier`] rather
/// than replacing it — the habitability modifier remains the general "how
/// survivable is this place" scalar applied to every output; a
/// `BodyModifier` is an additional, category-specific factor layered on
/// top (e.g. a gas giant might author an elevated `power_output` modifier
/// for its abundant fusion fuel, independent of how habitable it is).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BodyModifier {
    /// Which production category this modifier affects.
    pub category: YieldCategory,
    /// Multiplicative delta — `1.0` is neutral, `1.2` is +20%, `0.8` is −20%.
    pub multiplier: f32,
}

/// Look up the multiplier for `category` in `modifiers`, defaulting to
/// `1.0` (neutral) when no [`BodyModifier`] authors that category.
#[must_use]
pub fn category_modifier(modifiers: &[BodyModifier], category: YieldCategory) -> f32 {
    modifiers
        .iter()
        .find(|m| m.category == category)
        .map_or(1.0, |m| m.multiplier)
}

/// A system-wide, hex-independent resource endowment tag on a [`Body`]
/// (issue #232).
///
/// Only the founding colony's home body ever gets a full [`crate::map::PlanetMap`]
/// with hex-level [`crate::map::Deposit`]s — every other system body never has
/// a hex map generated at all in the live game. `BodyDeposit` is the lightweight
/// stand-in that lets "does this body have any `structural_ore`" be answered for
/// bodies without hex-level detail, so a future outpost/expedition system
/// (#233/#235) has something to check against. It intentionally carries no
/// location finer than "on this body" and no depletion/quantity — richness
/// is comparative flavor only, mirroring how hex `Deposit::richness` doesn't
/// currently gate production either (see issue #232's PR description for why
/// deposit-gating itself is explicitly out of scope here).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BodyDeposit {
    /// Content-pack commodity id (e.g. `"structural_ore"`).
    pub commodity_id: String,
    /// Relative abundance in `(0.0, 1.0]` — comparative flavor, not a budget.
    pub abundance: f32,
}

impl BodyDeposit {
    /// Create a new body-level deposit tag.
    #[must_use]
    pub fn new(commodity_id: impl Into<String>, abundance: f32) -> Self {
        Self {
            commodity_id: commodity_id.into(),
            abundance: abundance.clamp(0.001, 1.0),
        }
    }
}

/// A celestial body in the system node map.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Body {
    /// Stable identifier.
    pub id: BodyId,
    /// Display name.
    pub name: String,
    /// Kind of body.
    pub kind: BodyKind,
    /// Current system-role assignment.
    pub role: SystemRole,
    /// Distance from the system primary in arbitrary AU units (used for travel time).
    pub distance_au: f32,
    /// Atmospheric thickness/density band (issue #197).
    #[serde(default = "default_atmosphere_density")]
    pub atmosphere_density: AtmosphereDensity,
    /// Atmospheric chemical hazard band (issue #197).
    #[serde(default = "default_atmosphere_hazard")]
    pub atmosphere_hazard: AtmosphereHazard,
    /// Surface temperature band.
    #[serde(default = "default_temperature")]
    pub temperature: TemperatureBand,
    /// Surface gravity as a fraction of Earth-g.
    #[serde(default = "default_gravity_g")]
    pub gravity_g: f32,
    /// Ambient radiation exposure level.
    #[serde(default = "default_radiation")]
    pub radiation: RadiationLevel,
    /// Surface/composition archetype (issue #196). Content-authoring
    /// guidance — never an input to [`Body::habitability`].
    #[serde(default = "default_subtype")]
    pub subtype: PlanetarySubtype,
    /// Whether the body's rotation is locked to its orbital period (one
    /// hemisphere permanently faces the primary). Flavor-only for now — no
    /// habitability or production effect (issue #196).
    #[serde(default = "default_tidally_locked")]
    pub tidally_locked: bool,
    /// Axial tilt in degrees. Flavor-only for now.
    #[serde(default = "default_axial_tilt_deg")]
    pub axial_tilt_deg: f32,
    /// Rotation period in hours. Flavor-only for now; meaningless when
    /// `tidally_locked` is `true` (rotation period equals orbital period).
    #[serde(default = "default_rotation_period_hours")]
    pub rotation_period_hours: f32,
    /// Number of natural satellites orbiting this body. Flavor stat,
    /// distinct from `parent_body` below (which records what *this* body
    /// orbits, not what orbits it).
    #[serde(default = "default_moon_count")]
    pub moon_count: u32,
    /// The body this one orbits, if any. Lets a `Moon`-kind body's parent
    /// itself be a `Moon` (moon-of-a-moon) rather than only ever a planet
    /// or giant — issue #196's "planet itself is a moon of another body".
    #[serde(default)]
    pub parent_body: Option<BodyId>,
    /// Per-category production modifiers (issue #184) — e.g. a gas giant
    /// might author an elevated `power_output` modifier. Empty by default
    /// (every category neutral at `1.0`); stacks multiplicatively with
    /// [`Self::habitability_modifier`] rather than replacing it.
    #[serde(default)]
    pub modifiers: Vec<BodyModifier>,
    /// System-wide resource endowment tags (issue #232) — see [`BodyDeposit`].
    /// Empty by default; populated by `system_gen::distribute_system_resources`
    /// for procedurally generated systems.
    #[serde(default)]
    pub deposits: Vec<BodyDeposit>,
    /// Whether a survey expedition has completed at this body with at least
    /// a partial reveal (issue #235) — see
    /// `expedition::Command::LaunchSurveyExpedition`. Flavor/UI state only;
    /// does not gate anything mechanically (deposits are always true world
    /// state, matching #232's coverage-not-gating precedent).
    #[serde(default)]
    pub surveyed: bool,
    /// Named candidate colony/outpost site discovered by a full-reveal
    /// survey (issue #235), if any.
    #[serde(default)]
    pub candidate_site_name: Option<String>,
    /// Radial/angular density profile for belt-kind bodies (system-screen
    /// fix B2). `None` for non-belt bodies and for authored belts that don't
    /// specify one — in which case the belt falls back to a point marker.
    #[serde(default)]
    pub belt_profile: Option<BeltProfile>,
}

impl Body {
    /// Create a new unassigned body with neutral attributes.
    #[must_use]
    pub fn new(name: impl Into<String>, kind: BodyKind, distance_au: f32) -> Self {
        Self {
            id: BodyId::new(),
            name: name.into(),
            kind,
            role: SystemRole::Unassigned,
            distance_au,
            atmosphere_density: default_atmosphere_density(),
            atmosphere_hazard: default_atmosphere_hazard(),
            temperature: default_temperature(),
            gravity_g: default_gravity_g(),
            radiation: default_radiation(),
            subtype: default_subtype(),
            tidally_locked: default_tidally_locked(),
            axial_tilt_deg: default_axial_tilt_deg(),
            rotation_period_hours: default_rotation_period_hours(),
            moon_count: default_moon_count(),
            parent_body: None,
            modifiers: Vec::new(),
            deposits: Vec::new(),
            surveyed: false,
            candidate_site_name: None,
            belt_profile: None,
        }
    }

    /// Composite habitability score (0–100) derived from the body's attributes.
    ///
    /// The score is a weighted sum of atmosphere / temperature / gravity /
    /// radiation contributions. It is deterministic from the attributes so it
    /// never drifts out of sync with authored content.
    ///
    /// Atmosphere's contribution (issue #197) is `density_points × hazard_multiplier`:
    /// density carries the same 0-30 point scale the old flat `Atmosphere` enum
    /// used, and hazard scales it down — `Toxic` always zeroes it out
    /// regardless of density, matching the old enum's `Toxic => 0.0` exactly.
    #[must_use]
    pub fn habitability(&self) -> u8 {
        self.habitability_with_mitigations(&std::collections::HashSet::new())
    }

    /// Composite habitability score (0–100), as [`Self::habitability`], but
    /// treating every attribute in `mitigations` as its best band (issue #185).
    ///
    /// Used to apply tech-driven mitigation (e.g. radiation shielding) without
    /// altering the body's underlying attributes — the base score stays
    /// authoritative and available via [`Self::habitability`].
    #[must_use]
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    pub fn habitability_with_mitigations(
        &self,
        mitigations: &std::collections::HashSet<HabitabilityAttribute>,
    ) -> u8 {
        let density_points = if mitigations.contains(&HabitabilityAttribute::AtmosphereDensity) {
            30.0
        } else {
            match self.atmosphere_density {
                AtmosphereDensity::Breathable => 30.0,
                AtmosphereDensity::Thin => 15.0,
                AtmosphereDensity::Dense => 10.0,
                AtmosphereDensity::Vacuum => 5.0,
            }
        };
        let hazard_multiplier = if mitigations.contains(&HabitabilityAttribute::AtmosphereHazard) {
            1.0
        } else {
            match self.atmosphere_hazard {
                AtmosphereHazard::None => 1.0,
                AtmosphereHazard::Corrosive => 0.5,
                AtmosphereHazard::OxidizingCombustible => 0.25,
                AtmosphereHazard::Toxic => 0.0,
            }
        };
        let atmo = density_points * hazard_multiplier;
        let temp = if mitigations.contains(&HabitabilityAttribute::Temperature) {
            30.0
        } else {
            match self.temperature {
                TemperatureBand::Temperate => 30.0,
                TemperatureBand::Cold => 20.0,
                TemperatureBand::Hot => 15.0,
                TemperatureBand::Frozen => 5.0,
                TemperatureBand::Extreme => 0.0,
            }
        };
        // Gravity contributes up to 20 pts, peaking near 1.0 g. Not
        // mitigable — see [`HabitabilityAttribute`]'s doc comment.
        let g = self.gravity_g.clamp(0.0, 3.0);
        let grav = (20.0 - (g - 1.0).abs() * 20.0).max(0.0);
        let rad = if mitigations.contains(&HabitabilityAttribute::Radiation) {
            20.0
        } else {
            match self.radiation {
                RadiationLevel::Low => 20.0,
                RadiationLevel::Moderate => 10.0,
                RadiationLevel::High => 0.0,
            }
        };
        (atmo + temp + grav + rad).round().clamp(0.0, 100.0) as u8
    }

    /// Productivity multiplier derived from [`Self::habitability`].
    ///
    /// Range: `[0.75, 1.25]` — a habitability of `50` maps to `1.0` (neutral),
    /// `0` to `0.75` (−25 %), and `100` to `1.25` (+25 %). Applied as a
    /// multiplicative scalar on colony production outputs.
    #[must_use]
    pub fn habitability_modifier(&self) -> f32 {
        0.75 + f32::from(self.habitability()) * 0.005
    }

    /// [`Self::habitability_modifier`], but computed from
    /// [`Self::habitability_with_mitigations`] (issue #185).
    #[must_use]
    pub fn habitability_modifier_with_mitigations(
        &self,
        mitigations: &std::collections::HashSet<HabitabilityAttribute>,
    ) -> f32 {
        0.75 + f32::from(self.habitability_with_mitigations(mitigations)) * 0.005
    }

    /// Authored per-category production modifier for `category` (issue
    /// #184), defaulting to `1.0` (neutral) when unauthored. See
    /// [`category_modifier`].
    #[must_use]
    pub fn category_modifier(&self, category: YieldCategory) -> f32 {
        category_modifier(&self.modifiers, category)
    }

    /// Whether this body's habitability clears the founding threshold
    /// without a tech-unlocked override (issue #183).
    #[must_use]
    pub fn meets_founding_threshold(&self) -> bool {
        self.habitability() >= HABITABILITY_FOUNDING_THRESHOLD
    }
}

/// Minimum [`Body::habitability`] score required to found a colony without
/// the [`HARSH_WORLD_CAPABILITY_ID`] capability unlocked (issue #183).
pub const HABITABILITY_FOUNDING_THRESHOLD: u8 = 30;

/// Sols per calendar month, used to convert month-denominated travel times into
/// the sol cadence shipments now tick on (issue #332).
///
/// Mirrors [`crate::turn::DEFAULT_SOLS_PER_MONTH`]. Kept as a separate constant
/// because `system` must not depend on the turn processor's configured cadence —
/// a route's distance-derived travel time is a physical quantity, not a
/// function of how fast the player is advancing turns.
pub const SOLS_PER_MONTH: u32 = 30;

/// Capability slug (see [`crate::tech::TechEffect::UnlockCapability`]) that
/// overrides [`HABITABILITY_FOUNDING_THRESHOLD`], letting the player found
/// on any body regardless of habitability score once researched.
pub const HARSH_WORLD_CAPABILITY_ID: &str = "colonize_harsh_worlds";

/// A directed edge in the system node map representing a shipping route.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShippingRoute {
    /// Stable identifier.
    pub id: Uuid,
    /// Origin body.
    pub from: BodyId,
    /// Destination body.
    pub to: BodyId,
    /// Travel time in strategic months (derived from distance + propulsion tech).
    pub travel_time_months: u32,
}

/// System node map: bodies as nodes, shipping routes as edges.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemNodeMap {
    /// Display name of the star system (e.g. `"Vega"`); body names are derived
    /// from it. Set when the system is generated; empty until then.
    #[serde(default)]
    pub system_name: String,
    /// All celestial bodies in the system.
    pub bodies: HashMap<BodyId, Body>,
    /// Directed shipping routes between bodies.
    pub routes: HashMap<Uuid, ShippingRoute>,
    /// Propulsion technology level (1 = basic, higher = faster travel).
    pub propulsion_level: u32,
}

impl Default for SystemNodeMap {
    fn default() -> Self {
        Self::new()
    }
}

impl SystemNodeMap {
    /// Create an empty system node map.
    #[must_use]
    pub fn new() -> Self {
        Self {
            system_name: String::new(),
            bodies: HashMap::new(),
            routes: HashMap::new(),
            propulsion_level: 1,
        }
    }

    /// Add a body to the map and return its id.
    pub fn add_body(&mut self, body: Body) -> BodyId {
        let id = body.id.clone();
        self.bodies.insert(id.clone(), body);
        id
    }

    /// Compute travel time in strategic months between two bodies based on distance and propulsion.
    ///
    /// `travel_time = ceil(distance_au / propulsion_level)`, minimum 1.
    #[must_use]
    pub fn compute_travel_time(&self, from: &BodyId, to: &BodyId) -> Option<u32> {
        let a = self.bodies.get(from)?;
        let b = self.bodies.get(to)?;
        let dist = (a.distance_au - b.distance_au).abs();
        #[allow(
            clippy::cast_precision_loss,
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss
        )]
        let months = (dist / self.propulsion_level as f32).ceil() as u32;
        Some(months.max(1))
    }

    /// Add a shipping route between two bodies; computes travel time automatically.
    ///
    /// Returns `None` if either body does not exist.
    pub fn add_route(&mut self, from: BodyId, to: BodyId) -> Option<Uuid> {
        let travel_time = self.compute_travel_time(&from, &to)?;
        let id = Uuid::new_v4();
        self.routes.insert(
            id,
            ShippingRoute {
                id,
                from,
                to,
                travel_time_months: travel_time,
            },
        );
        Some(id)
    }
}

// ─── Hauler Fleet & Capacity ──────────────────────────────────────────────────

/// A cargo ship (hauler) available in the system fleet.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hauler {
    /// Stable identifier.
    pub id: Uuid,
    /// Maximum cargo units this ship can carry per trip.
    pub capacity: f64,
    /// Whether the hauler is currently busy on a shipping mission.
    pub in_transit: bool,
}

/// System-wide hauler fleet: managed resource for inter-body logistics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HaulerFleet {
    /// All haulers in the system.
    pub haulers: HashMap<Uuid, Hauler>,
}

impl HaulerFleet {
    /// Create an empty fleet.
    #[must_use]
    pub fn new() -> Self {
        Self {
            haulers: HashMap::new(),
        }
    }

    /// Register a new hauler with the given capacity and return its id.
    pub fn add_hauler(&mut self, capacity: f64) -> Uuid {
        let id = Uuid::new_v4();
        self.haulers.insert(
            id,
            Hauler {
                id,
                capacity,
                in_transit: false,
            },
        );
        id
    }

    /// Total capacity available (free haulers only).
    #[must_use]
    pub fn available_capacity(&self) -> f64 {
        self.haulers
            .values()
            .filter(|h| !h.in_transit)
            .map(|h| h.capacity)
            .sum()
    }

    /// Total fleet capacity (all haulers).
    #[must_use]
    pub fn total_capacity(&self) -> f64 {
        self.haulers.values().map(|h| h.capacity).sum()
    }
}

// ─── Cargo Shipment ───────────────────────────────────────────────────────────

/// A in-transit cargo shipment between two bodies.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CargoShipment {
    /// Stable identifier.
    pub id: Uuid,
    /// Origin body.
    pub from: BodyId,
    /// Destination body.
    pub to: BodyId,
    /// Commodity and quantity being shipped.
    pub cargo: Vec<(String, f64)>,
    /// Hauler assigned to this shipment.
    pub hauler_id: Uuid,
    /// **Sols** remaining until arrival (issue #332).
    ///
    /// Was strategic months, ticked once per 30 sols. Now ticked every sol, with
    /// the route's `travel_time_months` converted at dispatch, so a voyage takes
    /// the same elapsed time — it just reports progress every sol instead of
    /// jumping in 30-sol steps.
    pub turns_remaining: u32,
    /// Colony that should receive this cargo on arrival (if any).
    pub destination_colony: Option<crate::colony::ColonyId>,
}

// ─── Megaproject Framework ────────────────────────────────────────────────────

/// Stable identifier for a megaproject.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MegaprojectId(pub Uuid);

impl MegaprojectId {
    /// Create a new random [`MegaprojectId`].
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for MegaprojectId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for MegaprojectId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A single milestone within a megaproject.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MegaprojectMilestone {
    /// Milestone index (0-based, must be completed in order).
    pub index: u32,
    /// Human-readable label for this milestone.
    pub label: String,
    /// Commodity costs required to complete this milestone.
    pub resource_cost: Vec<(String, f64)>,
    /// Research points required to unlock this milestone.
    pub research_cost: f32,
    /// Resources already contributed toward this milestone.
    pub contributed: Vec<(String, f64)>,
    /// Research already contributed.
    pub research_contributed: f32,
    /// Whether this milestone has been completed.
    pub completed: bool,
}

impl MegaprojectMilestone {
    /// Returns true when all resource and research requirements have been met.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        if self.completed {
            return true;
        }
        // Check research
        if self.research_contributed < self.research_cost {
            return false;
        }
        // Check each commodity requirement
        for (commodity, required) in &self.resource_cost {
            let contributed: f64 = self
                .contributed
                .iter()
                .filter(|(c, _)| c == commodity)
                .map(|(_, q)| *q)
                .sum();
            if contributed < *required {
                return false;
            }
        }
        true
    }
}

/// Category of a megaproject.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MegaprojectKind {
    /// The capstone victory condition: launch an interstellar expedition.
    InterstellarExpedition,
    /// Build a wormhole gate connecting to another system.
    WormholeGate,
    /// Deploy a terraforming engine on a body.
    TerraformingEngine,
    /// Construct a system-scale power array.
    SystemPowerArray,
    /// A custom player-defined megaproject.
    Custom(String),
}

/// A megaproject: a multi-milestone construction pooled from the entire system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Megaproject {
    /// Stable identifier.
    pub id: MegaprojectId,
    /// Display name.
    pub name: String,
    /// Category of megaproject.
    pub kind: MegaprojectKind,
    /// Ordered list of milestones (must be completed in sequence).
    pub milestones: Vec<MegaprojectMilestone>,
    /// Whether the entire project has been completed (all milestones done).
    pub completed: bool,
}

impl Megaproject {
    /// Index of the next incomplete milestone, or `None` if all are complete.
    #[must_use]
    pub fn next_milestone_index(&self) -> Option<usize> {
        self.milestones.iter().position(|m| !m.completed)
    }

    /// Number of completed milestones.
    #[must_use]
    pub fn completed_milestones(&self) -> usize {
        self.milestones.iter().filter(|m| m.completed).count()
    }

    /// Returns true when all milestones have been completed.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.milestones.iter().all(|m| m.completed)
    }
}

// ─── Transport Capacity ───────────────────────────────────────────────────────

/// Passenger-transport capacity for migration batches (§6A / §8.3).
///
/// `haulers * colonists_per_hauler` gives the maximum colonists that can move
/// on a single route in one strategic month.  Excess demand is deferred to the
/// next batch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransportCapacity {
    /// Number of passenger haulers in the migration fleet.
    pub haulers: u32,
    /// Maximum colonists each hauler can carry per strategic month.
    pub colonists_per_hauler: u32,
}

impl TransportCapacity {
    /// Total colonist slots available per route per strategic month.
    #[must_use]
    pub fn total(&self) -> u32 {
        self.haulers.saturating_mul(self.colonists_per_hauler)
    }
}

impl Default for TransportCapacity {
    fn default() -> Self {
        // Default: 2 haulers × 50 colonists each = 100 per route per month.
        Self {
            haulers: 2,
            colonists_per_hauler: 50,
        }
    }
}

// ─── System State ─────────────────────────────────────────────────────────────

/// Top-level state container for the system zoom layer.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SystemState {
    /// Node map of bodies and shipping routes.
    pub node_map: SystemNodeMap,
    /// Fleet of cargo haulers (managed resource).
    pub hauler_fleet: HaulerFleet,
    /// Active and completed cargo shipments.
    pub shipments: HashMap<Uuid, CargoShipment>,
    /// All megaprojects (active and completed).
    pub megaprojects: HashMap<MegaprojectId, Megaproject>,
    /// Passenger-transport capacity for migration batches.
    pub transport_capacity: TransportCapacity,
}

impl SystemState {
    /// Create an empty system state.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

// ─── System Commands ──────────────────────────────────────────────────────────

/// Commands that mutate the system zoom layer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SystemCommand {
    /// Register a new celestial body in the system node map.
    AddBody {
        /// Display name.
        name: String,
        /// Kind of body.
        kind: BodyKind,
        /// Distance from system primary in AU.
        distance_au: f32,
    },
    /// Assign a system-role to a body.
    AssignRole {
        /// Target body.
        body_id: BodyId,
        /// New role to assign.
        role: SystemRole,
    },
    /// Overwrite a body's environmental attributes.
    ///
    /// Used by the content pack loader to populate authored attributes after
    /// [`SystemCommand::AddBody`] creates the body with neutral defaults.
    SetBodyAttributes {
        /// Target body.
        body_id: BodyId,
        /// Atmospheric thickness/density band (issue #197).
        atmosphere_density: AtmosphereDensity,
        /// Atmospheric chemical hazard band (issue #197).
        atmosphere_hazard: AtmosphereHazard,
        /// Surface temperature band.
        temperature: TemperatureBand,
        /// Surface gravity as a fraction of Earth-g.
        gravity_g: f32,
        /// Ambient radiation exposure level.
        radiation: RadiationLevel,
        /// Surface/composition archetype (issue #196).
        subtype: PlanetarySubtype,
        /// Whether rotation is tidally locked to the orbit.
        tidally_locked: bool,
        /// Axial tilt in degrees.
        axial_tilt_deg: f32,
        /// Rotation period in hours.
        rotation_period_hours: f32,
        /// Number of natural satellites orbiting this body.
        moon_count: u32,
    },
    /// Link a body to the body it orbits (issue #196).
    ///
    /// Separate from [`SystemCommand::SetBodyAttributes`] because the
    /// parent must already exist as a live [`Body`] with an assigned
    /// [`BodyId`] — content loading resolves this in a second pass after
    /// every body in a system has been added via
    /// [`SystemCommand::AddBody`], so an authored body can name a parent
    /// defined later in the same system.
    SetBodyParent {
        /// Target body.
        body_id: BodyId,
        /// The body `body_id` orbits.
        parent_body: BodyId,
    },
    /// Overwrite a body's per-category production modifiers (issue #184).
    ///
    /// Separate from [`SystemCommand::SetBodyAttributes`] since most
    /// authored bodies have no modifiers at all (empty `Vec` is the
    /// neutral default) — folding this into the attributes command would
    /// force every existing caller to thread through an always-empty field.
    SetBodyModifiers {
        /// Target body.
        body_id: BodyId,
        /// Replacement modifier list.
        modifiers: Vec<BodyModifier>,
    },
    /// Procedurally generate a star system from a seed (issue #199),
    /// replacing every existing body in the node map.
    ///
    /// Mirrors [`crate::Command::SeedPlanet`]'s "overwrite any previously
    /// seeded state" semantics, but scoped to the system-command surface
    /// (see [`crate::system_gen::generate_system`] for the generator and
    /// its playability guarantee).
    GenerateSystem {
        /// Deterministic seed — same seed always produces the same system.
        seed: u64,
        /// Deposit-generosity multiplier (issue #232), resolved by the
        /// caller from the active difficulty preset's
        /// [`crate::modifier::ModifiableQuantity::DepositAbundance`] scalar
        /// before constructing this command — `apply_system_command`
        /// operates on [`SystemState`] alone and has no reach into the
        /// difficulty scalar living on the outer `GameState`. `1.0` is
        /// neutral (the default when omitted, e.g. by older callers/saves).
        #[serde(default = "default_abundance_scalar")]
        abundance_scalar: f32,
        /// Distance (AU) treated as the center of the habitable zone
        /// (playtest feedback: exposed as a New Game slider). Defaults to
        /// [`crate::system_gen::SystemGenParams::default`]'s value.
        #[serde(default = "default_habitable_zone_center_au")]
        habitable_zone_center_au: f32,
        /// Minimum inner-planet (rocky world) count.
        #[serde(default = "default_min_inner_planets")]
        min_inner_planets: u32,
        /// Maximum inner-planet (rocky world) count.
        #[serde(default = "default_max_inner_planets")]
        max_inner_planets: u32,
    },
    /// Add a shipping route between two bodies (travel time auto-computed).
    AddShippingRoute {
        /// Origin body.
        from: BodyId,
        /// Destination body.
        to: BodyId,
    },
    /// Add a hauler to the system fleet.
    AddHauler {
        /// Maximum cargo units this hauler can carry.
        capacity: f64,
    },
    /// Dispatch a cargo shipment between two bodies.
    ///
    /// Fails if insufficient free hauler capacity is available.
    DispatchShipment {
        /// Origin body.
        from: BodyId,
        /// Destination body.
        to: BodyId,
        /// Commodity and quantity being shipped.
        cargo: Vec<(String, f64)>,
        /// Colony that should receive the cargo on arrival (optional).
        destination_colony: Option<crate::colony::ColonyId>,
    },
    /// Advance all in-transit shipments by one strategic month.
    ///
    /// Shipments that reach zero turns remaining are marked for arrival.
    AdvanceShipments,
    /// Register a new megaproject.
    RegisterMegaproject {
        /// Display name.
        name: String,
        /// Category of megaproject.
        kind: MegaprojectKind,
        /// Ordered milestone specifications.
        milestones: Vec<MilestoneSpec>,
    },
    /// Contribute resources to the current active milestone of a megaproject.
    ContributeToMegaproject {
        /// Target megaproject.
        project_id: MegaprojectId,
        /// Commodity contributions.
        resources: Vec<(String, f64)>,
        /// Research point contribution.
        research: f32,
    },
    /// Upgrade the propulsion technology level, reducing travel time across all routes.
    UpgradePropulsion {
        /// New propulsion level (must be higher than current).
        new_level: u32,
    },
}

/// Specification for a milestone when registering a megaproject.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MilestoneSpec {
    /// Human-readable label.
    pub label: String,
    /// Commodity costs.
    pub resource_cost: Vec<(String, f64)>,
    /// Research points required.
    pub research_cost: f32,
}

// ─── System Events ────────────────────────────────────────────────────────────

/// Events produced by the system zoom layer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SystemEvent {
    /// A new body was added to the system node map.
    BodyAdded {
        /// Assigned stable identifier.
        body_id: BodyId,
        /// Display name.
        name: String,
        /// Kind of body.
        kind: BodyKind,
        /// Distance from primary in AU.
        distance_au: f32,
    },
    /// A system-role was assigned to a body.
    RoleAssigned {
        /// Target body.
        body_id: BodyId,
        /// Previous role (for undo/display).
        previous_role: SystemRole,
        /// New role.
        new_role: SystemRole,
    },
    /// A body's environmental attributes were overwritten.
    BodyAttributesSet {
        /// Target body.
        body_id: BodyId,
        /// Atmospheric thickness/density band (issue #197).
        atmosphere_density: AtmosphereDensity,
        /// Atmospheric chemical hazard band (issue #197).
        atmosphere_hazard: AtmosphereHazard,
        /// Surface temperature band.
        temperature: TemperatureBand,
        /// Surface gravity as a fraction of Earth-g.
        gravity_g: f32,
        /// Ambient radiation exposure level.
        radiation: RadiationLevel,
        /// Surface/composition archetype.
        subtype: PlanetarySubtype,
        /// Whether rotation is tidally locked to the orbit.
        tidally_locked: bool,
        /// Axial tilt in degrees.
        axial_tilt_deg: f32,
        /// Rotation period in hours.
        rotation_period_hours: f32,
        /// Number of natural satellites orbiting this body.
        moon_count: u32,
    },
    /// A body was linked to the body it orbits.
    BodyParentSet {
        /// Target body.
        body_id: BodyId,
        /// The body `body_id` orbits.
        parent_body: BodyId,
    },
    /// A body's per-category production modifiers were overwritten (issue #184).
    BodyModifiersSet {
        /// Target body.
        body_id: BodyId,
        /// New modifier list.
        modifiers: Vec<BodyModifier>,
    },
    /// A star system was procedurally generated (issue #199), replacing
    /// every previously existing body.
    SystemGenerated {
        /// The seed used to generate this system.
        seed: u64,
        /// Ids of every generated body, in generation order.
        body_ids: Vec<BodyId>,
    },
    /// A shipping route was added between two bodies.
    ShippingRouteAdded {
        /// Route identifier.
        route_id: Uuid,
        /// Origin body.
        from: BodyId,
        /// Destination body.
        to: BodyId,
        /// Computed travel time in strategic months.
        travel_time_months: u32,
    },
    /// A hauler was added to the fleet.
    HaulerAdded {
        /// Assigned hauler identifier.
        hauler_id: Uuid,
        /// Cargo capacity.
        capacity: f64,
    },
    /// A cargo shipment was dispatched.
    ShipmentDispatched {
        /// Shipment identifier.
        shipment_id: Uuid,
        /// Origin body.
        from: BodyId,
        /// Destination body.
        to: BodyId,
        /// Cargo manifest.
        cargo: Vec<(String, f64)>,
        /// Hauler assigned to this mission.
        hauler_id: Uuid,
        /// Travel time in strategic months.
        travel_time_months: u32,
    },
    /// A cargo shipment arrived at its destination.
    ShipmentArrived {
        /// Shipment identifier.
        shipment_id: Uuid,
        /// Destination body.
        to: BodyId,
        /// Delivered cargo.
        cargo: Vec<(String, f64)>,
        /// Hauler that completed the mission (now free again).
        hauler_id: Uuid,
    },
    /// A megaproject was registered.
    MegaprojectRegistered {
        /// Assigned stable identifier.
        project_id: MegaprojectId,
        /// Display name.
        name: String,
        /// Category.
        kind: MegaprojectKind,
        /// Number of milestones.
        milestone_count: usize,
    },
    /// Resources were contributed to a megaproject milestone.
    MegaprojectContribution {
        /// Target megaproject.
        project_id: MegaprojectId,
        /// Milestone index that received the contribution.
        milestone_index: u32,
        /// Resources contributed.
        resources: Vec<(String, f64)>,
        /// Research contributed.
        research: f32,
    },
    /// A megaproject milestone was completed.
    MilestoneCompleted {
        /// Target megaproject.
        project_id: MegaprojectId,
        /// Index of the milestone that completed.
        milestone_index: u32,
        /// Label of the completed milestone.
        label: String,
    },
    /// A megaproject was fully completed (all milestones done).
    MegaprojectCompleted {
        /// The completed megaproject.
        project_id: MegaprojectId,
        /// Display name.
        name: String,
        /// Kind.
        kind: MegaprojectKind,
    },
    /// Propulsion technology was upgraded.
    PropulsionUpgraded {
        /// New propulsion level.
        new_level: u32,
    },
}

// ─── System Errors ────────────────────────────────────────────────────────────

/// Errors from the system zoom layer.
#[derive(Debug, Error)]
pub enum SystemError {
    /// The referenced body does not exist.
    #[error("body not found: {0}")]
    BodyNotFound(BodyId),
    /// The requested `PlanetarySubtype` isn't valid for the body's `BodyKind`.
    #[error("planetary subtype {subtype:?} is not valid for body kind {kind:?}")]
    IncompatiblePlanetarySubtype {
        /// The body kind involved.
        kind: BodyKind,
        /// The rejected subtype.
        subtype: PlanetarySubtype,
    },
    /// The referenced megaproject does not exist.
    #[error("megaproject not found: {0}")]
    MegaprojectNotFound(MegaprojectId),
    /// Not enough free hauler capacity to dispatch the shipment.
    #[error("insufficient hauler capacity: need {needed:.1} but only {available:.1} free")]
    InsufficientHaulerCapacity {
        /// Cargo units required.
        needed: f64,
        /// Free capacity available.
        available: f64,
    },
    /// No shipping route exists between the two bodies.
    #[error("no shipping route between {from} and {to}")]
    NoRouteFound {
        /// Origin.
        from: BodyId,
        /// Destination.
        to: BodyId,
    },
    /// The megaproject has already been completed.
    #[error("megaproject already completed: {0}")]
    MegaprojectAlreadyComplete(MegaprojectId),
    /// All milestones are complete (should not normally be reached via normal flow).
    #[error("no active milestone in megaproject {0}")]
    NoActiveMilestone(MegaprojectId),
    /// The new propulsion level is not higher than the current level.
    #[error("propulsion upgrade must be higher than current level {current}")]
    PropulsionDowngrade {
        /// Current level.
        current: u32,
    },
    /// A command argument is out of range or otherwise invalid.
    #[error("invalid argument: {0}")]
    InvalidArgument(String),
}

// ─── System Command Processor ─────────────────────────────────────────────────

/// Apply a [`SystemCommand`] to [`SystemState`] and return resulting [`SystemEvent`]s.
///
/// # Errors
///
/// Returns [`SystemError`] when the command cannot be applied.
///
/// # Panics
///
/// Panics if capacity-check logic is inconsistent and no hauler is assigned despite availability.
#[allow(clippy::too_many_lines)]
pub fn apply_system_command(
    state: &mut SystemState,
    cmd: &SystemCommand,
) -> Result<Vec<SystemEvent>, SystemError> {
    match cmd {
        SystemCommand::AddBody {
            name,
            kind,
            distance_au,
        } => {
            if *distance_au < 0.0 {
                return Err(SystemError::InvalidArgument(
                    "distance_au must be non-negative".into(),
                ));
            }
            let body = Body::new(name.clone(), kind.clone(), *distance_au);
            let body_id = body.id.clone();
            state.node_map.add_body(body);
            Ok(vec![SystemEvent::BodyAdded {
                body_id,
                name: name.clone(),
                kind: kind.clone(),
                distance_au: *distance_au,
            }])
        }

        SystemCommand::SetBodyAttributes {
            body_id,
            atmosphere_density,
            atmosphere_hazard,
            temperature,
            gravity_g,
            radiation,
            subtype,
            tidally_locked,
            axial_tilt_deg,
            rotation_period_hours,
            moon_count,
        } => {
            let body = state
                .node_map
                .bodies
                .get_mut(body_id)
                .ok_or_else(|| SystemError::BodyNotFound(body_id.clone()))?;
            if !subtype.compatible_with(&body.kind) {
                return Err(SystemError::IncompatiblePlanetarySubtype {
                    kind: body.kind.clone(),
                    subtype: *subtype,
                });
            }
            body.atmosphere_density = *atmosphere_density;
            body.atmosphere_hazard = *atmosphere_hazard;
            body.temperature = *temperature;
            body.gravity_g = *gravity_g;
            body.radiation = *radiation;
            body.subtype = *subtype;
            body.tidally_locked = *tidally_locked;
            body.axial_tilt_deg = *axial_tilt_deg;
            body.rotation_period_hours = *rotation_period_hours;
            body.moon_count = *moon_count;
            Ok(vec![SystemEvent::BodyAttributesSet {
                body_id: body_id.clone(),
                atmosphere_density: *atmosphere_density,
                atmosphere_hazard: *atmosphere_hazard,
                temperature: *temperature,
                gravity_g: *gravity_g,
                radiation: *radiation,
                subtype: *subtype,
                tidally_locked: *tidally_locked,
                axial_tilt_deg: *axial_tilt_deg,
                rotation_period_hours: *rotation_period_hours,
                moon_count: *moon_count,
            }])
        }

        SystemCommand::SetBodyParent {
            body_id,
            parent_body,
        } => {
            if !state.node_map.bodies.contains_key(parent_body) {
                return Err(SystemError::BodyNotFound(parent_body.clone()));
            }
            let body = state
                .node_map
                .bodies
                .get_mut(body_id)
                .ok_or_else(|| SystemError::BodyNotFound(body_id.clone()))?;
            body.parent_body = Some(parent_body.clone());
            Ok(vec![SystemEvent::BodyParentSet {
                body_id: body_id.clone(),
                parent_body: parent_body.clone(),
            }])
        }

        SystemCommand::SetBodyModifiers { body_id, modifiers } => {
            let body = state
                .node_map
                .bodies
                .get_mut(body_id)
                .ok_or_else(|| SystemError::BodyNotFound(body_id.clone()))?;
            body.modifiers.clone_from(modifiers);
            Ok(vec![SystemEvent::BodyModifiersSet {
                body_id: body_id.clone(),
                modifiers: modifiers.clone(),
            }])
        }

        SystemCommand::GenerateSystem {
            seed,
            abundance_scalar,
            habitable_zone_center_au,
            min_inner_planets,
            max_inner_planets,
        } => {
            state.node_map.bodies.clear();
            state.node_map.system_name = crate::system_gen::system_name_for_seed(*seed).to_string();
            let gen_params = crate::system_gen::SystemGenParams {
                habitable_zone_center_au: *habitable_zone_center_au,
                min_inner_planets: *min_inner_planets,
                max_inner_planets: *max_inner_planets,
                abundance_scalar: *abundance_scalar,
            };
            let generated = crate::system_gen::generate_system(*seed, &gen_params);
            let body_ids: Vec<BodyId> = generated
                .into_iter()
                .map(|body| state.node_map.add_body(body))
                .collect();
            Ok(vec![SystemEvent::SystemGenerated {
                seed: *seed,
                body_ids,
            }])
        }

        SystemCommand::AssignRole { body_id, role } => {
            let body = state
                .node_map
                .bodies
                .get_mut(body_id)
                .ok_or_else(|| SystemError::BodyNotFound(body_id.clone()))?;
            let previous_role = body.role.clone();
            body.role = role.clone();
            Ok(vec![SystemEvent::RoleAssigned {
                body_id: body_id.clone(),
                previous_role,
                new_role: role.clone(),
            }])
        }

        SystemCommand::AddShippingRoute { from, to } => {
            // Verify both bodies exist and compute travel time before mutating
            let travel_time = state
                .node_map
                .compute_travel_time(from, to)
                .ok_or_else(|| SystemError::NoRouteFound {
                    from: from.clone(),
                    to: to.clone(),
                })?;
            let route_id = Uuid::new_v4();
            state.node_map.routes.insert(
                route_id,
                ShippingRoute {
                    id: route_id,
                    from: from.clone(),
                    to: to.clone(),
                    travel_time_months: travel_time,
                },
            );
            Ok(vec![SystemEvent::ShippingRouteAdded {
                route_id,
                from: from.clone(),
                to: to.clone(),
                travel_time_months: travel_time,
            }])
        }

        SystemCommand::AddHauler { capacity } => {
            if *capacity <= 0.0 {
                return Err(SystemError::InvalidArgument(
                    "hauler capacity must be positive".into(),
                ));
            }
            let hauler_id = state.hauler_fleet.add_hauler(*capacity);
            Ok(vec![SystemEvent::HaulerAdded {
                hauler_id,
                capacity: *capacity,
            }])
        }

        SystemCommand::DispatchShipment {
            from,
            to,
            cargo,
            destination_colony,
        } => {
            // Verify bodies exist
            if !state.node_map.bodies.contains_key(from) {
                return Err(SystemError::BodyNotFound(from.clone()));
            }
            if !state.node_map.bodies.contains_key(to) {
                return Err(SystemError::BodyNotFound(to.clone()));
            }
            // Compute total cargo size
            let total: f64 = cargo.iter().map(|(_, q)| *q).sum();
            if total <= 0.0 {
                return Err(SystemError::InvalidArgument(
                    "cargo total must be positive".into(),
                ));
            }
            // Check available hauler capacity
            let available = state.hauler_fleet.available_capacity();
            if available < total {
                return Err(SystemError::InsufficientHaulerCapacity {
                    needed: total,
                    available,
                });
            }
            // Find a route between the bodies
            let travel_time = state
                .node_map
                .compute_travel_time(from, to)
                .ok_or_else(|| SystemError::NoRouteFound {
                    from: from.clone(),
                    to: to.clone(),
                })?;
            // Assign free haulers greedily until cargo is covered
            let mut remaining = total;
            let mut assigned_hauler_id = None;
            for hauler in state.hauler_fleet.haulers.values_mut() {
                if hauler.in_transit || remaining <= 0.0 {
                    continue;
                }
                let take = remaining.min(hauler.capacity);
                remaining -= take;
                hauler.in_transit = true;
                assigned_hauler_id = Some(hauler.id);
                if remaining <= 0.0 {
                    break;
                }
            }
            let hauler_id = assigned_hauler_id.expect("capacity check guarantees a hauler");
            let shipment_id = Uuid::new_v4();
            state.shipments.insert(
                shipment_id,
                CargoShipment {
                    id: shipment_id,
                    from: from.clone(),
                    to: to.clone(),
                    cargo: cargo.clone(),
                    hauler_id,
                    // `compute_travel_time` is distance-derived and expressed in
                    // months; shipments tick per sol now (issue #332), so
                    // convert once here rather than teaching the tick loop about
                    // two units.
                    turns_remaining: travel_time.saturating_mul(SOLS_PER_MONTH),
                    destination_colony: *destination_colony,
                },
            );
            Ok(vec![SystemEvent::ShipmentDispatched {
                shipment_id,
                from: from.clone(),
                to: to.clone(),
                cargo: cargo.clone(),
                hauler_id,
                travel_time_months: travel_time,
            }])
        }

        SystemCommand::AdvanceShipments => {
            // Pass 1: decrement all turns_remaining; collect IDs that reach zero.
            let mut arrived_ids: Vec<Uuid> = Vec::new();
            for (id, shipment) in &mut state.shipments {
                if shipment.turns_remaining > 0 {
                    shipment.turns_remaining -= 1;
                }
                if shipment.turns_remaining == 0 {
                    arrived_ids.push(*id);
                }
            }
            // Pass 2: remove arrived shipments and emit events.
            let mut events = Vec::new();
            for id in arrived_ids {
                let arrived = state.shipments.remove(&id).unwrap();
                if let Some(hauler) = state.hauler_fleet.haulers.get_mut(&arrived.hauler_id) {
                    hauler.in_transit = false;
                }
                events.push(SystemEvent::ShipmentArrived {
                    shipment_id: arrived.id,
                    to: arrived.to.clone(),
                    cargo: arrived.cargo.clone(),
                    hauler_id: arrived.hauler_id,
                });
            }
            Ok(events)
        }

        SystemCommand::RegisterMegaproject {
            name,
            kind,
            milestones,
        } => {
            if milestones.is_empty() {
                return Err(SystemError::InvalidArgument(
                    "megaproject must have at least one milestone".into(),
                ));
            }
            let project_id = MegaprojectId::new();
            let milestone_count = milestones.len();
            let built_milestones: Vec<MegaprojectMilestone> = milestones
                .iter()
                .enumerate()
                .map(|(i, spec)| MegaprojectMilestone {
                    #[allow(clippy::cast_possible_truncation)]
                    index: i as u32,
                    label: spec.label.clone(),
                    resource_cost: spec.resource_cost.clone(),
                    research_cost: spec.research_cost,
                    contributed: Vec::new(),
                    research_contributed: 0.0,
                    completed: false,
                })
                .collect();
            state.megaprojects.insert(
                project_id.clone(),
                Megaproject {
                    id: project_id.clone(),
                    name: name.clone(),
                    kind: kind.clone(),
                    milestones: built_milestones,
                    completed: false,
                },
            );
            Ok(vec![SystemEvent::MegaprojectRegistered {
                project_id,
                name: name.clone(),
                kind: kind.clone(),
                milestone_count,
            }])
        }

        SystemCommand::ContributeToMegaproject {
            project_id,
            resources,
            research,
        } => {
            let project = state
                .megaprojects
                .get_mut(project_id)
                .ok_or_else(|| SystemError::MegaprojectNotFound(project_id.clone()))?;
            if project.completed {
                return Err(SystemError::MegaprojectAlreadyComplete(project_id.clone()));
            }
            let milestone_idx = project
                .next_milestone_index()
                .ok_or_else(|| SystemError::NoActiveMilestone(project_id.clone()))?;

            let milestone = &mut project.milestones[milestone_idx];
            // Apply contribution
            milestone.research_contributed += research;
            for (commodity, qty) in resources {
                if let Some(entry) = milestone
                    .contributed
                    .iter_mut()
                    .find(|(c, _)| c == commodity)
                {
                    entry.1 += qty;
                } else {
                    milestone.contributed.push((commodity.clone(), *qty));
                }
            }

            #[allow(clippy::cast_possible_truncation)]
            let milestone_idx_u32 = milestone_idx as u32;
            let mut events = vec![SystemEvent::MegaprojectContribution {
                project_id: project_id.clone(),
                milestone_index: milestone_idx_u32,
                resources: resources.clone(),
                research: *research,
            }];

            // Check if milestone is now complete
            if milestone.is_complete() {
                milestone.completed = true;
                let label = milestone.label.clone();
                events.push(SystemEvent::MilestoneCompleted {
                    project_id: project_id.clone(),
                    milestone_index: milestone_idx_u32,
                    label,
                });
                // Check if entire project is now complete
                if project.is_complete() {
                    project.completed = true;
                    events.push(SystemEvent::MegaprojectCompleted {
                        project_id: project_id.clone(),
                        name: project.name.clone(),
                        kind: project.kind.clone(),
                    });
                }
            }

            Ok(events)
        }

        SystemCommand::UpgradePropulsion { new_level } => {
            let current = state.node_map.propulsion_level;
            if *new_level <= current {
                return Err(SystemError::PropulsionDowngrade { current });
            }
            state.node_map.propulsion_level = *new_level;
            Ok(vec![SystemEvent::PropulsionUpgraded {
                new_level: *new_level,
            }])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_state_with_two_bodies() -> (SystemState, BodyId, BodyId) {
        let mut state = SystemState::new();
        let events = apply_system_command(
            &mut state,
            &SystemCommand::AddBody {
                name: "Inner World".into(),
                kind: BodyKind::InnerPlanet,
                distance_au: 1.0,
            },
        )
        .unwrap();
        let inner_id = match &events[0] {
            SystemEvent::BodyAdded { body_id, .. } => body_id.clone(),
            _ => panic!("expected BodyAdded"),
        };
        let events = apply_system_command(
            &mut state,
            &SystemCommand::AddBody {
                name: "Outer Belt".into(),
                kind: BodyKind::AsteroidBelt,
                distance_au: 3.5,
            },
        )
        .unwrap();
        let outer_id = match &events[0] {
            SystemEvent::BodyAdded { body_id, .. } => body_id.clone(),
            _ => panic!("expected BodyAdded"),
        };
        (state, inner_id, outer_id)
    }

    #[test]
    fn add_body_registers_in_map() {
        let mut state = SystemState::new();
        let events = apply_system_command(
            &mut state,
            &SystemCommand::AddBody {
                name: "New Terra".into(),
                kind: BodyKind::InnerPlanet,
                distance_au: 1.0,
            },
        )
        .unwrap();
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], SystemEvent::BodyAdded { .. }));
        assert_eq!(state.node_map.bodies.len(), 1);
    }

    #[test]
    fn assign_role_changes_body_role() {
        let (mut state, inner_id, _) = make_state_with_two_bodies();
        let events = apply_system_command(
            &mut state,
            &SystemCommand::AssignRole {
                body_id: inner_id.clone(),
                role: SystemRole::Industry,
            },
        )
        .unwrap();
        assert!(matches!(
            events[0],
            SystemEvent::RoleAssigned {
                new_role: SystemRole::Industry,
                ..
            }
        ));
        assert_eq!(state.node_map.bodies[&inner_id].role, SystemRole::Industry);
    }

    #[test]
    fn body_habitability_earthlike_high() {
        // Breathable + temperate + 1.0 g + low radiation → the maximum score.
        let mut body = Body::new("Earth", BodyKind::InnerPlanet, 1.0);
        body.atmosphere_density = AtmosphereDensity::Breathable;
        body.atmosphere_hazard = AtmosphereHazard::None;
        body.temperature = TemperatureBand::Temperate;
        body.gravity_g = 1.0;
        body.radiation = RadiationLevel::Low;
        assert_eq!(body.habitability(), 100);
        assert!((body.habitability_modifier() - 1.25).abs() < 1e-4);
    }

    #[test]
    fn body_habitability_toxic_extreme_low() {
        // Toxic + extreme + 0 g + high radiation → the minimum score.
        let mut body = Body::new("Hellworld", BodyKind::InnerPlanet, 0.2);
        body.atmosphere_density = AtmosphereDensity::Dense;
        body.atmosphere_hazard = AtmosphereHazard::Toxic;
        body.temperature = TemperatureBand::Extreme;
        body.gravity_g = 0.0;
        body.radiation = RadiationLevel::High;
        assert_eq!(body.habitability(), 0);
        assert!((body.habitability_modifier() - 0.75).abs() < 1e-4);
    }

    #[test]
    fn habitability_with_mitigations_no_mitigations_matches_base_score() {
        let mut body = Body::new("Hellworld", BodyKind::InnerPlanet, 0.2);
        body.atmosphere_density = AtmosphereDensity::Dense;
        body.atmosphere_hazard = AtmosphereHazard::Toxic;
        body.temperature = TemperatureBand::Extreme;
        body.gravity_g = 0.0;
        body.radiation = RadiationLevel::High;
        assert_eq!(
            body.habitability_with_mitigations(&std::collections::HashSet::new()),
            body.habitability()
        );
    }

    #[test]
    fn mitigating_radiation_cancels_only_the_radiation_penalty() {
        // High radiation costs 20 pts vs the best band (issue #185). Mitigating
        // it should raise the score by exactly that much and leave every other
        // attribute's contribution untouched.
        let mut body = Body::new("Hellworld", BodyKind::InnerPlanet, 0.2);
        body.atmosphere_density = AtmosphereDensity::Dense;
        body.atmosphere_hazard = AtmosphereHazard::Toxic;
        body.temperature = TemperatureBand::Extreme;
        body.gravity_g = 0.0;
        body.radiation = RadiationLevel::High;
        let base = body.habitability();

        let mut mitigations = std::collections::HashSet::new();
        mitigations.insert(HabitabilityAttribute::Radiation);
        let mitigated = body.habitability_with_mitigations(&mitigations);

        assert_eq!(mitigated, base + 20);
        // The body's own attribute is untouched — base score is unaffected.
        assert_eq!(body.habitability(), base);
    }

    #[test]
    fn mitigating_atmosphere_hazard_cancels_toxic_zero_out() {
        // Toxic hazard zeroes atmosphere contribution regardless of density
        // (hazard_multiplier = 0.0). Mitigating AtmosphereHazard restores the
        // full density-based points.
        let mut body = Body::new("Toxic World", BodyKind::InnerPlanet, 1.0);
        body.atmosphere_density = AtmosphereDensity::Breathable; // 30 pts base
        body.atmosphere_hazard = AtmosphereHazard::Toxic; // ×0.0
        body.temperature = TemperatureBand::Temperate;
        body.gravity_g = 1.0;
        body.radiation = RadiationLevel::Low;
        assert_eq!(body.habitability(), 70); // 0 (atmo) + 30 + 20 + 20

        let mut mitigations = std::collections::HashSet::new();
        mitigations.insert(HabitabilityAttribute::AtmosphereHazard);
        assert_eq!(body.habitability_with_mitigations(&mitigations), 100);
    }

    #[test]
    fn habitability_modifier_with_mitigations_matches_score_formula() {
        let mut body = Body::new("Hellworld", BodyKind::InnerPlanet, 0.2);
        body.atmosphere_density = AtmosphereDensity::Dense;
        body.atmosphere_hazard = AtmosphereHazard::Toxic;
        body.temperature = TemperatureBand::Extreme;
        body.gravity_g = 0.0;
        body.radiation = RadiationLevel::High;

        let mut mitigations = std::collections::HashSet::new();
        mitigations.insert(HabitabilityAttribute::Radiation);
        mitigations.insert(HabitabilityAttribute::Temperature);
        let score = body.habitability_with_mitigations(&mitigations);
        let modifier = body.habitability_modifier_with_mitigations(&mitigations);
        assert!((modifier - (0.75 + f32::from(score) * 0.005)).abs() < 1e-6);
    }

    #[test]
    fn body_defaults_within_habitable_range() {
        // A body created via `Body::new` gets safe defaults (temperate / 1 g /
        // low radiation, atmosphere=none) so callers that don't set attributes
        // explicitly land in the habitable range — no penalty at production
        // time. Actual authored content always overrides the defaults.
        let body = Body::new("Placeholder", BodyKind::InnerPlanet, 1.0);
        assert!(body.habitability() >= 50);
        assert!(body.habitability_modifier() >= 1.0);
    }

    // ── Atmosphere density × hazard formula (issue #197) ─────────────────────

    fn body_with_atmosphere(density: AtmosphereDensity, hazard: AtmosphereHazard) -> Body {
        let mut body = Body::new("Test", BodyKind::InnerPlanet, 1.0);
        body.atmosphere_density = density;
        body.atmosphere_hazard = hazard;
        // Neutral non-atmosphere attributes so only the atmosphere term varies.
        body.temperature = TemperatureBand::Temperate;
        body.gravity_g = 1.0;
        body.radiation = RadiationLevel::Low;
        body
    }

    #[test]
    fn toxic_hazard_zeroes_atmosphere_contribution_regardless_of_density() {
        // Matches the old flat `Atmosphere::Toxic => 0.0` behaviour exactly,
        // no matter which density it's paired with.
        for density in [
            AtmosphereDensity::Vacuum,
            AtmosphereDensity::Thin,
            AtmosphereDensity::Breathable,
            AtmosphereDensity::Dense,
        ] {
            let body = body_with_atmosphere(density, AtmosphereHazard::Toxic);
            // temp(30) + grav(20) + rad(20) = 70; atmo must contribute 0.
            assert_eq!(
                body.habitability(),
                70,
                "density {density:?} + Toxic hazard must contribute 0 atmo points"
            );
        }
    }

    #[test]
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    fn hazard_free_atmosphere_reproduces_old_flat_enum_scores() {
        // Pre-#197 point values for the states that existed before the split.
        let cases: [(AtmosphereDensity, f32); 4] = [
            (AtmosphereDensity::Vacuum, 5.0),
            (AtmosphereDensity::Thin, 15.0),
            (AtmosphereDensity::Breathable, 30.0),
            (AtmosphereDensity::Dense, 10.0),
        ];
        for (density, expected_atmo) in cases {
            let body = body_with_atmosphere(density, AtmosphereHazard::None);
            // temp(30) + grav(20) + rad(20) = 70 baseline.
            assert_eq!(
                body.habitability(),
                (70.0 + expected_atmo).round() as u8,
                "density {density:?} + no hazard mismatch"
            );
        }
    }

    #[test]
    fn density_and_hazard_are_independently_representable() {
        // A Dense+Corrosive world (Venus-like) must score differently from
        // Dense+None — the exact combination the old flat enum couldn't express.
        let dense_none = body_with_atmosphere(AtmosphereDensity::Dense, AtmosphereHazard::None);
        let dense_corrosive =
            body_with_atmosphere(AtmosphereDensity::Dense, AtmosphereHazard::Corrosive);
        assert!(dense_none.habitability() > dense_corrosive.habitability());

        // Corrosive hazard applies the same multiplier regardless of density,
        // so the *ratio* holds across different densities too.
        let thin_none = body_with_atmosphere(AtmosphereDensity::Thin, AtmosphereHazard::None);
        let thin_corrosive =
            body_with_atmosphere(AtmosphereDensity::Thin, AtmosphereHazard::Corrosive);
        assert!(thin_none.habitability() > thin_corrosive.habitability());
    }

    #[test]
    fn oxidizing_combustible_scores_between_corrosive_and_toxic() {
        let base = AtmosphereDensity::Breathable;
        let none = body_with_atmosphere(base, AtmosphereHazard::None).habitability();
        let corrosive = body_with_atmosphere(base, AtmosphereHazard::Corrosive).habitability();
        let oxidizing =
            body_with_atmosphere(base, AtmosphereHazard::OxidizingCombustible).habitability();
        let toxic = body_with_atmosphere(base, AtmosphereHazard::Toxic).habitability();
        assert!(none > corrosive);
        assert!(corrosive > oxidizing);
        assert!(oxidizing > toxic);
    }

    #[test]
    fn breathable_density_is_breathable_helper() {
        assert!(AtmosphereDensity::Breathable.is_breathable());
        assert!(!AtmosphereDensity::Thin.is_breathable());
        assert!(!AtmosphereDensity::Dense.is_breathable());
        assert!(!AtmosphereDensity::Vacuum.is_breathable());
    }

    #[test]
    fn set_body_attributes_overwrites_and_emits_event() {
        let (mut state, inner_id, _) = make_state_with_two_bodies();
        let events = apply_system_command(
            &mut state,
            &SystemCommand::SetBodyAttributes {
                body_id: inner_id.clone(),
                atmosphere_density: AtmosphereDensity::Dense,
                atmosphere_hazard: AtmosphereHazard::Toxic,
                temperature: TemperatureBand::Extreme,
                gravity_g: 2.5,
                radiation: RadiationLevel::High,
                subtype: PlanetarySubtype::Molten,
                tidally_locked: true,
                axial_tilt_deg: 5.0,
                rotation_period_hours: 900.0,
                moon_count: 2,
            },
        )
        .unwrap();
        assert!(matches!(
            events[0],
            SystemEvent::BodyAttributesSet {
                atmosphere_hazard: AtmosphereHazard::Toxic,
                ..
            }
        ));
        let body = &state.node_map.bodies[&inner_id];
        assert_eq!(body.atmosphere_density, AtmosphereDensity::Dense);
        assert_eq!(body.atmosphere_hazard, AtmosphereHazard::Toxic);
        assert_eq!(body.temperature, TemperatureBand::Extreme);
        assert!((body.gravity_g - 2.5).abs() < 1e-6);
        assert_eq!(body.radiation, RadiationLevel::High);
        assert_eq!(body.subtype, PlanetarySubtype::Molten);
        assert!(body.tidally_locked);
        assert!((body.axial_tilt_deg - 5.0).abs() < 1e-6);
        assert!((body.rotation_period_hours - 900.0).abs() < 1e-6);
        assert_eq!(body.moon_count, 2);
    }

    #[test]
    fn set_body_attributes_rejects_incompatible_subtype() {
        let (mut state, inner_id, _) = make_state_with_two_bodies();
        let result = apply_system_command(
            &mut state,
            &SystemCommand::SetBodyAttributes {
                body_id: inner_id,
                atmosphere_density: AtmosphereDensity::Vacuum,
                atmosphere_hazard: AtmosphereHazard::None,
                temperature: TemperatureBand::Cold,
                gravity_g: 2.4,
                radiation: RadiationLevel::High,
                // GasGiant subtype on an InnerPlanet-kind body is invalid.
                subtype: PlanetarySubtype::GasGiant,
                tidally_locked: false,
                axial_tilt_deg: 0.0,
                rotation_period_hours: 10.0,
                moon_count: 0,
            },
        );
        assert!(matches!(
            result,
            Err(SystemError::IncompatiblePlanetarySubtype { .. })
        ));
    }

    #[test]
    fn set_body_parent_links_and_emits_event() {
        let (mut state, inner_id, outer_id) = make_state_with_two_bodies();
        let events = apply_system_command(
            &mut state,
            &SystemCommand::SetBodyParent {
                body_id: inner_id.clone(),
                parent_body: outer_id.clone(),
            },
        )
        .unwrap();
        assert!(matches!(events[0], SystemEvent::BodyParentSet { .. }));
        assert_eq!(state.node_map.bodies[&inner_id].parent_body, Some(outer_id));
    }

    #[test]
    fn set_body_parent_rejects_nonexistent_parent() {
        let (mut state, inner_id, _) = make_state_with_two_bodies();
        let bad_parent = BodyId::new();
        let result = apply_system_command(
            &mut state,
            &SystemCommand::SetBodyParent {
                body_id: inner_id,
                parent_body: bad_parent,
            },
        );
        assert!(matches!(result, Err(SystemError::BodyNotFound(_))));
    }

    #[test]
    fn set_body_modifiers_overwrites_and_reports_event() {
        let (mut state, inner_id, _) = make_state_with_two_bodies();
        let modifiers = vec![
            BodyModifier {
                category: YieldCategory::ScienceYield,
                multiplier: 1.3,
            },
            BodyModifier {
                category: YieldCategory::FoodYield,
                multiplier: 0.8,
            },
        ];
        let events = apply_system_command(
            &mut state,
            &SystemCommand::SetBodyModifiers {
                body_id: inner_id.clone(),
                modifiers: modifiers.clone(),
            },
        )
        .unwrap();
        assert!(matches!(events[0], SystemEvent::BodyModifiersSet { .. }));
        assert_eq!(state.node_map.bodies[&inner_id].modifiers, modifiers);
        assert!(
            (state.node_map.bodies[&inner_id].category_modifier(YieldCategory::ScienceYield) - 1.3)
                .abs()
                < 1e-6
        );
        // Unauthored category still defaults to neutral.
        assert!(
            (state.node_map.bodies[&inner_id].category_modifier(YieldCategory::PowerOutput) - 1.0)
                .abs()
                < 1e-6
        );
    }

    #[test]
    fn set_body_modifiers_rejects_nonexistent_body() {
        let mut state = SystemState::new();
        let bad_id = BodyId::new();
        let result = apply_system_command(
            &mut state,
            &SystemCommand::SetBodyModifiers {
                body_id: bad_id,
                modifiers: vec![],
            },
        );
        assert!(matches!(result, Err(SystemError::BodyNotFound(_))));
    }

    #[test]
    fn generate_system_populates_node_map_and_reports_body_ids() {
        let mut state = SystemState::new();
        let events = apply_system_command(
            &mut state,
            &SystemCommand::GenerateSystem {
                seed: 7,
                abundance_scalar: 1.0,
                habitable_zone_center_au: default_habitable_zone_center_au(),
                min_inner_planets: default_min_inner_planets(),
                max_inner_planets: default_max_inner_planets(),
            },
        )
        .unwrap();
        let SystemEvent::SystemGenerated { seed, body_ids } = &events[0] else {
            panic!("expected SystemGenerated");
        };
        assert_eq!(*seed, 7);
        assert!(!body_ids.is_empty());
        assert_eq!(body_ids.len(), state.node_map.bodies.len());
        for id in body_ids {
            assert!(state.node_map.bodies.contains_key(id));
        }
    }

    #[test]
    fn generate_system_replaces_any_existing_bodies() {
        let (mut state, inner_id, outer_id) = make_state_with_two_bodies();
        assert_eq!(state.node_map.bodies.len(), 2);
        apply_system_command(
            &mut state,
            &SystemCommand::GenerateSystem {
                seed: 3,
                abundance_scalar: 1.0,
                habitable_zone_center_au: default_habitable_zone_center_au(),
                min_inner_planets: default_min_inner_planets(),
                max_inner_planets: default_max_inner_planets(),
            },
        )
        .unwrap();
        // The two hand-added bodies must be gone — generation
        // wholesale-replaces the node map, mirroring Command::SeedPlanet's
        // "overwrite any previously seeded map" semantics.
        assert!(!state.node_map.bodies.contains_key(&inner_id));
        assert!(!state.node_map.bodies.contains_key(&outer_id));
        assert!(!state.node_map.bodies.is_empty());
    }

    #[test]
    fn assign_role_to_nonexistent_body_fails() {
        let mut state = SystemState::new();
        let bad_id = BodyId::new();
        let result = apply_system_command(
            &mut state,
            &SystemCommand::AssignRole {
                body_id: bad_id,
                role: SystemRole::Science,
            },
        );
        assert!(matches!(result, Err(SystemError::BodyNotFound(_))));
    }

    #[test]
    fn shipping_capacity_constraint_enforced() {
        let (mut state, inner_id, outer_id) = make_state_with_two_bodies();
        // Add a small hauler (capacity 10)
        apply_system_command(&mut state, &SystemCommand::AddHauler { capacity: 10.0 }).unwrap();
        // Try to ship 50 units — should fail
        let result = apply_system_command(
            &mut state,
            &SystemCommand::DispatchShipment {
                from: inner_id,
                to: outer_id,
                cargo: vec![("iron".into(), 50.0)],
                destination_colony: None,
            },
        );
        assert!(
            matches!(result, Err(SystemError::InsufficientHaulerCapacity { .. })),
            "expected InsufficientHaulerCapacity, got {:?}",
            result
        );
    }

    #[test]
    fn dispatch_shipment_with_sufficient_capacity() {
        let (mut state, inner_id, outer_id) = make_state_with_two_bodies();
        apply_system_command(&mut state, &SystemCommand::AddHauler { capacity: 100.0 }).unwrap();
        let events = apply_system_command(
            &mut state,
            &SystemCommand::DispatchShipment {
                from: inner_id.clone(),
                to: outer_id.clone(),
                cargo: vec![("food".into(), 50.0)],
                destination_colony: None,
            },
        )
        .unwrap();
        assert!(matches!(events[0], SystemEvent::ShipmentDispatched { .. }));
        assert_eq!(state.shipments.len(), 1);
        // Hauler should be marked in-transit; available capacity should drop
        assert_eq!(state.hauler_fleet.available_capacity(), 0.0);
    }

    #[test]
    fn shipment_tracking_and_hauler_capacity() {
        // Use bodies at distance 1.0 AU apart → travel_time = ceil(1.0/1) = 1 month.
        let mut state = SystemState::new();
        let e1 = apply_system_command(
            &mut state,
            &SystemCommand::AddBody {
                name: "A".into(),
                kind: BodyKind::InnerPlanet,
                distance_au: 0.0,
            },
        )
        .unwrap();
        let e2 = apply_system_command(
            &mut state,
            &SystemCommand::AddBody {
                name: "B".into(),
                kind: BodyKind::InnerPlanet,
                distance_au: 1.0,
            },
        )
        .unwrap();
        let body_a = match &e1[0] {
            SystemEvent::BodyAdded { body_id, .. } => body_id.clone(),
            _ => panic!(),
        };
        let body_b = match &e2[0] {
            SystemEvent::BodyAdded { body_id, .. } => body_id.clone(),
            _ => panic!(),
        };
        // Add a hauler and dispatch a shipment
        apply_system_command(&mut state, &SystemCommand::AddHauler { capacity: 100.0 }).unwrap();
        apply_system_command(
            &mut state,
            &SystemCommand::DispatchShipment {
                from: body_a.clone(),
                to: body_b.clone(),
                cargo: vec![("iron".into(), 50.0)],
                destination_colony: None,
            },
        )
        .unwrap();
        // Hauler is now busy
        assert_eq!(state.hauler_fleet.available_capacity(), 0.0);
        assert_eq!(state.shipments.len(), 1);
        // Verify travel time is 1 for bodies 1 AU apart with propulsion_level=1
        let travel_time = state
            .node_map
            .compute_travel_time(&body_a, &body_b)
            .unwrap();
        assert_eq!(travel_time, 1, "expected travel time of 1 month");
        // A shipment's countdown is in sols now (issue #332), so a 1-month
        // voyage is SOLS_PER_MONTH advances — same elapsed time, finer steps.
        assert_eq!(
            state.shipments.values().next().unwrap().turns_remaining,
            SOLS_PER_MONTH,
            "1 month of travel should be stored as SOLS_PER_MONTH sols"
        );
        for sol in 1..SOLS_PER_MONTH {
            apply_system_command(&mut state, &SystemCommand::AdvanceShipments).unwrap();
            assert_eq!(
                state.shipments.len(),
                1,
                "still in transit at sol {sol} of {SOLS_PER_MONTH}"
            );
        }
        // The final sol delivers it.
        let arrival_events =
            apply_system_command(&mut state, &SystemCommand::AdvanceShipments).unwrap();
        assert_eq!(
            state.shipments.len(),
            0,
            "shipment should be removed after arriving"
        );
        assert!(
            arrival_events
                .iter()
                .any(|e| matches!(e, SystemEvent::ShipmentArrived { .. })),
            "shipment should arrive after 1 month; events: {arrival_events:?}"
        );
        // Shipment is removed and hauler freed
        assert_eq!(state.shipments.len(), 0);
        assert_eq!(state.hauler_fleet.available_capacity(), 100.0);
    }

    #[test]
    fn megaproject_milestone_progression() {
        let mut state = SystemState::new();
        let events = apply_system_command(
            &mut state,
            &SystemCommand::RegisterMegaproject {
                name: "Interstellar Expedition".into(),
                kind: MegaprojectKind::InterstellarExpedition,
                milestones: vec![
                    MilestoneSpec {
                        label: "Hull Construction".into(),
                        resource_cost: vec![("steel".into(), 500.0)],
                        research_cost: 100.0,
                    },
                    MilestoneSpec {
                        label: "Drive Assembly".into(),
                        resource_cost: vec![("fuel".into(), 1000.0)],
                        research_cost: 200.0,
                    },
                ],
            },
        )
        .unwrap();

        let project_id = match &events[0] {
            SystemEvent::MegaprojectRegistered { project_id, .. } => project_id.clone(),
            _ => panic!("expected MegaprojectRegistered"),
        };

        // Contribute enough to complete milestone 0
        let events = apply_system_command(
            &mut state,
            &SystemCommand::ContributeToMegaproject {
                project_id: project_id.clone(),
                resources: vec![("steel".into(), 500.0)],
                research: 100.0,
            },
        )
        .unwrap();
        // Should get Contribution + MilestoneCompleted (but not MegaprojectCompleted yet)
        assert!(events.iter().any(|e| matches!(
            e,
            SystemEvent::MilestoneCompleted {
                milestone_index: 0,
                ..
            }
        )));
        assert!(!events
            .iter()
            .any(|e| matches!(e, SystemEvent::MegaprojectCompleted { .. })));

        // Milestone 1 is now active
        let project = &state.megaprojects[&project_id];
        assert_eq!(project.next_milestone_index(), Some(1));

        // Contribute enough to complete milestone 1
        let events = apply_system_command(
            &mut state,
            &SystemCommand::ContributeToMegaproject {
                project_id: project_id.clone(),
                resources: vec![("fuel".into(), 1000.0)],
                research: 200.0,
            },
        )
        .unwrap();
        // Now both MilestoneCompleted and MegaprojectCompleted should fire
        assert!(events.iter().any(|e| matches!(
            e,
            SystemEvent::MilestoneCompleted {
                milestone_index: 1,
                ..
            }
        )));
        assert!(events
            .iter()
            .any(|e| matches!(e, SystemEvent::MegaprojectCompleted { .. })));
        assert!(state.megaprojects[&project_id].completed);
    }

    #[test]
    fn contribute_to_completed_megaproject_fails() {
        let mut state = SystemState::new();
        let events = apply_system_command(
            &mut state,
            &SystemCommand::RegisterMegaproject {
                name: "Quick Build".into(),
                kind: MegaprojectKind::SystemPowerArray,
                milestones: vec![MilestoneSpec {
                    label: "Phase 1".into(),
                    resource_cost: vec![],
                    research_cost: 0.0,
                }],
            },
        )
        .unwrap();
        let project_id = match &events[0] {
            SystemEvent::MegaprojectRegistered { project_id, .. } => project_id.clone(),
            _ => panic!(),
        };
        // Complete it
        apply_system_command(
            &mut state,
            &SystemCommand::ContributeToMegaproject {
                project_id: project_id.clone(),
                resources: vec![],
                research: 0.0,
            },
        )
        .unwrap();
        // Try again — should fail
        let result = apply_system_command(
            &mut state,
            &SystemCommand::ContributeToMegaproject {
                project_id: project_id.clone(),
                resources: vec![],
                research: 0.0,
            },
        );
        assert!(matches!(
            result,
            Err(SystemError::MegaprojectAlreadyComplete(_))
        ));
    }

    #[test]
    fn propulsion_upgrade_reduces_travel_time() {
        let (mut state, inner_id, outer_id) = make_state_with_two_bodies();
        let t1 = state
            .node_map
            .compute_travel_time(&inner_id, &outer_id)
            .unwrap();
        apply_system_command(
            &mut state,
            &SystemCommand::UpgradePropulsion { new_level: 5 },
        )
        .unwrap();
        let t2 = state
            .node_map
            .compute_travel_time(&inner_id, &outer_id)
            .unwrap();
        assert!(
            t2 <= t1,
            "upgraded propulsion should reduce or equal travel time"
        );
    }

    #[test]
    fn propulsion_downgrade_rejected() {
        let mut state = SystemState::new();
        state.node_map.propulsion_level = 3;
        let result = apply_system_command(
            &mut state,
            &SystemCommand::UpgradePropulsion { new_level: 2 },
        );
        assert!(matches!(
            result,
            Err(SystemError::PropulsionDowngrade { .. })
        ));
    }

    #[test]
    fn world_specialization_all_roles() {
        let mut state = SystemState::new();
        let roles = [
            SystemRole::Industry,
            SystemRole::RawExtraction,
            SystemRole::Science,
            SystemRole::FuelProduction,
            SystemRole::PopulationHub,
            SystemRole::Unassigned,
        ];
        for role in &roles {
            let events = apply_system_command(
                &mut state,
                &SystemCommand::AddBody {
                    name: format!("{role:?} body"),
                    kind: BodyKind::InnerPlanet,
                    distance_au: 1.0,
                },
            )
            .unwrap();
            let body_id = match &events[0] {
                SystemEvent::BodyAdded { body_id, .. } => body_id.clone(),
                _ => panic!(),
            };
            apply_system_command(
                &mut state,
                &SystemCommand::AssignRole {
                    body_id: body_id.clone(),
                    role: role.clone(),
                },
            )
            .unwrap();
            assert_eq!(state.node_map.bodies[&body_id].role, *role);
        }
    }
}
