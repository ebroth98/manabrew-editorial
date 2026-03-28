use super::{parse_param, resolve_defined_player, EffectContext};
use crate::spellability::SpellAbility;

/// Resolve `DB$ Poison` / `SP$ Poison` — add poison counters to players.
///
/// Mirrors Java `PoisonEffect.java` (~45 lines).
///
/// Real card patterns:
/// - `DB$ Poison | Defined$ Player | Num$ 1`    (Ichor Rats — all players)
/// - `DB$ Poison | Defined$ Opponent | Num$ 1`  (Prologue to Phyresis)
/// - `DB$ Poison | Defined$ You | Num$ 1`       (Phyrexian Vatmother)
/// - `DB$ Poison | ValidTgts$ Player | Num$ 1`  (Hand of the Praetors — targeted)
/// - `DB$ Poison | Defined$ TriggeredTarget | Num$ 1` (trigger context)
pub fn resolve(ctx: &mut EffectContext, sa: &SpellAbility) {
    let amount = parse_param(&sa.ability_text, "Num$ ").unwrap_or(1);
    if amount == 0 {
        return;
    }

    let controller = sa.activating_player;

    // If targeting was used (ValidTgts$ Player), use the chosen target.
    if let Some(target_player) = sa.target_chosen.target_player {
        if ctx.game.player(target_player).is_alive() {
            if !crate::staticability::static_ability_cant_put_counter::any_cant_put_counter_on_player(
                &ctx.game.cards,
                target_player,
                &crate::card::CounterType::Poison,
            ) {
                if amount > 0 {
                    ctx.game.player_add_poison(target_player, amount);
                } else {
                    ctx.game.player_remove_poison(target_player, -amount);
                }
            }
        }
        return;
    }

    // Resolve Defined$ parameter.
    let defined = sa.params.get("Defined").unwrap_or("Opponent");

    // Special case: "Player" means ALL alive players (distinct from
    // resolve_defined_player which returns a single player).
    if defined == "Player" {
        let alive: Vec<_> = ctx.game.alive_players().into_iter().collect();
        for pid in alive {
            if !crate::staticability::static_ability_cant_put_counter::any_cant_put_counter_on_player(
                &ctx.game.cards,
                pid,
                &crate::card::CounterType::Poison,
            ) {
                if amount > 0 {
                    ctx.game.player_add_poison(pid, amount);
                } else {
                    ctx.game.player_remove_poison(pid, -amount);
                }
            }
        }
        return;
    }

    // Single player: You, Opponent, TriggeredTarget, etc.
    if let Some(pid) = resolve_defined_player(defined, controller, ctx.game) {
        if ctx.game.player(pid).is_alive() {
            if !crate::staticability::static_ability_cant_put_counter::any_cant_put_counter_on_player(
                &ctx.game.cards,
                pid,
                &crate::card::CounterType::Poison,
            ) {
                if amount > 0 {
                    ctx.game.player_add_poison(pid, amount);
                } else {
                    ctx.game.player_remove_poison(pid, -amount);
                }
            }
        }
    }
}
