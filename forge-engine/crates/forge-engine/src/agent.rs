use crate::game::GameState;
use crate::ids::{CardId, PlayerId};
use crate::mana::ManaPool;
use forge_foundation::PhaseType;

/// A target choice that can be a player, a card, or nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetChoice {
    Player(PlayerId),
    Card(CardId),
    None,
}

/// The action a player takes during a main phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MainPhaseAction {
    /// Pass priority / end main phase.
    Pass,
    /// Play a card from hand (land or spell).
    Play(CardId),
    /// Tap an untapped land on the battlefield to add its mana to the pool.
    ActivateMana(CardId),
    /// Untap a tapped land and remove its mana from the pool (undo tap).
    UntapMana(CardId),
    /// Activate an ability on a permanent. (source card, ability index)
    ActivateAbility(CardId, usize),
}

/// Trait for player decision-making. Decouples the engine from UI/AI.
/// Implementations can be interactive (prompt user), AI, or network-driven.
pub trait PlayerAgent {
    /// Called before each agent decision point with the current game state.
    /// Override this to capture snapshots for a UI or network layer.
    fn snapshot_state(&mut self, _game: &GameState, _mana_pools: &[ManaPool]) {}

    /// Called before library-peek choices (Scry, Surveil, Dig) so UI agents
    /// can build card info for the cards being revealed from the library.
    /// Receives `game` and the top-N card IDs the player is looking at.
    /// Default implementation is a no-op.
    fn on_library_peek(&mut self, _game: &GameState, _cards: &[CardId]) {}

    /// Choose which cards to keep in opening hand (mulligan decision).
    /// Returns true to keep, false to mulligan.
    fn mulligan_decision(&mut self, player: PlayerId, hand: &[CardId]) -> bool;

    /// Choose a main-phase action: play a card from hand, tap a land for mana, untap a land,
    /// activate an ability, or pass.
    /// `tappable_lands` lists untapped lands available for tapping.
    /// `untappable_lands` lists tapped lands that still have mana in the pool (can be untapped).
    /// `activatable` lists (card_id, ability_index) pairs for activated abilities that can be used.
    fn choose_action(
        &mut self,
        player: PlayerId,
        playable: &[CardId],
        tappable_lands: &[CardId],
        untappable_lands: &[CardId],
        activatable: &[(CardId, usize)],
    ) -> MainPhaseAction;

    /// Choose attackers from available creatures.
    /// Returns the set of creature CardIds to declare as attackers.
    fn choose_attackers(&mut self, player: PlayerId, available: &[CardId]) -> Vec<CardId>;

    /// Choose blockers. Returns pairs of (blocker, attacker).
    fn choose_blockers(
        &mut self,
        player: PlayerId,
        attackers: &[CardId],
        available_blockers: &[CardId],
    ) -> Vec<(CardId, CardId)>;

    /// Choose a target player (e.g. for Lightning Bolt targeting a player).
    fn choose_target_player(&mut self, player: PlayerId, valid: &[PlayerId]) -> Option<PlayerId>;

    /// Choose a target card (e.g. for Lightning Bolt targeting a creature).
    fn choose_target_card(&mut self, player: PlayerId, valid: &[CardId]) -> Option<CardId>;

    /// Choose a target card from a specific zone (e.g. Raise Dead from graveyard).
    fn choose_target_card_from_zone(
        &mut self,
        player: PlayerId,
        _zone: forge_foundation::ZoneType,
        valid: &[CardId],
    ) -> Option<CardId> {
        // Default implementation falls back to regular choose_target_card
        self.choose_target_card(player, valid)
    }

    /// Choose a target that can be a player or a card (e.g. "any target").
    fn choose_target_any(
        &mut self,
        player: PlayerId,
        valid_players: &[PlayerId],
        valid_cards: &[CardId],
    ) -> TargetChoice;

    /// Choose one permanent to sacrifice from the valid options.
    /// Default picks the first (used by AI agents).
    fn choose_sacrifice(&mut self, _player: PlayerId, valid: &[CardId]) -> Option<CardId> {
        valid.first().copied()
    }

    /// Choose which of the top `cards` (from Scry) to put on the bottom of the library.
    /// The rest will stay on top. Default: keep all on top (no cards sent to bottom).
    /// Mirrors Java's `PlayerController.chooseScryCriteria()`.
    fn choose_scry(&mut self, _player: PlayerId, _cards: &[CardId]) -> Vec<CardId> {
        vec![]
    }

    /// Choose which of the top `cards` (from Surveil) to put into the graveyard.
    /// The rest will go on top. Default: keep all on top (nothing milled).
    /// Mirrors Java's `Player.surveil()`.
    fn choose_surveil(&mut self, _player: PlayerId, _cards: &[CardId]) -> Vec<CardId> {
        vec![]
    }

    /// Choose up to `max` cards from `valid` to move to the destination zone (Dig effect).
    /// `optional` means the player is not required to choose any.
    /// Default: take first `max` cards.
    /// Mirrors Java's `PlayerController.chooseEntitiesForEffect()` used in DigEffect.
    fn choose_dig(
        &mut self,
        _player: PlayerId,
        valid: &[CardId],
        max: usize,
        _optional: bool,
    ) -> Vec<CardId> {
        valid.iter().copied().take(max).collect()
    }

    /// Choose an ordering for the top N cards being put back on the library (Ponder/Reorder).
    /// Returns the cards in desired order: index 0 will be placed deepest, last will be on top.
    /// Default: keep original order.
    fn choose_reorder_library(&mut self, _player: PlayerId, cards: &[CardId]) -> Vec<CardId> {
        cards.to_vec()
    }

    /// Choose whether to shuffle after looking at the top of the library (e.g. Ponder).
    /// Default: do not shuffle.
    fn choose_may_shuffle(&mut self, _player: PlayerId) -> bool {
        false
    }

    /// Choose which cards to discard from hand (for SP$ Discard effects).
    /// `hand` is the full hand, `num` is how many must be discarded.
    /// Default: discard the first `num` cards.
    fn choose_discard(&mut self, _player: PlayerId, hand: &[CardId], num: usize) -> Vec<CardId> {
        hand.iter().copied().take(num).collect()
    }

    /// Choose cards to discard at random (for Mode$ Random discard, e.g. Hypnotic Specter).
    /// The engine calls this instead of `choose_discard` when the discard is random.
    /// Default: discard the first `num` cards (same as choose_discard).
    /// Deterministic agents should override this to use their seeded RNG.
    fn choose_random_discard(
        &mut self,
        _player: PlayerId,
        hand: &[CardId],
        num: usize,
    ) -> Vec<CardId> {
        hand.iter().copied().take(num).collect()
    }

    /// Choose a target spell on the stack (for SP$ Counter effects).
    /// `valid` is a slice of stack entry IDs.
    /// Default: target the first (topmost) spell.
    fn choose_target_spell(&mut self, _player: PlayerId, valid: &[u32]) -> Option<u32> {
        valid.first().copied()
    }

    /// Choose N modes for a modal spell (SP$ Charm / Commands).
    ///
    /// `descriptions` — human-readable description of each mode.
    /// `min` — minimum number of modes to choose.
    /// `max` — maximum number of modes to choose.
    ///
    /// Returns indices into `descriptions` of the chosen modes, in order.
    /// Default: choose the first `min` modes (index 0, 1, …).
    fn choose_mode(
        &mut self,
        _player: PlayerId,
        descriptions: &[String],
        min: usize,
        _max: usize,
        _card_name: Option<&str>,
    ) -> Vec<usize> {
        (0..min.min(descriptions.len())).collect()
    }

    /// Choose whether an optional triggered ability fires.
    /// `description` is the trigger text shown to the player.
    /// `card_name` is the name of the source card (for UI display).
    /// `api` is the spell ability API type (e.g. "Pump", "PumpAll", "DealDamage").
    /// Returns true to allow the trigger, false to decline.
    /// Default: always allow (non-interactive agents accept all optional triggers).
    fn choose_optional_trigger(
        &mut self,
        _player: PlayerId,
        _description: &str,
        _card_name: Option<&str>,
        _api: Option<&str>,
    ) -> bool {
        true
    }

    /// Choose whether to pay the kicker cost for a spell.
    /// `kicker_cost` is the mana cost string (e.g. "W", "2 R").
    /// `card_name` is the name of the spell being cast (for UI display).
    /// Returns true to kick, false to cast without kicker.
    /// Default: don't kick (AI default).
    fn choose_kicker(
        &mut self,
        _player: PlayerId,
        _kicker_cost: &str,
        _card_name: Option<&str>,
    ) -> bool {
        false
    }

    /// Choose whether to pay the buyback cost for a spell.
    /// Returns true to pay buyback, false to cast normally.
    /// Default: don't pay buyback.
    fn choose_buyback(
        &mut self,
        _player: PlayerId,
        _buyback_cost: &str,
        _card_name: Option<&str>,
    ) -> bool {
        false
    }

    /// Choose how many times to pay the multikicker cost.
    /// `max_kicks` is the maximum affordable.
    /// Returns the number of times to kick (0 to max_kicks).
    /// Default: 0 (don't multikick).
    fn choose_multikicker(
        &mut self,
        _player: PlayerId,
        _cost: &str,
        _max_kicks: u32,
        _card_name: Option<&str>,
    ) -> u32 {
        0
    }

    /// Choose how many times to pay the replicate cost.
    /// `max_replicates` is the maximum affordable.
    /// Returns the number of replicates.
    /// Default: 0.
    fn choose_replicate(
        &mut self,
        _player: PlayerId,
        _cost: &str,
        _max_replicates: u32,
        _card_name: Option<&str>,
    ) -> u32 {
        0
    }

    /// Choose an alternative cost for a spell.
    /// `options` describes available casting options (e.g. "Normal cost: 3BB", "Spectacle: BR").
    /// Returns the index of the chosen option (0 = normal, 1+ = alternative).
    /// Default: 0 (normal cost).
    fn choose_alternative_cost(
        &mut self,
        _player: PlayerId,
        _options: &[String],
        _card_name: Option<&str>,
    ) -> usize {
        0
    }

    /// Choose a color (for ChooseColorEffect).
    /// `valid_colors` lists the legal color choices (e.g. ["White","Blue","Black","Red","Green"]).
    /// Default: pick the first valid color.
    fn choose_color(&mut self, _player: PlayerId, valid_colors: &[String]) -> Option<String> {
        valid_colors.first().cloned()
    }

    /// Choose cards for an effect (ChooseCardEffect, CloneEffect, etc.).
    /// `valid` lists eligible card IDs, `min`/`max` are the selection bounds.
    /// Default: pick up to `max` from the front of `valid`.
    fn choose_cards_for_effect(
        &mut self,
        _player: PlayerId,
        valid: &[CardId],
        _min: usize,
        max: usize,
    ) -> Vec<CardId> {
        valid.iter().copied().take(max).collect()
    }

    /// Choose a creature/card type (for ChooseType effect).
    /// `type_category` is "Creature", "Card", "Land", etc.
    /// `valid_types` lists the legal type choices.
    /// Default: pick the first valid type.
    fn choose_type(
        &mut self,
        _player: PlayerId,
        _type_category: &str,
        valid_types: &[String],
    ) -> Option<String> {
        valid_types.first().cloned()
    }

    /// Choose a card name (for NameCard effect).
    /// `valid_names` lists the legal card name choices (for ChooseFromList mode).
    /// Default: pick the first valid name.
    fn choose_card_name(&mut self, _player: PlayerId, valid_names: &[String]) -> Option<String> {
        valid_names.first().cloned()
    }

    /// Choose a number (for ChooseNumber effect).
    /// Default: pick the minimum.
    fn choose_number(&mut self, _player: PlayerId, min: i32, _max: i32) -> Option<i32> {
        Some(min)
    }

    /// Choose heads or tails for a coin flip.
    /// Returns true for heads, false for tails.
    /// Default: always call heads.
    fn flip_coin_call(&mut self, _player: PlayerId) -> bool {
        true
    }

    /// Choose whether to play a land or cast a spell when both are possible.
    /// Returns true for land, false for spell, None to pass.
    fn choose_land_or_spell(&mut self, player: PlayerId) -> Option<bool>;

    /// Notify the agent of a game event (for display/logging).
    fn notify(&mut self, message: &str);

    /// Display-only notification: a card was played (land or spell).
    /// Called on all agents so every player's UI can show the animation.
    fn notify_card_played(
        &mut self,
        _player: PlayerId,
        _card_id: CardId,
        _card_name: &str,
        _set_code: &str,
    ) {
    }

    /// Display-only notification: a new turn is starting for the given player.
    /// Called on all agents before any turn actions so the UI can show the turn flash first.
    fn notify_turn_changed(&mut self, _active_player: PlayerId, _turn_number: u32) {}

    /// Display-only notification: phase/step changed.
    /// Called on all agents so each client can update step UI even when no prompt is needed.
    fn notify_phase_changed(&mut self, _phase: PhaseType) {}

    /// Display-only notification: authoritative game state changed without
    /// necessarily changing turn/phase (e.g. stack item resolved).
    fn notify_state_changed(&mut self) {}
}

/// A simple agent that always passes priority and makes no choices.
/// Useful for testing.
pub struct PassAgent;

impl PlayerAgent for PassAgent {
    fn mulligan_decision(&mut self, _player: PlayerId, _hand: &[CardId]) -> bool {
        true // always keep
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
        Vec::new() // no attackers
    }

    fn choose_blockers(
        &mut self,
        _player: PlayerId,
        _attackers: &[CardId],
        _available_blockers: &[CardId],
    ) -> Vec<(CardId, CardId)> {
        Vec::new() // no blockers
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
        if let Some(&pid) = valid_players.first() {
            TargetChoice::Player(pid)
        } else if let Some(&cid) = valid_cards.first() {
            TargetChoice::Card(cid)
        } else {
            TargetChoice::None
        }
    }

    fn choose_sacrifice(&mut self, _player: PlayerId, valid: &[CardId]) -> Option<CardId> {
        valid.first().copied()
    }

    fn choose_land_or_spell(&mut self, _player: PlayerId) -> Option<bool> {
        None
    }

    fn notify(&mut self, _message: &str) {}
}
