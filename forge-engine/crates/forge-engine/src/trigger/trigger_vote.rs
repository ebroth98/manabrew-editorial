use super::trigger::TriggerMode;
use crate::{
    event::RunParams,
    game::GameState,
    ids::{CardId, PlayerId},
    parsing::Params,
    spellability::SpellAbility,
};

pub fn perform_test(
    mode: &TriggerMode,
    _params: &RunParams,
    _game: &GameState,
    _host_card: CardId,
    _host_controller: PlayerId,
) -> bool {
    let TriggerMode::Vote = mode else {
        panic!("Expected Vote mode");
    };
    true
}

pub fn parse_mode(_params: &Params) -> TriggerMode {
    TriggerMode::Vote
}

pub fn set_triggering_objects(
    sa: &mut SpellAbility,
    params: &RunParams,
    _host_card: CardId,
    host_controller: PlayerId,
    game: &GameState,
) {
    let Some(all_votes) = params.all_votes.as_ref() else {
        return;
    };

    let mut same = Vec::new();
    let mut diff = Vec::new();
    for (_, voters) in all_votes {
        let host_voted_here = voters.contains(&host_controller);
        for &player in voters {
            if player == host_controller || game.opponent_of(host_controller) != player {
                continue;
            }
            let bucket = if host_voted_here {
                &mut same
            } else {
                &mut diff
            };
            if !bucket.contains(&player) {
                bucket.push(player);
            }
        }
    }

    if !same.is_empty() {
        let csv = same
            .iter()
            .map(|player_id| player_id.0.to_string())
            .collect::<Vec<_>>()
            .join(",");
        sa.add_triggering_object("OpponentVotedSame", &csv);
    }
    if !diff.is_empty() {
        let csv = diff
            .iter()
            .map(|player_id| player_id.0.to_string())
            .collect::<Vec<_>>()
            .join(",");
        sa.add_triggering_object("OpponentVotedDiff", &csv);
    }
}
