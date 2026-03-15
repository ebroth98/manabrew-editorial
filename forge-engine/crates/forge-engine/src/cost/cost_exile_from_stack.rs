//! Exile spells from the stack as a cost. Mirrors Java's `CostExileFromStack`.
//!
//! NOTE: Payability check is in `can_pay_inner()` in `mod.rs` (the central dispatcher).

// NOTE: pay_as_decided is handled by GameLoop::pay_exile_from_stack_cost() in game_action.rs
// because it requires stack manipulation and agent interaction for target selection.
