//! Collect evidence as a cost. Mirrors Java's `CostCollectEvidence`.
//!
//! NOTE: Payability check is in `can_pay_inner()` in `mod.rs` (the central dispatcher).

// NOTE: pay_as_decided is handled by GameLoop::pay_collect_evidence_cost() in game_action.rs
// because it requires agent interaction for card selection and trigger firing (CollectEvidence).

pub const HASH_LKI: &str = "Collected";
pub const HASH_CARDS: &str = "CollectedCards";
