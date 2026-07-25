//! Unified read/write view over a colony's two stores (issue #304).
//!
//! A recipe's ingredient list mixes tradeable commodities with colony-local
//! resources — a fission reactor burns `fissile_fuel` (commodity) and emits
//! `power` (resource). Rather than teach every production and needs code path
//! about both pools, they take a [`ColonyStores`] and this one type dispatches
//! by id.
//!
//! `ContentRegistry::is_resource` is the only dispatch rule, and the loader
//! rejects an id declared as both, so the routing is unambiguous. An id that is
//! neither — content referencing something undeclared — falls through to the
//! commodity pool, matching the pre-#304 behaviour; the loader's
//! cross-reference validation is what actually prevents that case.

use crate::colony::{ColonyPool, ColonyResourcePool};
use crate::content::ContentRegistry;

/// Mutable view over a colony's commodity pool and resource pool together.
pub struct ColonyStores<'a> {
    commodities: &'a mut ColonyPool,
    resources: &'a mut ColonyResourcePool,
    registry: &'a ContentRegistry,
}

impl<'a> ColonyStores<'a> {
    /// Borrow both stores alongside the registry that classifies ids.
    pub fn new(
        commodities: &'a mut ColonyPool,
        resources: &'a mut ColonyResourcePool,
        registry: &'a ContentRegistry,
    ) -> Self {
        Self {
            commodities,
            resources,
            registry,
        }
    }

    /// Whether `id` routes to the resource pool.
    #[must_use]
    pub fn is_resource(&self, id: &str) -> bool {
        self.registry.is_resource(id)
    }

    /// Current amount of `id` in whichever store owns it.
    #[must_use]
    pub fn amount(&self, id: &str) -> f64 {
        if self.registry.is_resource(id) {
            self.resources.amount(id)
        } else {
            self.commodities.amount(id)
        }
    }

    /// Storage capacity for `id`; resources are uncapped (they last one sol).
    #[must_use]
    pub fn capacity(&self, id: &str) -> f64 {
        if self.registry.is_resource(id) {
            f64::INFINITY
        } else {
            self.commodities.capacity(id)
        }
    }

    /// Add `qty` of `id` to whichever store owns it.
    pub fn deposit(&mut self, id: &str, qty: f64) {
        if self.registry.is_resource(id) {
            self.resources.deposit(id, qty);
        } else {
            self.commodities.deposit(id, qty);
        }
    }

    /// Remove up to `qty` of `id`, returning the amount actually removed.
    pub fn withdraw(&mut self, id: &str, qty: f64) -> f64 {
        if self.registry.is_resource(id) {
            self.resources.withdraw(id, qty)
        } else {
            self.commodities.withdraw(id, qty)
        }
    }

    /// Immutable access to the commodity pool, for callers that specifically
    /// mean "tradeable stock" (capacity planning, trade, cargo).
    #[must_use]
    pub fn commodities(&self) -> &ColonyPool {
        self.commodities
    }

    /// Immutable access to the resource pool.
    #[must_use]
    pub fn resources(&self) -> &ColonyResourcePool {
        self.resources
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::types::{CommodityDef, Phase, ResourceDef, ResourceKind};

    fn registry() -> ContentRegistry {
        let mut reg = ContentRegistry::default();
        reg.insert_commodity(CommodityDef {
            id: "water".into(),
            name: "Water".into(),
            description: String::new(),
            category: "consumable".into(),
            phase: Phase::Liquid,
            base_value: 5.0,
            tradeable: true,
            tier: crate::content::CommodityTier::default(),
            weight: 1.0,
        });
        reg.insert_resource(ResourceDef {
            id: "power".into(),
            name: "Power".into(),
            description: String::new(),
            kind: ResourceKind::Flow,
            unit: "MW".into(),
        });
        reg
    }

    #[test]
    fn deposits_route_to_the_store_that_owns_the_id() {
        let reg = registry();
        let mut commodities = ColonyPool::new();
        let mut resources = ColonyResourcePool::new();
        let mut stores = ColonyStores::new(&mut commodities, &mut resources, &reg);

        stores.deposit("water", 10.0);
        stores.deposit("power", 24.0);

        assert!((stores.amount("water") - 10.0).abs() < f64::EPSILON);
        assert!((stores.amount("power") - 24.0).abs() < f64::EPSILON);
        // ...and each landed in the right underlying store, not both.
        assert!((commodities.amount("water") - 10.0).abs() < f64::EPSILON);
        assert_eq!(commodities.amount("power"), 0.0, "power is not cargo");
        assert!((resources.amount("power") - 24.0).abs() < f64::EPSILON);
        assert_eq!(resources.amount("water"), 0.0, "water is not a resource");
    }

    #[test]
    fn withdrawals_route_the_same_way() {
        let reg = registry();
        let mut commodities = ColonyPool::new();
        let mut resources = ColonyResourcePool::new();
        commodities.deposit("water", 10.0);
        resources.deposit("power", 24.0);
        let mut stores = ColonyStores::new(&mut commodities, &mut resources, &reg);

        assert!((stores.withdraw("water", 4.0) - 4.0).abs() < f64::EPSILON);
        assert!((stores.withdraw("power", 9.0) - 9.0).abs() < f64::EPSILON);
        assert!((commodities.amount("water") - 6.0).abs() < f64::EPSILON);
        assert!((resources.amount("power") - 15.0).abs() < f64::EPSILON);
    }

    #[test]
    fn resources_are_uncapped_because_they_last_a_single_sol() {
        let reg = registry();
        let mut commodities = ColonyPool::new();
        let mut resources = ColonyResourcePool::new();
        let stores = ColonyStores::new(&mut commodities, &mut resources, &reg);
        assert_eq!(stores.capacity("power"), f64::INFINITY);
    }

    #[test]
    fn an_undeclared_id_falls_through_to_the_commodity_pool() {
        let reg = registry();
        let mut commodities = ColonyPool::new();
        let mut resources = ColonyResourcePool::new();
        let mut stores = ColonyStores::new(&mut commodities, &mut resources, &reg);

        stores.deposit("mystery", 3.0);

        assert!((commodities.amount("mystery") - 3.0).abs() < f64::EPSILON);
        assert!(resources.is_empty());
    }
}
