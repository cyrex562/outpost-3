//! Assertion types and checker for the `harness check` subcommand.
//!
//! An `assertions.yaml` file contains a list of [`Assertion`] items.
//! Each assertion is checked against a [`BalanceReport`] and produces a
//! [`AssertionResult`] with PASS/FAIL status and a human-readable message.

use outpost_core::balance::{BalanceReport, BalanceVerdict};
use serde::{Deserialize, Serialize};

/// The expected balance status for a commodity assertion.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExpectedStatus {
    /// The commodity chain is closed (sustainable).
    Closed,
    /// A bottleneck exists (deficit) for this commodity.
    Bottleneck,
    /// The commodity is impossible to produce.
    Impossible,
}

/// A single assertion in an `assertions.yaml` bundle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Assertion {
    /// Commodity id this assertion targets, or `null` / omitted to target the overall verdict.
    #[serde(default)]
    pub commodity: Option<String>,
    /// Expected status of the commodity or the overall balance.
    pub expect: ExpectedStatus,
    /// Optional minimum net rate (units/sol) for the commodity. Only checked when `expect` is
    /// `closed`.
    #[serde(default)]
    pub min_net: Option<f32>,
    /// Optional human-readable label shown in output (defaults to commodity id or "overall").
    #[serde(default)]
    pub label: Option<String>,
}

/// The outcome of checking one [`Assertion`] against a [`BalanceReport`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssertionResult {
    /// Label shown in output.
    pub label: String,
    /// Whether the assertion passed.
    pub passed: bool,
    /// Human-readable reason for the result.
    pub message: String,
}

/// Check all assertions in `assertions` against `report`.
///
/// Returns one [`AssertionResult`] per assertion, in order.
pub fn check_assertions(assertions: &[Assertion], report: &BalanceReport) -> Vec<AssertionResult> {
    assertions.iter().map(|a| check_one(a, report)).collect()
}

fn check_one(assertion: &Assertion, report: &BalanceReport) -> AssertionResult {
    let label = assertion
        .label
        .clone()
        .or_else(|| assertion.commodity.clone())
        .unwrap_or_else(|| "overall".to_string());

    if let Some(id) = &assertion.commodity {
        check_commodity(assertion, id, &label, report)
    } else {
        check_overall(assertion, &label, report)
    }
}

/// Check a commodity-level assertion against the report.
fn check_commodity(
    assertion: &Assertion,
    id: &str,
    label: &str,
    report: &BalanceReport,
) -> AssertionResult {
    let Some(cb) = report.commodity(id) else {
        return AssertionResult {
            label: label.to_string(),
            passed: false,
            message: format!("commodity '{id}' not found in report"),
        };
    };

    match assertion.expect {
        ExpectedStatus::Closed => check_closed(assertion, label, cb.net_rate),
        ExpectedStatus::Bottleneck => {
            let passed = cb.net_rate < 0.0;
            let message = if passed {
                format!("net_rate={:.4} < 0 (bottleneck)", cb.net_rate)
            } else {
                format!(
                    "expected bottleneck but net_rate={:.4} (not a deficit)",
                    cb.net_rate
                )
            };
            AssertionResult {
                label: label.to_string(),
                passed,
                message,
            }
        }
        ExpectedStatus::Impossible => {
            let passed = cb.production_rate <= 0.0 && cb.consumption_rate > 0.0;
            let message = if passed {
                format!(
                    "production={:.4} consumption={:.4} (impossible)",
                    cb.production_rate, cb.consumption_rate
                )
            } else {
                format!(
                    "expected impossible but production={:.4} (has a source)",
                    cb.production_rate
                )
            };
            AssertionResult {
                label: label.to_string(),
                passed,
                message,
            }
        }
    }
}

/// Check the `closed` expectation with optional `min_net`.
fn check_closed(assertion: &Assertion, label: &str, net_rate: f64) -> AssertionResult {
    if net_rate < 0.0 {
        return AssertionResult {
            label: label.to_string(),
            passed: false,
            message: format!("expected closed but net_rate={net_rate:.4} (deficit)"),
        };
    }
    if let Some(min_net) = assertion.min_net {
        if net_rate < f64::from(min_net) {
            return AssertionResult {
                label: label.to_string(),
                passed: false,
                message: format!("net_rate={net_rate:.4} is below min_net={min_net:.4}"),
            };
        }
    }
    AssertionResult {
        label: label.to_string(),
        passed: true,
        message: format!("net_rate={net_rate:.4} ≥ 0 (closed)"),
    }
}

/// Check the overall verdict against the assertion.
fn check_overall(assertion: &Assertion, label: &str, report: &BalanceReport) -> AssertionResult {
    let verdict_matches = match assertion.expect {
        ExpectedStatus::Closed => matches!(report.verdict, BalanceVerdict::Closed),
        ExpectedStatus::Bottleneck => {
            matches!(report.verdict, BalanceVerdict::Bottleneck(_, _))
        }
        ExpectedStatus::Impossible => {
            matches!(report.verdict, BalanceVerdict::Impossible(_))
        }
    };

    let message = if verdict_matches {
        format!("verdict matches expected {:?}", assertion.expect)
    } else {
        format!(
            "expected {:?} but got {:?}",
            assertion.expect, report.verdict
        )
    };
    AssertionResult {
        label: label.to_string(),
        passed: verdict_matches,
        message,
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use outpost_core::balance::{BalanceCalculator, BuildingInstance, Flow};
    use outpost_core::content::loader::PackLoader;

    fn minimal_report() -> BalanceReport {
        let pack_yaml = "id: t\nname: T\nversion: '0.1.0'\n";
        let commodities_yaml = "
- id: ore
  name: Ore
  category: raw
  base_value: 1.0
- id: plate
  name: Plate
  category: metal
  base_value: 3.0
";
        let buildings_yaml = "
- id: smelter
  name: Smelter
  category: production
";
        let recipes_yaml = "
- id: smelt
  name: Smelt
  building: smelter
  cycle_sols: 1
  inputs:
    - id: ore
      quantity: 2.0
  outputs:
    - id: plate
      quantity: 1.0
";
        let files = vec![
            ("pack.yaml", pack_yaml),
            ("commodities.yaml", commodities_yaml),
            ("buildings.yaml", buildings_yaml),
            ("recipes.yaml", recipes_yaml),
        ];
        let registry = PackLoader::load(&files).unwrap();
        let building = registry.building("smelter").unwrap().clone();
        let recipe = registry.recipe("smelt").unwrap().clone();
        let instances = vec![BuildingInstance {
            building,
            recipe: Some(recipe),
        }];
        let imports = vec![Flow {
            commodity_id: "ore".to_string(),
            rate: 2.0,
        }];
        BalanceCalculator::compute(&instances, &imports)
    }

    #[test]
    fn assertion_closed_passes_when_closed() {
        let report = minimal_report();
        let assertions = vec![Assertion {
            commodity: Some("plate".to_string()),
            expect: ExpectedStatus::Closed,
            min_net: None,
            label: None,
        }];
        let results = check_assertions(&assertions, &report);
        assert!(
            results[0].passed,
            "plate should be closed: {}",
            results[0].message
        );
    }

    #[test]
    fn assertion_bottleneck_passes_when_deficit() {
        let report = minimal_report();
        // ore is consumed at 2/sol but the import exactly covers it — net=0 (EXACT, not deficit)
        // Let's assert bottleneck for a missing commodity by using a report where ore is not imported
        let pack_yaml = "id: t\nname: T\nversion: '0.1.0'\n";
        let commodities_yaml = "
- id: ore
  name: Ore
  category: raw
  base_value: 1.0
- id: plate
  name: Plate
  category: metal
  base_value: 3.0
";
        let buildings_yaml = "- id: smelter\n  name: Smelter\n  category: production\n";
        let recipes_yaml = "
- id: smelt
  name: Smelt
  building: smelter
  cycle_sols: 1
  inputs:
    - id: ore
      quantity: 2.0
  outputs:
    - id: plate
      quantity: 1.0
";
        let files = vec![
            ("pack.yaml", pack_yaml),
            ("commodities.yaml", commodities_yaml),
            ("buildings.yaml", buildings_yaml),
            ("recipes.yaml", recipes_yaml),
        ];
        let registry = outpost_core::content::loader::PackLoader::load(&files).unwrap();
        let building = registry.building("smelter").unwrap().clone();
        let recipe = registry.recipe("smelt").unwrap().clone();
        let instances = vec![BuildingInstance {
            building,
            recipe: Some(recipe),
        }];
        // No imports — ore is never produced → Impossible, not Bottleneck
        // For a bottleneck we need partial import
        let imports = vec![Flow {
            commodity_id: "ore".to_string(),
            rate: 1.0, // only half what's needed
        }];
        let deficit_report = BalanceCalculator::compute(&instances, &imports);
        let assertions = vec![Assertion {
            commodity: Some("ore".to_string()),
            expect: ExpectedStatus::Bottleneck,
            min_net: None,
            label: None,
        }];
        let results = check_assertions(&assertions, &deficit_report);
        assert!(
            results[0].passed,
            "ore should be bottleneck: {}",
            results[0].message
        );
    }

    #[test]
    fn assertion_closed_fails_when_deficit() {
        let pack_yaml = "id: t\nname: T\nversion: '0.1.0'\n";
        let commodities_yaml = "
- id: ore
  name: Ore
  category: raw
  base_value: 1.0
- id: plate
  name: Plate
  category: metal
  base_value: 3.0
";
        let buildings_yaml = "- id: smelter\n  name: Smelter\n  category: production\n";
        let recipes_yaml = "
- id: smelt
  name: Smelt
  building: smelter
  cycle_sols: 1
  inputs:
    - id: ore
      quantity: 2.0
  outputs:
    - id: plate
      quantity: 1.0
";
        let files = vec![
            ("pack.yaml", pack_yaml),
            ("commodities.yaml", commodities_yaml),
            ("buildings.yaml", buildings_yaml),
            ("recipes.yaml", recipes_yaml),
        ];
        let registry = outpost_core::content::loader::PackLoader::load(&files).unwrap();
        let building = registry.building("smelter").unwrap().clone();
        let recipe = registry.recipe("smelt").unwrap().clone();
        let instances = vec![BuildingInstance {
            building,
            recipe: Some(recipe),
        }];
        let imports = vec![Flow {
            commodity_id: "ore".to_string(),
            rate: 1.0,
        }];
        let report = BalanceCalculator::compute(&instances, &imports);
        let assertions = vec![Assertion {
            commodity: Some("ore".to_string()),
            expect: ExpectedStatus::Closed,
            min_net: None,
            label: None,
        }];
        let results = check_assertions(&assertions, &report);
        assert!(!results[0].passed, "should fail: {}", results[0].message);
    }

    #[test]
    fn assertion_min_net_fails_when_below_threshold() {
        let report = minimal_report();
        // plate net_rate = 1.0/sol, require min_net = 5.0 → should fail
        let assertions = vec![Assertion {
            commodity: Some("plate".to_string()),
            expect: ExpectedStatus::Closed,
            min_net: Some(5.0),
            label: None,
        }];
        let results = check_assertions(&assertions, &report);
        assert!(
            !results[0].passed,
            "should fail min_net check: {}",
            results[0].message
        );
    }

    #[test]
    fn assertion_overall_verdict_closed_passes() {
        let report = minimal_report();
        let assertions = vec![Assertion {
            commodity: None,
            expect: ExpectedStatus::Closed,
            min_net: None,
            label: None,
        }];
        let results = check_assertions(&assertions, &report);
        assert!(
            results[0].passed,
            "overall should be closed: {}",
            results[0].message
        );
    }

    #[test]
    fn unknown_commodity_fails_with_message() {
        let report = minimal_report();
        let assertions = vec![Assertion {
            commodity: Some("unobtanium".to_string()),
            expect: ExpectedStatus::Closed,
            min_net: None,
            label: None,
        }];
        let results = check_assertions(&assertions, &report);
        assert!(!results[0].passed);
        assert!(
            results[0].message.contains("not found"),
            "got: {}",
            results[0].message
        );
    }
}
