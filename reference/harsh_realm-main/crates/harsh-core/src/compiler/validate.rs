//! Semantic validation of a compiled record: the two embedded expression
//! languages — condition strings (`when:`) and dice strings — are parsed so a
//! malformed expression is a compile-time `SyntaxError` rather than a runtime
//! surprise (grammar spec §6).
//!
//! Outcome-branch `when:` predicates (`crit`/`success`/`degree >= 1`) are a
//! separate result-keyed mini-language, not the full condition DSL, so they are
//! NOT validated here.

use crate::compiler::diagnostics::Diagnostic;
use crate::dsl::parse_condition;
use crate::ir::effect::Trigger;
use crate::ir::ComponentRecord;
use crate::resolvers::parse_damage_expr;

/// Validate the dice and condition strings embedded in a record.
pub fn validate_semantics(record: &ComponentRecord) -> Vec<Diagnostic> {
    let mut diags = Vec::new();

    for trigger in triggers_of(record) {
        if let Err(e) = parse_condition(&trigger.when) {
            diags.push(Diagnostic::syntax(format!(
                "invalid `when` condition {:?}: {e}",
                trigger.when
            )));
        }
    }

    for dice in dice_of(record) {
        if let Err(e) = parse_damage_expr(dice) {
            diags.push(Diagnostic::syntax(format!("invalid dice {dice:?}: {e}")));
        }
    }

    diags
}

/// Every trigger carried by a record (standalone or owned by a trait/status/skill).
pub(crate) fn triggers_of(record: &ComponentRecord) -> Vec<&Trigger> {
    match record {
        ComponentRecord::Trigger(t) => vec![t],
        ComponentRecord::Trait(t) => t.triggers.iter().collect(),
        ComponentRecord::StatusEffect(s) => s.triggers.iter().collect(),
        ComponentRecord::Skill(s) => s.triggers.iter().collect(),
        _ => Vec::new(),
    }
}

/// Dice-expression string fields on a record.
fn dice_of(record: &ComponentRecord) -> Vec<&str> {
    match record {
        ComponentRecord::Creature(c) => vec![c.damage.as_str()],
        ComponentRecord::Item(i) => i.damage.as_deref().into_iter().collect(),
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn record(value: serde_json::Value) -> ComponentRecord {
        serde_json::from_value(value).unwrap()
    }

    #[test]
    fn valid_condition_and_dice_pass() {
        let r = record(json!({
            "component_type": "trigger", "id": "t", "on": "combat.hit",
            "when": "has_tag(self, 'undead') and entity.hp > 0", "do": []
        }));
        assert!(validate_semantics(&r).is_empty());

        let c = record(json!({
            "component_type": "creature", "id": "x", "name": "X", "hd": 1,
            "ac": 12, "attack_bonus": 1, "damage": "2d6+1"
        }));
        assert!(validate_semantics(&c).is_empty());
    }

    #[test]
    fn malformed_condition_is_a_syntax_error() {
        let r = record(json!({
            "component_type": "trigger", "id": "t", "on": "x",
            "when": "entity.hp >= >= 1", "do": []
        }));
        let diags = validate_semantics(&r);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].is_syntax());
    }

    #[test]
    fn malformed_dice_is_a_syntax_error() {
        let c = record(json!({
            "component_type": "creature", "id": "x", "name": "X", "hd": 1,
            "ac": 12, "attack_bonus": 1, "damage": "1dd6"
        }));
        let diags = validate_semantics(&c);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("invalid dice"));
    }
}
