//! Add mana to pool as a cost. Mirrors Java's `CostAddMana`.
//!
//! NOTE: Payability check is in `can_pay_inner()` in `mod.rs` (the central dispatcher).

use forge_foundation::mana::ManaAtom;

use crate::ids::{CardId, PlayerId};
use crate::mana::{Mana, ManaPool};

/// Pay by adding mana to the player's pool.
/// Mirrors Java's `CostAddMana.payAsDecided()`.
pub fn pay_as_decided(
    pool: &mut ManaPool,
    source: CardId,
    _player: PlayerId,
    amount: i32,
    mana_type: &str,
) -> bool {
    let atom = match mana_type.to_uppercase().as_str() {
        "W" | "WHITE" => ManaAtom::WHITE,
        "U" | "BLUE" => ManaAtom::BLUE,
        "B" | "BLACK" => ManaAtom::BLACK,
        "R" | "RED" => ManaAtom::RED,
        "G" | "GREEN" => ManaAtom::GREEN,
        "C" | "COLORLESS" => ManaAtom::COLORLESS,
        _ => ManaAtom::COLORLESS,
    };
    for _ in 0..amount {
        let mut m = Mana::simple(atom);
        m.source_card = Some(source);
        pool.add_mana(m);
    }
    true
}
