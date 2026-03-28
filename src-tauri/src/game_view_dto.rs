use std::collections::HashMap;

use forge_engine_core::game::GameState;
use forge_engine_core::ids::{CardId, PlayerId};
use forge_engine_core::mana::ManaPool;
use forge_engine_core::spellability::SpellAbility;
use forge_foundation::ZoneType;
use serde::{Deserialize, Serialize};

use crate::ids_codec::{card_id_str, player_id_str, stack_id_str};

/// Frontend-compatible game state snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameViewDto {
    pub game_id: String,
    pub turn: u32,
    pub step: String,
    /// Declared blockers for the current combat: blocker -> attacker.
    pub combat_assignments: Vec<CombatAssignmentDto>,
    pub active_player_id: String,
    pub priority_player_id: String,
    pub players: Vec<PlayerDto>,
    pub my_hand: Vec<CardDto>,
    pub battlefield: Vec<CardDto>,
    pub stack: Vec<StackObjectDto>,
    pub exile: Vec<CardDto>,
    pub graveyard: Vec<CardDto>,
    pub opponent_graveyard: Vec<CardDto>,
    pub opponent_exile: Vec<CardDto>,
    /// Cards in my command zone (typically just the commander).
    pub my_command_zone: Vec<CardDto>,
    /// Cards in the opponent's command zone.
    pub opponent_command_zone: Vec<CardDto>,
    pub game_over: bool,
    pub winner_id: Option<String>,
    /// The player who is the current monarch (issue #22).
    pub monarch_id: Option<String>,
    /// The player who holds the initiative (issue #22).
    pub initiative_holder_id: Option<String>,
}

impl GameViewDto {
    pub fn empty(game_id: String) -> Self {
        Self {
            game_id,
            turn: 0,
            step: "main1".into(),
            combat_assignments: vec![],
            active_player_id: String::new(),
            priority_player_id: String::new(),
            players: vec![],
            my_hand: vec![],
            battlefield: vec![],
            stack: vec![],
            exile: vec![],
            graveyard: vec![],
            opponent_graveyard: vec![],
            opponent_exile: vec![],
            my_command_zone: vec![],
            opponent_command_zone: vec![],
            game_over: false,
            winner_id: None,
            monarch_id: None,
            initiative_holder_id: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CombatAssignmentDto {
    pub blocker_id: String,
    pub attacker_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerDto {
    pub id: String,
    pub name: String,
    pub is_human: bool,
    pub life: i32,
    pub poison: i32,
    pub hand_count: usize,
    pub library_count: usize,
    pub graveyard_count: usize,
    pub exile_count: usize,
    pub mana_pool: HashMap<String, i32>,
    /// Commander damage received: source card id string → total damage.
    pub commander_damage: HashMap<String, i32>,
    /// Energy counters (Kaladesh block).
    pub energy_counters: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CardDto {
    pub id: String,
    pub name: String,
    pub set_code: String,
    pub card_number: String,
    pub color: String,
    pub mana_cost: String,
    pub cmc: i32,
    pub types: Vec<String>,
    pub subtypes: Vec<String>,
    pub supertypes: Vec<String>,
    pub power: Option<String>,
    pub toughness: Option<String>,
    /// Base power before any modifiers (for buff/debuff color-coding).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_power: Option<i32>,
    /// Base toughness before any modifiers (for buff/debuff color-coding).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_toughness: Option<i32>,
    pub text: String,
    pub is_playable: bool,
    pub is_selected: bool,
    pub is_choosable: bool,
    pub controller_id: String,
    pub owner_id: String,
    pub zone_id: String,
    pub tapped: bool,
    pub keywords: Vec<String>,
    /// Active counters: counter type name → count. Only non-zero entries included.
    pub counters: HashMap<String, i32>,
    pub damage: i32,
    pub summoning_sick: bool,
    pub is_token: bool,
    /// True if this card has an alternate face (DFC: Transform, Modal DFC).
    pub is_double_faced: bool,
    /// True if this card is currently showing its back face.
    pub is_transformed: bool,
    /// True if this card is face-down (Morph, Manifest).
    pub is_face_down: bool,
    /// True if this card is currently bestowed (attached as an Aura).
    pub is_bestowed: bool,
    /// True if this card is phased out (issue #22).
    pub phased_out: bool,
    /// True if this creature has been exerted (won't untap next untap step).
    pub exerted: bool,
    /// ID of the card this permanent is attached to (equipment host, enchanted creature).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attached_to: Option<String>,
    /// IDs of cards attached to this permanent (equipment, auras).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub attachment_ids: Vec<String>,
    /// Flashback cost string, if the card has flashback (e.g. "1 R").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flashback_cost: Option<String>,
    /// Kicker cost string, if the card has kicker (e.g. "W").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kicker_cost: Option<String>,
    /// Effective mana cost after static ability reductions/increases.
    /// Only set when different from `mana_cost` and the card is playable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effective_mana_cost: Option<String>,
    /// Madness cost string, if the card has madness (e.g. "R").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub madness_cost: Option<String>,
    /// True if this card is currently exiled via Madness replacement.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub is_madness_exiled: bool,
    /// True if this card has been plotted (exiled face-up, castable for free later).
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub is_plotted: bool,
    /// True if this card was exiled via Warp (castable from exile for normal cost).
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub is_warp_exiled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StackObjectDto {
    pub id: String,
    pub source_id: String,
    /// The player who cast/activated this spell or ability.
    pub controller_id: String,
    pub name: String,
    pub text: String,
    /// True when this stack entry is a permanent spell (creature, artifact,
    /// enchantment, planeswalker) that will resolve onto the battlefield.
    /// False for instants, sorceries, and activated/triggered abilities.
    pub is_permanent_spell: bool,
    /// Normalized chosen targets for this stack object, flattened across the
    /// root ability and sub-ability chain in evaluation order.
    pub targets: Vec<StackTargetDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StackTargetDto {
    pub kind: StackTargetKindDto,
    /// Engine identifier encoded for frontend lookups:
    /// - Card => "card-<id>"
    /// - Player => "player-<index>"
    /// - Stack => "stack-<id>"
    pub id: String,
    /// Zero-based index in the spell-ability chain (root ability = 0).
    pub node_index: u32,
    /// Zero-based target slot index inside this node.
    pub target_index: u32,
    /// Whether this target is hostile (damage, destroy, counter) vs friendly (buff, heal).
    pub hostile: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StackTargetKindDto {
    Card,
    Player,
    Stack,
}

/// Determine if a spell ability's effect is hostile based on its API type.
pub fn is_hostile_api(sa: &SpellAbility) -> bool {
    use forge_engine_core::ability::api_type::ApiType;
    match sa.api {
        Some(ApiType::DealDamage)
        | Some(ApiType::Destroy)
        | Some(ApiType::DestroyAll)
        | Some(ApiType::Counter)
        | Some(ApiType::Sacrifice)
        | Some(ApiType::SacrificeAll)
        | Some(ApiType::ChangeZone)
        | Some(ApiType::ChangeZoneAll)
        | Some(ApiType::LoseLife)
        | Some(ApiType::Debuff) => true,
        _ => false,
    }
}

fn collect_stack_targets(root: &SpellAbility) -> Vec<StackTargetDto> {
    let mut out = Vec::new();
    let mut node_index = 0u32;
    let mut current = Some(root);

    while let Some(sa) = current {
        let mut target_index = 0u32;
        let hostile = is_hostile_api(sa);

        if let Some(cid) = sa.target_chosen.target_card {
            out.push(StackTargetDto {
                kind: StackTargetKindDto::Card,
                id: card_id_str(cid),
                node_index,
                target_index,
                hostile,
            });
            target_index += 1;
        }
        if let Some(pid) = sa.target_chosen.target_player {
            out.push(StackTargetDto {
                kind: StackTargetKindDto::Player,
                id: player_id_str(pid),
                node_index,
                target_index,
                hostile,
            });
            target_index += 1;
        }
        if let Some(stack_id) = sa.target_chosen.target_stack_entry {
            out.push(StackTargetDto {
                kind: StackTargetKindDto::Stack,
                id: stack_id_str(stack_id),
                node_index,
                target_index,
                hostile,
            });
        }

        node_index += 1;
        current = sa.sub_ability.as_deref();
    }

    out
}

fn mana_pool_to_map(pool: &ManaPool) -> HashMap<String, i32> {
    let mut m = HashMap::new();
    m.insert("W".into(), pool.white());
    m.insert("U".into(), pool.blue());
    m.insert("B".into(), pool.black());
    m.insert("R".into(), pool.red());
    m.insert("G".into(), pool.green());
    m.insert("C".into(), pool.colorless());
    m
}

fn phase_to_step(phase: forge_foundation::PhaseType) -> &'static str {
    use forge_foundation::PhaseType::*;
    match phase {
        Untap => "untap",
        Upkeep => "upkeep",
        Draw => "draw",
        Main1 => "main1",
        CombatBegin => "begin_combat",
        CombatDeclareAttackers => "declare_attackers",
        CombatDeclareBlockers => "declare_blockers",
        CombatFirstStrikeDamage => "first_strike_damage",
        CombatDamage => "combat_damage",
        CombatEnd => "end_combat",
        Main2 => "main2",
        EndOfTurn => "end",
        Cleanup => "cleanup",
    }
}

fn should_show_command_zone_card(game: &GameState, cid: CardId) -> bool {
    let card = game.card(cid);
    !(card.type_line.core_types.is_empty()
        && card
            .type_line
            .subtypes
            .iter()
            .any(|subtype| subtype.eq_ignore_ascii_case("Effect")))
}

pub fn card_to_dto(
    game: &GameState,
    cid: CardId,
    playable_ids: &[CardId],
    choosable_ids: &[CardId],
    zone_label: &str,
) -> CardDto {
    let card = game.card(cid);
    let types: Vec<String> = card
        .type_line
        .core_types
        .iter()
        .map(|ct| ct.name().to_string())
        .collect();
    let subtypes: Vec<String> = card.type_line.subtypes.clone();
    let supertypes: Vec<String> = card
        .type_line
        .supertypes
        .iter()
        .map(|st| st.name().to_string())
        .collect();

    let power = card.base_power.map(|_| card.power().to_string());
    let toughness = card.base_toughness.map(|_| card.toughness().to_string());
    let base_power = card.base_power;
    let base_toughness = card.base_toughness;

    // Collect non-zero counters, using the variant name as key (e.g. "P1P1", "M1M1", "Loyalty")
    let counters: HashMap<String, i32> = card
        .counters
        .iter()
        .filter(|(_, &v)| v > 0)
        .map(|(k, &v)| (format!("{k:?}"), v))
        .collect();

    // Build ability text from abilities
    let text = card
        .abilities
        .iter()
        .filter_map(|a| {
            // Extract SpellDescription$ if present
            for part in a.split('|') {
                let part = part.trim();
                if let Some(desc) = part.strip_prefix("SpellDescription$ ") {
                    return Some(desc.to_string());
                }
            }
            None
        })
        .collect::<Vec<_>>()
        .join("\n");

    // Face-down cards show as nameless 2/2 creatures with no info
    let morph_pt = forge_engine_core::spellability::MORPH_PT.to_string();
    let (
        name,
        types,
        subtypes,
        supertypes,
        power,
        toughness,
        base_power,
        base_toughness,
        text,
        color,
        mana_cost_str,
        cmc,
    ) = if card.face_down && card.zone == ZoneType::Battlefield {
        (
            "Face-down creature".to_string(),
            vec!["Creature".to_string()],
            vec![],
            vec![],
            Some(morph_pt.clone()),
            Some(morph_pt),
            None,
            None,
            String::new(),
            String::new(),
            String::new(),
            0,
        )
    } else {
        (
            card.card_name.clone(),
            types,
            subtypes,
            supertypes,
            power,
            toughness,
            base_power,
            base_toughness,
            text,
            card.color.to_string(),
            card.mana_cost.to_string(),
            card.mana_cost.cmc(),
        )
    };

    CardDto {
        id: card_id_str(cid),
        name,
        set_code: card.set_code.clone().unwrap_or_default(),
        card_number: card.card_number.clone().unwrap_or_default(),
        color,
        mana_cost: mana_cost_str,
        cmc,
        types,
        subtypes,
        supertypes,
        power,
        toughness,
        base_power,
        base_toughness,
        text,
        is_playable: playable_ids.contains(&cid),
        is_selected: false,
        is_choosable: choosable_ids.contains(&cid),
        controller_id: player_id_str(card.controller),
        owner_id: player_id_str(card.owner),
        zone_id: zone_label.to_string(),
        tapped: card.tapped,
        // Merge intrinsic keywords with those granted by continuous effects (layer 6)
        // and temporary pump keywords (KW$ parameter, until end of turn).
        keywords: {
            let mut all_kw = card.keywords.as_string_list();
            for k in card
                .granted_keywords
                .iter_strings()
                .chain(card.pump_keywords.iter_strings())
            {
                if !all_kw.iter().any(|e| e.eq_ignore_ascii_case(k)) {
                    all_kw.push(k.to_string());
                }
            }
            all_kw
        },
        counters,
        damage: card.damage,
        summoning_sick: card.summoning_sick && !card.has_haste(),
        is_token: card.is_token,
        is_double_faced: card.other_part.is_some(),
        flashback_cost: card.get_flashback_cost(),
        kicker_cost: card.get_kicker_cost(),
        is_transformed: card.is_transformed,
        is_face_down: card.face_down,
        is_bestowed: card.is_bestowed,
        attached_to: card.attached_to.map(card_id_str),
        attachment_ids: card
            .attachments
            .iter()
            .map(|&aid| card_id_str(aid))
            .collect(),
        phased_out: card.phased_out,
        exerted: card.exerted,
        effective_mana_cost: {
            let is_command_zone_commander =
                card.zone == ZoneType::Command && game.player_is_commander(card.controller, cid);
            if (playable_ids.contains(&cid) || is_command_zone_commander) && !card.is_land() {
                let cost_adj = forge_engine_core::staticability::static_ability_cost_change::compute_cost_adjustment(
                    game, card, card.controller, card.zone,
                );
                let mut adjusted = if !cost_adj.is_empty() {
                    cost_adj.apply(&card.mana_cost)
                } else {
                    card.mana_cost.clone()
                };

                if is_command_zone_commander {
                    let commander_tax = game.player_commander_tax(card.controller, cid);
                    if commander_tax > 0 {
                        adjusted = adjusted.add(&forge_foundation::ManaCost::generic(commander_tax));
                    }
                }

                let adjusted_str = adjusted.to_string();
                if adjusted_str != card.mana_cost.to_string() {
                    Some(adjusted_str)
                } else {
                    None
                }
            } else {
                None
            }
        },
        madness_cost: card.get_madness_cost(),
        is_madness_exiled: card.has_keyword(forge_engine_core::card::KEYWORD_MADNESS_EXILED),
        is_plotted: card
            .keywords
            .iter_strings()
            .chain(card.granted_keywords.iter_strings())
            .any(|kw| kw.starts_with(forge_engine_core::card::KEYWORD_PLOTTED_PREFIX)),
        is_warp_exiled: card.has_keyword(forge_engine_core::card::KEYWORD_WARP_EXILED),
    }
}

impl GameViewDto {
    pub fn from_engine(
        game: &GameState,
        mana_pools: &[ManaPool],
        human_player: PlayerId,
        game_id: &str,
        playable_ids: &[CardId],
        choosable_ids: &[CardId],
    ) -> Self {
        let mut players = Vec::new();
        for &pid in &game.player_order {
            let ps = game.player(pid);
            let pool = mana_pools.get(pid.index()).cloned().unwrap_or_default();
            let commander_damage: HashMap<String, i32> = ps
                .commander_damage_received
                .iter()
                .map(|(&card_raw_id, &dmg)| (card_id_str(CardId(card_raw_id)), dmg))
                .collect();
            players.push(PlayerDto {
                id: player_id_str(pid),
                name: ps.name.clone(),
                is_human: pid == human_player,
                life: ps.life,
                poison: ps.poison_counters,
                hand_count: game.cards_in_zone(ZoneType::Hand, pid).len(),
                library_count: game.cards_in_zone(ZoneType::Library, pid).len(),
                graveyard_count: game.cards_in_zone(ZoneType::Graveyard, pid).len(),
                exile_count: game.cards_in_zone(ZoneType::Exile, pid).len(),
                mana_pool: mana_pool_to_map(&pool),
                commander_damage,
                energy_counters: ps.energy_counters,
            });
        }

        // Hand cards — only for the human player
        let my_hand: Vec<CardDto> = game
            .cards_in_zone(ZoneType::Hand, human_player)
            .iter()
            .map(|&cid| card_to_dto(game, cid, playable_ids, choosable_ids, "hand"))
            .collect();

        // Battlefield — all players
        let mut battlefield = Vec::new();
        for &pid in &game.player_order {
            for &cid in game.cards_in_zone(ZoneType::Battlefield, pid) {
                battlefield.push(card_to_dto(
                    game,
                    cid,
                    playable_ids,
                    choosable_ids,
                    "battlefield",
                ));
            }
        }

        // Stack
        let stack: Vec<StackObjectDto> = game
            .stack
            .iter()
            .map(|entry| {
                let name = entry
                    .spell_ability
                    .source
                    .map(|cid| game.card(cid).card_name.clone())
                    .unwrap_or_else(|| "Ability".to_string());
                StackObjectDto {
                    id: format!("stack-{}", entry.id),
                    source_id: entry
                        .spell_ability
                        .source
                        .map(|c| card_id_str(c))
                        .unwrap_or_default(),
                    controller_id: player_id_str(entry.spell_ability.activating_player),
                    name,
                    text: entry.spell_ability.ability_text.clone(),
                    is_permanent_spell: entry.is_creature_spell || entry.is_permanent_spell,
                    targets: collect_stack_targets(&entry.spell_ability),
                }
            })
            .collect();

        // Graveyard — human player
        let graveyard: Vec<CardDto> = game
            .cards_in_zone(ZoneType::Graveyard, human_player)
            .iter()
            .map(|&cid| card_to_dto(game, cid, playable_ids, choosable_ids, "graveyard"))
            .collect();

        // Exile — human player
        let exile: Vec<CardDto> = game
            .cards_in_zone(ZoneType::Exile, human_player)
            .iter()
            .map(|&cid| card_to_dto(game, cid, playable_ids, choosable_ids, "exile"))
            .collect();

        // Opponent graveyard and exile
        let opponent_player = game
            .player_order
            .iter()
            .copied()
            .find(|&pid| pid != human_player);
        let opponent_graveyard: Vec<CardDto> = opponent_player
            .map(|pid| {
                game.cards_in_zone(ZoneType::Graveyard, pid)
                    .iter()
                    .map(|&cid| card_to_dto(game, cid, &[], &[], "graveyard"))
                    .collect()
            })
            .unwrap_or_default();
        let opponent_exile: Vec<CardDto> = opponent_player
            .map(|pid| {
                game.cards_in_zone(ZoneType::Exile, pid)
                    .iter()
                    .map(|&cid| card_to_dto(game, cid, &[], &[], "exile"))
                    .collect()
            })
            .unwrap_or_default();

        // Command zones
        let my_command_zone: Vec<CardDto> = game
            .cards_in_zone(ZoneType::Command, human_player)
            .iter()
            .copied()
            .filter(|&cid| should_show_command_zone_card(game, cid))
            .map(|cid| card_to_dto(game, cid, playable_ids, choosable_ids, "command"))
            .collect();

        let opponent_command_zone: Vec<CardDto> = opponent_player
            .map(|pid| {
                game.cards_in_zone(ZoneType::Command, pid)
                    .iter()
                    .copied()
                    .filter(|&cid| should_show_command_zone_card(game, cid))
                    .map(|cid| card_to_dto(game, cid, &[], &[], "command"))
                    .collect()
            })
            .unwrap_or_default();

        GameViewDto {
            game_id: game_id.to_string(),
            turn: game.turn.turn_number,
            step: phase_to_step(game.turn.phase).to_string(),
            combat_assignments: game
                .turn
                .combat_block_assignments
                .iter()
                .map(|(blocker, attacker)| CombatAssignmentDto {
                    blocker_id: card_id_str(*blocker),
                    attacker_id: card_id_str(*attacker),
                })
                .collect(),
            active_player_id: player_id_str(game.active_player()),
            priority_player_id: player_id_str(game.turn.priority_player),
            players,
            my_hand,
            battlefield,
            stack,
            exile,
            graveyard,
            opponent_graveyard,
            opponent_exile,
            my_command_zone,
            opponent_command_zone,
            game_over: game.game_over,
            winner_id: game.winner.map(player_id_str),
            monarch_id: game.monarch.map(player_id_str),
            initiative_holder_id: game.initiative_holder.map(player_id_str),
        }
    }
}
