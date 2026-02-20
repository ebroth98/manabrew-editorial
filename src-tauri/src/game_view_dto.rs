use std::collections::HashMap;

use forge_engine_core::game::GameState;
use forge_engine_core::ids::{CardId, PlayerId};
use forge_engine_core::mana_pool::ManaPool;
use forge_foundation::ZoneType;
use serde::{Deserialize, Serialize};

/// Frontend-compatible game state snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameViewDto {
    pub game_id: String,
    pub turn: u32,
    pub step: String,
    pub active_player_id: String,
    pub priority_player_id: String,
    pub players: Vec<PlayerDto>,
    pub my_hand: Vec<CardDto>,
    pub battlefield: Vec<CardDto>,
    pub stack: Vec<StackObjectDto>,
    pub exile: Vec<CardDto>,
    pub graveyard: Vec<CardDto>,
    pub game_over: bool,
    pub winner_id: Option<String>,
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
    pub text: String,
    pub is_playable: bool,
    pub is_selected: bool,
    pub is_choosable: bool,
    pub controller_id: String,
    pub owner_id: String,
    pub zone_id: String,
    pub tapped: bool,
    pub keywords: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StackObjectDto {
    pub id: String,
    pub source_id: String,
    pub name: String,
    pub text: String,
}

fn player_id_str(pid: PlayerId) -> String {
    format!("player-{}", pid.0)
}

fn card_id_str(cid: CardId) -> String {
    format!("card-{}", cid.0)
}

fn mana_pool_to_map(pool: &ManaPool) -> HashMap<String, i32> {
    let mut m = HashMap::new();
    m.insert("W".into(), pool.white);
    m.insert("U".into(), pool.blue);
    m.insert("B".into(), pool.black);
    m.insert("R".into(), pool.red);
    m.insert("G".into(), pool.green);
    m.insert("C".into(), pool.colorless);
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
        CombatFirstStrikeDamage => "combat_damage",
        CombatDamage => "combat_damage",
        CombatEnd => "end_combat",
        Main2 => "main2",
        EndOfTurn => "end",
        Cleanup => "cleanup",
    }
}

fn card_to_dto(
    game: &GameState,
    cid: CardId,
    playable_ids: &[CardId],
    choosable_ids: &[CardId],
    zone_label: &str,
) -> CardDto {
    let card = game.card(cid);
    let types: Vec<String> = card.type_line.core_types.iter().map(|ct| ct.name().to_string()).collect();
    let subtypes: Vec<String> = card.type_line.subtypes.clone();
    let supertypes: Vec<String> = card.type_line.supertypes.iter().map(|st| st.name().to_string()).collect();

    let power = card.base_power.map(|_| card.power().to_string());
    let toughness = card.base_toughness.map(|_| card.toughness().to_string());

    // Build ability text from abilities
    let text = card.abilities.iter()
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

    CardDto {
        id: card_id_str(cid),
        name: card.card_name.clone(),
        set_code: String::new(),
        card_number: String::new(),
        color: card.color.to_string(),
        mana_cost: card.mana_cost.to_string(),
        cmc: card.mana_cost.cmc(),
        types,
        subtypes,
        supertypes,
        power,
        toughness,
        text,
        is_playable: playable_ids.contains(&cid),
        is_selected: false,
        is_choosable: choosable_ids.contains(&cid),
        controller_id: player_id_str(card.controller),
        owner_id: player_id_str(card.owner),
        zone_id: zone_label.to_string(),
        tapped: card.tapped,
        keywords: card.keywords.clone(),
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
                battlefield.push(card_to_dto(game, cid, playable_ids, choosable_ids, "battlefield"));
            }
        }

        // Stack
        let stack: Vec<StackObjectDto> = game.stack.iter().map(|entry| {
            let name = entry.source
                .map(|cid| game.card(cid).card_name.clone())
                .unwrap_or_else(|| "Ability".to_string());
            StackObjectDto {
                id: format!("stack-{}", entry.id),
                source_id: entry.source.map(|c| card_id_str(c)).unwrap_or_default(),
                name,
                text: entry.ability_text.clone(),
            }
        }).collect();

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

        GameViewDto {
            game_id: game_id.to_string(),
            turn: game.turn.turn_number,
            step: phase_to_step(game.turn.phase).to_string(),
            active_player_id: player_id_str(game.active_player()),
            priority_player_id: player_id_str(game.turn.priority_player),
            players,
            my_hand,
            battlefield,
            stack,
            exile,
            graveyard,
            game_over: game.game_over,
            winner_id: game.winner.map(player_id_str),
        }
    }
}
