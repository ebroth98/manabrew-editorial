use std::io::{self, Write};

use forge_engine_core::agent::PlayerAgent;
use forge_engine_core::card::CardInstance;
use forge_engine_core::game::GameState;
use forge_engine_core::game_loop::GameLoop;
use forge_engine_core::ids::{CardId, PlayerId};
use forge_foundation::{CardTypeLine, ColorSet, ManaCost, ZoneType};
use rand::SeedableRng;

// ── ANSI colors ──────────────────────────────────────────────────────

const RESET: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";
const DIM: &str = "\x1b[2m";
const RED: &str = "\x1b[31m";
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const BLUE: &str = "\x1b[34m";
const MAGENTA: &str = "\x1b[35m";
const CYAN: &str = "\x1b[36m";
const WHITE: &str = "\x1b[37m";

// ── Card display helpers ─────────────────────────────────────────────

fn card_color(card: &CardInstance) -> &'static str {
    let c = &card.color;
    let count = [c.has_red(), c.has_green(), c.has_blue(), c.has_white(), c.has_black()]
        .iter()
        .filter(|&&x| x)
        .count();

    if count == 1 {
        if c.has_red() { return RED; }
        if c.has_green() { return GREEN; }
        if c.has_blue() { return BLUE; }
        if c.has_white() { return YELLOW; }
        if c.has_black() { return MAGENTA; }
    }
    if card.is_land() { DIM } else { WHITE }
}

fn format_card(card: &CardInstance) -> String {
    let color = card_color(card);
    let tapped = if card.tapped { " (T)" } else { "" };
    let sick = if card.is_creature() && card.summoning_sick && card.zone == ZoneType::Battlefield {
        " [sick]"
    } else {
        ""
    };

    if card.is_creature() {
        format!("{}{}{} {}/{}{}{}",
            color, card.card_name, tapped, card.power(), card.toughness(), sick, RESET)
    } else if card.is_land() {
        format!("{}{}{}{}", color, card.card_name, tapped, RESET)
    } else {
        format!("{}{} [{}]{}", color, card.card_name, card.mana_cost, RESET)
    }
}

fn format_card_with_cost(card: &CardInstance) -> String {
    let color = card_color(card);
    if card.is_land() {
        format!("{}{}{}", color, card.card_name, RESET)
    } else if card.is_creature() {
        format!("{}{} {}/{} [{}]{}",
            color, card.card_name, card.power(), card.toughness(), card.mana_cost, RESET)
    } else {
        format!("{}{} [{}]{}", color, card.card_name, card.mana_cost, RESET)
    }
}

// ── Display game state ───────────────────────────────────────────────

fn display_board(game: &GameState, you: PlayerId, opp: PlayerId) {
    let opp_state = game.player(opp);
    let you_state = game.player(you);

    println!();
    println!("{}================================================================{}", BOLD, RESET);

    // Opponent
    println!("  {}{}{}{} Life: {}{}{}  |  Hand: {}  Library: {}  GY: {}",
        RED, BOLD, opp_state.name, RESET,
        BOLD, opp_state.life, RESET,
        game.zone(ZoneType::Hand, opp).len(),
        game.zone(ZoneType::Library, opp).len(),
        game.zone(ZoneType::Graveyard, opp).len(),
    );

    let opp_bf = game.cards_in_zone(ZoneType::Battlefield, opp);
    if !opp_bf.is_empty() {
        let cards: Vec<String> = opp_bf.iter().map(|&cid| format_card(game.card(cid))).collect();
        println!("  Battlefield: {}", cards.join(", "));
    } else {
        println!("  Battlefield: {}(empty){}", DIM, RESET);
    }

    println!("{}  ----------------------------------------------------------{}", DIM, RESET);

    // You
    let your_bf = game.cards_in_zone(ZoneType::Battlefield, you);
    if !your_bf.is_empty() {
        let cards: Vec<String> = your_bf.iter().map(|&cid| format_card(game.card(cid))).collect();
        println!("  Battlefield: {}", cards.join(", "));
    } else {
        println!("  Battlefield: {}(empty){}", DIM, RESET);
    }

    println!("  {}{}{}{} Life: {}{}{}  |  Library: {}  GY: {}",
        GREEN, BOLD, you_state.name, RESET,
        BOLD, you_state.life, RESET,
        game.zone(ZoneType::Library, you).len(),
        game.zone(ZoneType::Graveyard, you).len(),
    );

    let hand = game.cards_in_zone(ZoneType::Hand, you);
    if !hand.is_empty() {
        let cards: Vec<String> = hand.iter().map(|&cid| format_card_with_cost(game.card(cid))).collect();
        println!("  Hand: {}", cards.join(", "));
    } else {
        println!("  Hand: {}(empty){}", DIM, RESET);
    }

    println!("  {}Turn {} | Phase: {:?} | Active: {}{}",
        DIM, game.turn.turn_number, game.turn.phase,
        game.player(game.active_player()).name, RESET,
    );
    println!("{}================================================================{}", BOLD, RESET);
}

// ── Input helpers ────────────────────────────────────────────────────

fn read_line() -> String {
    let mut input = String::new();
    io::stdout().flush().unwrap();
    io::stdin().read_line(&mut input).unwrap();
    input.trim().to_string()
}

fn read_number(prompt: &str, max: usize) -> Option<usize> {
    print!("{}", prompt);
    let input = read_line();
    if input.is_empty() || input == "p" || input == "pass" {
        return None;
    }
    match input.parse::<usize>() {
        Ok(n) if n < max => Some(n),
        _ => {
            println!("  Invalid choice.");
            None
        }
    }
}

fn read_numbers(prompt: &str, max: usize) -> Vec<usize> {
    print!("{}", prompt);
    let input = read_line();
    if input.is_empty() || input == "none" || input == "n" {
        return Vec::new();
    }
    if input == "all" || input == "a" {
        return (0..max).collect();
    }
    input
        .split(|c: char| c == ',' || c.is_whitespace())
        .filter_map(|s| s.trim().parse::<usize>().ok())
        .filter(|&n| n < max)
        .collect()
}

// ── Interactive Player Agent ─────────────────────────────────────────
// This agent reads from the GameState directly via a raw pointer that
// we update before each interaction. This is safe because we control
// the call pattern: set pointer -> call agent method -> return.

struct InteractiveAgent {
    game_ptr: *const GameState,
}

impl InteractiveAgent {
    fn new() -> Self {
        InteractiveAgent {
            game_ptr: std::ptr::null(),
        }
    }

    fn game(&self) -> &GameState {
        assert!(!self.game_ptr.is_null());
        unsafe { &*self.game_ptr }
    }
}

impl PlayerAgent for InteractiveAgent {
    fn mulligan_decision(&mut self, _player: PlayerId, hand: &[CardId]) -> bool {
        let game = self.game();
        println!("\n{}{}Opening Hand:{}", CYAN, BOLD, RESET);
        for (i, &cid) in hand.iter().enumerate() {
            println!("  {}: {}", i, format_card_with_cost(game.card(cid)));
        }
        print!("{}Keep this hand? (y/n): {}", CYAN, RESET);
        let input = read_line();
        input != "n" && input != "no"
    }

    fn choose_action(&mut self, player: PlayerId, playable: &[CardId]) -> Option<CardId> {
        let game = self.game();

        // Display the board each time we're asked for an action
        let opp = game.opponent_of(player);
        display_board(game, player, opp);

        if playable.is_empty() {
            println!("  {}No playable cards.{}", DIM, RESET);
            return None;
        }

        println!("\n{}{}Playable cards:{}", CYAN, BOLD, RESET);
        for (i, &cid) in playable.iter().enumerate() {
            let card = game.card(cid);
            let action = if card.is_land() { "Play" } else { "Cast" };
            println!("  {}{}{}: {} {}", BOLD, i, RESET, action, format_card_with_cost(card));
        }
        println!("  {}(enter number to play, or 'p' to pass){}", DIM, RESET);

        read_number(&format!("{}> {}", CYAN, RESET), playable.len())
            .map(|idx| playable[idx])
    }

    fn choose_attackers(&mut self, _player: PlayerId, available: &[CardId]) -> Vec<CardId> {
        if available.is_empty() {
            return Vec::new();
        }

        let game = self.game();
        println!("\n{}{}Declare Attackers:{}", CYAN, BOLD, RESET);
        for (i, &cid) in available.iter().enumerate() {
            let card = game.card(cid);
            println!("  {}{}{}: {} {}/{}",
                BOLD, i, RESET, card.card_name, card.power(), card.toughness());
        }
        println!("  {}(numbers separated by spaces, 'all', or 'n' for none){}", DIM, RESET);

        let indices = read_numbers(&format!("{}Attack> {}", CYAN, RESET), available.len());
        indices.into_iter().map(|i| available[i]).collect()
    }

    fn choose_blockers(
        &mut self,
        _player: PlayerId,
        attackers: &[CardId],
        available_blockers: &[CardId],
    ) -> Vec<(CardId, CardId)> {
        if attackers.is_empty() || available_blockers.is_empty() {
            return Vec::new();
        }

        let game = self.game();
        println!("\n{}{}Incoming Attackers:{}", RED, BOLD, RESET);
        for (i, &cid) in attackers.iter().enumerate() {
            let card = game.card(cid);
            println!("  {}{}[A{}]{}: {} {}/{}",
                RED, BOLD, i, RESET, card.card_name, card.power(), card.toughness());
        }

        println!("{}{}Your Blockers:{}", GREEN, BOLD, RESET);
        for (i, &cid) in available_blockers.iter().enumerate() {
            let card = game.card(cid);
            println!("  {}{}[B{}]{}: {} {}/{}",
                GREEN, BOLD, i, RESET, card.card_name, card.power(), card.toughness());
        }

        println!("  {}Enter block assignments as 'B0=A0 B1=A0' or 'n' for no blocks{}", DIM, RESET);
        print!("{}Block> {}", CYAN, RESET);
        let input = read_line();

        if input.is_empty() || input == "n" || input == "none" {
            return Vec::new();
        }

        let mut blocks = Vec::new();
        for assignment in input.split_whitespace() {
            let parts: Vec<&str> = assignment.split('=').collect();
            if parts.len() == 2 {
                let b_idx = parts[0].trim_start_matches(|c: char| c.is_alphabetic()).parse::<usize>();
                let a_idx = parts[1].trim_start_matches(|c: char| c.is_alphabetic()).parse::<usize>();
                if let (Ok(b), Ok(a)) = (b_idx, a_idx) {
                    if b < available_blockers.len() && a < attackers.len() {
                        blocks.push((available_blockers[b], attackers[a]));
                    }
                }
            }
        }
        blocks
    }

    fn choose_target_player(&mut self, _player: PlayerId, valid: &[PlayerId]) -> Option<PlayerId> {
        if valid.len() == 1 {
            let game = self.game();
            let target = game.player(valid[0]);
            println!("  {}Targeting {} (life: {}){}", CYAN, target.name, target.life, RESET);
            return Some(valid[0]);
        }

        let game = self.game();
        println!("\n{}Choose target player:{}", CYAN, RESET);
        for (i, &pid) in valid.iter().enumerate() {
            let p = game.player(pid);
            println!("  {}: {} (life: {})", i, p.name, p.life);
        }
        read_number(&format!("{}Target> {}", CYAN, RESET), valid.len()).map(|i| valid[i])
    }

    fn choose_target_card(&mut self, _player: PlayerId, valid: &[CardId]) -> Option<CardId> {
        if valid.is_empty() {
            return None;
        }
        let game = self.game();
        println!("\n{}Choose target creature:{}", CYAN, RESET);
        for (i, &cid) in valid.iter().enumerate() {
            println!("  {}: {}", i, format_card(game.card(cid)));
        }
        read_number(&format!("{}Target> {}", CYAN, RESET), valid.len()).map(|i| valid[i])
    }

    fn choose_land_or_spell(&mut self, _player: PlayerId) -> Option<bool> {
        None
    }

    fn notify(&mut self, message: &str) {
        println!("  {}>> {}{}", CYAN, message, RESET);
    }
}

// ── Simple AI Agent ──────────────────────────────────────────────────

struct SimpleAiAgent;

impl PlayerAgent for SimpleAiAgent {
    fn mulligan_decision(&mut self, _: PlayerId, _: &[CardId]) -> bool { true }

    fn choose_action(&mut self, _: PlayerId, playable: &[CardId]) -> Option<CardId> {
        playable.first().copied()
    }

    fn choose_attackers(&mut self, _: PlayerId, available: &[CardId]) -> Vec<CardId> {
        available.to_vec()
    }

    fn choose_blockers(
        &mut self, _: PlayerId, attackers: &[CardId], available_blockers: &[CardId],
    ) -> Vec<(CardId, CardId)> {
        // Block biggest attacker with first available blocker
        if !attackers.is_empty() && !available_blockers.is_empty() {
            vec![(available_blockers[0], attackers[0])]
        } else {
            Vec::new()
        }
    }

    fn choose_target_player(&mut self, _: PlayerId, valid: &[PlayerId]) -> Option<PlayerId> {
        valid.first().copied()
    }

    fn choose_target_card(&mut self, _: PlayerId, valid: &[CardId]) -> Option<CardId> {
        valid.first().copied()
    }

    fn choose_land_or_spell(&mut self, _: PlayerId) -> Option<bool> { None }

    fn notify(&mut self, message: &str) {
        println!("  {}>> [AI] {}{}", RED, message, RESET);
    }
}

// ── Card constructors ────────────────────────────────────────────────

fn make_mountain(owner: PlayerId) -> CardInstance {
    CardInstance::new(CardId(0), "Mountain".to_string(), owner,
        CardTypeLine::parse("Basic Land - Mountain"),
        ManaCost::no_cost(), ColorSet::COLORLESS, None, None, vec![], vec![])
}

fn make_forest(owner: PlayerId) -> CardInstance {
    CardInstance::new(CardId(0), "Forest".to_string(), owner,
        CardTypeLine::parse("Basic Land - Forest"),
        ManaCost::no_cost(), ColorSet::COLORLESS, None, None, vec![], vec![])
}

fn make_lightning_bolt(owner: PlayerId) -> CardInstance {
    CardInstance::new(CardId(0), "Lightning Bolt".to_string(), owner,
        CardTypeLine::parse("Instant"),
        ManaCost::parse("R"), ColorSet::RED, None, None, vec![],
        vec!["SP$ DealDamage | ValidTgts$ Any | NumDmg$ 3 | SpellDescription$ CARDNAME deals 3 damage to any target.".to_string()])
}

fn make_grizzly_bears(owner: PlayerId) -> CardInstance {
    CardInstance::new(CardId(0), "Grizzly Bears".to_string(), owner,
        CardTypeLine::parse("Creature - Bear"),
        ManaCost::parse("1 G"), ColorSet::GREEN, Some(2), Some(2), vec![], vec![])
}

fn make_grey_ogre(owner: PlayerId) -> CardInstance {
    CardInstance::new(CardId(0), "Gray Ogre".to_string(), owner,
        CardTypeLine::parse("Creature - Ogre"),
        ManaCost::parse("2 R"), ColorSet::RED, Some(2), Some(2), vec![], vec![])
}

fn make_hill_giant(owner: PlayerId) -> CardInstance {
    CardInstance::new(CardId(0), "Hill Giant".to_string(), owner,
        CardTypeLine::parse("Creature - Giant"),
        ManaCost::parse("3 R"), ColorSet::RED, Some(3), Some(3), vec![], vec![])
}

fn make_centaur_courser(owner: PlayerId) -> CardInstance {
    CardInstance::new(CardId(0), "Centaur Courser".to_string(), owner,
        CardTypeLine::parse("Creature - Centaur Warrior"),
        ManaCost::parse("2 G"), ColorSet::GREEN, Some(3), Some(3), vec![], vec![])
}

fn make_craw_wurm(owner: PlayerId) -> CardInstance {
    CardInstance::new(CardId(0), "Craw Wurm".to_string(), owner,
        CardTypeLine::parse("Creature - Wurm"),
        ManaCost::parse("4 G G"), ColorSet::GREEN, Some(6), Some(4), vec![], vec![])
}

// ── Deck building ────────────────────────────────────────────────────

fn build_red_deck(game: &mut GameState, owner: PlayerId) {
    for _ in 0..17 { let c = game.create_card(make_mountain(owner)); game.move_card(c, ZoneType::Library, owner); }
    for _ in 0..4 { let c = game.create_card(make_lightning_bolt(owner)); game.move_card(c, ZoneType::Library, owner); }
    for _ in 0..4 { let c = game.create_card(make_grey_ogre(owner)); game.move_card(c, ZoneType::Library, owner); }
    for _ in 0..4 { let c = game.create_card(make_hill_giant(owner)); game.move_card(c, ZoneType::Library, owner); }
}

fn build_green_deck(game: &mut GameState, owner: PlayerId) {
    for _ in 0..17 { let c = game.create_card(make_forest(owner)); game.move_card(c, ZoneType::Library, owner); }
    for _ in 0..4 { let c = game.create_card(make_grizzly_bears(owner)); game.move_card(c, ZoneType::Library, owner); }
    for _ in 0..4 { let c = game.create_card(make_centaur_courser(owner)); game.move_card(c, ZoneType::Library, owner); }
    for _ in 0..4 { let c = game.create_card(make_craw_wurm(owner)); game.move_card(c, ZoneType::Library, owner); }
}

// ── Main ─────────────────────────────────────────────────────────────

fn main() {
    println!("{}{}", BOLD, CYAN);
    println!(r"  +=========================================+");
    println!(r"  |     FORGE ENGINE  -  MTG CLI            |");
    println!(r"  |     Vanilla Creatures + Burn            |");
    println!(r"  +=========================================+");
    println!("{}", RESET);

    println!("  Choose your deck:");
    println!("    {}{}1{}: Red Burn (Mountains + Lightning Bolts + Ogres + Hill Giants)", RED, BOLD, RESET);
    println!("    {}{}2{}: Green Stompy (Forests + Grizzly Bears + Centaurs + Craw Wurms)", GREEN, BOLD, RESET);
    print!("{}> {}", CYAN, RESET);
    let deck_choice = read_line();

    let player_deck_is_red = deck_choice != "2";

    let mut game = GameState::new(&["You", "AI Opponent"], 20);
    let p0 = PlayerId(0);
    let p1 = PlayerId(1);

    if player_deck_is_red {
        build_red_deck(&mut game, p0);
        build_green_deck(&mut game, p1);
        println!("\n  {}You are playing Red Burn.{}", RED, RESET);
        println!("  {}AI is playing Green Stompy.{}\n", GREEN, RESET);
    } else {
        build_green_deck(&mut game, p0);
        build_red_deck(&mut game, p1);
        println!("\n  {}You are playing Green Stompy.{}", GREEN, RESET);
        println!("  {}AI is playing Red Burn.{}\n", RED, RESET);
    }

    let mut game_loop = GameLoop::new(2);
    let mut rng = rand::rngs::StdRng::from_entropy();

    // Shuffle and draw
    game_loop.setup(&mut game, &mut rng);

    // The InteractiveAgent holds a raw pointer to the game state.
    // We update it before each turn. This is safe because:
    // 1. GameState lives on the stack in main() and outlives all agent calls
    // 2. We only read through the pointer during agent callbacks
    // 3. The game_loop borrows &mut game, but the agent only reads
    //    through the pointer during its own callback (not simultaneously with mutation)
    let human = InteractiveAgent::new();
    let ai = SimpleAiAgent;

    let mut agents: Vec<Box<dyn PlayerAgent>> = vec![Box::new(human), Box::new(ai)];

    println!("  {}Game Start! Good luck.{}", BOLD, RESET);
    println!("  {}(Type a number to choose, 'p' to pass, 'all'/'n' for attackers){}\n", DIM, RESET);

    // Game loop — update the raw pointer before each turn
    while !game.game_over && game.turn.turn_number <= 50 {
        // Update the pointer so the interactive agent can read game state
        // Safety: game is alive for the entire loop, and the pointer is only
        // dereferenced during synchronous agent callbacks within run_turn.
        let game_ptr = &game as *const GameState;
        // We need to reach into the boxed agent to set the pointer
        // Since Box<dyn PlayerAgent> erases the type, we use a helper
        set_game_ptr(&mut agents[0], game_ptr);

        let active = game.active_player();
        if active == p0 {
            println!("\n{}{}--- Turn {} - Your Turn ---{}",
                BOLD, MAGENTA, game.turn.turn_number, RESET);
        } else {
            println!("\n{}{}--- Turn {} - AI's Turn ---{}",
                BOLD, DIM, game.turn.turn_number, RESET);
        }

        game_loop.run_turn(&mut game, &mut agents, &mut rng);
    }

    // Game over
    println!("\n{}================================================================{}", BOLD, RESET);
    if let Some(winner) = game.winner {
        if winner == p0 {
            println!("  {}{} YOU WIN! {} Congratulations!", BOLD, GREEN, RESET);
        } else {
            println!("  {}{} YOU LOSE! {} {} wins.", BOLD, RED, RESET, game.player(winner).name);
        }
    } else {
        println!("  {}Draw -- game reached turn limit.{}", YELLOW, RESET);
    }
    println!("  Final life: You = {}, AI = {}", game.player(p0).life, game.player(p1).life);
    println!("{}================================================================{}", BOLD, RESET);
}

/// Set the game state pointer on the InteractiveAgent inside a Box<dyn PlayerAgent>.
/// This relies on InteractiveAgent being the first field layout of the trait object.
fn set_game_ptr(agent: &mut Box<dyn PlayerAgent>, ptr: *const GameState) {
    // We know agents[0] is an InteractiveAgent. Use the trait object's data pointer.
    let data_ptr = &mut **agent as *mut dyn PlayerAgent as *mut InteractiveAgent;
    unsafe {
        (*data_ptr).game_ptr = ptr;
    }
}
