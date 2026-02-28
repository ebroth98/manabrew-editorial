use forge_foundation::ZoneType;

use crate::card::CardInstance;
use crate::ids::PlayerId;
use crate::staticability::StaticMode;

pub fn block_restrict_num(cards: &[CardInstance], defender: PlayerId) -> i32 {
    let mut num = i32::MAX;
    for source in cards.iter().filter(|c| c.zone == ZoneType::Battlefield) {
        for st_ab in source
            .static_abilities
            .iter()
            .filter(|sa| sa.mode == StaticMode::BlockRestrict)
        {
            let valid = st_ab.params.get("ValidDefender").map(String::as_str);
            if !matches_valid_player(valid, defender, source.controller) {
                continue;
            }
            let n = st_ab
                .params
                .get("MaxBlockers")
                .map(|s| eval_amount(source, s))
                .unwrap_or(1);
            if n < num {
                num = n;
            }
        }
    }
    num
}

fn matches_valid_player(valid: Option<&str>, player: PlayerId, source_controller: PlayerId) -> bool {
    match valid {
        None => true,
        Some(v) if v.eq_ignore_ascii_case("Player") => true,
        Some(v) if v.eq_ignore_ascii_case("You") || v.eq_ignore_ascii_case("YouCtrl") => {
            player == source_controller
        }
        Some(v) if v.eq_ignore_ascii_case("Opponent") || v.eq_ignore_ascii_case("OppCtrl") => {
            player != source_controller
        }
        _ => true,
    }
}

fn eval_amount(source: &CardInstance, expr: &str) -> i32 {
    if let Ok(n) = expr.parse::<i32>() {
        return n;
    }
    if let Some(svar) = source.svars.get(expr) {
        if let Ok(n) = svar.parse::<i32>() {
            return n;
        }
        let mut sa = crate::spellability::SpellAbility::new_simple(
            Some(source.id),
            source.controller,
            "AB$ Internal",
        );
        sa.kicked = false;
        return crate::ability::effects::evaluate_svar(svar, &sa);
    }
    1
}
