use std::collections::HashMap;
use std::env;
use std::path::PathBuf;
use std::sync::{mpsc as std_mpsc, Once, OnceLock};

use forge_agent_interface::agent_impl::PromptAgent;
use forge_agent_interface::game_view_dto::GameViewDto;
use forge_agent_interface::prompt::{AgentPrompt, AgentPromptInner, PlayerAction};
use forge_agent_interface::simple_ai::spawn_simple_ai_prompt_responder;
use forge_carddb::CardDatabase;
use forge_engine_core::agent::PlayerAgent;
use forge_engine_core::game::GameState;
use forge_engine_core::game_loop::GameLoop;
use forge_engine_core::ids::PlayerId;
use forge_game_runtime::deck::{
    card_rules_to_instance, force_commander_by_name, instantiate_registered_players,
    prepare_registered_player, DeckCardIdentity,
};
use forge_game_runtime::mpsc_transport::MpscTransport as NodeTransport;
use forge_server::protocol::CardIdentity;
use memmap2::Mmap;
use rand::SeedableRng;
use tracing::{info, warn};

use crate::config::workspace_root;

pub fn run_hosted_engine_game(
    game_id: String,
    player_names: Vec<String>,
    deck_lists: Vec<Vec<CardIdentity>>,
    commander_names: Vec<Option<String>>,
    local_player_index: Option<usize>,
    starting_life: i32,
    remote_prompt_tx: std_mpsc::Sender<(usize, AgentPrompt)>,
    remote_response_rxs: Vec<(usize, std_mpsc::Receiver<PlayerAction>)>,
) {
    let num_players = player_names.len();
    let mut prepared_players = Vec::with_capacity(num_players);
    for i in 0..num_players {
        let identities = deck_lists[i]
            .iter()
            .map(|identity| DeckCardIdentity {
                name: identity.name.clone(),
                set_code: identity.set_code.clone(),
                section: None,
            })
            .collect::<Vec<_>>();
        let mut prepared =
            prepare_registered_player(player_names[i].clone(), get_card_db(), &identities);
        prepared.registered.starting_life = starting_life;
        if let Some(ref commander_name) = commander_names[i] {
            if !force_commander_by_name(&mut prepared, commander_name) {
                warn!(commander_name, "commander name not found in selected deck");
            }
        }
        prepared_players.push(prepared);
    }

    let registered = prepared_players
        .iter()
        .map(|player| player.registered.clone())
        .collect::<Vec<_>>();
    let mut game = GameState::new_from_registered_players(&registered);
    instantiate_registered_players(&mut game, prepared_players);

    let local_ai = local_player_index.map(|player_index| {
        let (ai_prompt_tx, ai_prompt_rx) = std_mpsc::channel::<AgentPrompt>();
        let (ai_response_tx, ai_response_rx) = std_mpsc::channel::<PlayerAction>();
        spawn_simple_ai_prompt_responder(ai_prompt_rx, ai_response_tx);
        (
            player_index,
            Box::new(PromptAgent::new(
                PlayerId(player_index as u32),
                game_id.clone(),
                NodeTransport::new_ai(ai_prompt_tx, ai_response_rx),
            )) as Box<dyn PlayerAgent>,
        )
    });

    let mut remote_rx_map: HashMap<usize, std_mpsc::Receiver<PlayerAction>> =
        remote_response_rxs.into_iter().collect();

    let mut local_ai = local_ai;
    let mut agents: Vec<Box<dyn PlayerAgent>> = Vec::with_capacity(num_players);
    for i in 0..num_players {
        if Some(i) == local_player_index {
            agents.push(local_ai.take().expect("missing local ai agent").1);
        } else {
            let response_rx = remote_rx_map
                .remove(&i)
                .expect("missing remote response receiver");
            agents.push(Box::new(PromptAgent::new(
                PlayerId(i as u32),
                game_id.clone(),
                NodeTransport::new_relay(i, remote_prompt_tx.clone(), response_rx),
            )));
        }
    }

    let mut game_loop = GameLoop::new(num_players);
    if std::env::var("FORGE_ENGINE_GAME_LOG").is_err() {
        game_loop.game_log.set_enabled(true);
    }
    game_loop.experimental_restore_snapshot =
        std::env::var("FORGE_ENGINE_RESTORE_SNAPSHOT").is_ok();

    let token_db = get_token_db();
    for (script_name, rules) in token_db.iter() {
        let template = card_rules_to_instance(rules, PlayerId(0));
        game_loop.register_token(script_name.clone(), template);
    }

    let mut rng = rand::rngs::StdRng::from_entropy();
    let _winner = game_loop.run(&mut game, &mut agents, &mut rng, 5000);

    for i in 0..num_players {
        if Some(i) == local_player_index {
            continue;
        }
        let pid = PlayerId(i as u32);
        let game_view =
            GameViewDto::from_engine(&game, &game_loop.mana_pools, pid, &game_id, &[], &[]);
        let _ = remote_prompt_tx.send((
            i,
            AgentPrompt {
                deciding_player_id: format!("player-{i}"),
                display_events: vec![],
                inner: AgentPromptInner::GameOver { game_view },
            },
        ));
    }
}

/// Card and token databases come from a single rkyv archive bundle —
/// `src-tauri/build.rs` produces it, and the node panics with a clear hint
/// if it's missing (rather than silently degrading to an FS scan).
static CARD_DB: OnceLock<CardDatabase> = OnceLock::new();
static TOKEN_DB: OnceLock<CardDatabase> = OnceLock::new();
static DB_INIT: Once = Once::new();

fn ensure_dbs_loaded() {
    DB_INIT.call_once(|| {
        let archive_path = cardset_archive_path();
        info!(path = %archive_path.display(), "loading card + token databases from archive");
        let file = std::fs::File::open(&archive_path).unwrap_or_else(|e| {
            panic!(
                "Cardset archive not found at {}: {e}. Run `cargo build -p forge-web` (or `yarn build:wasm`) to build it.",
                archive_path.display()
            )
        });
        let mmap = unsafe { Mmap::map(&file).expect("mmap cardset archive") };
        let bundle =
            CardDatabase::load_from_archive(&mmap).expect("load cardset archive");
        info!(
            cards_loaded = bundle.cards_result.loaded,
            cards_failed = bundle.cards_result.failed,
            tokens_loaded = bundle.tokens_result.loaded,
            tokens_failed = bundle.tokens_result.failed,
            "loaded archive"
        );
        for (file, error) in bundle.tokens_result.errors.iter().take(10) {
            warn!(file, %error, "token parse error");
        }
        let _ = CARD_DB.set(bundle.cards);
        let _ = TOKEN_DB.set(bundle.tokens);
    });
}

fn get_card_db() -> &'static CardDatabase {
    ensure_dbs_loaded();
    CARD_DB.get().expect("card db must be initialized")
}

fn get_token_db() -> &'static CardDatabase {
    ensure_dbs_loaded();
    TOKEN_DB.get().expect("token db must be initialized")
}

fn cardset_archive_path() -> PathBuf {
    env::var("CARDSET_ARCHIVE")
        .map(PathBuf::from)
        .unwrap_or_else(|_| workspace_root().join("src-tauri/resources/cardset.rkyv"))
}
