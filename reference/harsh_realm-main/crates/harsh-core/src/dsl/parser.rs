//! Recursive-descent parser for the condition DSL — Rust port of
//! `harsh_realm.dsl.parser`.
//!
//! The tokenizer reproduces the Python tokenizer's *leftmost-first* alternation
//! (word operators like `and`/`in` match as prefixes, `true`/`false`/`null`
//! are reclassified from identifiers to symbols) so the two parsers produce
//! byte-identical ASTs — the basis for the R1 parity gate.

use serde_json::json;

use crate::ir::condition::{BinaryOperator, Expr, LiteralKind, PathRoot, UnaryOperator};

/// A parse failure with a byte offset into the source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub message: String,
    pub pos: usize,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} at byte {}", self.message, self.pos)
    }
}

impl std::error::Error for ParseError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TokenType {
    Identifier,
    Number,
    String,
    Symbol,
    Eof,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Token {
    ty: TokenType,
    value: String,
    pos: usize,
}

/// Multi-character symbol alternatives, in the exact order the Python SYMBOL
/// regex lists them (leftmost-first matters: `lte` before `lt`, `eq`/`in` as
/// word-operator prefixes).
const MULTI_SYMBOLS: &[&str] = &[
    "==", "!=", "<=", ">=", "&&", "||", "eq", "neq", "lte", "gte", "lt", "gt", "and", "or", "not",
    "in",
];
const SINGLE_SYMBOLS: &str = "=<>!+-*/().,[]";
/// Identifier values reclassified as symbols (the Python word-set check). The
/// operator words also appear in MULTI_SYMBOLS; these add the literals.
const WORD_SYMBOLS: &[&str] = &[
    "eq", "neq", "lt", "lte", "gt", "gte", "and", "or", "not", "in", "true", "false", "null",
];

fn tokenize(source: &str) -> Result<Vec<Token>, ParseError> {
    let bytes = source.as_bytes();
    let mut tokens = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i] as char;

        // Whitespace (SKIP): consume the run.
        if c == ' ' || c == '\t' || c == '\n' || c == '\r' {
            i += 1;
            continue;
        }

        // NUMBER: \d+(\.\d*)?
        if c.is_ascii_digit() {
            let start = i;
            while i < bytes.len() && (bytes[i] as char).is_ascii_digit() {
                i += 1;
            }
            if i < bytes.len() && bytes[i] as char == '.' {
                i += 1;
                while i < bytes.len() && (bytes[i] as char).is_ascii_digit() {
                    i += 1;
                }
            }
            tokens.push(Token {
                ty: TokenType::Number,
                value: source[start..i].to_string(),
                pos: start,
            });
            continue;
        }

        // STRING: '[^']*' | "[^"]*"
        if c == '\'' || c == '"' {
            if let Some(rel) = source[i + 1..].find(c) {
                let inner = source[i + 1..i + 1 + rel].to_string();
                tokens.push(Token {
                    ty: TokenType::String,
                    value: inner,
                    pos: i,
                });
                i += 1 + rel + 1;
                continue;
            }
            // Unterminated string falls through to MISMATCH (matches Python).
        }

        // SYMBOL: multi-char alternatives first (in order), then single chars.
        if let Some(sym) = MULTI_SYMBOLS.iter().find(|s| source[i..].starts_with(**s)) {
            tokens.push(Token {
                ty: TokenType::Symbol,
                value: (*sym).to_string(),
                pos: i,
            });
            i += sym.len();
            continue;
        }
        if SINGLE_SYMBOLS.contains(c) {
            tokens.push(Token {
                ty: TokenType::Symbol,
                value: c.to_string(),
                pos: i,
            });
            i += 1;
            continue;
        }

        // IDENTIFIER: [a-zA-Z_][a-zA-Z0-9_]*  (then word-set reclassification)
        if c.is_ascii_alphabetic() || c == '_' {
            let start = i;
            i += 1;
            while i < bytes.len() {
                let ch = bytes[i] as char;
                if ch.is_ascii_alphanumeric() || ch == '_' {
                    i += 1;
                } else {
                    break;
                }
            }
            let value = source[start..i].to_string();
            let ty = if WORD_SYMBOLS.contains(&value.as_str()) {
                TokenType::Symbol
            } else {
                TokenType::Identifier
            };
            tokens.push(Token {
                ty,
                value,
                pos: start,
            });
            continue;
        }

        return Err(ParseError {
            message: format!("Unexpected character {c:?}"),
            pos: i,
        });
    }
    tokens.push(Token {
        ty: TokenType::Eof,
        value: String::new(),
        pos: source.len(),
    });
    Ok(tokens)
}

const ROOTS: &[&str] = &["entity", "event", "world", "target", "self", "local"];

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn new(source: &str) -> Result<Self, ParseError> {
        Ok(Self {
            tokens: tokenize(source)?,
            pos: 0,
        })
    }

    fn parse(&mut self) -> Result<Expr, ParseError> {
        let result = self.expr()?;
        if !self.is_at_end() {
            return Err(self.err(format!("Unexpected token {:?}", self.peek().value)));
        }
        Ok(result)
    }

    fn expr(&mut self) -> Result<Expr, ParseError> {
        self.or_expr()
    }

    fn or_expr(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.and_expr()?;
        while self.match_symbol(&["or", "||"]) {
            let right = self.and_expr()?;
            expr = bin(BinaryOperator::Or, expr, right);
        }
        Ok(expr)
    }

    fn and_expr(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.not_expr()?;
        while self.match_symbol(&["and", "&&"]) {
            let right = self.not_expr()?;
            expr = bin(BinaryOperator::And, expr, right);
        }
        Ok(expr)
    }

    fn not_expr(&mut self) -> Result<Expr, ParseError> {
        if self.match_symbol(&["not", "!"]) {
            let operand = self.not_expr()?;
            return Ok(Expr::UnaryOp {
                op: UnaryOperator::Not,
                operand: Box::new(operand),
            });
        }
        self.comparison()
    }

    fn comparison(&mut self) -> Result<Expr, ParseError> {
        let expr = self.arith()?;
        let symbol = self.peek().value.clone();
        let op = match symbol.as_str() {
            "==" | "eq" => Some(BinaryOperator::Eq),
            "!=" | "neq" => Some(BinaryOperator::Ne),
            "<" | "lt" => Some(BinaryOperator::Lt),
            "<=" | "lte" => Some(BinaryOperator::Le),
            ">" | "gt" => Some(BinaryOperator::Gt),
            ">=" | "gte" => Some(BinaryOperator::Ge),
            "in" => Some(BinaryOperator::In),
            _ => None,
        };
        if let Some(op) = op {
            self.advance();
            let right = self.arith()?;
            return Ok(bin(op, expr, right));
        }
        Ok(expr)
    }

    fn arith(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.term()?;
        while self.match_symbol(&["+", "-"]) {
            let op = if self.previous().value == "+" {
                BinaryOperator::Add
            } else {
                BinaryOperator::Sub
            };
            let right = self.term()?;
            expr = bin(op, expr, right);
        }
        Ok(expr)
    }

    fn term(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.unary()?;
        while self.match_symbol(&["*", "/"]) {
            let op = if self.previous().value == "*" {
                BinaryOperator::Mul
            } else {
                BinaryOperator::Div
            };
            let right = self.unary()?;
            expr = bin(op, expr, right);
        }
        Ok(expr)
    }

    fn unary(&mut self) -> Result<Expr, ParseError> {
        if self.match_symbol(&["-"]) {
            let operand = self.unary()?;
            return Ok(Expr::UnaryOp {
                op: UnaryOperator::Neg,
                operand: Box::new(operand),
            });
        }
        self.primary()
    }

    fn primary(&mut self) -> Result<Expr, ParseError> {
        if self.match_symbol(&["("]) {
            let expr = self.expr()?;
            self.consume_symbol(")", "Expect ')' after expression")?;
            return Ok(expr);
        }

        if self.match_symbol(&["["]) {
            let mut items = Vec::new();
            if !self.check_symbol("]") {
                items.push(self.expr()?);
                while self.match_symbol(&[","]) {
                    items.push(self.expr()?);
                }
            }
            self.consume_symbol("]", "Expect ']' after list items")?;
            return Ok(Expr::ListLiteral { items });
        }

        let token = self.peek().clone();

        if token.ty == TokenType::Number {
            self.advance();
            return Ok(number_literal(&token.value));
        }

        if token.ty == TokenType::String {
            self.advance();
            return Ok(Expr::Literal {
                kind: LiteralKind::Str,
                value: json!(token.value),
            });
        }

        if self.match_symbol(&["true"]) {
            return Ok(Expr::Literal {
                kind: LiteralKind::Bool,
                value: json!(true),
            });
        }
        if self.match_symbol(&["false"]) {
            return Ok(Expr::Literal {
                kind: LiteralKind::Bool,
                value: json!(false),
            });
        }
        if self.match_symbol(&["null"]) {
            return Ok(Expr::Literal {
                kind: LiteralKind::Null,
                value: json!(null),
            });
        }

        if self.check_root() {
            return self.path();
        }

        if token.ty == TokenType::Identifier {
            if self.peek_next().value == "(" {
                return self.func_call();
            }
            return Err(self.err(format!("Unexpected identifier {:?}", token.value)));
        }

        Err(self.err(format!("Expect expression, got {:?}", token.value)))
    }

    fn path(&mut self) -> Result<Expr, ParseError> {
        let root_str = self.advance().value.clone();
        let root = match root_str.as_str() {
            "entity" => PathRoot::Entity,
            "event" => PathRoot::Event,
            "world" => PathRoot::World,
            "target" => PathRoot::Target,
            "self" => PathRoot::SelfRef,
            "local" => PathRoot::Local,
            other => return Err(self.err(format!("Unknown path root {other:?}"))),
        };
        let mut parts = Vec::new();
        while self.match_symbol(&["."]) {
            let token = self.consume(TokenType::Identifier, "Expect field name after '.'")?;
            parts.push(token.value);
        }
        Ok(Expr::Path { root, parts })
    }

    fn func_call(&mut self) -> Result<Expr, ParseError> {
        let name = self
            .consume(TokenType::Identifier, "Expect function name")?
            .value;
        self.consume_symbol("(", "Expect '(' after function name")?;
        let mut args = Vec::new();
        if !self.check_symbol(")") {
            args.push(self.expr()?);
            while self.match_symbol(&[","]) {
                args.push(self.expr()?);
            }
        }
        self.consume_symbol(")", "Expect ')' after arguments")?;
        Ok(Expr::FuncCall { name, args })
    }

    // --- token utilities ---

    fn peek(&self) -> &Token {
        &self.tokens[self.pos]
    }

    fn peek_next(&self) -> &Token {
        if self.is_at_end() {
            self.tokens.last().unwrap()
        } else {
            &self.tokens[self.pos + 1]
        }
    }

    fn previous(&self) -> &Token {
        &self.tokens[self.pos - 1]
    }

    fn is_at_end(&self) -> bool {
        self.peek().ty == TokenType::Eof
    }

    fn advance(&mut self) -> &Token {
        if !self.is_at_end() {
            self.pos += 1;
        }
        self.previous()
    }

    fn match_symbol(&mut self, symbols: &[&str]) -> bool {
        for s in symbols {
            if self.check_symbol(s) {
                self.advance();
                return true;
            }
        }
        false
    }

    fn check_symbol(&self, symbol: &str) -> bool {
        if self.is_at_end() {
            return false;
        }
        self.peek().ty == TokenType::Symbol && self.peek().value == symbol
    }

    fn check_root(&self) -> bool {
        if self.is_at_end() {
            return false;
        }
        ROOTS.contains(&self.peek().value.as_str())
    }

    fn consume(&mut self, ty: TokenType, message: &str) -> Result<Token, ParseError> {
        if self.peek().ty == ty {
            return Ok(self.advance().clone());
        }
        Err(self.err(message.to_string()))
    }

    fn consume_symbol(&mut self, symbol: &str, message: &str) -> Result<Token, ParseError> {
        if self.check_symbol(symbol) {
            return Ok(self.advance().clone());
        }
        Err(self.err(message.to_string()))
    }

    fn err(&self, message: String) -> ParseError {
        ParseError {
            message,
            pos: self.peek().pos,
        }
    }
}

fn bin(op: BinaryOperator, left: Expr, right: Expr) -> Expr {
    Expr::BinaryOp {
        op,
        left: Box::new(left),
        right: Box::new(right),
    }
}

fn number_literal(text: &str) -> Expr {
    if text.contains('.') {
        // float(text); Python keeps it as a float value.
        let f: f64 = text.parse().unwrap_or(0.0);
        Expr::Literal {
            kind: LiteralKind::Float,
            value: json!(f),
        }
    } else {
        let n: i64 = text.parse().unwrap_or(0);
        Expr::Literal {
            kind: LiteralKind::Int,
            value: json!(n),
        }
    }
}

/// Parse a condition expression string into an [`Expr`].
///
/// Mirrors `parse_condition`: a blank/whitespace-only source parses to the
/// boolean literal `true`.
pub fn parse_condition(source: &str) -> Result<Expr, ParseError> {
    if source.trim().is_empty() {
        return Ok(Expr::Literal {
            kind: LiteralKind::Bool,
            value: json!(true),
        });
    }
    Parser::new(source)?.parse()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn parse_json(src: &str) -> Value {
        serde_json::to_value(parse_condition(src).unwrap()).unwrap()
    }

    #[test]
    fn blank_parses_to_true() {
        assert_eq!(
            parse_json("   "),
            serde_json::json!({"node_type": "literal", "kind": "bool", "value": true})
        );
    }

    #[test]
    fn precedence_and_binds_tighter_than_or() {
        // a or b and c  =>  a or (b and c)
        let ast = parse_condition("entity.a or entity.b and entity.c").unwrap();
        match ast {
            Expr::BinaryOp {
                op: BinaryOperator::Or,
                right,
                ..
            } => assert!(matches!(
                *right,
                Expr::BinaryOp {
                    op: BinaryOperator::And,
                    ..
                }
            )),
            other => panic!("expected top-level or, got {other:?}"),
        }
    }

    #[test]
    fn comparison_and_arithmetic() {
        let v = parse_json("entity.hp >= 1 + 2");
        assert_eq!(v["op"], serde_json::json!(">="));
        assert_eq!(v["right"]["op"], serde_json::json!("+"));
    }

    #[test]
    fn word_operators_match_symbol_forms() {
        // `gte` is the word form of `>=`.
        let a = parse_json("entity.hp gte 1");
        let b = parse_json("entity.hp >= 1");
        assert_eq!(a, b);
    }

    #[test]
    fn func_call_with_args() {
        let v = parse_json("has_tag(entity, 'undead')");
        assert_eq!(v["node_type"], serde_json::json!("func_call"));
        assert_eq!(v["name"], serde_json::json!("has_tag"));
        assert_eq!(v["args"][1]["value"], serde_json::json!("undead"));
    }

    #[test]
    fn list_and_in_operator() {
        let v = parse_json("entity.class in ['warrior', 'expert']");
        assert_eq!(v["op"], serde_json::json!("in"));
        assert_eq!(v["right"]["node_type"], serde_json::json!("list"));
    }

    #[test]
    fn unterminated_string_errors() {
        assert!(parse_condition("'oops").is_err());
    }

    #[test]
    fn float_vs_int_literal_kind() {
        assert_eq!(parse_json("3")["kind"], serde_json::json!("int"));
        assert_eq!(parse_json("3.5")["kind"], serde_json::json!("float"));
    }
}
