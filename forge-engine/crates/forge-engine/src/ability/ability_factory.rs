//! AbilityFactory — factory for creating spell abilities from card scripts.
//!
//! Mirrors Java's `AbilityFactory.java`.
//! Parses ability strings (AB$, SP$, DB$, ST$ prefixed) and constructs
//! the corresponding `SpellAbility` with all sub-abilities resolved.

use std::collections::HashMap;

use crate::ability::api_type::ApiType;
use crate::card::Card;
use crate::cost::parse_cost;
use crate::cost::{Cost, CostPart};
use crate::game::GameState;
use crate::ids::{CardId, PlayerId};
use crate::parsing::keys::ST;
use crate::parsing::{keys, Params};
use crate::spellability::target_restrictions::TargetRestrictions;
use crate::spellability::{SpellAbility, TargetChoices};
use forge_foundation::ZoneType;

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
        if params.has(keys::AB) {
            Some(AbilityRecordType::Ability)
        } else if params.has(keys::SP) {
            Some(AbilityRecordType::Spell)
        } else if params.has(ST) {
            Some(AbilityRecordType::StaticAbility)
        } else if params.has(keys::DB) {
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
    let host = game.card(card_id);
    build_spell_ability_from_host_card(host, ability_text, player)
}

/// Build a SpellAbility chain from script text using a concrete host card.
///
/// This is the closest Rust equivalent of Java `AbilityFactory.getAbility(...)`
/// for contexts that have a `Card` object but not full `GameState`.
pub fn build_spell_ability_from_host_card(
    host: &Card,
    ability_text: &str,
    player: PlayerId,
) -> SpellAbility {
    let params = Params::from_raw(ability_text);
    let record_type = AbilityRecordType::from_params(&params).unwrap_or_else(|| {
        panic!(
            "AbilityFactory::build_spell_ability requires AB$/SP$/ST$/DB$ ability text; got: {:?}",
            ability_text
        )
    });
    build_spell_ability_of_type(host, ability_text, player, record_type)
}

/// Build a spell ability for card-casting contexts.
///
/// This mirrors Java's Spell object construction for vanilla cards:
/// if a card has no explicit SP$ line, create a spell-shaped ability probe
/// from the card's intrinsic mana cost and default hand-zone restriction.
pub fn build_spell_ability_for_card_cast(
    game: &GameState,
    card_id: CardId,
    player: PlayerId,
) -> SpellAbility {
    if let Some(spell_ability_text) = game
        .card(card_id)
        .abilities
        .iter()
        .find(|a| Params::from_raw(a).has(keys::SP))
        .cloned()
    {
        let host = game.card(card_id);
        let mut sa = build_spell_ability_of_type(
            host,
            &spell_ability_text,
            player,
            AbilityRecordType::Spell,
        );
        // Card-cast context: if SP$ omitted Cost$, default to card mana cost.
        if sa.pay_costs.is_none() {
            sa.pay_costs = Some(Cost {
                parts: vec![CostPart::Mana {
                    cost: host.mana_cost.clone(),
                    x_min: 0,
                    is_exiled_creature_cost: false,
                    is_enchanted_creature_cost: false,
                    is_cost_pay_any_number_of_times: false,
                    max_waterbend: None,
                }],
                has_tap: false,
                mandatory: false,
            });
        }
        // Aura enchantments with SP$ but no ValidTgts$: inject Enchant-derived targeting.
        // Some aura cards have SP$ lines for ETB effects but rely on the Enchant keyword
        // for targeting. Without this, the aura can target anything.
        if sa.target_restrictions.is_none() && host.type_line.has_subtype("Aura") {
            let enchant_type = host.get_keyword_cost("Enchant").unwrap_or_default();
            let valid_tgts = crate::parsing::enchant_type_to_valid_tgts(&enchant_type);
            let params_str = format!("ValidTgts$ {}", valid_tgts);
            sa.target_restrictions = TargetRestrictions::new(&Params::from_raw(&params_str));
        }
        return sa;
    }

    // Vanilla fallback: no SP$ ability text. Build a castable spell probe
    // with Java-like Spell defaults (hand zone + card mana cost).
    let mut restriction = crate::spellability::SpellAbilityRestriction::default();
    restriction.variables.set_zone(ZoneType::Hand);
    let condition = crate::spellability::SpellAbilityCondition::default();
    let card = game.card(card_id);

    // Aura enchantments: derive targeting from "Enchant <type>" keyword.
    // Mirrors Java's Spell constructor which reads the Enchant keyword to
    // set up ValidTgts$ automatically for aura spells.
    let target_restrictions = if card.type_line.has_subtype("Aura") {
        let enchant_type = card.get_keyword_cost("Enchant").unwrap_or_default();
        let valid_tgts = crate::parsing::enchant_type_to_valid_tgts(&enchant_type);
        let params_str = format!("ValidTgts$ {}", valid_tgts);
        TargetRestrictions::new(&Params::from_raw(&params_str))
    } else {
        None
    };

    SpellAbility {
        api: None,
        source: Some(card_id),
        original_host: card.effect_source,
        activating_player: player,
        targeting_player: None,
        ability_text: String::new(),
        params: Params::from_raw(""),
        target_restrictions,
        target_chosen: TargetChoices::default(),
        pay_costs: Some(Cost {
            parts: vec![CostPart::Mana {
                cost: card.mana_cost.clone(),
                x_min: 0,
                is_exiled_creature_cost: false,
                is_enchanted_creature_cost: false,
                is_cost_pay_any_number_of_times: false,
                max_waterbend: None,
            }],
            has_tap: false,
            mandatory: false,
        }),
        sub_ability: None,
        is_spell: true,
        is_trigger: false,
        is_activated: false,
        trigger_source: None,
        source_trigger_id: None,
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
        optional_costs: Vec::new(),
        paid_hash: std::collections::HashMap::new(),
        mana_part: None,
        convoke_tapped: Vec::new(),
        spliced_cards: Vec::new(),
        announce_vars: std::collections::HashMap::new(),
        sacrificed_as_emerge: None,
        sacrificed_as_offering: None,
        description: String::new(),
        stack_description: String::new(),
        is_mana_ability: false,
        is_land_ability: false,
        trigger_objects: std::collections::HashMap::new(),
        trigger_spell_abilities: std::collections::HashMap::new(),
        restriction,
        condition,
        rollback_effects: Vec::new(),
        optional_keyword_amounts: std::collections::HashMap::new(),
        pips_to_reduce: Vec::new(),
        last_state: std::collections::HashMap::new(),
        change_zone_table: None,
        damage_map: None,
        prevent_map: None,
    }
}

fn build_spell_ability_of_type(
    host: &Card,
    ability_text: &str,
    player: PlayerId,
    record_type: AbilityRecordType,
) -> SpellAbility {
    let params = Params::from_raw(ability_text);
    let api = record_type
        .api_type_of(&params)
        .and_then(ApiType::smart_value_of);
    let target_restrictions = TargetRestrictions::new(&params);
    let cost = parse_ability_cost(host, &params, record_type);
    let mut restriction = crate::spellability::SpellAbilityRestriction::default();
    let mut condition = crate::spellability::SpellAbilityCondition::default();

    match record_type {
        AbilityRecordType::Spell => restriction.variables.set_zone(ZoneType::Hand),
        AbilityRecordType::Ability
        | AbilityRecordType::StaticAbility
        | AbilityRecordType::SubAbility => restriction.variables.set_zone(ZoneType::Battlefield),
    }
    restriction.set_restrictions(&params);
    condition.set_conditions(&params);

    // Recursively build sub-ability chain from SVars
    let sub_ability = if let Some(sub_svar_name) = params.get(keys::SUB_ABILITY) {
        if let Some(sub_text) = host.svars.get(sub_svar_name).cloned() {
            Some(Box::new(build_spell_ability_from_host_card(
                host, &sub_text, player,
            )))
        } else {
            None
        }
    } else {
        None
    };

    SpellAbility {
        api,
        source: Some(host.id),
        original_host: host.effect_source,
        activating_player: player,
        targeting_player: None,
        ability_text: ability_text.to_string(),
        params,
        target_restrictions,
        target_chosen: TargetChoices::default(),
        pay_costs: cost,
        sub_ability,
        is_spell: record_type == AbilityRecordType::Spell,
        is_trigger: false,
        is_activated: record_type == AbilityRecordType::Ability,
        trigger_source: None,
        source_trigger_id: None,
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
        optional_costs: Vec::new(),
        paid_hash: std::collections::HashMap::new(),
        mana_part: None,
        convoke_tapped: Vec::new(),
        spliced_cards: Vec::new(),
        announce_vars: std::collections::HashMap::new(),
        sacrificed_as_emerge: None,
        sacrificed_as_offering: None,
        description: String::new(),
        stack_description: String::new(),
        is_mana_ability: false,
        is_land_ability: false,
        trigger_objects: std::collections::HashMap::new(),
        trigger_spell_abilities: std::collections::HashMap::new(),
        restriction,
        condition,
        rollback_effects: Vec::new(),
        optional_keyword_amounts: std::collections::HashMap::new(),
        pips_to_reduce: Vec::new(),
        last_state: std::collections::HashMap::new(),
        change_zone_table: None,
        damage_map: None,
        prevent_map: None,
    }
}

fn parse_ability_cost(
    _host: &Card,
    params: &Params,
    record_type: AbilityRecordType,
) -> Option<Cost> {
    if record_type == AbilityRecordType::SubAbility {
        return None;
    }
    params.get(keys::COST).map(parse_cost)
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

    #[test]
    fn test_record_type_from_params_static() {
        let params = Params::from_raw("ST$ Continuous");
        assert_eq!(
            AbilityRecordType::from_params(&params),
            Some(AbilityRecordType::StaticAbility)
        );
    }
}
