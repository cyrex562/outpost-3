//! Concrete compute-callable registry for the procedure runner.
//!
//! Ported from `src/harsh_realm/procedures/compute_registry.py`. Python's async
//! callables become synchronous `Fn` closures returning a [`ProcedureValue`].

use std::collections::BTreeMap;

use crate::runtime::JsonObject;

use super::schema::ProcedureValue;

/// A registered compute callable: receives params, returns a value.
pub type ComputeCallable = Box<dyn Fn(&JsonObject) -> Result<ProcedureValue, String>>;

/// Stores and dispatches compute callables by qualified content id.
#[derive(Default)]
pub struct ComputeRegistry {
    callables: BTreeMap<String, ComputeCallable>,
}

impl ComputeRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        ComputeRegistry {
            callables: BTreeMap::new(),
        }
    }

    /// Register a callable under a qualified compute id.
    ///
    /// Returns an error if a callable is already registered under that id.
    pub fn register(&mut self, qualified_name: &str, fn_: ComputeCallable) -> Result<(), String> {
        if self.callables.contains_key(qualified_name) {
            return Err(format!(
                "Compute callable {qualified_name:?} is already registered."
            ));
        }
        self.callables.insert(qualified_name.to_string(), fn_);
        Ok(())
    }

    /// Invoke a registered callable and return its result.
    ///
    /// Returns an error if no callable is registered under `qualified_name`.
    pub fn invoke(
        &self,
        qualified_name: &str,
        params: &JsonObject,
    ) -> Result<ProcedureValue, String> {
        let fn_ = self.callables.get(qualified_name).ok_or_else(|| {
            format!("No compute callable registered for {qualified_name:?}.")
        })?;
        fn_(params)
    }

    /// Return registered compute ids in stable sorted order.
    pub fn list_registered(&self) -> Vec<String> {
        self.callables.keys().cloned().collect()
    }

    /// Register the built-in `xwn-core` compute callables.
    ///
    /// Ported from the deleted Python pack hook (`content/xwn-core/code/__init__.py`):
    /// `xwn-core:power_level_or_preset` and `xwn-core:disposition_from_chaos`,
    /// invoked by the `xwn-core:procedures.une_personality` procedure.
    pub fn register_builtins(&mut self) -> Result<(), String> {
        self.register("xwn-core:power_level_or_preset", Box::new(power_level_or_preset))?;
        self.register("xwn-core:disposition_from_chaos", Box::new(disposition_from_chaos))?;
        Ok(())
    }

    /// Construct a registry pre-populated with the built-in `xwn-core` callables.
    pub fn with_builtins() -> Self {
        let mut registry = ComputeRegistry::new();
        registry
            .register_builtins()
            .expect("built-in compute ids are unique");
        registry
    }
}

/// `xwn-core:power_level_or_preset` — return `preset` if provided, else `rolled`.
fn power_level_or_preset(params: &JsonObject) -> Result<ProcedureValue, String> {
    let preset = string_param(params, "preset");
    if !preset.is_empty() {
        return Ok(ProcedureValue::String(preset));
    }
    Ok(ProcedureValue::String(string_param(params, "rolled")))
}

/// `xwn-core:disposition_from_chaos` — high chaos (>=7) nudges disposition down,
/// low chaos (<=3) nudges it up, otherwise neutral.
fn disposition_from_chaos(params: &JsonObject) -> Result<ProcedureValue, String> {
    let chaos = int_param(params, "chaos_factor", 5);
    let delta = if chaos >= 7 {
        -1
    } else if chaos <= 3 {
        1
    } else {
        0
    };
    Ok(ProcedureValue::from(delta))
}

/// Read a string param, coercing a non-string scalar via its display form.
fn string_param(params: &JsonObject, key: &str) -> String {
    match params.get(key) {
        Some(ProcedureValue::String(s)) => s.clone(),
        Some(ProcedureValue::Null) | None => String::new(),
        Some(other) => other.to_string(),
    }
}

/// Read an integer param, accepting numbers or numeric strings (mirrors the
/// Python hook's `int(str(...))`), falling back to `default`.
fn int_param(params: &JsonObject, key: &str, default: i64) -> i64 {
    match params.get(key) {
        Some(ProcedureValue::Number(n)) => n.as_i64().unwrap_or(default),
        Some(ProcedureValue::String(s)) => s.trim().parse().unwrap_or(default),
        _ => default,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_invoke_and_list() {
        let mut registry = ComputeRegistry::new();
        registry
            .register(
                "xwn-core:compute.double",
                Box::new(|params| {
                    let n = params.get("n").and_then(|v| v.as_i64()).unwrap_or(0);
                    Ok(serde_json::json!(n * 2))
                }),
            )
            .unwrap();
        registry
            .register("xwn-core:compute.noop", Box::new(|_| Ok(serde_json::json!(null))))
            .unwrap();

        let mut params = JsonObject::new();
        params.insert("n".into(), serde_json::json!(21));
        assert_eq!(
            registry.invoke("xwn-core:compute.double", &params).unwrap(),
            serde_json::json!(42)
        );
        assert_eq!(
            registry.list_registered(),
            vec![
                "xwn-core:compute.double".to_string(),
                "xwn-core:compute.noop".to_string()
            ]
        );
    }

    #[test]
    fn duplicate_registration_is_error() {
        let mut registry = ComputeRegistry::new();
        registry
            .register("c", Box::new(|_| Ok(serde_json::json!(1))))
            .unwrap();
        assert!(registry
            .register("c", Box::new(|_| Ok(serde_json::json!(2))))
            .is_err());
    }

    #[test]
    fn missing_callable_is_error() {
        let registry = ComputeRegistry::new();
        assert!(registry.invoke("nope", &JsonObject::new()).is_err());
    }

    #[test]
    fn with_builtins_registers_xwn_core_hooks() {
        let registry = ComputeRegistry::with_builtins();
        assert_eq!(
            registry.list_registered(),
            vec![
                "xwn-core:disposition_from_chaos".to_string(),
                "xwn-core:power_level_or_preset".to_string(),
            ]
        );
    }

    #[test]
    fn power_level_or_preset_prefers_preset() {
        let registry = ComputeRegistry::with_builtins();
        let mut params = JsonObject::new();
        params.insert("preset".into(), serde_json::json!("Boss"));
        params.insert("rolled".into(), serde_json::json!("Mook"));
        assert_eq!(
            registry.invoke("xwn-core:power_level_or_preset", &params).unwrap(),
            serde_json::json!("Boss")
        );
    }

    #[test]
    fn power_level_or_preset_falls_back_to_rolled() {
        let registry = ComputeRegistry::with_builtins();
        let mut params = JsonObject::new();
        params.insert("preset".into(), serde_json::json!(""));
        params.insert("rolled".into(), serde_json::json!("Mook"));
        assert_eq!(
            registry.invoke("xwn-core:power_level_or_preset", &params).unwrap(),
            serde_json::json!("Mook")
        );
    }

    #[test]
    fn disposition_from_chaos_thresholds() {
        let registry = ComputeRegistry::with_builtins();
        let invoke = |chaos: i64| {
            let mut params = JsonObject::new();
            params.insert("chaos_factor".into(), serde_json::json!(chaos));
            registry.invoke("xwn-core:disposition_from_chaos", &params).unwrap()
        };
        assert_eq!(invoke(8), serde_json::json!(-1));
        assert_eq!(invoke(7), serde_json::json!(-1));
        assert_eq!(invoke(5), serde_json::json!(0));
        assert_eq!(invoke(3), serde_json::json!(1));
        assert_eq!(invoke(1), serde_json::json!(1));
    }

    #[test]
    fn disposition_from_chaos_accepts_numeric_string() {
        let registry = ComputeRegistry::with_builtins();
        let mut params = JsonObject::new();
        params.insert("chaos_factor".into(), serde_json::json!("9"));
        assert_eq!(
            registry.invoke("xwn-core:disposition_from_chaos", &params).unwrap(),
            serde_json::json!(-1)
        );
    }
}
