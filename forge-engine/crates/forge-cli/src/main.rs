use std::collections::BTreeMap;
use std::io::{self, Write};

use forge_engine_core::agent::{MainPhaseAction, PlayerAgent, TargetChoice};
use forge_engine_core::card::CardInstance;
use forge_engine_core::game::GameState;
use forge_engine_core::game_loop::GameLoop;
use forge_engine_core::combat::DefenderId;
use forge_engine_core::ids::{CardId, PlayerId};
use forge_engine_core::trigger::parse_trigger;
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
    let count = [
        c.has_red(),
        c.has_green(),
        c.has_blue(),
        c.has_white(),
        c.has_black(),
    ]
    .iter()
    .filter(|&&x| x)
    .count();

    if count == 1 {
        if c.has_red() {
            return RED;
        }
        if c.has_green() {
            return GREEN;
        }
        if c.has_blue() {
            return BLUE;
        }
        if c.has_white() {
            return YELLOW;
        }
        if c.has_black() {
            return MAGENTA;
        }
    }
    if card.is_land() {
        DIM
    } else {
        WHITE
    }
}

fn format_keywords(card: &CardInstance) -> String {
    if card.keywords.is_empty() {
        String::new()
    } else {
        format!(" {{{}}}", card.keywords.join(", "))
    }
}

fn format_triggers(card: &CardInstance) -> String {
    if card.triggers.is_empty() {
        return String::new();
    }
    let descs: Vec<&str> = card
        .triggers
        .iter()
        .filter(|t| !t.description.is_empty())
        .map(|t| t.description.as_str())
        .collect();
    if descs.is_empty() {
        return String::new();
    }
    format!(" <{}>", descs.join("; "))
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
        let kw = format_keywords(card);
        let trig = format_triggers(card);
        format!(
            "{}{}{} {}/{}{}{}{}{}",
            color,
            card.card_name,
            tapped,
            card.power(),
            card.toughness(),
            kw,
            trig,
            sick,
            RESET
        )
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
        let kw = format_keywords(card);
        format!(
            "{}{} {}/{}{} [{}]{}",
            color,
            card.card_name,
            card.power(),
            card.toughness(),
            kw,
            card.mana_cost,
            RESET
        )
    } else {
        format!("{}{} [{}]{}", color, card.card_name, card.mana_cost, RESET)
    }
}

// ── Display game state ───────────────────────────────────────────────

fn display_board(game: &GameState, you: PlayerId, opp: PlayerId) {
    let opp_state = game.player(opp);
    let you_state = game.player(you);

    println!();
    println!(
        "{}================================================================{}",
        BOLD, RESET
    );

    // Opponent
    println!(
        "  {}{}{}{} Life: {}{}{}  |  Hand: {}  Library: {}  GY: {}",
        RED,
        BOLD,
        opp_state.name,
        RESET,
        BOLD,
        opp_state.life,
        RESET,
        game.zone(ZoneType::Hand, opp).len(),
        game.zone(ZoneType::Library, opp).len(),
        game.zone(ZoneType::Graveyard, opp).len(),
    );

    let opp_bf = game.cards_in_zone(ZoneType::Battlefield, opp);
    if !opp_bf.is_empty() {
        let cards: Vec<String> = opp_bf
            .iter()
            .map(|&cid| format_card(game.card(cid)))
            .collect();
        println!("  Battlefield: {}", cards.join(", "));
    } else {
        println!("  Battlefield: {}(empty){}", DIM, RESET);
    }

    println!(
        "{}  ----------------------------------------------------------{}",
        DIM, RESET
    );

    // You
    let your_bf = game.cards_in_zone(ZoneType::Battlefield, you);
    if !your_bf.is_empty() {
        let cards: Vec<String> = your_bf
            .iter()
            .map(|&cid| format_card(game.card(cid)))
            .collect();
        println!("  Battlefield: {}", cards.join(", "));
    } else {
        println!("  Battlefield: {}(empty){}", DIM, RESET);
    }

    println!(
        "  {}{}{}{} Life: {}{}{}  |  Library: {}  GY: {}",
        GREEN,
        BOLD,
        you_state.name,
        RESET,
        BOLD,
        you_state.life,
        RESET,
        game.zone(ZoneType::Library, you).len(),
        game.zone(ZoneType::Graveyard, you).len(),
    );

    let hand = game.cards_in_zone(ZoneType::Hand, you);
    if !hand.is_empty() {
        let cards: Vec<String> = hand
            .iter()
            .map(|&cid| format_card_with_cost(game.card(cid)))
            .collect();
        println!("  Hand: {}", cards.join(", "));
    } else {
        println!("  Hand: {}(empty){}", DIM, RESET);
    }

    println!(
        "  {}Turn {} | Phase: {:?} | Active: {}{}",
        DIM,
        game.turn.turn_number,
        game.turn.phase,
        game.player(game.active_player()).name,
        RESET,
    );
    println!(
        "{}================================================================{}",
        BOLD, RESET
    );
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
    fn mulligan_decision(&mut self, _player: PlayerId, hand: &[CardId], mulligan_count: u32) -> bool {
        let game = self.game();
        if mulligan_count > 0 {
            println!(
                "\n{}{}Opening Hand (mulligan #{} — must put {} card{} back):{}", CYAN, BOLD,
                mulligan_count, mulligan_count,
                if mulligan_count == 1 { "" } else { "s" }, RESET
            );
        } else {
            println!("\n{}{}Opening Hand:{}", CYAN, BOLD, RESET);
        }
        for (i, &cid) in hand.iter().enumerate() {
            println!("  {}: {}", i, format_card_with_cost(game.card(cid)));
        }
        print!("{}Keep this hand? (y/n): {}", CYAN, RESET);
        let input = read_line();
        input != "n" && input != "no"
    }

    fn choose_cards_to_bottom(&mut self, _player: PlayerId, hand: &[CardId], count: usize) -> Vec<CardId> {
        let game = self.game();
        println!(
            "\n{}{}Choose {} card{} to put on the bottom of your library:{}",
            CYAN, BOLD, count,
            if count == 1 { "" } else { "s" }, RESET
        );
        for (i, &cid) in hand.iter().enumerate() {
            println!("  {}: {}", i, format_card_with_cost(game.card(cid)));
        }
        let mut chosen = Vec::new();
        while chosen.len() < count {
            print!(
                "{}Card {} of {} (enter index): {}",
                CYAN,
                chosen.len() + 1,
                count,
                RESET
            );
            let input = read_line();
            if let Ok(idx) = input.trim().parse::<usize>() {
                if idx < hand.len() && !chosen.contains(&hand[idx]) {
                    chosen.push(hand[idx]);
                } else {
                    println!("  {}Invalid or duplicate choice.{}", RED, RESET);
                }
            } else {
                println!("  {}Enter a number.{}", RED, RESET);
            }
        }
        chosen
    }

    fn choose_action(
        &mut self,
        player: PlayerId,
        playable: &[CardId],
        tappable_lands: &[CardId],
        untappable_lands: &[CardId],
        activatable: &[(CardId, usize)],
    ) -> MainPhaseAction {
        let game = self.game();

        let opp = game.opponent_of(player);
        display_board(game, player, opp);

        if playable.is_empty()
            && tappable_lands.is_empty()
            && untappable_lands.is_empty()
            && activatable.is_empty()
        {
            println!("  {}No actions available.{}", DIM, RESET);
            return MainPhaseAction::Pass;
        }

        // Build a unified action list: untap lands, tap lands, play cards, activate abilities
        let mut actions: Vec<MainPhaseAction> = Vec::new();
        println!("\n{}{}Available actions:{}", CYAN, BOLD, RESET);
        for &cid in untappable_lands {
            let card = game.card(cid);
            println!(
                "  {}{}{}:  Untap {} (undo mana)",
                BOLD,
                actions.len(),
                RESET,
                card.card_name
            );
            actions.push(MainPhaseAction::UntapMana(cid));
        }
        for &cid in tappable_lands {
            let card = game.card(cid);
            println!(
                "  {}{}{}:  Tap {} (add mana)",
                BOLD,
                actions.len(),
                RESET,
                card.card_name
            );
            actions.push(MainPhaseAction::ActivateMana(cid));
        }
        for &cid in playable {
            let card = game.card(cid);
            let verb = if card.is_land() { "Play" } else { "Cast" };
            println!(
                "  {}{}{}: {} {}",
                BOLD,
                actions.len(),
                RESET,
                verb,
                format_card_with_cost(card)
            );
            actions.push(MainPhaseAction::Play(cid));
        }
        for &(cid, ab_idx) in activatable {
            let card = game.card(cid);
            let desc = card
                .activated_abilities
                .iter()
                .find(|a| a.ability_index == ab_idx)
                .and_then(|a| a.params.get("SpellDescription"))
                .map(|s| s.as_str())
                .unwrap_or("Activate ability");
            println!(
                "  {}{}{}: Activate {} - {}",
                BOLD,
                actions.len(),
                RESET,
                card.card_name,
                desc
            );
            actions.push(MainPhaseAction::ActivateAbility(cid, ab_idx));
        }
        println!("  {}(enter number to act, or 'p' to pass){}", DIM, RESET);

        read_number(&format!("{}> {}", CYAN, RESET), actions.len())
            .map(|idx| actions[idx])
            .unwrap_or(MainPhaseAction::Pass)
    }

    fn choose_attackers(&mut self, _player: PlayerId, available: &[CardId], possible_defenders: &[DefenderId]) -> Vec<(CardId, DefenderId)> {
        if available.is_empty() {
            return Vec::new();
        }

        let game = self.game();
        println!("\n{}{}Declare Attackers:{}", CYAN, BOLD, RESET);
        for (i, &cid) in available.iter().enumerate() {
            let card = game.card(cid);
            let kw = format_keywords(card);
            println!(
                "  {}{}{}: {} {}/{}{}",
                BOLD,
                i,
                RESET,
                card.card_name,
                card.power(),
                card.toughness(),
                kw
            );
        }
        println!(
            "  {}(numbers separated by spaces, 'all', or 'n' for none){}",
            DIM, RESET
        );

        let indices = read_numbers(&format!("{}Attack> {}", CYAN, RESET), available.len());
        indices.into_iter().map(|i| (available[i], possible_defenders[0])).collect()
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
            let kw = format_keywords(card);
            println!(
                "  {}{}[A{}]{}: {} {}/{}{}",
                RED,
                BOLD,
                i,
                RESET,
                card.card_name,
                card.power(),
                card.toughness(),
                kw
            );
        }

        println!("{}{}Your Blockers:{}", GREEN, BOLD, RESET);
        for (i, &cid) in available_blockers.iter().enumerate() {
            let card = game.card(cid);
            let kw = format_keywords(card);
            println!(
                "  {}{}[B{}]{}: {} {}/{}{}",
                GREEN,
                BOLD,
                i,
                RESET,
                card.card_name,
                card.power(),
                card.toughness(),
                kw
            );
        }

        println!(
            "  {}Enter block assignments as 'B0=A0 B1=A0' or 'n' for no blocks{}",
            DIM, RESET
        );
        print!("{}Block> {}", CYAN, RESET);
        let input = read_line();

        if input.is_empty() || input == "n" || input == "none" {
            return Vec::new();
        }

        let mut blocks = Vec::new();
        for assignment in input.split_whitespace() {
            let parts: Vec<&str> = assignment.split('=').collect();
            if parts.len() == 2 {
                let b_idx = parts[0]
                    .trim_start_matches(|c: char| c.is_alphabetic())
                    .parse::<usize>();
                let a_idx = parts[1]
                    .trim_start_matches(|c: char| c.is_alphabetic())
                    .parse::<usize>();
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
            println!(
                "  {}Targeting {} (life: {}){}",
                CYAN, target.name, target.life, RESET
            );
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

    fn choose_target_any(
        &mut self,
        _player: PlayerId,
        valid_players: &[PlayerId],
        valid_cards: &[CardId],
    ) -> TargetChoice {
        let game = self.game();
        let total = valid_players.len() + valid_cards.len();
        if total == 0 {
            return TargetChoice::None;
        }

        println!(
            "\n{}{}Choose target (player or creature):{}",
            CYAN, BOLD, RESET
        );
        let mut idx = 0;
        for &pid in valid_players {
            let p = game.player(pid);
            println!("  {}{}{}: {} (life: {})", BOLD, idx, RESET, p.name, p.life);
            idx += 1;
        }
        for &cid in valid_cards {
            println!(
                "  {}{}{}: {}",
                BOLD,
                idx,
                RESET,
                format_card(game.card(cid))
            );
            idx += 1;
        }

        match read_number(&format!("{}Target> {}", CYAN, RESET), total) {
            Some(i) if i < valid_players.len() => TargetChoice::Player(valid_players[i]),
            Some(i) => TargetChoice::Card(valid_cards[i - valid_players.len()]),
            None => {
                // Default to first player if available
                if let Some(&pid) = valid_players.first() {
                    TargetChoice::Player(pid)
                } else if let Some(&cid) = valid_cards.first() {
                    TargetChoice::Card(cid)
                } else {
                    TargetChoice::None
                }
            }
        }
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
    fn mulligan_decision(&mut self, _: PlayerId, _: &[CardId], _: u32) -> bool {
        true
    }

    fn choose_action(
        &mut self,
        _: PlayerId,
        playable: &[CardId],
        _: &[CardId],
        _: &[CardId],
        _: &[(CardId, usize)],
    ) -> MainPhaseAction {
        playable
            .first()
            .copied()
            .map(MainPhaseAction::Play)
            .unwrap_or(MainPhaseAction::Pass)
    }

    fn choose_attackers(&mut self, _: PlayerId, available: &[CardId], possible_defenders: &[DefenderId]) -> Vec<(CardId, DefenderId)> {
        available.iter().map(|&a| (a, possible_defenders[0])).collect()
    }

    fn choose_blockers(
        &mut self,
        _: PlayerId,
        attackers: &[CardId],
        available_blockers: &[CardId],
    ) -> Vec<(CardId, CardId)> {
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

    fn choose_target_any(
        &mut self,
        _player: PlayerId,
        valid_players: &[PlayerId],
        valid_cards: &[CardId],
    ) -> TargetChoice {
        // AI prefers targeting creatures if available, else players
        if let Some(&cid) = valid_cards.first() {
            TargetChoice::Card(cid)
        } else if let Some(&pid) = valid_players.first() {
            TargetChoice::Player(pid)
        } else {
            TargetChoice::None
        }
    }

    fn choose_land_or_spell(&mut self, _: PlayerId) -> Option<bool> {
        None
    }

    fn notify(&mut self, message: &str) {
        println!("  {}>> [AI] {}{}", RED, message, RESET);
    }
}

// ── Card constructors ────────────────────────────────────────────────

// -- Lands --

fn make_mountain(owner: PlayerId) -> CardInstance {
    CardInstance::new(
        CardId(0),
        "Mountain".to_string(),
        owner,
        CardTypeLine::parse("Basic Land - Mountain"),
        ManaCost::no_cost(),
        ColorSet::COLORLESS,
        None,
        None,
        vec![],
        vec![],
    )
}

fn make_forest(owner: PlayerId) -> CardInstance {
    CardInstance::new(
        CardId(0),
        "Forest".to_string(),
        owner,
        CardTypeLine::parse("Basic Land - Forest"),
        ManaCost::no_cost(),
        ColorSet::COLORLESS,
        None,
        None,
        vec![],
        vec![],
    )
}

fn make_plains(owner: PlayerId) -> CardInstance {
    CardInstance::new(
        CardId(0),
        "Plains".to_string(),
        owner,
        CardTypeLine::parse("Basic Land - Plains"),
        ManaCost::no_cost(),
        ColorSet::COLORLESS,
        None,
        None,
        vec![],
        vec![],
    )
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

fn make_swamp(owner: PlayerId) -> CardInstance {
    CardInstance::new(
        CardId(0),
        "Swamp".to_string(),
        owner,
        CardTypeLine::parse("Basic Land - Swamp"),
        ManaCost::no_cost(),
        ColorSet::COLORLESS,
        None,
        None,
        vec![],
        vec![],
    )
}

// -- Red spells --

fn make_lightning_bolt(owner: PlayerId) -> CardInstance {
    CardInstance::new(CardId(0), "Lightning Bolt".to_string(), owner,
        CardTypeLine::parse("Instant"),
        ManaCost::parse("R"), ColorSet::RED, None, None, vec![],
        vec!["SP$ DealDamage | ValidTgts$ Any | NumDmg$ 3 | SpellDescription$ CARDNAME deals 3 damage to any target.".to_string()])
}

fn make_shock(owner: PlayerId) -> CardInstance {
    CardInstance::new(CardId(0), "Shock".to_string(), owner,
        CardTypeLine::parse("Instant"),
        ManaCost::parse("R"), ColorSet::RED, None, None, vec![],
        vec!["SP$ DealDamage | ValidTgts$ Any | NumDmg$ 2 | SpellDescription$ CARDNAME deals 2 damage to any target.".to_string()])
}

// -- Green spells --

fn make_giant_growth(owner: PlayerId) -> CardInstance {
    CardInstance::new(CardId(0), "Giant Growth".to_string(), owner,
        CardTypeLine::parse("Instant"),
        ManaCost::parse("G"), ColorSet::GREEN, None, None, vec![],
        vec!["SP$ Pump | ValidTgts$ Creature | NumAtt$ 3 | NumDef$ 3 | SpellDescription$ Target creature gets +3/+3 until end of turn.".to_string()])
}

// -- Black spells --

fn make_doom_blade(owner: PlayerId) -> CardInstance {
    CardInstance::new(CardId(0), "Doom Blade".to_string(), owner,
        CardTypeLine::parse("Instant"),
        ManaCost::parse("1 B"), ColorSet::BLACK, None, None, vec![],
        vec!["SP$ Destroy | ValidTgts$ Creature.nonBlack | SpellDescription$ Destroy target nonblack creature.".to_string()])
}

// -- Blue spells --

fn make_divination(owner: PlayerId) -> CardInstance {
    CardInstance::new(
        CardId(0),
        "Divination".to_string(),
        owner,
        CardTypeLine::parse("Sorcery"),
        ManaCost::parse("2 U"),
        ColorSet::BLUE,
        None,
        None,
        vec![],
        vec!["SP$ Draw | NumCards$ 2 | SpellDescription$ Draw two cards.".to_string()],
    )
}

// -- Red creatures --

fn make_grey_ogre(owner: PlayerId) -> CardInstance {
    CardInstance::new(
        CardId(0),
        "Gray Ogre".to_string(),
        owner,
        CardTypeLine::parse("Creature - Ogre"),
        ManaCost::parse("2 R"),
        ColorSet::RED,
        Some(2),
        Some(2),
        vec![],
        vec![],
    )
}

fn make_hill_giant(owner: PlayerId) -> CardInstance {
    CardInstance::new(
        CardId(0),
        "Hill Giant".to_string(),
        owner,
        CardTypeLine::parse("Creature - Giant"),
        ManaCost::parse("3 R"),
        ColorSet::RED,
        Some(3),
        Some(3),
        vec![],
        vec![],
    )
}

// -- Green creatures --

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

fn make_centaur_courser(owner: PlayerId) -> CardInstance {
    CardInstance::new(
        CardId(0),
        "Centaur Courser".to_string(),
        owner,
        CardTypeLine::parse("Creature - Centaur Warrior"),
        ManaCost::parse("2 G"),
        ColorSet::GREEN,
        Some(3),
        Some(3),
        vec![],
        vec![],
    )
}

fn make_craw_wurm(owner: PlayerId) -> CardInstance {
    CardInstance::new(
        CardId(0),
        "Craw Wurm".to_string(),
        owner,
        CardTypeLine::parse("Creature - Wurm"),
        ManaCost::parse("4 G G"),
        ColorSet::GREEN,
        Some(6),
        Some(4),
        vec![],
        vec![],
    )
}

fn make_garruks_companion(owner: PlayerId) -> CardInstance {
    CardInstance::new(
        CardId(0),
        "Garruk's Companion".to_string(),
        owner,
        CardTypeLine::parse("Creature - Beast"),
        ManaCost::parse("G G"),
        ColorSet::GREEN,
        Some(3),
        Some(2),
        vec!["Trample".to_string()],
        vec![],
    )
}

fn make_giant_spider(owner: PlayerId) -> CardInstance {
    CardInstance::new(
        CardId(0),
        "Giant Spider".to_string(),
        owner,
        CardTypeLine::parse("Creature - Spider"),
        ManaCost::parse("3 G"),
        ColorSet::GREEN,
        Some(2),
        Some(4),
        vec!["Reach".to_string()],
        vec![],
    )
}

// -- White creatures --

fn make_savannah_lions(owner: PlayerId) -> CardInstance {
    CardInstance::new(
        CardId(0),
        "Savannah Lions".to_string(),
        owner,
        CardTypeLine::parse("Creature - Cat"),
        ManaCost::parse("W"),
        ColorSet::WHITE,
        Some(2),
        Some(1),
        vec![],
        vec![],
    )
}

fn make_white_knight(owner: PlayerId) -> CardInstance {
    CardInstance::new(
        CardId(0),
        "White Knight".to_string(),
        owner,
        CardTypeLine::parse("Creature - Human Knight"),
        ManaCost::parse("W W"),
        ColorSet::WHITE,
        Some(2),
        Some(2),
        vec!["First Strike".to_string()],
        vec![],
    )
}

fn make_serra_angel(owner: PlayerId) -> CardInstance {
    CardInstance::new(
        CardId(0),
        "Serra Angel".to_string(),
        owner,
        CardTypeLine::parse("Creature - Angel"),
        ManaCost::parse("3 W W"),
        ColorSet::WHITE,
        Some(4),
        Some(4),
        vec!["Flying".to_string(), "Vigilance".to_string()],
        vec![],
    )
}

// -- Black creatures --

fn make_typhoid_rats(owner: PlayerId) -> CardInstance {
    CardInstance::new(
        CardId(0),
        "Typhoid Rats".to_string(),
        owner,
        CardTypeLine::parse("Creature - Rat"),
        ManaCost::parse("B"),
        ColorSet::BLACK,
        Some(1),
        Some(1),
        vec!["Deathtouch".to_string()],
        vec![],
    )
}

fn make_vampire_nighthawk(owner: PlayerId) -> CardInstance {
    CardInstance::new(
        CardId(0),
        "Vampire Nighthawk".to_string(),
        owner,
        CardTypeLine::parse("Creature - Vampire Shaman"),
        ManaCost::parse("1 B B"),
        ColorSet::BLACK,
        Some(2),
        Some(3),
        vec![
            "Flying".to_string(),
            "Deathtouch".to_string(),
            "Lifelink".to_string(),
        ],
        vec![],
    )
}

// -- Defender creatures --

fn make_wall_of_ice(owner: PlayerId) -> CardInstance {
    CardInstance::new(
        CardId(0),
        "Wall of Ice".to_string(),
        owner,
        CardTypeLine::parse("Creature - Wall"),
        ManaCost::parse("2 G"),
        ColorSet::GREEN,
        Some(0),
        Some(7),
        vec!["Defender".to_string()],
        vec![],
    )
}

// -- Trigger creatures --

fn make_mulldrifter(owner: PlayerId) -> CardInstance {
    let mut next_id = 0;
    let trigger = parse_trigger(
        "Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigDraw | TriggerDescription$ When Mulldrifter enters the battlefield, draw two cards.",
        &mut next_id,
    ).unwrap();

    let mut svars = BTreeMap::new();
    svars.insert(
        "TrigDraw".to_string(),
        "DB$ Draw | Defined$ You | NumCards$ 2".to_string(),
    );

    let mut card = CardInstance::new(
        CardId(0),
        "Mulldrifter".to_string(),
        owner,
        CardTypeLine::parse("Creature - Elemental"),
        ManaCost::parse("4 U"),
        ColorSet::BLUE,
        Some(2),
        Some(2),
        vec!["Flying".to_string()],
        vec![],
    );
    card.triggers = vec![trigger];
    card.svars = svars;
    card
}

fn make_soul_warden(owner: PlayerId) -> CardInstance {
    let mut next_id = 0;
    let trigger = parse_trigger(
        "Mode$ ChangesZone | Destination$ Battlefield | ValidCard$ Creature.Other | Execute$ TrigGain | TriggerDescription$ Whenever another creature enters the battlefield, you gain 1 life.",
        &mut next_id,
    ).unwrap();

    let mut svars = BTreeMap::new();
    svars.insert(
        "TrigGain".to_string(),
        "DB$ GainLife | Defined$ You | LifeAmount$ 1".to_string(),
    );

    let mut card = CardInstance::new(
        CardId(0),
        "Soul Warden".to_string(),
        owner,
        CardTypeLine::parse("Creature - Human Cleric"),
        ManaCost::parse("W"),
        ColorSet::WHITE,
        Some(1),
        Some(1),
        vec![],
        vec![],
    );
    card.triggers = vec![trigger];
    card.svars = svars;
    card
}

fn make_guttersnipe(owner: PlayerId) -> CardInstance {
    let mut next_id = 0;
    let trigger = parse_trigger(
        "Mode$ SpellCast | ValidCard$ Instant,Sorcery | ValidActivatingPlayer$ You | Execute$ TrigDmg | TriggerDescription$ Whenever you cast an instant or sorcery spell, Guttersnipe deals 2 damage to each opponent.",
        &mut next_id,
    ).unwrap();

    let mut svars = BTreeMap::new();
    svars.insert(
        "TrigDmg".to_string(),
        "DB$ DealDamage | Defined$ Opponent | NumDmg$ 2".to_string(),
    );

    let mut card = CardInstance::new(
        CardId(0),
        "Guttersnipe".to_string(),
        owner,
        CardTypeLine::parse("Creature - Goblin Shaman"),
        ManaCost::parse("2 R"),
        ColorSet::RED,
        Some(2),
        Some(2),
        vec![],
        vec![],
    );
    card.triggers = vec![trigger];
    card.svars = svars;
    card
}

// ── Deck building ────────────────────────────────────────────────────

fn add_cards(
    game: &mut GameState,
    owner: PlayerId,
    count: usize,
    make: fn(PlayerId) -> CardInstance,
) {
    for _ in 0..count {
        let c = game.create_card(make(owner));
        game.move_card(c, ZoneType::Library, owner);
    }
}

fn build_red_burn_deck(game: &mut GameState, owner: PlayerId) {
    // 17 Mountains, 4 Lightning Bolt, 4 Shock, 3 Gray Ogre, 3 Hill Giant, 3 Guttersnipe = 34
    add_cards(game, owner, 17, make_mountain);
    add_cards(game, owner, 4, make_lightning_bolt);
    add_cards(game, owner, 4, make_shock);
    add_cards(game, owner, 3, make_grey_ogre);
    add_cards(game, owner, 3, make_hill_giant);
    add_cards(game, owner, 3, make_guttersnipe);
}

fn build_green_stompy_deck(game: &mut GameState, owner: PlayerId) {
    // 17 Forests, 4 Giant Growth, 3 Grizzly Bears, 2 Centaur Courser,
    // 3 Garruk's Companion, 2 Giant Spider, 2 Wall of Ice, 2 Craw Wurm = 35
    add_cards(game, owner, 17, make_forest);
    add_cards(game, owner, 4, make_giant_growth);
    add_cards(game, owner, 3, make_grizzly_bears);
    add_cards(game, owner, 2, make_centaur_courser);
    add_cards(game, owner, 3, make_garruks_companion);
    add_cards(game, owner, 2, make_giant_spider);
    add_cards(game, owner, 2, make_wall_of_ice);
    add_cards(game, owner, 2, make_craw_wurm);
}

fn build_white_aggro_deck(game: &mut GameState, owner: PlayerId) {
    // 17 Plains, 4 Savannah Lions, 3 White Knight, 3 Serra Angel, 3 Soul Warden = 30
    add_cards(game, owner, 17, make_plains);
    add_cards(game, owner, 4, make_savannah_lions);
    add_cards(game, owner, 3, make_white_knight);
    add_cards(game, owner, 3, make_serra_angel);
    add_cards(game, owner, 3, make_soul_warden);
}

fn build_black_control_deck(game: &mut GameState, owner: PlayerId) {
    // 13 Swamps, 4 Islands, 4 Doom Blade, 2 Divination,
    // 3 Typhoid Rats, 3 Vampire Nighthawk, 2 Mulldrifter = 31
    add_cards(game, owner, 13, make_swamp);
    add_cards(game, owner, 4, make_island);
    add_cards(game, owner, 4, make_doom_blade);
    add_cards(game, owner, 2, make_divination);
    add_cards(game, owner, 3, make_typhoid_rats);
    add_cards(game, owner, 3, make_vampire_nighthawk);
    add_cards(game, owner, 2, make_mulldrifter);
}

// ── Main ─────────────────────────────────────────────────────────────

fn main() {
    println!("{}{}", BOLD, CYAN);
    println!(r"  +=========================================+");
    println!(r"  |     FORGE ENGINE  -  MTG CLI            |");
    println!(r"  |     Keywords + Targeting + Effects       |");
    println!(r"  +=========================================+");
    println!("{}", RESET);

    println!("  Choose your deck:");
    println!(
        "    {}{}1{}: Red Burn      (Bolts + Shocks + Ogres + Giants)",
        RED, BOLD, RESET
    );
    println!(
        "    {}{}2{}: Green Stompy  (Giant Growth + Trample + Reach + Wurms)",
        GREEN, BOLD, RESET
    );
    println!(
        "    {}{}3{}: White Aggro   (Savannah Lions + First Strike + Flying + Vigilance)",
        YELLOW, BOLD, RESET
    );
    println!(
        "    {}{}4{}: Black Control (Doom Blade + Divination + Deathtouch + Lifelink)",
        MAGENTA, BOLD, RESET
    );
    print!("{}> {}", CYAN, RESET);
    let deck_choice = read_line();

    let mut game = GameState::new(&["You", "AI Opponent"], 20);
    let p0 = PlayerId(0);
    let p1 = PlayerId(1);

    // Build player deck and pick an AI opponent
    match deck_choice.as_str() {
        "2" => {
            build_green_stompy_deck(&mut game, p0);
            build_red_burn_deck(&mut game, p1);
            println!("\n  {}You are playing Green Stompy.{}", GREEN, RESET);
            println!("  {}AI is playing Red Burn.{}\n", RED, RESET);
        }
        "3" => {
            build_white_aggro_deck(&mut game, p0);
            build_black_control_deck(&mut game, p1);
            println!("\n  {}You are playing White Aggro.{}", YELLOW, RESET);
            println!("  {}AI is playing Black Control.{}\n", MAGENTA, RESET);
        }
        "4" => {
            build_black_control_deck(&mut game, p0);
            build_white_aggro_deck(&mut game, p1);
            println!("\n  {}You are playing Black Control.{}", MAGENTA, RESET);
            println!("  {}AI is playing White Aggro.{}\n", YELLOW, RESET);
        }
        _ => {
            // Default: Red Burn
            build_red_burn_deck(&mut game, p0);
            build_green_stompy_deck(&mut game, p1);
            println!("\n  {}You are playing Red Burn.{}", RED, RESET);
            println!("  {}AI is playing Green Stompy.{}\n", GREEN, RESET);
        }
    }

    let mut game_loop = GameLoop::new(2);
    let mut rng = rand::rngs::StdRng::from_entropy();

    let human = InteractiveAgent::new();
    let ai = SimpleAiAgent;
    let mut agents: Vec<Box<dyn PlayerAgent>> = vec![Box::new(human), Box::new(ai)];

    let game_ptr = &game as *const GameState;
    set_game_ptr(&mut agents[0], game_ptr);
    game_loop.setup(&mut game, &mut agents, &mut rng);

    println!("  {}Game Start! Good luck.{}", BOLD, RESET);
    println!(
        "  {}(Type a number to choose, 'p' to pass, 'all'/'n' for attackers){}\n",
        DIM, RESET
    );

    while !game.game_over && game.turn.turn_number <= 50 {
        let game_ptr = &game as *const GameState;
        set_game_ptr(&mut agents[0], game_ptr);

        let active = game.active_player();
        if active == p0 {
            println!(
                "\n{}{}--- Turn {} - Your Turn ---{}",
                BOLD, MAGENTA, game.turn.turn_number, RESET
            );
        } else {
            println!(
                "\n{}{}--- Turn {} - AI's Turn ---{}",
                BOLD, DIM, game.turn.turn_number, RESET
            );
        }

        game_loop.run_turn(&mut game, &mut agents, &mut rng);
    }

    // Game over
    println!(
        "\n{}================================================================{}",
        BOLD, RESET
    );
    if let Some(winner) = game.winner {
        if winner == p0 {
            println!("  {}{} YOU WIN! {} Congratulations!", BOLD, GREEN, RESET);
        } else {
            println!(
                "  {}{} YOU LOSE! {} {} wins.",
                BOLD,
                RED,
                RESET,
                game.player(winner).name
            );
        }
    } else {
        println!("  {}Draw -- game reached turn limit.{}", YELLOW, RESET);
    }
    println!(
        "  Final life: You = {}, AI = {}",
        game.player(p0).life,
        game.player(p1).life
    );
    println!(
        "{}================================================================{}",
        BOLD, RESET
    );
}

/// Set the game state pointer on the InteractiveAgent inside a Box<dyn PlayerAgent>.
fn set_game_ptr(agent: &mut Box<dyn PlayerAgent>, ptr: *const GameState) {
    let data_ptr = &mut **agent as *mut dyn PlayerAgent as *mut InteractiveAgent;
    unsafe {
        (*data_ptr).game_ptr = ptr;
    }
}
