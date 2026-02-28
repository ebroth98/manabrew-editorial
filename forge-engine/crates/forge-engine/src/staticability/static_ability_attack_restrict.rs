use forge_foundation::ZoneType;

use crate::card::CardInstance;
use crate::ids::PlayerId;
use crate::staticability::StaticMode;

pub fn global_attack_restrict(cards: &[CardInstance]) -> Option<i32> {
    let mut max: Option<i32> = None;
    for source in cards.iter().filter(|c| c.zone == ZoneType::Battlefield) {
        for st_ab in source
            .static_abilities
            .iter()
            .filter(|sa| sa.mode == StaticMode::AttackRestrict)
        {
            if st_ab.params.contains_key("ValidDefender") {
                continue;
            }
            let m = st_ab
                .params
                .get("MaxAttackers")
                .map(|s| eval_amount(source, s))
                .unwrap_or(1);
            max = Some(max.map_or(m, |x| x.min(m)));
        }
    }
    max
}

pub fn attack_restrict_num_for_defender(cards: &[CardInstance], defender: PlayerId) -> Option<i32> {
    let mut max: Option<i32> = None;
    for source in cards.iter().filter(|c| c.zone == ZoneType::Battlefield) {
        for st_ab in source
            .static_abilities
            .iter()
            .filter(|sa| sa.mode == StaticMode::AttackRestrict)
        {
            let Some(valid_defender) = st_ab.params.get("ValidDefender").map(String::as_str) else {
                continue;
            };
            if !matches_valid_defender(valid_defender, defender, source.controller) {
                continue;
            }
            let m = st_ab
                .params
                .get("MaxAttackers")
                .map(|s| eval_amount(source, s))
                .unwrap_or(1);
            max = Some(max.map_or(m, |x| x.min(m)));
        }
    }
    max
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

fn matches_valid_defender(valid: &str, defender: PlayerId, source_controller: PlayerId) -> bool {
    if valid.eq_ignore_ascii_case("Player") {
        return true;
    }
    if valid.eq_ignore_ascii_case("You") || valid.eq_ignore_ascii_case("YouCtrl") {
        return defender == source_controller;
    }
    if valid.eq_ignore_ascii_case("Opponent") || valid.eq_ignore_ascii_case("OppCtrl") {
        return defender != source_controller;
    }
    true
}
