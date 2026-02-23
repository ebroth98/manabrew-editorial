use forge_engine_core::ability::effects::{resolve_effect, EffectContext};
/// Integration test for counterspells and control effects
///
/// This test verifies that:
/// 1. Counterspells properly counter spells on the stack
/// 2. Control effects properly change controller of permanents
///
/// These features were identified as broken in PR #37's priority system implementation.
use forge_engine_core::agent::{MainPhaseAction, PlayerAgent, TargetChoice};
use forge_engine_core::card::CardInstance;
use forge_engine_core::game::GameState;
use forge_engine_core::ids::{CardId, PlayerId};
use forge_engine_core::spellability::{SpellAbility, StackEntry};
use forge_foundation::{CardTypeLine, ColorSet, ManaCost, ZoneType};

/// A simple agent that always passes (for testing)
struct PassAgent;

impl PlayerAgent for PassAgent {
    fn mulligan_decision(&mut self, _player: PlayerId, _hand: &[CardId]) -> bool {
        true
    }

    fn choose_action(
        &mut self,
        _player: PlayerId,
        _playable: &[CardId],
        _tappable_lands: &[CardId],
        _untappable_lands: &[CardId],
        _activatable: &[(CardId, usize)],
    ) -> MainPhaseAction {
        MainPhaseAction::Pass
    }

    fn choose_attackers(&mut self, _player: PlayerId, _available: &[CardId]) -> Vec<CardId> {
        Vec::new()
    }

    fn choose_blockers(
        &mut self,
        _player: PlayerId,
        _attackers: &[CardId],
        _available_blockers: &[CardId],
    ) -> Vec<(CardId, CardId)> {
        Vec::new()
    }

    fn choose_target_player(&mut self, _player: PlayerId, valid: &[PlayerId]) -> Option<PlayerId> {
        valid.first().copied()
    }

    fn choose_target_card(&mut self, _player: PlayerId, valid: &[CardId]) -> Option<CardId> {
        valid.first().copied()
    }

    fn choose_target_any(
        &mut self,
        _player: PlayerId,
        valid_players: &[PlayerId],
        valid_cards: &[CardId],
    ) -> TargetChoice {
        if let Some(&pid) = valid_players.last() {
            TargetChoice::Player(pid)
        } else if let Some(&cid) = valid_cards.first() {
            TargetChoice::Card(cid)
        } else {
            TargetChoice::None
        }
    }

    fn choose_land_or_spell(&mut self, _player: PlayerId) -> Option<bool> {
        None
    }

    fn notify(&mut self, _message: &str) {}
}

fn make_island(owner: PlayerId) -> CardInstance {
    CardInstance::new(
        CardId(0),
        "Island".to_string(),
        owner,
        CardTypeLine::parse("Basic Land - Island"),
        ManaCost::no_cost(),
        ColorSet::COLORLESS,
        None,
        None,
        vec![],
        vec![],
    )
}

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

/// Test basic control change functionality
#[test]
fn test_basic_control_change() {
    let mut game = GameState::new(&["Alice", "Bob"], 20);
    let p0 = PlayerId(0); // Alice
    let p1 = PlayerId(1); // Bob

    // Bob has a Grizzly Bears on the battlefield
    let bears = game.create_card(make_grizzly_bears(p1));
    game.move_card(bears, ZoneType::Battlefield, p1);
    game.card_mut(bears).summoning_sick = false;

    // Verify initial state
    let bears_card = game.card(bears);
    assert_eq!(
        bears_card.controller, p1,
        "Bears initially controlled by Bob"
    );
    assert_eq!(bears_card.owner, p1, "Bears owned by Bob");

    // Change control to Alice
    game.change_controller(bears, p0);

    // Verify new state
    let bears_card = game.card(bears);
    assert_eq!(bears_card.controller, p0, "Bears now controlled by Alice");
    assert_eq!(bears_card.owner, p1, "Bears still owned by Bob");

    // Verify zone tracking
    let alice_battlefield = game.zone(ZoneType::Battlefield, p0);
    let bob_battlefield = game.zone(ZoneType::Battlefield, p1);

    assert_eq!(alice_battlefield.len(), 1, "Alice should have Bears");
    assert_eq!(bob_battlefield.len(), 0, "Bob should have no creatures");
}

/// Test that Counter effect properly moves card to graveyard
#[test]
fn test_counterspell_moves_to_graveyard() {
    let mut game = GameState::new(&["Alice", "Bob"], 20);
    let p0 = PlayerId(0);
    let p1 = PlayerId(1);

    // Create a Lightning Bolt to be countered
    let lightning_bolt = CardInstance::new(
        CardId(0),
        "Lightning Bolt".to_string(),
        p1,
        CardTypeLine::parse("Instant"),
        ManaCost::parse("R"),
        ColorSet::RED,
        None,
        None,
        vec![],
        vec!["SP$ DealDamage | ValidTgts$ Any | NumDmg$ 3".to_string()],
    );
    let bolt_card = game.create_card(lightning_bolt);
    game.move_card(bolt_card, ZoneType::Stack, p1);

    // Put the bolt on the stack
    let sa = SpellAbility::new_simple(
        Some(bolt_card),
        p1,
        "SP$ DealDamage | ValidTgts$ Any | NumDmg$ 3",
    );
    let entry = StackEntry {
        id: 0,
        spell_ability: sa,
        is_creature_spell: false,
        is_permanent_spell: false,
    };
    game.stack.push(entry);

    let bolt_on_stack = game.stack.peek().unwrap().id;

    // Create counterspell SA targeting the bolt
    let counterspell = CardInstance::new(
        CardId(0),
        "Counterspell".to_string(),
        p0,
        CardTypeLine::parse("Instant"),
        ManaCost::parse("U U"),
        ColorSet::BLUE,
        None,
        None,
        vec![],
        vec!["SP$ Counter | TargetType$ Spell".to_string()],
    );
    let counterspell_card = game.create_card(counterspell);

    let mut counter_sa = SpellAbility::new_simple(
        Some(counterspell_card),
        p0,
        "SP$ Counter | TargetType$ Spell",
    );
    counter_sa.target_chosen.target_stack_entry = Some(bolt_on_stack);

    // Setup game loop for effect resolution
    use forge_engine_core::game_loop::GameLoop;
    let mut game_loop = GameLoop::new(2);
    let mut pass_agents: Vec<Box<dyn PlayerAgent>> = vec![Box::new(PassAgent), Box::new(PassAgent)];

    let mut ctx = EffectContext {
        game: &mut game,
        agents: &mut pass_agents,
        trigger_handler: &mut game_loop.trigger_handler,
        token_templates: &game_loop.token_templates,
        mana_pools: &mut game_loop.mana_pools,
        parent_target_card: None,
    };

    // Resolve the counter effect
    resolve_effect(&mut ctx, &counter_sa);

    // Verify Bolt moved to graveyard
    let bob_graveyard = game.zone(ZoneType::Graveyard, p1);
    assert_eq!(
        bob_graveyard.len(),
        1,
        "Bob should have 1 card in graveyard"
    );
    assert_eq!(
        game.card(bob_graveyard.cards[0]).card_name,
        "Lightning Bolt",
        "Countered Lightning Bolt should be in graveyard"
    );

    // Verify stack is empty
    assert!(game.stack.is_empty(), "Stack should be empty after counter");
}

/// Test ControlGain effect with Untap parameter
#[test]
fn test_control_gain_with_untap() {
    let mut game = GameState::new(&["Alice", "Bob"], 20);
    let p0 = PlayerId(0); // Alice gaining control
    let p1 = PlayerId(1); // Bob losing control

    // Create a tapped Grizzly Bears for Bob
    let bears = game.create_card(make_grizzly_bears(p1));
    game.move_card(bears, ZoneType::Battlefield, p1);
    game.card_mut(bears).tapped = true;
    game.card_mut(bears).summoning_sick = false;

    // Verify initial state
    let bears_card = game.card(bears);
    assert_eq!(
        bears_card.controller, p1,
        "Bears initially controlled by Bob"
    );
    assert!(bears_card.tapped, "Bears initially tapped");

    // Create ControlGain spell with Untap
    let control_spell = CardInstance::new(
        CardId(0),
        "Temporary Control".to_string(),
        p0,
        CardTypeLine::parse("Instant"),
        ManaCost::parse("2 U"),
        ColorSet::BLUE,
        None,
        None,
        vec![],
        vec!["SP$ ControlGain | ValidTgts$ Creature | Untap$ True".to_string()],
    );
    let control_card = game.create_card(control_spell);

    let mut control_sa = SpellAbility::new_simple(
        Some(control_card),
        p0,
        "SP$ ControlGain | ValidTgts$ Creature | Untap$ True",
    );
    control_sa.target_chosen.target_card = Some(bears);

    // Setup game loop for effect resolution
    use forge_engine_core::game_loop::GameLoop;
    let mut game_loop = GameLoop::new(2);
    let mut pass_agents: Vec<Box<dyn PlayerAgent>> = vec![Box::new(PassAgent), Box::new(PassAgent)];

    let mut ctx = EffectContext {
        game: &mut game,
        agents: &mut pass_agents,
        trigger_handler: &mut game_loop.trigger_handler,
        token_templates: &game_loop.token_templates,
        mana_pools: &mut game_loop.mana_pools,
        parent_target_card: None,
    };

    // Resolve the control effect
    resolve_effect(&mut ctx, &control_sa);

    // Verify control changed and creature untapped
    let bears_card = game.card(bears);
    assert_eq!(bears_card.controller, p0, "Bears now controlled by Alice");
    assert!(!bears_card.tapped, "Bears should be untapped");
}
