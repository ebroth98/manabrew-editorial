use crate::card::valid_filter;
use crate::game::GameState;
use crate::ids::PlayerId;
use crate::parsing::keys;
use crate::spellability::SpellAbility;
use crate::staticability::StaticMode;

pub fn cant_discard(
    game: &GameState,
    player: PlayerId,
    cause: Option<&SpellAbility>,
    is_effect: bool,
) -> bool {
    for card in game
        .cards
        .iter()
        .filter(|c| c.zone.is_static_ability_source())
    {
        for st_ab in card
            .static_abilities
            .iter()
            .filter(|sa| sa.mode == StaticMode::CantDiscard && sa.zones_check(card.zone))
        {
            if apply_cant_discard_ability(st_ab, player, card.controller, cause, is_effect) {
                return true;
            }
        }
    }
    false
}

pub fn apply_cant_discard_ability(
    st_ab: &crate::staticability::StaticAbility,
    player: PlayerId,
    source_controller: PlayerId,
    cause: Option<&SpellAbility>,
    is_effect: bool,
) -> bool {
    if !valid_filter::matches_valid_player_opt(
        st_ab.params.get(keys::VALID_PLAYER),
        player,
        source_controller,
    ) {
        return false;
    }
    if let Some(for_cost) = st_ab.params.get(keys::FOR_COST) {
        // ForCost=True means it applies to costs, not effects
        // Java: "True".equalsIgnoreCase(ForCost) == effect → return false
        if for_cost.eq_ignore_ascii_case("True") == is_effect {
            return false;
        }
    }
    if !super::static_ability_cant_sacrifice::matches_valid_cause(
        st_ab.params.get(keys::VALID_CAUSE),
        cause,
    ) {
        return false;
    }
    true
}
