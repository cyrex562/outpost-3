//! Pure evaluator for condition expressions — Rust port of
//! `harsh_realm.dsl.evaluator` + `dsl.context`.
//!
//! Unlike the Python evaluator (async, service-backed), this evaluator is
//! synchronous and side-effect-free: the caller materializes entity state into
//! an [`EvalContext`] (resolving `entity.hp`, tags, traits, etc. ahead of time),
//! and the engine evaluates against that snapshot. This keeps the core pure and
//! embeddable. Operator and function semantics match the Python evaluator for
//! the realistic value types used in conditions (numbers, strings, bools,
//! lists); see the tests for the pinned behaviours.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::ir::condition::{BinaryOperator, Expr, PathRoot, UnaryOperator};

/// Raised when evaluation fails (type mismatch, unknown path/function, etc.).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvalError(pub String);

impl std::fmt::Display for EvalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for EvalError {}

fn err<T>(msg: impl Into<String>) -> Result<T, EvalError> {
    Err(EvalError(msg.into()))
}

/// A materialized view of one entity, supplied by the host.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EntityView {
    #[serde(default)]
    pub id: String,
    /// Pre-resolved scalar/struct fields (e.g. `hp`, `max_hp`, `attributes`).
    #[serde(default)]
    pub fields: Map<String, Value>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub traits: Vec<String>,
    #[serde(default)]
    pub statuses: Vec<String>,
    /// Action ids this entity can attempt (for `has_action`).
    #[serde(default)]
    pub actions: Vec<String>,
    /// Skill ids this entity has any proficiency in (for `has_skill`).
    #[serde(default)]
    pub skills: Vec<String>,
}

impl EntityView {
    /// The combined object view used for path access: fields + id/tags/traits.
    fn object(&self) -> Value {
        let mut obj = self.fields.clone();
        obj.entry("id".to_string())
            .or_insert_with(|| Value::String(self.id.clone()));
        obj.insert(
            "tags".to_string(),
            Value::Array(self.tags.iter().cloned().map(Value::String).collect()),
        );
        obj.insert(
            "traits".to_string(),
            Value::Array(self.traits.iter().cloned().map(Value::String).collect()),
        );
        Value::Object(obj)
    }
}

/// The data available to a condition evaluation. The host fills this in.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EvalContext {
    /// Entities addressable by id (must include `self_id` and any `target_id`).
    #[serde(default)]
    pub entities: BTreeMap<String, EntityView>,
    #[serde(default)]
    pub self_id: String,
    #[serde(default)]
    pub target_id: Option<String>,
    #[serde(default)]
    pub event: Map<String, Value>,
    #[serde(default)]
    pub world: Map<String, Value>,
    #[serde(default)]
    pub local: Map<String, Value>,
}

impl EvalContext {
    fn entity(&self, id: &str) -> Result<&EntityView, EvalError> {
        self.entities
            .get(id)
            .ok_or_else(|| EvalError(format!("Unknown entity in context: {id}")))
    }
}

/// Evaluate an expression to a JSON value.
pub fn evaluate(expr: &Expr, ctx: &EvalContext) -> Result<Value, EvalError> {
    match expr {
        Expr::Literal { value, .. } => Ok(value.clone()),
        Expr::Path { root, parts } => resolve_path(*root, parts, ctx),
        Expr::BinaryOp { op, left, right } => eval_binary(*op, left, right, ctx),
        Expr::UnaryOp { op, operand } => eval_unary(*op, operand, ctx),
        Expr::FuncCall { name, args } => eval_func(name, args, ctx),
        Expr::ListLiteral { items } => {
            let mut out = Vec::with_capacity(items.len());
            for item in items {
                out.push(evaluate(item, ctx)?);
            }
            Ok(Value::Array(out))
        }
    }
}

/// Evaluate an expression and require a boolean result.
pub fn evaluate_to_bool(expr: &Expr, ctx: &EvalContext) -> Result<bool, EvalError> {
    match evaluate(expr, ctx)? {
        Value::Bool(b) => Ok(b),
        other => err(format!("Expression evaluated to {other}, expected bool")),
    }
}

fn eval_binary(
    op: BinaryOperator,
    left: &Expr,
    right: &Expr,
    ctx: &EvalContext,
) -> Result<Value, EvalError> {
    // Short-circuiting logical operators require boolean operands.
    if op == BinaryOperator::And || op == BinaryOperator::Or {
        let l = as_bool(&evaluate(left, ctx)?, op)?;
        if op == BinaryOperator::And && !l {
            return Ok(Value::Bool(false));
        }
        if op == BinaryOperator::Or && l {
            return Ok(Value::Bool(true));
        }
        let r = as_bool(&evaluate(right, ctx)?, op)?;
        return Ok(Value::Bool(r));
    }

    let l = evaluate(left, ctx)?;
    let r = evaluate(right, ctx)?;

    match op {
        BinaryOperator::Eq => Ok(Value::Bool(py_eq(&l, &r))),
        BinaryOperator::Ne => Ok(Value::Bool(!py_eq(&l, &r))),
        BinaryOperator::Lt => Ok(Value::Bool(compare(&l, &r)? == std::cmp::Ordering::Less)),
        BinaryOperator::Le => Ok(Value::Bool(compare(&l, &r)? != std::cmp::Ordering::Greater)),
        BinaryOperator::Gt => Ok(Value::Bool(compare(&l, &r)? == std::cmp::Ordering::Greater)),
        BinaryOperator::Ge => Ok(Value::Bool(compare(&l, &r)? != std::cmp::Ordering::Less)),
        BinaryOperator::In => eval_in(&l, &r),
        BinaryOperator::Add => arith_add(&l, &r),
        BinaryOperator::Sub => arith_num(&l, &r, |a, b| a - b, |a, b| a - b, "-"),
        BinaryOperator::Mul => arith_num(&l, &r, |a, b| a * b, |a, b| a * b, "*"),
        BinaryOperator::Div => {
            let (a, b) = (
                as_num(&l).ok_or_else(|| EvalError("'/' requires numeric operands".into()))?,
                as_num(&r).ok_or_else(|| EvalError("'/' requires numeric operands".into()))?,
            );
            Ok(json_f64(a / b))
        }
        BinaryOperator::And | BinaryOperator::Or => unreachable!("handled above"),
    }
}

fn eval_unary(op: UnaryOperator, operand: &Expr, ctx: &EvalContext) -> Result<Value, EvalError> {
    let v = evaluate(operand, ctx)?;
    match op {
        UnaryOperator::Not => match v {
            Value::Bool(b) => Ok(Value::Bool(!b)),
            other => err(format!("'not' requires boolean operand, got {other}")),
        },
        UnaryOperator::Neg => {
            if let Some(i) = as_i64(&v) {
                if is_intish(&v) {
                    return Ok(Value::from(-i));
                }
            }
            match as_num(&v) {
                Some(n) => Ok(json_f64(-n)),
                None => err(format!("Unary '-' requires numeric operand, got {v}")),
            }
        }
    }
}

fn eval_func(name: &str, args: &[Expr], ctx: &EvalContext) -> Result<Value, EvalError> {
    let values: Vec<Value> = args
        .iter()
        .map(|a| evaluate(a, ctx))
        .collect::<Result<_, _>>()?;

    match name {
        "has_tag" => {
            check_args(name, &values, 2)?;
            let id = arg_to_entity_id(&values[0])?;
            let tag = value_to_str(&values[1]);
            Ok(Value::Bool(ctx.entity(&id)?.tags.contains(&tag)))
        }
        "has_trait" => {
            check_args(name, &values, 2)?;
            let id = arg_to_entity_id(&values[0])?;
            let trait_id = value_to_str(&values[1]);
            Ok(Value::Bool(ctx.entity(&id)?.traits.contains(&trait_id)))
        }
        "has_status" => {
            check_args(name, &values, 2)?;
            let id = arg_to_entity_id(&values[0])?;
            let status_id = value_to_str(&values[1]);
            Ok(Value::Bool(ctx.entity(&id)?.statuses.contains(&status_id)))
        }
        "has_action" => {
            check_args(name, &values, 2)?;
            let id = arg_to_entity_id(&values[0])?;
            let action_id = value_to_str(&values[1]);
            Ok(Value::Bool(ctx.entity(&id)?.actions.contains(&action_id)))
        }
        "has_skill" => {
            check_args(name, &values, 2)?;
            let id = arg_to_entity_id(&values[0])?;
            let skill_id = value_to_str(&values[1]);
            Ok(Value::Bool(ctx.entity(&id)?.skills.contains(&skill_id)))
        }
        "len" => {
            check_args(name, &values, 1)?;
            match &values[0] {
                Value::Array(a) => Ok(Value::from(a.len())),
                Value::String(s) => Ok(Value::from(s.chars().count())),
                Value::Object(o) => Ok(Value::from(o.len())),
                other => err(format!("'len' requires collection, got {other}")),
            }
        }
        "min" => fold_minmax(&values, true),
        "max" => fold_minmax(&values, false),
        "abs" => {
            check_args(name, &values, 1)?;
            if is_intish(&values[0]) {
                if let Some(i) = as_i64(&values[0]) {
                    return Ok(Value::from(i.abs()));
                }
            }
            match as_num(&values[0]) {
                Some(n) => Ok(json_f64(n.abs())),
                None => err(format!("'abs' requires numeric operand, got {}", values[0])),
            }
        }
        other => err(format!("Unknown helper function: {other}")),
    }
}

// --- path resolution -------------------------------------------------------

fn resolve_path(root: PathRoot, parts: &[String], ctx: &EvalContext) -> Result<Value, EvalError> {
    match root {
        PathRoot::Entity | PathRoot::SelfRef => resolve_entity(ctx.entity(&ctx.self_id)?, parts),
        PathRoot::Target => {
            let id = ctx
                .target_id
                .as_ref()
                .ok_or_else(|| EvalError("Cannot resolve 'target': no target entity".into()))?;
            resolve_entity(ctx.entity(id)?, parts)
        }
        PathRoot::Event => walk(&Value::Object(ctx.event.clone()), parts, "event"),
        PathRoot::World => walk(&Value::Object(ctx.world.clone()), parts, "world"),
        PathRoot::Local => walk(&Value::Object(ctx.local.clone()), parts, "local"),
    }
}

fn resolve_entity(entity: &EntityView, parts: &[String]) -> Result<Value, EvalError> {
    if parts.is_empty() {
        return Ok(Value::String(entity.id.clone()));
    }
    walk(&entity.object(), parts, "entity")
}

fn walk(root: &Value, parts: &[String], root_name: &str) -> Result<Value, EvalError> {
    let mut current = root;
    let mut path = root_name.to_string();
    for part in parts {
        match current {
            Value::Object(map) => match map.get(part) {
                Some(next) => current = next,
                None => return err(format!("Path not found: {path}.{part}")),
            },
            _ => return err(format!("Path not found: {path}.{part}")),
        }
        path.push('.');
        path.push_str(part);
    }
    Ok(current.clone())
}

// --- value helpers (Python-matching numeric semantics) ---------------------

fn is_intish(v: &Value) -> bool {
    v.is_i64() || v.is_u64() || v.is_boolean()
}

fn as_i64(v: &Value) -> Option<i64> {
    match v {
        Value::Number(n) => n.as_i64(),
        Value::Bool(b) => Some(*b as i64),
        _ => None,
    }
}

/// Numeric coercion including bool (Python treats `bool` as an `int` subclass).
fn as_num(v: &Value) -> Option<f64> {
    match v {
        Value::Number(n) => n.as_f64(),
        Value::Bool(b) => Some(if *b { 1.0 } else { 0.0 }),
        _ => None,
    }
}

fn json_f64(n: f64) -> Value {
    serde_json::Number::from_f64(n)
        .map(Value::Number)
        .unwrap_or(Value::Null)
}

fn as_bool(v: &Value, op: BinaryOperator) -> Result<bool, EvalError> {
    match v {
        Value::Bool(b) => Ok(*b),
        other => err(format!("'{op:?}' requires boolean operands, got {other}")),
    }
}

/// Python `==` semantics for the value types used in conditions.
fn py_eq(a: &Value, b: &Value) -> bool {
    match (as_num(a), as_num(b)) {
        (Some(x), Some(y)) => x == y,
        _ => a == b,
    }
}

/// Ordering for `<`, `<=`, `>`, `>=`. Numbers compare numerically; strings
/// lexicographically; anything else is a type error (as in Python).
fn compare(a: &Value, b: &Value) -> Result<std::cmp::Ordering, EvalError> {
    if let (Some(x), Some(y)) = (as_num(a), as_num(b)) {
        return x
            .partial_cmp(&y)
            .ok_or_else(|| EvalError("cannot compare NaN".into()));
    }
    if let (Value::String(x), Value::String(y)) = (a, b) {
        return Ok(x.cmp(y));
    }
    err(format!("cannot order {a} and {b}"))
}

fn eval_in(left: &Value, right: &Value) -> Result<Value, EvalError> {
    match right {
        Value::Array(items) => Ok(Value::Bool(items.iter().any(|e| py_eq(e, left)))),
        Value::String(s) => match left {
            Value::String(needle) => Ok(Value::Bool(s.contains(needle.as_str()))),
            _ => err("'in' over a string requires a string left operand"),
        },
        other => err(format!(
            "'in' requires container right operand, got {other}"
        )),
    }
}

fn arith_add(l: &Value, r: &Value) -> Result<Value, EvalError> {
    if as_num(l).is_some() && as_num(r).is_some() {
        return arith_num(l, r, |a, b| a + b, |a, b| a + b, "+");
    }
    match (l, r) {
        (Value::String(a), Value::String(b)) => Ok(Value::String(format!("{a}{b}"))),
        (Value::Array(a), Value::Array(b)) => {
            let mut out = a.clone();
            out.extend(b.clone());
            Ok(Value::Array(out))
        }
        _ => err(format!("'+' failed for {l} and {r}")),
    }
}

fn arith_num(
    l: &Value,
    r: &Value,
    int_op: fn(i64, i64) -> i64,
    float_op: fn(f64, f64) -> f64,
    sym: &str,
) -> Result<Value, EvalError> {
    let (a, b) = match (as_num(l), as_num(r)) {
        (Some(a), Some(b)) => (a, b),
        _ => {
            return err(format!(
                "'{sym}' requires numeric operands, got {l} and {r}"
            ))
        }
    };
    if is_intish(l) && is_intish(r) {
        let (ai, bi) = (as_i64(l).unwrap(), as_i64(r).unwrap());
        Ok(Value::from(int_op(ai, bi)))
    } else {
        Ok(json_f64(float_op(a, b)))
    }
}

fn fold_minmax(values: &[Value], want_min: bool) -> Result<Value, EvalError> {
    let mut iter = values.iter();
    let mut best = iter
        .next()
        .ok_or_else(|| EvalError("min/max of empty args".into()))?
        .clone();
    for v in iter {
        let ord = compare(v, &best)?;
        let take = if want_min {
            ord == std::cmp::Ordering::Less
        } else {
            ord == std::cmp::Ordering::Greater
        };
        if take {
            best = v.clone();
        }
    }
    Ok(best)
}

fn check_args(name: &str, args: &[Value], expected: usize) -> Result<(), EvalError> {
    if args.len() != expected {
        return err(format!(
            "Function {name:?} expects {expected} args, got {}",
            args.len()
        ));
    }
    Ok(())
}

fn arg_to_entity_id(arg: &Value) -> Result<String, EvalError> {
    match arg {
        Value::String(s) => Ok(s.clone()),
        Value::Object(o) => match o.get("id") {
            Some(Value::String(s)) => Ok(s.clone()),
            _ => err("Expected entity ID or object with string id"),
        },
        other => err(format!("Expected entity ID or object with ID, got {other}")),
    }
}

fn value_to_str(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dsl::parser::parse_condition;
    use serde_json::json;

    fn ctx_with(entity: EntityView) -> EvalContext {
        let mut entities = BTreeMap::new();
        let id = entity.id.clone();
        entities.insert(id.clone(), entity);
        EvalContext {
            entities,
            self_id: id,
            ..Default::default()
        }
    }

    fn eval_str(src: &str, ctx: &EvalContext) -> Value {
        evaluate(&parse_condition(src).unwrap(), ctx).unwrap()
    }

    #[test]
    fn arithmetic_and_comparison() {
        let ctx = EvalContext::default();
        assert_eq!(eval_str("1 + 2 * 3", &ctx), json!(7));
        assert_eq!(eval_str("(1 + 2) * 3", &ctx), json!(9));
        assert_eq!(eval_str("3 >= 2", &ctx), json!(true));
        assert_eq!(eval_str("3 / 2", &ctx), json!(1.5));
        assert_eq!(eval_str("3 == 3.0", &ctx), json!(true));
    }

    #[test]
    fn short_circuit_logic() {
        let ctx = EvalContext::default();
        assert_eq!(eval_str("false and (1 / 0 == 0)", &ctx), json!(false));
        assert_eq!(eval_str("true or (1 / 0 == 0)", &ctx), json!(true));
        assert_eq!(eval_str("not false", &ctx), json!(true));
    }

    #[test]
    fn entity_field_and_hp_path() {
        let mut fields = Map::new();
        fields.insert("hp".into(), json!(5));
        fields.insert("class".into(), json!("warrior"));
        let ctx = ctx_with(EntityView {
            id: "pc".into(),
            fields,
            ..Default::default()
        });
        assert_eq!(eval_str("entity.hp >= 1", &ctx), json!(true));
        assert_eq!(eval_str("entity.class == 'warrior'", &ctx), json!(true));
        // Bare `entity` resolves to its id.
        assert_eq!(eval_str("entity == 'pc'", &ctx), json!(true));
    }

    #[test]
    fn has_tag_and_in_operator() {
        let ctx = ctx_with(EntityView {
            id: "pc".into(),
            tags: vec!["undead".into(), "boss".into()],
            ..Default::default()
        });
        assert_eq!(eval_str("has_tag(entity, 'undead')", &ctx), json!(true));
        assert_eq!(eval_str("has_tag(entity, 'angel')", &ctx), json!(false));
        assert_eq!(eval_str("'boss' in entity.tags", &ctx), json!(true));
    }

    #[test]
    fn event_and_local_paths() {
        let mut event = Map::new();
        event.insert("damage".into(), json!(8));
        let mut local = Map::new();
        local.insert("threshold".into(), json!(5));
        let ctx = EvalContext {
            event,
            local,
            ..Default::default()
        };
        assert_eq!(
            eval_str("event.damage > local.threshold", &ctx),
            json!(true)
        );
    }

    #[test]
    fn functions_min_max_len_abs() {
        let ctx = EvalContext::default();
        assert_eq!(eval_str("min(3, 1, 2)", &ctx), json!(1));
        assert_eq!(eval_str("max(3, 1, 2)", &ctx), json!(3));
        assert_eq!(eval_str("len(['a', 'b'])", &ctx), json!(2));
        assert_eq!(eval_str("abs(0 - 4)", &ctx), json!(4));
    }

    #[test]
    fn type_errors_surface() {
        let ctx = EvalContext::default();
        assert!(evaluate(&parse_condition("1 and true").unwrap(), &ctx).is_err());
        assert!(evaluate(&parse_condition("'a' < 3").unwrap(), &ctx).is_err());
    }

    #[test]
    fn missing_path_errors() {
        let ctx = ctx_with(EntityView {
            id: "pc".into(),
            ..Default::default()
        });
        assert!(evaluate(&parse_condition("entity.nope").unwrap(), &ctx).is_err());
    }

    #[test]
    fn has_action_and_has_skill() {
        let ctx = ctx_with(EntityView {
            id: "pc".into(),
            actions: vec!["parry".into()],
            skills: vec!["stab".into()],
            ..Default::default()
        });
        assert_eq!(eval_str("has_action(entity, 'parry')", &ctx), json!(true));
        assert_eq!(eval_str("has_action(entity, 'dodge')", &ctx), json!(false));
        assert_eq!(eval_str("has_skill(entity, 'stab')", &ctx), json!(true));
        assert_eq!(eval_str("has_skill(entity, 'judo')", &ctx), json!(false));
    }
}
