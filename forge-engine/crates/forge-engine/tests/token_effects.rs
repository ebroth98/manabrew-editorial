/// Integration tests for Token Creation and Copy Effects (Issue #14).

use forge_engine_core::agent::{PassAgent, PlayerAgent};
use forge_engine_core::card::CardInstance;
use forge_engine_core::game::GameState;
use forge_engine_core::game_loop::GameLoop;
use forge_engine_core::ids::{CardId, PlayerId};
use forge_engine_core::spellability::StackEntry;
use forge_foundation::{CardTypeLine, ColorSet, ManaCost, ZoneType};

// ── Helpers ──────────────────────────────────────────────────────────

fn make_grizzly_bears(owner: PlayerId) -> CardInstance {
    CardInstance::new(
        CardId(0),
        "Grizzly Bears".to_string(),
        owner,
        CardTypeLine::parse("Creature - Bear"),
        ManaCost::parse("1 G"),
        ColorSet::GREEN,
        Some(2),
        Some(2),
        vec![],
        vec![],
    )
}

fn make_goblin_token(owner: PlayerId) -> CardInstance {
    let mut card = CardInstance::new(
        CardId(0),
        "Goblin Token".to_string(),
        owner,
        CardTypeLine::parse("Creature Goblin"),
        ManaCost::no_cost(),
        ColorSet::RED,
        Some(1),
        Some(1),
        vec![],
        vec![],
    );
    card.is_token = true;
    card
}

fn make_soldier_token(owner: PlayerId) -> CardInstance {
    let mut card = CardInstance::new(
        CardId(0),
        "Soldier Token".to_string(),
        owner,
        CardTypeLine::parse("Creature Soldier"),
        ManaCost::no_cost(),
        ColorSet::WHITE,
        Some(1),
        Some(1),
        vec![],
        vec![],
    );
    card.is_token = true;
    card
}

fn pass_agents() -> Vec<Box<dyn PlayerAgent>> {
    vec![Box::new(PassAgent), Box::new(PassAgent)]
}

fn push_activated_entry(
    game: &mut GameState,
    controller: PlayerId,
    ability_text: &str,
    target_card: Option<CardId>,
) {
    game.stack.push(StackEntry {
        id: 0,
        source: None,
        controller,
        ability_text: ability_text.to_string(),
        is_creature_spell: false,
        is_permanent_spell: false,
        target_player: None,
        target_card,
        is_triggered_ability: false,
        is_activated_ability: true,
        trigger_source: None,
        trigger_index: None,
    });
}

// ── Token Creation Tests ──────────────────────────────────────────────

/// Token effect creates N tokens on the battlefield for the controller.
#[test]
fn test_create_single_token() {
    let mut game = GameState::new(&["Alice", "Bob"], 20);
    let p0 = PlayerId(0);

    let ability = "SP$ Token | TokenAmount$ 1 | TokenScript$ r_1_1_goblin | TokenOwner$ You";
    push_activated_entry(&mut game, p0, ability, None);

    let mut agents = pass_agents();
    let mut game_loop = GameLoop::new(2);
    // Register the goblin token template
    game_loop.register_token("r_1_1_goblin", make_goblin_token(p0));

    game_loop.resolve_stack(&mut game, &mut agents);

    assert_eq!(
        game.zone(ZoneType::Battlefield, p0).len(),
        1,
        "Alice should have 1 token on the battlefield"
    );
    let token_id = game.zone(ZoneType::Battlefield, p0).cards[0];
    assert!(game.card(token_id).is_token, "Created card should be a token");
    assert_eq!(game.card(token_id).card_name, "Goblin Token");
    assert_eq!(game.card(token_id).base_power, Some(1));
    assert_eq!(game.card(token_id).base_toughness, Some(1));
}

/// Token effect creates multiple tokens (TokenAmount$ 3).
#[test]
fn test_create_multiple_tokens() {
    let mut game = GameState::new(&["Alice", "Bob"], 20);
    let p0 = PlayerId(0);

    let ability = "SP$ Token | TokenAmount$ 3 | TokenScript$ r_1_1_goblin | TokenOwner$ You";
    push_activated_entry(&mut game, p0, ability, None);

    let mut agents = pass_agents();
    let mut game_loop = GameLoop::new(2);
    game_loop.register_token("r_1_1_goblin", make_goblin_token(p0));
    game_loop.resolve_stack(&mut game, &mut agents);

    assert_eq!(
        game.zone(ZoneType::Battlefield, p0).len(),
        3,
        "Alice should have 3 Goblin tokens"
    );
    for &tid in &game.zone(ZoneType::Battlefield, p0).cards.clone() {
        assert!(game.card(tid).is_token);
    }
}

/// TokenOwner$ Opponent creates the token for the opponent.
#[test]
fn test_token_for_opponent() {
    let mut game = GameState::new(&["Alice", "Bob"], 20);
    let p0 = PlayerId(0);
    let p1 = PlayerId(1);

    let ability = "SP$ Token | TokenAmount$ 2 | TokenScript$ w_1_1_soldier | TokenOwner$ Opponent";
    push_activated_entry(&mut game, p0, ability, None);

    let mut agents = pass_agents();
    let mut game_loop = GameLoop::new(2);
    game_loop.register_token("w_1_1_soldier", make_soldier_token(p0));
    game_loop.resolve_stack(&mut game, &mut agents);

    assert_eq!(
        game.zone(ZoneType::Battlefield, p1).len(),
        2,
        "Bob should have 2 soldier tokens (TokenOwner$ Opponent)"
    );
    assert_eq!(game.zone(ZoneType::Battlefield, p0).len(), 0);
}

/// Missing token script logs a warning and creates nothing (no panic).
#[test]
fn test_missing_token_script_is_silent() {
    let mut game = GameState::new(&["Alice", "Bob"], 20);
    let p0 = PlayerId(0);

    let ability = "SP$ Token | TokenAmount$ 1 | TokenScript$ nonexistent_token | TokenOwner$ You";
    push_activated_entry(&mut game, p0, ability, None);

    let mut agents = pass_agents();
    let mut game_loop = GameLoop::new(2);
    // Intentionally do NOT register the script
    game_loop.resolve_stack(&mut game, &mut agents);

    assert_eq!(game.zone(ZoneType::Battlefield, p0).len(), 0, "No token should be created");
}

// ── Token Cease-to-Exist Tests ────────────────────────────────────────

/// Tokens cease to exist when they leave the battlefield (CR 110.5g).
#[test]
fn test_token_ceases_to_exist_on_death() {
    let mut game = GameState::new(&["Alice", "Bob"], 20);
    let p0 = PlayerId(0);

    // Place a token directly on the battlefield
    let token = game.create_card(make_goblin_token(p0));
    game.move_card(token, ZoneType::Battlefield, p0);
    assert_eq!(game.zone(ZoneType::Battlefield, p0).len(), 1);

    // Now deal lethal damage and run SBAs
    game.deal_damage_to_card(token, 5);
    game.check_state_based_actions();

    // Token should be gone from all zones
    assert_eq!(game.zone(ZoneType::Battlefield, p0).len(), 0, "Token should leave battlefield");
    assert_eq!(game.zone(ZoneType::Graveyard, p0).len(), 0, "Token should NOT go to graveyard");
    assert_eq!(game.card(token).zone, ZoneType::None, "Token zone should be None (ceased to exist)");
}

/// Regular (non-token) creatures still go to the graveyard when they die.
#[test]
fn test_non_token_goes_to_graveyard_on_death() {
    let mut game = GameState::new(&["Alice", "Bob"], 20);
    let p0 = PlayerId(0);

    let bears = game.create_card(make_grizzly_bears(p0));
    game.move_card(bears, ZoneType::Battlefield, p0);
    game.deal_damage_to_card(bears, 5);
    game.check_state_based_actions();

    assert_eq!(game.zone(ZoneType::Graveyard, p0).len(), 1, "Regular creature should go to graveyard");
    assert_eq!(game.card(bears).zone, ZoneType::Graveyard);
}

// ── CopyPermanent Tests ───────────────────────────────────────────────

/// CopyPermanent creates a copy of a targeted creature on the battlefield.
#[test]
fn test_copy_permanent() {
    let mut game = GameState::new(&["Alice", "Bob"], 20);
    let p0 = PlayerId(0);
    let p1 = PlayerId(1);

    // Bob has a Grizzly Bears on the battlefield
    let bears = game.create_card(make_grizzly_bears(p1));
    game.move_card(bears, ZoneType::Battlefield, p1);

    // Alice copies it (Clone effect)
    let ability = "SP$ CopyPermanent";
    push_activated_entry(&mut game, p0, ability, Some(bears));

    let mut agents = pass_agents();
    let mut game_loop = GameLoop::new(2);
    game_loop.resolve_stack(&mut game, &mut agents);

    // Alice should now have a copy
    assert_eq!(game.zone(ZoneType::Battlefield, p0).len(), 1, "Alice should have the copy");
    // Bob still has the original
    assert_eq!(game.zone(ZoneType::Battlefield, p1).len(), 1, "Bob still has the original");

    let copy_id = game.zone(ZoneType::Battlefield, p0).cards[0];
    assert!(game.card(copy_id).is_token, "Copy should be flagged as token");
    assert_eq!(game.card(copy_id).card_name, "Grizzly Bears");
    assert_eq!(game.card(copy_id).base_power, Some(2));
    assert_eq!(game.card(copy_id).base_toughness, Some(2));
}

/// CopyPermanent with PumpKeywords$ adds the keyword to the copy.
#[test]
fn test_copy_permanent_with_pump_keywords() {
    let mut game = GameState::new(&["Alice", "Bob"], 20);
    let p0 = PlayerId(0);
    let p1 = PlayerId(1);

    let bears = game.create_card(make_grizzly_bears(p1));
    game.move_card(bears, ZoneType::Battlefield, p1);

    let ability = "SP$ CopyPermanent | PumpKeywords$ Haste";
    push_activated_entry(&mut game, p0, ability, Some(bears));

    let mut agents = pass_agents();
    let mut game_loop = GameLoop::new(2);
    game_loop.resolve_stack(&mut game, &mut agents);

    let copy_id = game.zone(ZoneType::Battlefield, p0).cards[0];
    assert!(
        game.card(copy_id).granted_keywords.contains(&"Haste".to_string()),
        "Copy should have Haste from PumpKeywords$"
    );
}

/// Copy-tokens cease to exist when they leave the battlefield.
#[test]
fn test_copy_ceases_to_exist_on_leaving() {
    let mut game = GameState::new(&["Alice", "Bob"], 20);
    let p0 = PlayerId(0);
    let p1 = PlayerId(1);

    let bears = game.create_card(make_grizzly_bears(p1));
    game.move_card(bears, ZoneType::Battlefield, p1);

    let ability = "SP$ CopyPermanent";
    push_activated_entry(&mut game, p0, ability, Some(bears));

    let mut agents = pass_agents();
    let mut game_loop = GameLoop::new(2);
    game_loop.resolve_stack(&mut game, &mut agents);

    let copy_id = game.zone(ZoneType::Battlefield, p0).cards[0];
    // Move copy to graveyard (simulating death)
    game.move_card(copy_id, ZoneType::Graveyard, p0);

    assert_eq!(game.card(copy_id).zone, ZoneType::None, "Copy should cease to exist");
    assert_eq!(game.zone(ZoneType::Graveyard, p0).len(), 0, "Copy should NOT be in graveyard");
}
