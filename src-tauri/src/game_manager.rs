use std::collections::BTreeMap;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;

use forge_engine_core::agent::PlayerAgent;
use forge_engine_core::card::CardInstance;
use forge_engine_core::game::GameState;
use forge_engine_core::game_loop::GameLoop;
use forge_engine_core::ids::{CardId, PlayerId};
use forge_engine_core::trigger::parse_trigger;
use forge_foundation::{CardTypeLine, ColorSet, ManaCost, ZoneType};
use rand::SeedableRng;
use tauri::{AppHandle, Emitter, Manager};

use crate::ai_agent::SimpleAiAgent;
use crate::game_view_dto::GameViewDto;
use crate::prompt::{AgentPrompt, PlayerAction};
use crate::tauri_agent::TauriAgent;

pub struct GameManager {
    pub session: Mutex<Option<GameSession>>,
    pub latest_prompt: Arc<Mutex<Option<AgentPrompt>>>,
}

pub struct GameSession {
    pub game_id: String,
    pub response_tx: mpsc::Sender<PlayerAction>,
    pub thread_handle: Option<thread::JoinHandle<()>>,
}

impl GameManager {
    pub fn new() -> Self {
        Self {
            session: Mutex::new(None),
            latest_prompt: Arc::new(Mutex::new(None)),
        }
    }

    pub fn get_latest_prompt(&self) -> Option<AgentPrompt> {
        self.latest_prompt.lock().ok().and_then(|g| g.clone())
    }

    pub fn start_game(&self, app: AppHandle, deck_choice: &str) -> Result<String, String> {
        let mut session_guard = self.session.lock().map_err(|e| e.to_string())?;

        // End existing session if any
        if let Some(old) = session_guard.take() {
            drop(old.response_tx); // signal game thread to stop (recv returns Err)
            // Don't join — let the old thread wind down while the new game starts
        }
        // Clear any stale prompt from the previous game
        if let Ok(mut lp) = self.latest_prompt.lock() {
            *lp = None;
        }

        let game_id = format!("game-{}", uuid_simple());
        let game_id_clone = game_id.clone();
        let deck = deck_choice.to_string();

        // Channels
        let (prompt_tx, prompt_rx) = mpsc::channel::<AgentPrompt>();
        let (response_tx, response_rx) = mpsc::channel::<PlayerAction>();
        let (notify_tx, notify_rx) = mpsc::channel::<String>();

        let response_tx_clone = response_tx.clone();

        // Prompt forwarder thread: reads prompts, stores latest, and emits Tauri events
        let app_prompt = app.clone();
        let latest_prompt = self.latest_prompt.clone();
        thread::spawn(move || {
            eprintln!("[prompt_fwd] Prompt forwarder started");
            let window = app_prompt.get_webview_window("main");
            while let Ok(prompt) = prompt_rx.recv() {
                eprintln!("[prompt_fwd] Got prompt, storing and emitting...");
                // Store latest prompt for polling
                if let Ok(mut lp) = latest_prompt.lock() {
                    *lp = Some(prompt.clone());
                }
                // Emit via both window and app to cover all listener types
                if let Some(ref w) = window {
                    let _ = w.emit("game:prompt", &prompt);
                }
                match app_prompt.emit("game:prompt", &prompt) {
                    Ok(()) => eprintln!("[prompt_fwd] Event emitted OK"),
                    Err(e) => eprintln!("[prompt_fwd] Event emit FAILED: {}", e),
                }
            }
            eprintln!("[prompt_fwd] Prompt forwarder ended (channel closed)");
        });

        // Notify forwarder thread
        let app_notify = app.clone();
        thread::spawn(move || {
            let window = app_notify.get_webview_window("main");
            while let Ok(msg) = notify_rx.recv() {
                let _ = if let Some(ref w) = window {
                    w.emit("game:log", &msg)
                } else {
                    app_notify.emit("game:log", &msg)
                };
            }
        });

        // Game thread
        let handle = thread::spawn(move || {
            eprintln!("[game_thread] Starting game: {} with deck: {}", game_id_clone, deck);
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                run_game(game_id_clone.clone(), deck, prompt_tx, response_rx, notify_tx);
            }));
            match result {
                Ok(()) => eprintln!("[game_thread] Game {} finished normally", game_id_clone),
                Err(e) => {
                    let msg = if let Some(s) = e.downcast_ref::<String>() {
                        s.clone()
                    } else if let Some(s) = e.downcast_ref::<&str>() {
                        s.to_string()
                    } else {
                        "Unknown panic".to_string()
                    };
                    eprintln!("[game_thread] PANIC in game {}: {}", game_id_clone, msg);
                }
            }
        });

        *session_guard = Some(GameSession {
            game_id: game_id.clone(),
            response_tx: response_tx_clone,
            thread_handle: Some(handle),
        });

        Ok(game_id)
    }

    pub fn respond(&self, app: AppHandle, action: PlayerAction) -> Result<(), String> {
        if matches!(action, PlayerAction::Concede) {
            // Build a synthetic game-over prompt using the last known game view
            let game_view = {
                let lp = self.latest_prompt.lock().map_err(|e| e.to_string())?;
                let base_view = lp.as_ref().and_then(|p| match p {
                    AgentPrompt::ChooseAction { game_view, .. } => Some(game_view.clone()),
                    AgentPrompt::ChooseAttackers { game_view, .. } => Some(game_view.clone()),
                    AgentPrompt::ChooseBlockers { game_view, .. } => Some(game_view.clone()),
                    AgentPrompt::ChooseTargetPlayer { game_view, .. } => Some(game_view.clone()),
                    AgentPrompt::ChooseTargetCard { game_view, .. } => Some(game_view.clone()),
                    AgentPrompt::ChooseTargetAny { game_view, .. } => Some(game_view.clone()),
                    AgentPrompt::Mulligan { game_view, .. } => Some(game_view.clone()),
                    AgentPrompt::GameOver { game_view } => Some(game_view.clone()),
                });
                let mut view = base_view.unwrap_or_else(|| GameViewDto {
                    game_id: String::new(),
                    turn: 0,
                    step: "main1".into(),
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
                    game_over: false,
                    winner_id: None,
                });
                let opponent_id = view.players.iter().find(|p| !p.is_human).map(|p| p.id.clone());
                view.game_over = true;
                view.winner_id = opponent_id;
                view
            };
            let prompt = AgentPrompt::GameOver { game_view };
            if let Ok(mut lp) = self.latest_prompt.lock() {
                *lp = Some(prompt.clone());
            }
            let _ = app.emit("game:prompt", &prompt);
            // Unblock the game thread with a no-op
            let session_guard = self.session.lock().map_err(|e| e.to_string())?;
            if let Some(session) = session_guard.as_ref() {
                let _ = session.response_tx.send(PlayerAction::PlayCard { card_id: None });
            }
            return Ok(());
        }

        let session_guard = self.session.lock().map_err(|e| e.to_string())?;
        if let Some(session) = session_guard.as_ref() {
            session
                .response_tx
                .send(action)
                .map_err(|e| format!("Game thread not responding: {}", e))?;
            Ok(())
        } else {
            Err("No active game session".into())
        }
    }

    pub fn end_game(&self) -> Result<(), String> {
        let mut session_guard = self.session.lock().map_err(|e| e.to_string())?;
        if let Some(session) = session_guard.take() {
            drop(session.response_tx); // signals game thread to stop
            // Don't join here — let the thread wind down on its own so end_game returns fast
        }
        // Clear latest prompt so the next game doesn't see stale state
        if let Ok(mut lp) = self.latest_prompt.lock() {
            *lp = None;
        }
        Ok(())
    }
}

fn uuid_simple() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    format!("{:08x}{:08x}", rng.gen::<u32>(), rng.gen::<u32>())
}

fn run_game(
    game_id: String,
    deck_choice: String,
    prompt_tx: mpsc::Sender<AgentPrompt>,
    response_rx: mpsc::Receiver<PlayerAction>,
    notify_tx: mpsc::Sender<String>,
) {
    let p0 = PlayerId(0);
    let p1 = PlayerId(1);

    let mut game = GameState::new(&["You", "AI Opponent"], 20);

    // Build decks
    match deck_choice.as_str() {
        "green_stompy" => {
            build_green_stompy_deck(&mut game, p0);
            build_red_burn_deck(&mut game, p1);
        }
        "white_aggro" => {
            build_white_aggro_deck(&mut game, p0);
            build_black_control_deck(&mut game, p1);
        }
        "black_control" => {
            build_black_control_deck(&mut game, p0);
            build_white_aggro_deck(&mut game, p1);
        }
        _ => {
            // Default: Red Burn
            build_red_burn_deck(&mut game, p0);
            build_green_stompy_deck(&mut game, p1);
        }
    }

    let human = TauriAgent::new(p0, game_id.clone(), prompt_tx.clone(), response_rx, notify_tx);
    let ai = SimpleAiAgent;

    let mut agents: Vec<Box<dyn PlayerAgent>> = vec![Box::new(human), Box::new(ai)];
    let mut game_loop = GameLoop::new(2);
    let mut rng = rand::rngs::StdRng::from_entropy();

    let winner = game_loop.run(&mut game, &mut agents, &mut rng, 50);

    // Send final game-over prompt
    let final_view = GameViewDto::from_engine(&game, &game_loop.mana_pools, p0, &game_id, &[], &[]);
    let _ = prompt_tx.send(AgentPrompt::GameOver {
        game_view: final_view,
    });

    let _ = winner; // winner is also in the game_view
}

// ── Card constructors (copied from forge-cli) ──────────────────────────

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

fn make_plains(owner: PlayerId) -> CardInstance {
    CardInstance::new(CardId(0), "Plains".to_string(), owner,
        CardTypeLine::parse("Basic Land - Plains"),
        ManaCost::no_cost(), ColorSet::COLORLESS, None, None, vec![], vec![])
}

fn make_island(owner: PlayerId) -> CardInstance {
    CardInstance::new(CardId(0), "Island".to_string(), owner,
        CardTypeLine::parse("Basic Land - Island"),
        ManaCost::no_cost(), ColorSet::COLORLESS, None, None, vec![], vec![])
}

fn make_swamp(owner: PlayerId) -> CardInstance {
    CardInstance::new(CardId(0), "Swamp".to_string(), owner,
        CardTypeLine::parse("Basic Land - Swamp"),
        ManaCost::no_cost(), ColorSet::COLORLESS, None, None, vec![], vec![])
}

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

fn make_giant_growth(owner: PlayerId) -> CardInstance {
    CardInstance::new(CardId(0), "Giant Growth".to_string(), owner,
        CardTypeLine::parse("Instant"),
        ManaCost::parse("G"), ColorSet::GREEN, None, None, vec![],
        vec!["SP$ Pump | ValidTgts$ Creature | NumAtt$ 3 | NumDef$ 3 | SpellDescription$ Target creature gets +3/+3 until end of turn.".to_string()])
}

fn make_doom_blade(owner: PlayerId) -> CardInstance {
    CardInstance::new(CardId(0), "Doom Blade".to_string(), owner,
        CardTypeLine::parse("Instant"),
        ManaCost::parse("1 B"), ColorSet::BLACK, None, None, vec![],
        vec!["SP$ Destroy | ValidTgts$ Creature.nonBlack | SpellDescription$ Destroy target nonblack creature.".to_string()])
}

fn make_divination(owner: PlayerId) -> CardInstance {
    CardInstance::new(CardId(0), "Divination".to_string(), owner,
        CardTypeLine::parse("Sorcery"),
        ManaCost::parse("2 U"), ColorSet::BLUE, None, None, vec![],
        vec!["SP$ Draw | NumCards$ 2 | SpellDescription$ Draw two cards.".to_string()])
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

fn make_grizzly_bears(owner: PlayerId) -> CardInstance {
    CardInstance::new(CardId(0), "Grizzly Bears".to_string(), owner,
        CardTypeLine::parse("Creature - Bear"),
        ManaCost::parse("1 G"), ColorSet::GREEN, Some(2), Some(2), vec![], vec![])
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

fn make_garruks_companion(owner: PlayerId) -> CardInstance {
    CardInstance::new(CardId(0), "Garruk's Companion".to_string(), owner,
        CardTypeLine::parse("Creature - Beast"),
        ManaCost::parse("G G"), ColorSet::GREEN, Some(3), Some(2),
        vec!["Trample".to_string()], vec![])
}

fn make_giant_spider(owner: PlayerId) -> CardInstance {
    CardInstance::new(CardId(0), "Giant Spider".to_string(), owner,
        CardTypeLine::parse("Creature - Spider"),
        ManaCost::parse("3 G"), ColorSet::GREEN, Some(2), Some(4),
        vec!["Reach".to_string()], vec![])
}

fn make_savannah_lions(owner: PlayerId) -> CardInstance {
    CardInstance::new(CardId(0), "Savannah Lions".to_string(), owner,
        CardTypeLine::parse("Creature - Cat"),
        ManaCost::parse("W"), ColorSet::WHITE, Some(2), Some(1), vec![], vec![])
}

fn make_white_knight(owner: PlayerId) -> CardInstance {
    CardInstance::new(CardId(0), "White Knight".to_string(), owner,
        CardTypeLine::parse("Creature - Human Knight"),
        ManaCost::parse("W W"), ColorSet::WHITE, Some(2), Some(2),
        vec!["First Strike".to_string()], vec![])
}

fn make_serra_angel(owner: PlayerId) -> CardInstance {
    CardInstance::new(CardId(0), "Serra Angel".to_string(), owner,
        CardTypeLine::parse("Creature - Angel"),
        ManaCost::parse("3 W W"), ColorSet::WHITE, Some(4), Some(4),
        vec!["Flying".to_string(), "Vigilance".to_string()], vec![])
}

fn make_typhoid_rats(owner: PlayerId) -> CardInstance {
    CardInstance::new(CardId(0), "Typhoid Rats".to_string(), owner,
        CardTypeLine::parse("Creature - Rat"),
        ManaCost::parse("B"), ColorSet::BLACK, Some(1), Some(1),
        vec!["Deathtouch".to_string()], vec![])
}

fn make_vampire_nighthawk(owner: PlayerId) -> CardInstance {
    CardInstance::new(CardId(0), "Vampire Nighthawk".to_string(), owner,
        CardTypeLine::parse("Creature - Vampire Shaman"),
        ManaCost::parse("1 B B"), ColorSet::BLACK, Some(2), Some(3),
        vec!["Flying".to_string(), "Deathtouch".to_string(), "Lifelink".to_string()], vec![])
}

fn make_wall_of_ice(owner: PlayerId) -> CardInstance {
    CardInstance::new(CardId(0), "Wall of Ice".to_string(), owner,
        CardTypeLine::parse("Creature - Wall"),
        ManaCost::parse("2 G"), ColorSet::GREEN, Some(0), Some(7),
        vec!["Defender".to_string()], vec![])
}

fn make_guttersnipe(owner: PlayerId) -> CardInstance {
    let mut next_id = 0;
    let trigger = parse_trigger(
        "Mode$ SpellCast | ValidCard$ Instant,Sorcery | ValidActivatingPlayer$ You | Execute$ TrigDmg | TriggerDescription$ Whenever you cast an instant or sorcery spell, Guttersnipe deals 2 damage to each opponent.",
        &mut next_id,
    ).unwrap();

    let mut svars = BTreeMap::new();
    svars.insert("TrigDmg".to_string(), "DB$ DealDamage | Defined$ Opponent | NumDmg$ 2".to_string());

    let mut card = CardInstance::new(CardId(0), "Guttersnipe".to_string(), owner,
        CardTypeLine::parse("Creature - Goblin Shaman"),
        ManaCost::parse("2 R"), ColorSet::RED, Some(2), Some(2),
        vec![], vec![]);
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
    svars.insert("TrigGain".to_string(), "DB$ GainLife | Defined$ You | LifeAmount$ 1".to_string());

    let mut card = CardInstance::new(CardId(0), "Soul Warden".to_string(), owner,
        CardTypeLine::parse("Creature - Human Cleric"),
        ManaCost::parse("W"), ColorSet::WHITE, Some(1), Some(1),
        vec![], vec![]);
    card.triggers = vec![trigger];
    card.svars = svars;
    card
}

fn make_mulldrifter(owner: PlayerId) -> CardInstance {
    let mut next_id = 0;
    let trigger = parse_trigger(
        "Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigDraw | TriggerDescription$ When Mulldrifter enters the battlefield, draw two cards.",
        &mut next_id,
    ).unwrap();

    let mut svars = BTreeMap::new();
    svars.insert("TrigDraw".to_string(), "DB$ Draw | Defined$ You | NumCards$ 2".to_string());

    let mut card = CardInstance::new(CardId(0), "Mulldrifter".to_string(), owner,
        CardTypeLine::parse("Creature - Elemental"),
        ManaCost::parse("4 U"), ColorSet::BLUE, Some(2), Some(2),
        vec!["Flying".to_string()], vec![]);
    card.triggers = vec![trigger];
    card.svars = svars;
    card
}

// ── Deck builders ──────────────────────────────────────────────────

fn add_cards(game: &mut GameState, owner: PlayerId, count: usize, make: fn(PlayerId) -> CardInstance) {
    for _ in 0..count {
        let c = game.create_card(make(owner));
        game.move_card(c, ZoneType::Library, owner);
    }
}

fn build_red_burn_deck(game: &mut GameState, owner: PlayerId) {
    add_cards(game, owner, 17, make_mountain);
    add_cards(game, owner, 4, make_lightning_bolt);
    add_cards(game, owner, 4, make_shock);
    add_cards(game, owner, 3, make_grey_ogre);
    add_cards(game, owner, 3, make_hill_giant);
    add_cards(game, owner, 3, make_guttersnipe);
}

fn build_green_stompy_deck(game: &mut GameState, owner: PlayerId) {
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
    add_cards(game, owner, 17, make_plains);
    add_cards(game, owner, 4, make_savannah_lions);
    add_cards(game, owner, 3, make_white_knight);
    add_cards(game, owner, 3, make_serra_angel);
    add_cards(game, owner, 3, make_soul_warden);
}

fn build_black_control_deck(game: &mut GameState, owner: PlayerId) {
    add_cards(game, owner, 13, make_swamp);
    add_cards(game, owner, 4, make_island);
    add_cards(game, owner, 4, make_doom_blade);
    add_cards(game, owner, 2, make_divination);
    add_cards(game, owner, 3, make_typhoid_rats);
    add_cards(game, owner, 3, make_vampire_nighthawk);
    add_cards(game, owner, 2, make_mulldrifter);
}
