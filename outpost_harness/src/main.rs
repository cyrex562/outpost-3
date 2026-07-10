//! Outpost 3 balance harness CLI.
//!
//! # Subcommands
//!
//! ## `harness run <pack_dir> <colony_config> [--json]`
//!
//! Loads a content pack and a colony configuration, runs the static
//! flow-balance calculator, then prints either a human-readable report
//! or machine-readable JSON.
//!
//! Exit codes for `run`:
//! - 0 — Closed (sustainable, at least one binding constraint)
//! - 1 — Bottleneck or Trivial
//! - 2 — Impossible (structurally broken chain)
//! - 3 — Error (bad args, missing files, parse failure)
//!
//! ## `harness check <bundle_dir> [--json]`
//!
//! Loads a self-contained bundle directory containing:
//! - Content pack files (`pack.yaml`, `commodities.yaml`, etc.)
//! - `colony.yaml` — colony configuration
//! - `assertions.yaml` — list of pass/fail assertions
//!
//! Runs the balance calculator, checks each assertion, and prints PASS/FAIL.
//!
//! Exit codes for `check`:
//! - 0 — all assertions passed
//! - 1 — one or more assertions failed
//! - 2 — error (missing files, parse failure)

use std::process;

use outpost_core::balance::{
    BalanceCalculator, BalanceReport, BalanceVerdict, BuildingInstance, Flow,
};
use outpost_core::content::loader::{PackLoader, RawFile};
use outpost_core::content::registry::ContentRegistry;

mod assertions;
mod config;
use assertions::{check_assertions, Assertion, AssertionResult};
use config::ColonyConfig;

fn main() {
    match run() {
        Ok(code) => process::exit(code),
        Err(e) => {
            eprintln!("error: {e}");
            process::exit(3);
        }
    }
}

/// Top-level entry: parse args, dispatch subcommand.
///
/// Returns the process exit code on success, or an error string.
fn run() -> Result<i32, String> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        return Err(format!(
            "usage: {} <run|check> ...\n  run   <pack_dir> <colony_config> [--json]\n  check <bundle_dir> [--json]",
            args[0]
        ));
    }

    match args[1].as_str() {
        "run" => run_subcommand(&args),
        "check" => check_subcommand(&args),
        other => Err(format!(
            "unknown subcommand '{other}'. usage: {} <run|check>",
            args[0]
        )),
    }
}

/// `harness run <pack_dir> <colony_config> [--json]`
fn run_subcommand(args: &[String]) -> Result<i32, String> {
    if args.len() < 4 {
        return Err(format!(
            "usage: {} run <pack_dir> <colony_config> [--json]",
            args[0]
        ));
    }

    let pack_dir = &args[2];
    let colony_config_path = &args[3];
    let json_mode = args.iter().any(|a| a == "--json");

    // Load content pack.
    let registry = load_pack(pack_dir)?;

    // Parse colony config.
    let colony_config = load_colony_config(colony_config_path)?;

    // Resolve building and recipe definitions from the registry.
    let building_instances = resolve_instances(&colony_config, &registry)?;

    // Resolve import flows.
    let imports: Vec<Flow> = colony_config
        .imports
        .iter()
        .map(|imp| Flow {
            commodity_id: imp.commodity_id.clone(),
            rate: imp.rate,
        })
        .collect();

    // Run calculator.
    let report = BalanceCalculator::compute(&building_instances, &imports);

    // Determine exit code from verdict.
    let exit_code = verdict_exit_code(&report.verdict);

    // Print report.
    if json_mode {
        print_json(&report)?;
    } else {
        print_human(&report);
    }

    Ok(exit_code)
}

/// `harness check <bundle_dir> [--json]`
///
/// A bundle directory contains:
/// - Content pack files (`pack.yaml`, `commodities.yaml`, `buildings.yaml`, `recipes.yaml`)
/// - `colony.yaml` — colony configuration (same schema as used by `run`)
/// - `assertions.yaml` — list of assertions to check
///
/// Exit codes:
/// - 0 — all assertions passed
/// - 1 — one or more assertions failed
/// - 2 — error (missing files, parse failure)
fn check_subcommand(args: &[String]) -> Result<i32, String> {
    if args.len() < 3 {
        return Err(format!("usage: {} check <bundle_dir> [--json]", args[0]));
    }

    let bundle_dir = &args[2];
    let json_mode = args.iter().any(|a| a == "--json");

    let dir = std::path::Path::new(bundle_dir);
    if !dir.is_dir() {
        return Err(format!("bundle directory not found: {bundle_dir}"));
    }

    // Load content pack from bundle dir.
    let registry = load_pack(bundle_dir)?;

    // Load colony config from bundle dir.
    let colony_path = dir.join("colony.yaml");
    if !colony_path.exists() {
        return Err(format!("bundle missing colony.yaml in {bundle_dir}"));
    }
    let colony_config = load_colony_config(colony_path.to_str().ok_or("invalid path")?)?;

    // Load assertions.
    let assertions_path = dir.join("assertions.yaml");
    if !assertions_path.exists() {
        return Err(format!("bundle missing assertions.yaml in {bundle_dir}"));
    }
    let assertions_text = std::fs::read_to_string(&assertions_path)
        .map_err(|e| format!("failed to read assertions.yaml: {e}"))?;
    let assertion_list: Vec<Assertion> = serde_yaml::from_str(&assertions_text)
        .map_err(|e| format!("invalid assertions.yaml: {e}"))?;

    // Resolve instances and run calculator.
    let building_instances = resolve_instances(&colony_config, &registry)?;
    let imports: Vec<Flow> = colony_config
        .imports
        .iter()
        .map(|imp| Flow {
            commodity_id: imp.commodity_id.clone(),
            rate: imp.rate,
        })
        .collect();
    let report = BalanceCalculator::compute(&building_instances, &imports);

    // Check assertions.
    let results = check_assertions(&assertion_list, &report);

    let all_passed = results.iter().all(|r| r.passed);

    if json_mode {
        print_check_json(&results)?;
    } else {
        print_check_human(&results, all_passed);
    }

    Ok(i32::from(!all_passed))
}

/// Print check results as JSON array.
fn print_check_json(results: &[AssertionResult]) -> Result<(), String> {
    let json = serde_json::to_string_pretty(results)
        .map_err(|e| format!("failed to serialize results: {e}"))?;
    println!("{json}");
    Ok(())
}

/// Print check results in human-readable form.
fn print_check_human(results: &[AssertionResult], all_passed: bool) {
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║                  ASSERTION CHECK RESULTS                    ║");
    println!("╠══════════════════════════════════════════════════════════════╣");
    for r in results {
        let status = if r.passed { "PASS" } else { "FAIL" };
        println!("  [{status}] {}: {}", r.label, r.message);
    }
    println!("╠══════════════════════════════════════════════════════════════╣");
    let summary = if all_passed {
        format!("  Overall: PASS ({} assertion(s) passed)", results.len())
    } else {
        let failed = results.iter().filter(|r| !r.passed).count();
        format!(
            "  Overall: FAIL ({failed}/{} assertion(s) failed)",
            results.len()
        )
    };
    println!("{summary}");
    println!("╚══════════════════════════════════════════════════════════════╝");
}

/// Load and parse a content pack from the given directory path.
fn load_pack(pack_dir: &str) -> Result<ContentRegistry, String> {
    let dir = std::path::Path::new(pack_dir);
    if !dir.is_dir() {
        return Err(format!("pack directory not found: {pack_dir}"));
    }

    // Collect all YAML files in the directory.
    let file_names = [
        "pack.yaml",
        "commodities.yaml",
        "buildings.yaml",
        "recipes.yaml",
    ];
    let mut raw_contents: Vec<(String, String)> = Vec::new();

    for name in &file_names {
        let path = dir.join(name);
        if path.exists() {
            let text = std::fs::read_to_string(&path)
                .map_err(|e| format!("failed to read {}: {e}", path.display()))?;
            raw_contents.push(((*name).to_string(), text));
        }
    }

    // Build RawFile slice — we need lifetime-tied refs.
    let raw_files: Vec<RawFile<'_>> = raw_contents
        .iter()
        .map(|(name, text)| (name.as_str(), text.as_str()))
        .collect();

    PackLoader::load(&raw_files).map_err(|e| format!("content pack error: {e}"))
}

/// Parse a colony config YAML file.
fn load_colony_config(path: &str) -> Result<ColonyConfig, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| format!("failed to read colony config {path}: {e}"))?;
    serde_yaml::from_str(&text).map_err(|e| format!("invalid colony config {path}: {e}"))
}

/// Resolve building instances from the config using the content registry.
fn resolve_instances(
    config: &ColonyConfig,
    registry: &ContentRegistry,
) -> Result<Vec<BuildingInstance>, String> {
    let mut instances = Vec::new();

    for entry in &config.buildings {
        let building = registry
            .building(&entry.building_id)
            .ok_or_else(|| format!("unknown building id: {}", entry.building_id))?
            .clone();

        let recipe = if let Some(ref rid) = entry.recipe_id {
            let r = registry
                .recipe(rid)
                .ok_or_else(|| format!("unknown recipe id: {rid}"))?
                .clone();
            Some(r)
        } else {
            None
        };

        instances.push(BuildingInstance { building, recipe });
    }

    Ok(instances)
}

/// Map a [`BalanceVerdict`] to a process exit code.
fn verdict_exit_code(verdict: &BalanceVerdict) -> i32 {
    match verdict {
        BalanceVerdict::Closed => 0,
        BalanceVerdict::Bottleneck(_, _) | BalanceVerdict::Trivial => 1,
        BalanceVerdict::Impossible(_) => 2,
    }
}

/// Print the report as pretty-printed JSON.
fn print_json(report: &BalanceReport) -> Result<(), String> {
    let json = serde_json::to_string_pretty(report)
        .map_err(|e| format!("failed to serialize report: {e}"))?;
    println!("{json}");
    Ok(())
}

/// Print a human-readable balance report.
fn print_human(report: &BalanceReport) {
    // ── Per-commodity table ──────────────────────────────────────────────────
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║              COMMODITY BALANCE REPORT                       ║");
    println!("╠══════════════════════════════════════════════════════════════╣");
    println!(
        "  {:<20} {:>10} {:>12} {:>8}  Status",
        "Commodity", "Prod/sol", "Cons/sol", "Net"
    );
    println!("  {}", "-".repeat(60));

    for cb in &report.commodities {
        let status = commodity_status(cb.net_rate, cb.production_rate, cb.consumption_rate);
        println!(
            "  {:<20} {:>10.2} {:>12.2} {:>8.2}  {}",
            cb.commodity_id, cb.production_rate, cb.consumption_rate, cb.net_rate, status
        );
    }

    // ── Bottleneck highlight ─────────────────────────────────────────────────
    if let BalanceVerdict::Bottleneck(ref id, rate) = report.verdict {
        println!("  {}", "-".repeat(60));
        println!("  *** BOTTLENECK: {id} ({rate:+.2} units/sol)");
    }
    if let BalanceVerdict::Impossible(ref id) = report.verdict {
        println!("  {}", "-".repeat(60));
        println!("  *** IMPOSSIBLE: {id} has no source");
    }

    // ── Overall verdict ──────────────────────────────────────────────────────
    println!("╠══════════════════════════════════════════════════════════════╣");
    let verdict_str = match &report.verdict {
        BalanceVerdict::Closed => {
            "CLOSED  — chain is sustainable (binding constraint present)".to_string()
        }
        BalanceVerdict::Trivial => {
            "TRIVIAL — all surpluses are comfortable; no binding constraint".to_string()
        }
        BalanceVerdict::Bottleneck(id, rate) => {
            format!("BOTTLENECK — {id} deficit at {rate:+.2} units/sol")
        }
        BalanceVerdict::Impossible(id) => {
            format!("IMPOSSIBLE — {id} is consumed but never produced")
        }
    };
    println!("  Verdict: {verdict_str}");
    println!("╚══════════════════════════════════════════════════════════════╝");
}

/// Return a short status label for a commodity row.
fn commodity_status(net: f64, prod: f64, cons: f64) -> &'static str {
    if prod <= 0.0 && cons > 0.0 {
        "MISSING"
    } else if net < 0.0 {
        "DEFICIT"
    } else if net.abs() < 1e-9 {
        "EXACT"
    } else if cons <= 0.0 {
        "SURPLUS"
    } else {
        "OK"
    }
}

// ─── Integration tests ────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a minimal registry with iron_ore, iron_plate, smelter, smelt_iron.
    fn minimal_registry() -> ContentRegistry {
        use outpost_core::content::loader::PackLoader;

        let pack_yaml = r"
id: test-pack
name: Test Pack
version: '0.1.0'
";
        let commodities_yaml = r"
- id: iron_ore
  name: Iron Ore
  category: ore
  base_value: 1.0
- id: iron_plate
  name: Iron Plate
  category: metal
  base_value: 3.0
";
        let buildings_yaml = r"
- id: smelter
  name: Smelter
  category: production
";
        let recipes_yaml = r"
- id: smelt_iron
  name: Smelt Iron
  building: smelter
  cycle_sols: 2
  inputs:
    - id: iron_ore
      quantity: 2.0
  outputs:
    - id: iron_plate
      quantity: 1.0
";
        let files = vec![
            ("pack.yaml", pack_yaml),
            ("commodities.yaml", commodities_yaml),
            ("buildings.yaml", buildings_yaml),
            ("recipes.yaml", recipes_yaml),
        ];
        PackLoader::load(&files).expect("pack load should succeed")
    }

    #[test]
    fn resolve_instances_with_valid_registry() {
        let registry = minimal_registry();
        let config = ColonyConfig {
            buildings: vec![config::BuildingEntry {
                building_id: "smelter".into(),
                recipe_id: Some("smelt_iron".into()),
            }],
            imports: vec![config::ImportEntry {
                commodity_id: "iron_ore".into(),
                rate: 1.0,
            }],
        };
        let instances = resolve_instances(&config, &registry).expect("should resolve");
        assert_eq!(instances.len(), 1);
        assert_eq!(instances[0].building.id, "smelter");
        assert!(instances[0].recipe.is_some());
    }

    #[test]
    fn resolve_instances_unknown_building_returns_error() {
        let registry = minimal_registry();
        let config = ColonyConfig {
            buildings: vec![config::BuildingEntry {
                building_id: "nonexistent".into(),
                recipe_id: None,
            }],
            imports: vec![],
        };
        let err = resolve_instances(&config, &registry).unwrap_err();
        assert!(err.contains("unknown building id"), "got: {err}");
    }

    #[test]
    fn resolve_instances_unknown_recipe_returns_error() {
        let registry = minimal_registry();
        let config = ColonyConfig {
            buildings: vec![config::BuildingEntry {
                building_id: "smelter".into(),
                recipe_id: Some("no_such_recipe".into()),
            }],
            imports: vec![],
        };
        let err = resolve_instances(&config, &registry).unwrap_err();
        assert!(err.contains("unknown recipe id"), "got: {err}");
    }

    #[test]
    fn verdict_exit_codes_are_correct() {
        assert_eq!(verdict_exit_code(&BalanceVerdict::Closed), 0);
        assert_eq!(verdict_exit_code(&BalanceVerdict::Trivial), 1);
        assert_eq!(
            verdict_exit_code(&BalanceVerdict::Bottleneck("x".into(), -1.0)),
            1
        );
        assert_eq!(
            verdict_exit_code(&BalanceVerdict::Impossible("x".into())),
            2
        );
    }

    #[test]
    fn json_output_is_valid_json() {
        let registry = minimal_registry();
        let config = ColonyConfig {
            buildings: vec![config::BuildingEntry {
                building_id: "smelter".into(),
                recipe_id: Some("smelt_iron".into()),
            }],
            imports: vec![config::ImportEntry {
                commodity_id: "iron_ore".into(),
                rate: 1.0,
            }],
        };
        let instances = resolve_instances(&config, &registry).unwrap();
        let report = BalanceCalculator::compute(&instances, &[]);

        let mut buf = Vec::new();
        let json = serde_json::to_string_pretty(&report).expect("serialise");
        buf.extend_from_slice(json.as_bytes());

        // Must round-trip cleanly.
        let back: outpost_core::balance::BalanceReport =
            serde_json::from_slice(&buf).expect("deserialise");
        assert_eq!(back.commodities.len(), report.commodities.len());
    }

    #[test]
    fn human_report_runs_without_panic() {
        let registry = minimal_registry();
        let config = ColonyConfig {
            buildings: vec![config::BuildingEntry {
                building_id: "smelter".into(),
                recipe_id: Some("smelt_iron".into()),
            }],
            imports: vec![config::ImportEntry {
                commodity_id: "iron_ore".into(),
                rate: 1.0,
            }],
        };
        let instances = resolve_instances(&config, &registry).unwrap();
        let report = BalanceCalculator::compute(&instances, &[]);
        // Should not panic.
        print_human(&report);
    }

    #[test]
    fn load_pack_rejects_missing_directory() {
        let err = load_pack("/nonexistent/dir/xyz").unwrap_err();
        assert!(err.contains("not found"), "got: {err}");
    }

    #[test]
    fn check_subcommand_missing_bundle_dir_returns_error() {
        let args: Vec<String> = vec![
            "harness".into(),
            "check".into(),
            "/nonexistent/bundle/xyz".into(),
        ];
        let err = check_subcommand(&args).unwrap_err();
        assert!(err.contains("not found"), "got: {err}");
    }

    #[test]
    fn check_subcommand_missing_colony_yaml_returns_error() {
        // Use a temp dir with pack files but no colony.yaml
        let tmp = std::env::temp_dir().join("outpost_check_test_no_colony");
        std::fs::create_dir_all(&tmp).unwrap();
        // Write a minimal pack.yaml so load_pack succeeds
        std::fs::write(tmp.join("pack.yaml"), "id: t\nname: T\nversion: '0.1.0'\n").unwrap();
        let args: Vec<String> = vec![
            "harness".into(),
            "check".into(),
            tmp.to_str().unwrap().to_string(),
        ];
        let err = check_subcommand(&args).unwrap_err();
        assert!(err.contains("colony.yaml"), "got: {err}");
        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn check_subcommand_all_pass_returns_zero() {
        use std::fs;

        let tmp = std::env::temp_dir().join("outpost_check_test_all_pass");
        fs::create_dir_all(&tmp).unwrap();

        fs::write(tmp.join("pack.yaml"), "id: t\nname: T\nversion: '0.1.0'\n").unwrap();
        fs::write(
            tmp.join("commodities.yaml"),
            "- id: ore\n  name: Ore\n  category: raw\n  base_value: 1.0\n\
             - id: plate\n  name: Plate\n  category: metal\n  base_value: 3.0\n",
        )
        .unwrap();
        fs::write(
            tmp.join("buildings.yaml"),
            "- id: smelter\n  name: Smelter\n  category: production\n",
        )
        .unwrap();
        fs::write(
            tmp.join("recipes.yaml"),
            "- id: smelt\n  name: Smelt\n  building: smelter\n  cycle_sols: 1\n  \
             inputs:\n    - id: ore\n      quantity: 2.0\n  \
             outputs:\n    - id: plate\n      quantity: 1.0\n",
        )
        .unwrap();
        fs::write(
            tmp.join("colony.yaml"),
            "buildings:\n  - building_id: smelter\n    recipe_id: smelt\n\
             imports:\n  - commodity_id: ore\n    rate: 2.0\n",
        )
        .unwrap();
        fs::write(
            tmp.join("assertions.yaml"),
            "- commodity: plate\n  expect: closed\n- expect: closed\n",
        )
        .unwrap();

        let args: Vec<String> = vec![
            "harness".into(),
            "check".into(),
            tmp.to_str().unwrap().to_string(),
        ];
        let code = check_subcommand(&args).expect("should succeed");
        assert_eq!(code, 0, "all assertions should pass");

        fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn check_subcommand_failing_assertion_returns_one() {
        use std::fs;

        let tmp = std::env::temp_dir().join("outpost_check_test_fail");
        fs::create_dir_all(&tmp).unwrap();

        fs::write(tmp.join("pack.yaml"), "id: t\nname: T\nversion: '0.1.0'\n").unwrap();
        fs::write(
            tmp.join("commodities.yaml"),
            "- id: ore\n  name: Ore\n  category: raw\n  base_value: 1.0\n\
             - id: plate\n  name: Plate\n  category: metal\n  base_value: 3.0\n",
        )
        .unwrap();
        fs::write(
            tmp.join("buildings.yaml"),
            "- id: smelter\n  name: Smelter\n  category: production\n",
        )
        .unwrap();
        fs::write(
            tmp.join("recipes.yaml"),
            "- id: smelt\n  name: Smelt\n  building: smelter\n  cycle_sols: 1\n  \
             inputs:\n    - id: ore\n      quantity: 2.0\n  \
             outputs:\n    - id: plate\n      quantity: 1.0\n",
        )
        .unwrap();
        fs::write(
            tmp.join("colony.yaml"),
            // Only 1.0 ore imported — deficit → Bottleneck
            "buildings:\n  - building_id: smelter\n    recipe_id: smelt\n\
             imports:\n  - commodity_id: ore\n    rate: 1.0\n",
        )
        .unwrap();
        fs::write(
            tmp.join("assertions.yaml"),
            // Expects closed overall — but it's a bottleneck → should fail
            "- expect: closed\n  label: overall-should-fail\n",
        )
        .unwrap();

        let args: Vec<String> = vec![
            "harness".into(),
            "check".into(),
            tmp.to_str().unwrap().to_string(),
        ];
        let code = check_subcommand(&args).expect("should run without error");
        assert_eq!(code, 1, "failing assertion should return exit code 1");

        fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn check_subcommand_json_output_is_valid() {
        use std::fs;

        let tmp = std::env::temp_dir().join("outpost_check_test_json");
        fs::create_dir_all(&tmp).unwrap();

        fs::write(tmp.join("pack.yaml"), "id: t\nname: T\nversion: '0.1.0'\n").unwrap();
        fs::write(
            tmp.join("commodities.yaml"),
            "- id: ore\n  name: Ore\n  category: raw\n  base_value: 1.0\n\
             - id: plate\n  name: Plate\n  category: metal\n  base_value: 3.0\n",
        )
        .unwrap();
        fs::write(
            tmp.join("buildings.yaml"),
            "- id: smelter\n  name: Smelter\n  category: production\n",
        )
        .unwrap();
        fs::write(
            tmp.join("recipes.yaml"),
            "- id: smelt\n  name: Smelt\n  building: smelter\n  cycle_sols: 1\n  \
             inputs:\n    - id: ore\n      quantity: 2.0\n  \
             outputs:\n    - id: plate\n      quantity: 1.0\n",
        )
        .unwrap();
        fs::write(
            tmp.join("colony.yaml"),
            "buildings:\n  - building_id: smelter\n    recipe_id: smelt\n\
             imports:\n  - commodity_id: ore\n    rate: 2.0\n",
        )
        .unwrap();
        fs::write(
            tmp.join("assertions.yaml"),
            "- commodity: plate\n  expect: closed\n",
        )
        .unwrap();

        // Build the results manually to test JSON serialisation
        let registry = load_pack(tmp.to_str().unwrap()).unwrap();
        let colony = load_colony_config(tmp.join("colony.yaml").to_str().unwrap()).unwrap();
        let instances = resolve_instances(&colony, &registry).unwrap();
        let imports: Vec<outpost_core::balance::Flow> = colony
            .imports
            .iter()
            .map(|i| outpost_core::balance::Flow {
                commodity_id: i.commodity_id.clone(),
                rate: i.rate,
            })
            .collect();
        let report = outpost_core::balance::BalanceCalculator::compute(&instances, &imports);
        let assertion_list: Vec<crate::assertions::Assertion> =
            serde_yaml::from_str("- commodity: plate\n  expect: closed\n").unwrap();
        let results = crate::assertions::check_assertions(&assertion_list, &report);
        let json_str = serde_json::to_string_pretty(&results).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        assert!(parsed.is_array());
        assert_eq!(parsed.as_array().unwrap().len(), 1);

        fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn colony_config_round_trips_yaml() {
        let yaml = r"
buildings:
  - building_id: smelter
    recipe_id: smelt_iron
  - building_id: smelter
imports:
  - commodity_id: iron_ore
    rate: 5.0
";
        let cfg: ColonyConfig = serde_yaml::from_str(yaml).expect("parse");
        assert_eq!(cfg.buildings.len(), 2);
        assert_eq!(cfg.buildings[0].recipe_id.as_deref(), Some("smelt_iron"));
        assert!(cfg.buildings[1].recipe_id.is_none());
        assert_eq!(cfg.imports.len(), 1);
        assert!((cfg.imports[0].rate - 5.0).abs() < 1e-9);
    }
}
