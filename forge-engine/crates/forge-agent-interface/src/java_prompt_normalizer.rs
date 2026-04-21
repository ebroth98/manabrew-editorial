use std::collections::HashMap;

use serde_json::{json, Value};

pub fn normalize_java_prompt(prompt: Value) -> Value {
    if !is_java_prompt(&prompt) {
        return prompt;
    }

    let actions = to_actions(prompt.get("actions"));
    let game_view = snapshot_to_game_view(
        prompt.get("snapshot").unwrap_or(&Value::Null),
        prompt.get("sessionId"),
        &actions,
    );
    let player = as_usize(prompt.get("player"), 0);
    let prompt_type = if player == 0 {
        "chooseAction"
    } else {
        "stateUpdate"
    };

    if prompt.get("kind").and_then(Value::as_str) == Some("choose_discard") {
        return json!({
            "type": if player == 0 { "chooseDiscard" } else { "stateUpdate" },
            "gameView": game_view,
            "displayEvents": [],
            "handCardIds": to_card_ids(prompt.get("cards")),
            "numToDiscard": as_usize(prompt.get("max"), as_usize(prompt.get("min"), 1)),
            "autoPassDisabled": true,
        });
    }
    if prompt.get("kind").and_then(Value::as_str) == Some("mulligan") {
        return json!({
            "type": if player == 0 { "mulligan" } else { "stateUpdate" },
            "gameView": game_view,
            "displayEvents": [],
            "handCardIds": to_card_ids(prompt.get("cards")),
            "mulliganCount": as_usize(prompt.get("count"), 0),
        });
    }
    if prompt.get("kind").and_then(Value::as_str) == Some("mulligan_put_back") {
        return json!({
            "type": if player == 0 { "mulliganPutBack" } else { "stateUpdate" },
            "gameView": game_view,
            "displayEvents": [],
            "handCardIds": to_card_ids(prompt.get("cards")),
            "cards": to_prompt_cards(prompt.get("cards")),
            "count": as_usize(prompt.get("count"), as_usize(prompt.get("max"), 0)),
        });
    }
    if prompt.get("kind").and_then(Value::as_str) == Some("choose_attackers") {
        return json!({
            "type": if player == 0 { "chooseAttackers" } else { "stateUpdate" },
            "gameView": game_view,
            "displayEvents": [],
            "availableAttackerIds": to_card_ids(prompt.get("attackers")),
            "possibleDefenderIds": to_defender_ids(prompt.get("defenders")),
            "autoPassDisabled": true,
        });
    }

    let my_hand = game_view
        .get("myHand")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let my_command_zone = game_view
        .get("myCommandZone")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut playable_options = Vec::new();
    let mut playable_card_ids_by_key =
        card_ids_by_key(my_hand.iter().chain(my_command_zone.iter()));
    let mut playable_card_ids_by_name =
        card_ids_by_name(my_hand.iter().chain(my_command_zone.iter()));
    for action in &actions {
        let Some(card_name) = action.get("cardName").and_then(Value::as_str) else {
            continue;
        };
        let card_id = action
            .get("cardKey")
            .and_then(Value::as_str)
            .and_then(|card_key| playable_card_ids_by_key.get(card_key).cloned())
            .or_else(|| {
                let card_id = pop_card_id(&mut playable_card_ids_by_name, card_name)?;
                if let Some(card_key) = action.get("cardKey").and_then(Value::as_str) {
                    playable_card_ids_by_key.insert(card_key.to_string(), card_id.clone());
                }
                Some(card_id)
            });
        if let Some(card_id) = card_id {
            playable_options.push(json!({
                "cardId": card_id,
                "mode": action.get("id").and_then(Value::as_str).unwrap_or_default(),
                "modeLabel": action.get("label").and_then(Value::as_str).unwrap_or_default(),
            }));
        }
    }

    let battlefield = game_view
        .get("battlefield")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut activatable_ability_ids = Vec::new();
    let mut mana_ability_options = Vec::new();
    let mut tappable_land_ids = Vec::new();
    let mut battlefield_card_ids_by_key = card_ids_by_key(
        battlefield
            .iter()
            .filter(|card| card.get("controllerId").and_then(Value::as_str) == Some("player-0")),
    );
    let mut battlefield_card_ids_by_name = card_ids_by_name(
        battlefield
            .iter()
            .filter(|card| card.get("controllerId").and_then(Value::as_str) == Some("player-0")),
    );
    for action in &actions {
        let kind = action.get("kind").and_then(Value::as_str);
        if kind != Some("mana") && kind != Some("ability") {
            continue;
        }
        let Some(card_name) = action.get("cardName").and_then(Value::as_str) else {
            continue;
        };
        let card_id = action
            .get("cardKey")
            .and_then(Value::as_str)
            .and_then(|card_key| battlefield_card_ids_by_key.get(card_key).cloned())
            .or_else(|| {
                let card_id = pop_card_id(&mut battlefield_card_ids_by_name, card_name)?;
                if let Some(card_key) = action.get("cardKey").and_then(Value::as_str) {
                    battlefield_card_ids_by_key.insert(card_key.to_string(), card_id.clone());
                }
                Some(card_id)
            });
        if let Some(card_id) = card_id {
            let ability = json!({
                "cardId": card_id,
                "abilityIndex": action_index(action.get("id")).unwrap_or(usize::MAX),
                "description": action.get("label").and_then(Value::as_str).unwrap_or_default(),
                "isManaAbility": kind == Some("mana"),
            });
            if kind == Some("mana") {
                if !tappable_land_ids.iter().any(|id| id == &card_id) {
                    tappable_land_ids.push(card_id);
                }
                mana_ability_options.push(ability);
            } else {
                activatable_ability_ids.push(ability);
            }
        }
    }

    json!({
        "type": prompt_type,
        "gameView": game_view,
        "displayEvents": [],
        "playableCardIds": playable_options
            .iter()
            .filter_map(|option| option.get("cardId").and_then(Value::as_str))
            .collect::<Vec<_>>(),
        "playableOptions": playable_options,
        "autoPassDisabled": true,
        "tappableLandIds": tappable_land_ids,
        "untappableLandIds": [],
        "activatableAbilityIds": activatable_ability_ids,
        "manaAbilityOptions": mana_ability_options,
    })
}

fn is_java_prompt(prompt: &Value) -> bool {
    matches!(
        prompt.get("kind").and_then(Value::as_str),
        Some("priority" | "choose_discard" | "mulligan" | "mulligan_put_back" | "choose_attackers")
    ) && prompt.get("snapshot").is_some_and(Value::is_object)
}

fn snapshot_to_game_view(snapshot: &Value, session_id: Option<&Value>, actions: &[Value]) -> Value {
    let players_source = snapshot
        .get("players")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut players: Vec<Value> = players_source
        .iter()
        .enumerate()
        .map(|(index, player)| to_player(player, index))
        .collect();
    while players.len() < 2 {
        let index = players.len();
        players.push(to_player(&Value::Null, index));
    }

    let action_card_names: Vec<&str> = actions
        .iter()
        .filter_map(|action| action.get("cardName").and_then(Value::as_str))
        .collect();
    let active_player_id = player_id(snapshot.get("active_player"));
    let mut battlefield = Vec::new();
    for (player_index, player) in players_source.iter().enumerate() {
        for (card_index, card) in player
            .get("battlefield_cards")
            .or_else(|| player.get("battlefield"))
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .enumerate()
        {
            battlefield.push(to_card(
                &card,
                player_index,
                card_index,
                "battlefield",
                &action_card_names,
            ));
        }
    }
    let stack: Vec<Value> = snapshot
        .get("stack")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .enumerate()
        .map(|(index, entry)| to_stack_object(&entry, index, &active_player_id))
        .collect();
    let my_hand = zone_cards(
        players_source.first(),
        "hand_cards",
        0,
        "hand",
        &action_card_names,
    );
    let my_command_zone = zone_cards(
        players_source.first(),
        "command_zone_cards",
        0,
        "command",
        &action_card_names,
    );
    let opponent_command_zone = zone_cards(
        players_source.get(1),
        "command_zone_cards",
        1,
        "opponentCommand",
        &action_card_names,
    );
    let graveyard = zone_cards(
        players_source.first(),
        "graveyard",
        0,
        "graveyard",
        &action_card_names,
    );
    let opponent_graveyard = zone_cards(
        players_source.get(1),
        "graveyard",
        1,
        "opponentGraveyard",
        &action_card_names,
    );
    let exile = zone_cards(
        players_source.first(),
        "exile",
        0,
        "exile",
        &action_card_names,
    );
    let opponent_exile = zone_cards(
        players_source.get(1),
        "exile",
        1,
        "opponentExile",
        &action_card_names,
    );

    json!({
        "gameId": session_id.and_then(Value::as_str).unwrap_or("engine-game"),
        "turn": as_i64(snapshot.get("turn"), 0),
        "step": normalize_step(snapshot.get("phase")),
        "activePlayerId": active_player_id,
        "priorityPlayerId": player_id(snapshot.get("priority_player")),
        "players": players,
        "myHand": my_hand,
        "battlefield": battlefield,
        "stack": stack,
        "exile": exile,
        "graveyard": graveyard,
        "opponentGraveyard": opponent_graveyard,
        "opponentExile": opponent_exile,
        "myCommandZone": my_command_zone,
        "opponentCommandZone": opponent_command_zone,
        "combatAssignments": [],
        "gameOver": snapshot.get("game_over").and_then(Value::as_bool).unwrap_or(false),
        "winnerId": snapshot.get("winner").and_then(Value::as_u64).map(|_| player_id(snapshot.get("winner"))),
        "monarchId": null,
        "initiativeHolderId": null,
    })
}

fn to_player(player: &Value, fallback_index: usize) -> Value {
    let index = as_usize(player.get("index"), fallback_index);
    json!({
        "id": player_id(Some(&json!(index))),
        "name": player.get("name").and_then(Value::as_str).unwrap_or("Player"),
        "isHuman": index == 0,
        "life": as_i64(player.get("life"), 20),
        "poison": as_i64(player.get("poison"), 0),
        "handCount": array_len(player.get("hand")),
        "libraryCount": as_i64(player.get("library_size"), 0),
        "graveyardCount": array_len(player.get("graveyard")),
        "exileCount": array_len(player.get("exile")),
        "manaPool": {},
        "commanderDamage": {},
        "energyCounters": 0,
    })
}

fn zone_cards(
    player: Option<&Value>,
    source_zone: &str,
    player_index: usize,
    zone_id: &str,
    action_card_names: &[&str],
) -> Vec<Value> {
    player
        .and_then(|player| {
            player.get(source_zone).or_else(|| {
                source_zone
                    .strip_suffix("_cards")
                    .and_then(|fallback_zone| player.get(fallback_zone))
            })
        })
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .enumerate()
        .map(|(card_index, card)| {
            to_card(&card, player_index, card_index, zone_id, action_card_names)
        })
        .collect()
}

fn to_card(
    card: &Value,
    player_index: usize,
    card_index: usize,
    zone_id: &str,
    action_card_names: &[&str],
) -> Value {
    let name = card
        .as_str()
        .or_else(|| card.get("name").and_then(Value::as_str))
        .unwrap_or("Unknown Card");
    let power = card.get("power").and_then(Value::as_i64);
    let toughness = card.get("toughness").and_then(Value::as_i64);
    let controller_index = card
        .get("controller")
        .and_then(Value::as_u64)
        .map(|value| value as usize)
        .unwrap_or(player_index);
    let id = card
        .get("id")
        .and_then(Value::as_str)
        .map(normalize_card_id)
        .unwrap_or_else(|| format!("engine-card-{player_index}-{zone_id}-{card_index}"));
    json!({
        "id": id,
        "name": name,
        "setCode": "",
        "cardNumber": "",
        "color": "",
        "manaCost": "",
        "types": [],
        "subtypes": [],
        "supertypes": [],
        "power": power.map(|value| value.to_string()),
        "toughness": toughness.map(|value| value.to_string()),
        "basePower": power,
        "baseToughness": toughness,
        "text": "",
        "isPlayable": action_card_names.contains(&name),
        "isSelected": false,
        "isChoosable": false,
        "controllerId": format!("player-{controller_index}"),
        "ownerId": format!("player-{player_index}"),
        "zoneId": zone_id,
        "tapped": card.get("tapped").and_then(Value::as_bool).unwrap_or(false),
        "counters": card.get("counters").cloned().unwrap_or_else(|| json!({})),
        "damage": card.get("damage").and_then(Value::as_i64),
        "summoningSick": card.get("summoning_sick").and_then(Value::as_bool).unwrap_or(false),
    })
}

fn to_stack_object(entry: &Value, index: usize, controller_id: &str) -> Value {
    json!({
        "id": format!("engine-stack-{index}"),
        "sourceId": format!("engine-stack-source-{index}"),
        "controllerId": controller_id,
        "name": entry.as_str().unwrap_or("Stack object"),
        "text": "",
        "isPermanentSpell": false,
        "targets": [],
    })
}

fn to_actions(actions: Option<&Value>) -> Vec<Value> {
    actions
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .enumerate()
        .filter_map(|(fallback_index, action)| {
            let index = as_usize(action.get("index"), fallback_index);
            let raw_label = action
                .get("label")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let label = format_action_label(raw_label);
            (!label.is_empty()).then(|| {
                json!({
                    "id": format!("prompt-action-{index}"),
                    "label": label,
                    "cardName": action_card_name(raw_label),
                    "cardKey": action_card_key(raw_label),
                    "kind": action_kind(raw_label),
                })
            })
        })
        .collect()
}

fn card_ids_by_name<'a, I>(cards: I) -> HashMap<String, Vec<String>>
where
    I: IntoIterator<Item = &'a Value>,
{
    let mut ids_by_name: HashMap<String, Vec<String>> = HashMap::new();
    for card in cards {
        let Some(name) = card.get("name").and_then(Value::as_str) else {
            continue;
        };
        let Some(id) = card.get("id").and_then(Value::as_str) else {
            continue;
        };
        ids_by_name
            .entry(name.to_string())
            .or_default()
            .push(id.to_string());
    }
    ids_by_name
}

fn card_ids_by_key<'a, I>(cards: I) -> HashMap<String, String>
where
    I: IntoIterator<Item = &'a Value>,
{
    let mut ids_by_key = HashMap::new();
    for card in cards {
        let Some(id) = card.get("id").and_then(Value::as_str) else {
            continue;
        };
        if let Some(key) = id.strip_prefix("engine-card-") {
            if !key.contains('-') {
                ids_by_key.insert(key.to_string(), id.to_string());
            }
        }
    }
    ids_by_key
}

fn pop_card_id(ids_by_name: &mut HashMap<String, Vec<String>>, card_name: &str) -> Option<String> {
    let ids = ids_by_name.get_mut(card_name)?;
    if ids.is_empty() {
        None
    } else {
        Some(ids.remove(0))
    }
}

fn to_card_ids(cards: Option<&Value>) -> Vec<String> {
    cards
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter_map(|card| {
            card.get("id")
                .and_then(Value::as_str)
                .map(normalize_card_id)
        })
        .collect()
}

fn to_prompt_cards(cards: Option<&Value>) -> Vec<Value> {
    cards
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter_map(|card| {
            let id = card.get("id").and_then(Value::as_str)?;
            let name = card
                .get("label")
                .and_then(Value::as_str)
                .unwrap_or("Unknown Card");
            Some(json!({
                "id": normalize_card_id(id),
                "name": name,
            }))
        })
        .collect()
}

fn to_defender_ids(defenders: Option<&Value>) -> Vec<Value> {
    defenders
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter_map(|defender| {
            let id = defender.get("id").and_then(Value::as_str)?;
            let label = defender.get("label").and_then(Value::as_str).unwrap_or(id);
            Some(json!({
                "id": id,
                "label": label,
            }))
        })
        .collect()
}

fn normalize_card_id(id: &str) -> String {
    id.strip_prefix("java-card-")
        .map(|suffix| format!("engine-card-{suffix}"))
        .unwrap_or_else(|| id.to_string())
}

fn action_kind(label: &str) -> Option<&'static str> {
    match strip_action_suffix(label)
        .split_once(':')
        .map(|(kind, _)| kind)
    {
        Some("LAND") => Some("play"),
        Some("SPELL") => Some("cast"),
        Some("CYCLE") => Some("ability"),
        Some("MANA") => Some("mana"),
        Some("AB") => Some("ability"),
        _ => None,
    }
}

fn format_action_label(label: &str) -> String {
    let normalized = strip_action_suffix(label);
    let Some((kind, card_name)) = normalized.split_once(':') else {
        return normalized;
    };
    let display_name = action_display_name(card_name);
    match kind {
        "LAND" => format!("Play {display_name}"),
        "SPELL" => format!("Cast {display_name}"),
        "CYCLE" => format!("Cycle {display_name}"),
        "MANA" => format!("Activate mana: {display_name}"),
        "AB" => format!("Activate {display_name}"),
        _ => normalized,
    }
}

fn action_card_name(label: &str) -> Option<String> {
    strip_action_suffix(label)
        .split_once(':')
        .map(|(_, card_name)| action_host_name(card_name).to_string())
}

fn action_display_name(card_name: &str) -> &str {
    card_name
        .split_once('|')
        .map(|(_, face_name)| face_name)
        .unwrap_or(card_name)
}

fn action_host_name(card_name: &str) -> &str {
    card_name
        .split_once('|')
        .map(|(host, _)| host)
        .unwrap_or(card_name)
}

fn action_card_key(label: &str) -> Option<String> {
    label
        .split('@')
        .nth(1)
        .filter(|key| !key.is_empty())
        .map(str::to_string)
}

fn strip_action_suffix(label: &str) -> String {
    label
        .split('@')
        .next()
        .unwrap_or(label)
        .split('$')
        .next()
        .unwrap_or(label)
        .to_string()
}

fn action_index(value: Option<&Value>) -> Option<usize> {
    value
        .and_then(Value::as_str)
        .and_then(|id| id.strip_prefix("prompt-action-"))
        .and_then(|index| index.parse::<usize>().ok())
}

fn player_id(index: Option<&Value>) -> String {
    format!("player-{}", as_usize(index, 0))
}

fn normalize_step(value: Option<&Value>) -> &'static str {
    match value.and_then(Value::as_str).unwrap_or_default() {
        "Untap" | "untap" => "untap",
        "Upkeep" | "upkeep" => "upkeep",
        "Draw" | "draw" => "draw",
        "Main1" | "main1" => "main1",
        "CombatBegin" | "begin_combat" => "begin_combat",
        "CombatDeclareAttackers" | "declare_attackers" => "declare_attackers",
        "CombatDeclareBlockers" | "declare_blockers" => "declare_blockers",
        "CombatFirstStrikeDamage" | "first_strike_damage" => "first_strike_damage",
        "CombatDamage" | "combat_damage" => "combat_damage",
        "CombatEnd" | "end_combat" => "end_combat",
        "Main2" | "main2" => "main2",
        "EndOfTurn" | "end" => "end",
        "Cleanup" | "cleanup" => "cleanup",
        _ => "untap",
    }
}

fn as_usize(value: Option<&Value>, fallback: usize) -> usize {
    value
        .and_then(Value::as_u64)
        .map(|value| value as usize)
        .unwrap_or(fallback)
}

fn as_i64(value: Option<&Value>, fallback: i64) -> i64 {
    value.and_then(Value::as_i64).unwrap_or(fallback)
}

fn array_len(value: Option<&Value>) -> usize {
    value.and_then(Value::as_array).map(Vec::len).unwrap_or(0)
}
