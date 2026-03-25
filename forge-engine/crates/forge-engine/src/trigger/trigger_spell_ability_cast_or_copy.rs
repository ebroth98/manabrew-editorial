use crate::{
    event::RunParams,
    game::GameState,
    ids::{CardId, PlayerId},
    parsing::{keys, Params},
};

use super::trigger::{check_card_filter, check_player_filter, TriggerMode};

pub fn perform_test(
    mode: &TriggerMode,
    params: &RunParams,
    game: &GameState,
    host_card: CardId,
    host_controller: PlayerId,
) -> bool {
    match mode {
        TriggerMode::SpellCast {
            valid_card,
            valid_activating_player,
        }
        | TriggerMode::AbilityCast {
            valid_card,
            valid_activating_player,
        }
        | TriggerMode::SpellAbilityCast {
            valid_card,
            valid_activating_player,
        }
        | TriggerMode::SpellCastAll {
            valid_card,
            valid_activating_player,
        }
        | TriggerMode::SpellCastOnce {
            valid_card,
            valid_activating_player,
        }
        | TriggerMode::SpellCastOfType {
            valid_card,
            valid_activating_player,
        }
        | TriggerMode::SpellCopied {
            valid_card,
            valid_activating_player,
        }
        | TriggerMode::SpellCopy {
            valid_card,
            valid_activating_player,
        }
        | TriggerMode::SpellAbilityCopy {
            valid_card,
            valid_activating_player,
        }
        | TriggerMode::SpellCastOrCopy {
            valid_card,
            valid_activating_player,
        } => {
            check_card_filter(
                valid_card,
                params.spell_card,
                host_card,
                host_controller,
                game,
            ) && check_player_filter(
                valid_activating_player,
                params.spell_controller,
                host_controller,
            )
        }
        _ => panic!("Expected spell/ability cast-or-copy mode"),
    }
}

pub fn parse_mode(mode_name: &str, params: &Params) -> TriggerMode {
    let valid_card = params.get_cloned(keys::VALID_CARD);
    let valid_activating_player = params.get_cloned(keys::VALID_ACTIVATING_PLAYER);
    match mode_name {
        "SpellCast" => TriggerMode::SpellCast {
            valid_card,
            valid_activating_player,
        },
        "AbilityCast" => TriggerMode::AbilityCast {
            valid_card,
            valid_activating_player,
        },
        "SpellAbilityCast" => TriggerMode::SpellAbilityCast {
            valid_card,
            valid_activating_player,
        },
        "SpellCastOrCopy" => TriggerMode::SpellCastOrCopy {
            valid_card,
            valid_activating_player,
        },
        "SpellCopied" => TriggerMode::SpellCopied {
            valid_card,
            valid_activating_player,
        },
        "SpellAbilityCopy" => TriggerMode::SpellAbilityCopy {
            valid_card,
            valid_activating_player,
        },
        "SpellCopy" => TriggerMode::SpellCopy {
            valid_card,
            valid_activating_player,
        },
        "SpellCastAll" => TriggerMode::SpellCastAll {
            valid_card,
            valid_activating_player,
        },
        "SpellCastOnce" => TriggerMode::SpellCastOnce {
            valid_card,
            valid_activating_player,
        },
        "SpellCastOfType" => TriggerMode::SpellCastOfType {
            valid_card,
            valid_activating_player,
        },
        _ => panic!("Unsupported spell/ability cast-or-copy mode: {mode_name}"),
    }
}
