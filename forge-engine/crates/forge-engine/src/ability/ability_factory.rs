//! AbilityFactory — factory for creating spell abilities from card scripts.
//!
//! Mirrors Java's `AbilityFactory.java`.
//! Parses ability strings (AB$, SP$, DB$, ST$ prefixed) and constructs
//! the corresponding `SpellAbility` with all sub-abilities resolved.

use std::collections::HashMap;

use crate::ability::api_type::ApiType;
use crate::cost::parse_cost;
use crate::game::GameState;
use crate::ids::{CardId, PlayerId};
use crate::parsing::{keys, Params};
use crate::spellability::target_restrictions::TargetRestrictions;
use crate::spellability::{SpellAbility, TargetChoices};

/// The record type prefix for an ability definition.
/// Mirrors Java's `AbilityFactory.AbilityRecordType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AbilityRecordType {
    /// AB$ — activated ability
    Ability,
    /// SP$ — spell ability
    Spell,
    /// ST$ — static ability
    StaticAbility,
    /// DB$ — sub-ability
    SubAbility,
}

impl AbilityRecordType {
    /// The script prefix for this record type.
    pub fn prefix(&self) -> &'static str {
        match self {
            AbilityRecordType::Ability => "AB",
            AbilityRecordType::Spell => "SP",
            AbilityRecordType::StaticAbility => "ST",
            AbilityRecordType::SubAbility => "DB",
        }
    }

    /// Determine the record type from a parsed parameter map.
    pub fn from_params(params: &Params) -> Option<AbilityRecordType> {
        if params.has("AB") {
            Some(AbilityRecordType::Ability)
        } else if params.has("SP") {
            Some(AbilityRecordType::Spell)
        } else if params.has("ST") {
            Some(AbilityRecordType::StaticAbility)
        } else if params.has("DB") {
            Some(AbilityRecordType::SubAbility)
        } else {
            None
        }
    }

    /// Get the API type string from parsed parameters for this record type.
    pub fn api_type_of<'a>(&self, params: &'a Params) -> Option<&'a str> {
        params.get(self.prefix())
    }
}

/// Keys used for additional sub-abilities in ability scripts.
/// Mirrors Java's `AbilityFactory.additionalAbilityKeys`.
pub const ADDITIONAL_ABILITY_KEYS: &[&str] = &[
    "WinSubAbility",
    "OtherwiseSubAbility",
    "BidSubAbility",
    "ChooseNumberSubAbility",
    "Lowest",
    "Highest",
    "NotLowest",
    "GuessCorrect",
    "GuessWrong",
    "MatchedAbility",
    "UnmatchedAbility",
    "HeadsSubAbility",
    "TailsSubAbility",
    "LoseSubAbility",
    "TrueSubAbility",
    "FalseSubAbility",
    "ChosenPile",
    "UnchosenPile",
    "RepeatSubAbility",
    "Execute",
    "FallbackAbility",
    "ChooseSubAbility",
    "CantChooseSubAbility",
    "RegenerationAbility",
    "ReturnAbility",
    "GiftAbility",
    "VoteSubAbility",
    "VoteTiedAbility",
];

/// Parse a pipe-delimited ability string into a key-value map.
/// Mirrors Java's `AbilityFactory.getMapParams()`.
pub fn get_map_params(ab_string: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for segment in ab_string.split('|') {
        let segment = segment.trim();
        if let Some(idx) = segment.find('$') {
            let key = segment[..idx].trim().to_string();
            let value = segment[idx + 1..].trim().to_string();
            map.insert(key, value);
        }
    }
    map
}

/// Build a SpellAbility chain from a card's ability text, walking SubAbility$
/// SVars to construct the linked list.
/// Mirrors Java's `AbilityFactory.getAbility()` + sub-ability chain construction.
pub fn build_spell_ability(
    game: &GameState,
    card_id: CardId,
    ability_text: &str,
    player: PlayerId,
) -> SpellAbility {
    let params = Params::from_raw(ability_text);
    let api = params
        .get(keys::SP)
        .or_else(|| params.get(keys::DB))
        .or_else(|| params.get(keys::AB))
        .and_then(|s| ApiType::smart_value_of(s));
    let target_restrictions = TargetRestrictions::new(&params);
    let cost = params.get(keys::COST).map(parse_cost);

    // Recursively build sub-ability chain from SVars
    let sub_ability = if let Some(sub_svar_name) = params.get(keys::SUB_ABILITY) {
        if let Some(sub_text) = game.card(card_id).svars.get(sub_svar_name).cloned() {
            Some(Box::new(build_spell_ability(
                game, card_id, &sub_text, player,
            )))
        } else {
            None
        }
    } else {
        None
    };

    SpellAbility {
        api,
        source: Some(card_id),
        activating_player: player,
        targeting_player: None,
        ability_text: ability_text.to_string(),
        params,
        target_restrictions,
        target_chosen: TargetChoices::default(),
        pay_costs: cost,
        sub_ability,
        is_spell: false,
        is_trigger: false,
        is_activated: false,
        trigger_source: None,
        trigger_index: None,
        alt_cost: None,
        kicked: false,
        buyback_paid: false,
        overloaded: false,
        is_copy: false,
        kick_count: 0,
        replicate_count: 0,
        optional_generic_cost_paid: false,
        trigger_remembered_amount: 0,
        x_mana_cost_paid: 0,
        discarded_cost_cards: Vec::new(),
        change_zone_table: None,
        damage_map: None,
        prevent_map: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_map_params() {
        let input = "AB$ DealDamage | Cost$ T | NumDmg$ 1";
        let map = get_map_params(input);
        assert_eq!(map.get("AB").unwrap(), "DealDamage");
        assert_eq!(map.get("Cost").unwrap(), "T");
        assert_eq!(map.get("NumDmg").unwrap(), "1");
    }

    #[test]
    fn test_record_type_from_params() {
        let params = Params::from_raw("DB$ Draw | NumCards$ 2");
        assert_eq!(
            AbilityRecordType::from_params(&params),
            Some(AbilityRecordType::SubAbility)
        );
    }
}
