//! Action & skill IR records (R5-A) — the data layer over the R5-P resolution
//! primitives.
//!
//! An [`Action`] is a configured contest (or `apply`): an L1 archetype, or an L2
//! concrete action that sets `base: <archetype>` and overrides fields. A [`Skill`]
//! is an L3 overlay that *grants* actions and *modifies* them (modifier sources
//! targeting `action.<id>`). See `docs/design/2026-06-19-action-resolution-
//! primitives-design.md` §5 and the decomposition catalog.
//!
//! These reuse the runtime resolution config types (`Targeting`, `Activation`,
//! `RollMechanic`, `Roller`) so there is one definition of each shape.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ir::components::{Modifier, Prerequisite};
use crate::ir::effect::{Effect, Trigger};
use crate::resolution::contest::Roller;
use crate::resolution::economy::Activation;
use crate::resolution::mechanic::RollMechanic;
use crate::resolution::targeting::Targeting;

/// The resolution structure of an action. `reaction` is modelled as a trigger on
/// the reaction window, not an action kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ActionKind {
    Contest,
    Apply,
}

/// A difficulty reference: an inline value or a key into the difficulty table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum DifficultyRef {
    Value(i64),
    Key(String),
}

/// Where a contest's target number comes from. Externally tagged, matching the
/// authoring surface (`{ defense: ac }`, `{ difficulty: 8 }`, …).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TnSource {
    /// A fixed target number.
    Static(i64),
    /// Resolved from a difficulty value/key.
    Difficulty(DifficultyRef),
    /// The target's named defense value (e.g. `ac`).
    Defense(String),
    /// An opposed roll (a skill/expression the target rolls).
    Opposed(String),
    /// A save DC the defender rolls against.
    EffectDc(i64),
}

/// What the roll draws on. `mechanic` defaults to the world's mechanic; `skill`/
/// `attribute` may be null (e.g. a save the defender rolls).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RollSpec {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mechanic: Option<RollMechanic>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skill: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attribute: Option<String>,
}

/// A contest's resolution config.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ActionResolution {
    #[serde(default = "default_roller")]
    pub roller: Roller,
    pub tn_source: TnSource,
    pub roll_spec: RollSpec,
}

fn default_roller() -> Roller {
    Roller::Actor
}

/// An action prerequisite: a structured requirement or a raw condition string.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum ActionPrerequisite {
    Structured(Prerequisite),
    Condition(String),
}

/// One outcome branch: a `when` predicate over the result (`crit`/`success`/
/// `miss`/`fumble`, or a `degree`/`raw_margin` expression) and the effects to run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct OutcomeBranch {
    pub when: String,
    #[serde(default, rename = "do")]
    pub do_: Vec<Effect>,
}

/// An action record (L1 archetype or L2 concrete via `base`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Action {
    pub id: String,
    /// The archetype this action specializes (L2); `None` for an L1 archetype.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base: Option<String>,
    #[serde(default = "default_kind")]
    pub kind: ActionKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolution: Option<ActionResolution>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub targeting: Option<Targeting>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub activation: Option<Activation>,
    #[serde(default)]
    pub prerequisites: Vec<ActionPrerequisite>,
    /// Inline modifiers on the action itself (e.g. aimed shot +1, snap shot −2).
    #[serde(default)]
    pub modifiers: Vec<Modifier>,
    #[serde(default)]
    pub outcome: Vec<OutcomeBranch>,
    #[serde(default)]
    pub tags: Vec<String>,
}

fn default_kind() -> ActionKind {
    ActionKind::Contest
}

/// One action a skill governs: per-level modifiers, optionally gated by a tag.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct SkillGoverns {
    pub action: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requires_tag: Option<String>,
    #[serde(default)]
    pub per_level: Vec<Modifier>,
}

/// A skill record (L3 overlay): governs/modifies actions and grants actions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Skill {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub governs: Vec<SkillGoverns>,
    /// Actions/reactions this skill makes available.
    #[serde(default)]
    pub grants: Vec<String>,
    /// Special reactions/triggers the skill adds.
    #[serde(default)]
    pub triggers: Vec<Trigger>,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn l2_action_round_trips_with_base() {
        let value = json!({
            "id": "unarmed_strike",
            "base": "melee_attack",
            "resolution": {
                "tn_source": { "defense": "ac" },
                "roll_spec": { "mechanic": "xwn_d20_attack", "skill": "punch", "attribute": "STR" }
            },
            "targeting": { "shape": "single", "range_band": "melee" },
            "outcome": [
                { "when": "success", "do": [
                    { "kind": "emit_damage", "params": { "packet": { "amount": 2 }, "to": "target" } }
                ] }
            ],
            "tags": ["attack", "melee", "unarmed"]
        });
        let action: Action = serde_json::from_value(value).unwrap();
        assert_eq!(action.base.as_deref(), Some("melee_attack"));
        assert_eq!(action.kind, ActionKind::Contest); // default
        let res = action.resolution.unwrap();
        assert_eq!(res.roller, Roller::Actor); // default
        assert_eq!(res.tn_source, TnSource::Defense("ac".into()));
        assert_eq!(res.roll_spec.mechanic, Some(RollMechanic::XwnD20Attack));
        assert_eq!(action.outcome[0].when, "success");
    }

    #[test]
    fn tn_source_variants() {
        let d: TnSource = serde_json::from_value(json!({ "difficulty": 8 })).unwrap();
        assert_eq!(d, TnSource::Difficulty(DifficultyRef::Value(8)));
        let k: TnSource = serde_json::from_value(json!({ "difficulty": "hard" })).unwrap();
        assert_eq!(k, TnSource::Difficulty(DifficultyRef::Key("hard".into())));
        let o: TnSource = serde_json::from_value(json!({ "opposed": "skill.notice" })).unwrap();
        assert_eq!(o, TnSource::Opposed("skill.notice".into()));
        let e: TnSource = serde_json::from_value(json!({ "effect_dc": 13 })).unwrap();
        assert_eq!(e, TnSource::EffectDc(13));
    }

    #[test]
    fn prerequisite_accepts_struct_or_condition() {
        let value = json!([
            { "kind": "attribute_min", "arg": "DEX", "value": 13 },
            "has_trait(self, 'arcane_caster')"
        ]);
        let prereqs: Vec<ActionPrerequisite> = serde_json::from_value(value).unwrap();
        assert!(matches!(prereqs[0], ActionPrerequisite::Structured(_)));
        assert!(matches!(prereqs[1], ActionPrerequisite::Condition(_)));
    }

    #[test]
    fn skill_overlay_round_trips() {
        let value = json!({
            "id": "shoot",
            "name": "Shoot",
            "governs": [
                { "action": "ranged_attack",
                  "per_level": [ { "target": "action.ranged_attack.roll", "value": 1 } ] }
            ],
            "grants": ["aimed_shot", "snap_shot"],
            "tags": ["combat", "ranged"]
        });
        let skill: Skill = serde_json::from_value(value).unwrap();
        assert_eq!(skill.governs[0].action, "ranged_attack");
        assert_eq!(skill.governs[0].per_level[0].value, 1);
        assert_eq!(skill.grants, vec!["aimed_shot", "snap_shot"]);
    }
}
