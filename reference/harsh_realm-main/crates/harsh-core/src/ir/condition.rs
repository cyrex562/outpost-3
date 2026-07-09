//! Condition expression AST — the Rust IR mirror of `harsh_realm.dsl.ast`.
//!
//! The serde representation is deliberately byte-compatible with the Python
//! Pydantic models: the same `node_type` discriminator, the same operator
//! spellings, and the same field names. This lets existing authored records and
//! the R1 parser/evaluator port share one wire format and makes Python⇄Rust
//! parity testing a direct equality check.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A condition expression node.
///
/// Internally tagged on `node_type`, matching `dsl/ast.py`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "node_type", rename_all = "snake_case")]
pub enum Expr {
    /// An int, float, str, bool, or null literal.
    Literal {
        kind: LiteralKind,
        /// The literal payload; `kind` disambiguates its JSON type.
        value: serde_json::Value,
    },
    /// A dotted path access like `entity.hp` or `event.target_id`.
    Path {
        root: PathRoot,
        #[serde(default)]
        parts: Vec<String>,
    },
    /// A two-operand operation: arithmetic, comparison, or logical.
    BinaryOp {
        op: BinaryOperator,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    /// A one-operand operation: `not` or numeric negation.
    UnaryOp {
        op: UnaryOperator,
        operand: Box<Expr>,
    },
    /// A whitelisted helper function call.
    FuncCall {
        name: String,
        #[serde(default)]
        args: Vec<Expr>,
    },
    /// A bracketed list expression like `[1, 2, 3]`.
    #[serde(rename = "list")]
    ListLiteral {
        #[serde(default)]
        items: Vec<Expr>,
    },
}

/// The concrete type tag carried by a literal node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum LiteralKind {
    Int,
    Float,
    Str,
    Bool,
    Null,
}

/// The root namespace a [`Expr::Path`] resolves against.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PathRoot {
    Entity,
    Event,
    World,
    Target,
    /// `self` is a Rust keyword, so the variant is spelled `SelfRef`.
    #[serde(rename = "self")]
    SelfRef,
    Local,
}

/// Binary operators. Spellings match the Python DSL exactly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum BinaryOperator {
    #[serde(rename = "==")]
    Eq,
    #[serde(rename = "!=")]
    Ne,
    #[serde(rename = "<")]
    Lt,
    #[serde(rename = "<=")]
    Le,
    #[serde(rename = ">")]
    Gt,
    #[serde(rename = ">=")]
    Ge,
    #[serde(rename = "+")]
    Add,
    #[serde(rename = "-")]
    Sub,
    #[serde(rename = "*")]
    Mul,
    #[serde(rename = "/")]
    Div,
    #[serde(rename = "and")]
    And,
    #[serde(rename = "or")]
    Or,
    #[serde(rename = "in")]
    In,
}

/// Unary operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum UnaryOperator {
    #[serde(rename = "not")]
    Not,
    #[serde(rename = "-")]
    Neg,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn literal_round_trips_with_python_shape() {
        let value = json!({"node_type": "literal", "kind": "int", "value": 5});
        let expr: Expr = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(
            expr,
            Expr::Literal {
                kind: LiteralKind::Int,
                value: json!(5),
            }
        );
        assert_eq!(serde_json::to_value(&expr).unwrap(), value);
    }

    #[test]
    fn path_defaults_empty_parts() {
        let value = json!({"node_type": "path", "root": "entity", "parts": ["hp"]});
        let expr: Expr = serde_json::from_value(value).unwrap();
        assert_eq!(
            expr,
            Expr::Path {
                root: PathRoot::Entity,
                parts: vec!["hp".to_string()],
            }
        );
    }

    #[test]
    fn self_root_serializes_as_self() {
        let expr = Expr::Path {
            root: PathRoot::SelfRef,
            parts: vec!["id".into()],
        };
        let v = serde_json::to_value(&expr).unwrap();
        assert_eq!(v["root"], json!("self"));
    }

    #[test]
    fn binary_op_uses_symbol_spelling() {
        let value = json!({
            "node_type": "binary_op",
            "op": ">=",
            "left": {"node_type": "path", "root": "entity", "parts": ["hp"]},
            "right": {"node_type": "literal", "kind": "int", "value": 1}
        });
        let expr: Expr = serde_json::from_value(value.clone()).unwrap();
        match &expr {
            Expr::BinaryOp { op, .. } => assert_eq!(*op, BinaryOperator::Ge),
            other => panic!("expected binary op, got {other:?}"),
        }
        assert_eq!(serde_json::to_value(&expr).unwrap(), value);
    }

    #[test]
    fn list_literal_serializes_as_list_tag() {
        let expr = Expr::ListLiteral { items: vec![] };
        let v = serde_json::to_value(&expr).unwrap();
        assert_eq!(v["node_type"], json!("list"));
    }

    #[test]
    fn func_call_and_unary_round_trip() {
        let value = json!({
            "node_type": "unary_op",
            "op": "not",
            "operand": {
                "node_type": "func_call",
                "name": "has_tag",
                "args": [{"node_type": "literal", "kind": "str", "value": "undead"}]
            }
        });
        let expr: Expr = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(serde_json::to_value(&expr).unwrap(), value);
    }
}
