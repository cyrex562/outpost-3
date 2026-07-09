//! The fixed resolution stage sequence — the contract the host's event loop
//! walks (emitting each as a `*.…` event so modifiers and reactions can attach).
//!
//! The *sequence* is fixed (a brand-new stage is engine work); packs extend
//! behaviour only by attaching modifiers / reactions / effects at existing
//! stages. The damage stages run only when the outcome produced a damage packet
//! (a miss skips them straight to `Completed`).

use serde::{Deserialize, Serialize};

/// One stage in a resolution. Ordered; see [`Stage::SEQUENCE`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Stage {
    /// Resolution begins; costs are paid here.
    Started,
    /// Environment/stance/buff modifiers (condition-gated sources) land here.
    ModifiersAssembled,
    /// The `ResolutionResult` is produced.
    Rolled,
    /// Reactions fire: alter the TN, run a nested contest, or cancel the result.
    ReactionWindow,
    /// success/margin/degree selects the outcome branch.
    Outcome,
    /// Damage amount + wounding (only when the outcome emits damage).
    DamagePacketCreated,
    /// DR / armor / absorbing-pool mitigation.
    DamageMitigated,
    /// Pool routing + application (the pre-write `_requested` window).
    DamageApplied,
    /// Resolution finished.
    Completed,
}

impl Stage {
    /// The canonical order. The host emits the event for each in turn (skipping
    /// the three damage stages when no packet was produced).
    pub const SEQUENCE: [Stage; 9] = [
        Stage::Started,
        Stage::ModifiersAssembled,
        Stage::Rolled,
        Stage::ReactionWindow,
        Stage::Outcome,
        Stage::DamagePacketCreated,
        Stage::DamageMitigated,
        Stage::DamageApplied,
        Stage::Completed,
    ];

    /// The dotted event type the host emits at this stage.
    pub fn event_type(self) -> &'static str {
        match self {
            Stage::Started => "resolution.started",
            Stage::ModifiersAssembled => "resolution.modifiers_assembled",
            Stage::Rolled => "resolution.rolled",
            Stage::ReactionWindow => "resolution.reaction_window",
            Stage::Outcome => "resolution.outcome",
            Stage::DamagePacketCreated => "damage.packet_created",
            Stage::DamageMitigated => "damage.mitigated",
            Stage::DamageApplied => "damage.applied",
            Stage::Completed => "resolution.completed",
        }
    }

    /// Whether this stage is part of the damage sub-sequence (skipped on a miss).
    pub fn is_damage_stage(self) -> bool {
        matches!(
            self,
            Stage::DamagePacketCreated | Stage::DamageMitigated | Stage::DamageApplied
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sequence_is_ordered_and_complete() {
        // The reaction window comes after the roll and before the outcome.
        let pos = |s: Stage| Stage::SEQUENCE.iter().position(|&x| x == s).unwrap();
        assert!(pos(Stage::Rolled) < pos(Stage::ReactionWindow));
        assert!(pos(Stage::ReactionWindow) < pos(Stage::Outcome));
        assert!(pos(Stage::Outcome) < pos(Stage::DamagePacketCreated));
        assert_eq!(Stage::SEQUENCE.first(), Some(&Stage::Started));
        assert_eq!(Stage::SEQUENCE.last(), Some(&Stage::Completed));
    }

    #[test]
    fn event_types_are_namespaced() {
        assert_eq!(
            Stage::ReactionWindow.event_type(),
            "resolution.reaction_window"
        );
        assert_eq!(Stage::DamageApplied.event_type(), "damage.applied");
    }

    #[test]
    fn only_damage_stages_flagged() {
        let damage: Vec<_> = Stage::SEQUENCE
            .iter()
            .filter(|s| s.is_damage_stage())
            .collect();
        assert_eq!(damage.len(), 3);
        assert!(!Stage::Rolled.is_damage_stage());
    }

    #[test]
    fn stage_serializes_snake_case() {
        let json = serde_json::to_string(&Stage::ReactionWindow).unwrap();
        assert_eq!(json, "\"reaction_window\"");
    }
}
