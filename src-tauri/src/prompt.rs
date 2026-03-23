use serde::{Deserialize, Serialize};

use crate::game_view_dto::{CardDto, GameViewDto};

/// A display-only event that the frontend should animate before rendering the prompt's game state.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum DisplayEvent {
    #[serde(rename_all = "camelCase")]
    CardPlayed {
        card_id: String,
        card_name: String,
        set_code: String,
        player_id: String,
    },
    #[serde(rename_all = "camelCase")]
    TurnChanged {
        active_player_id: String,
        active_player_name: String,
        turn_number: u32,
    },
}

/// Sent from game thread to frontend: what the human player must decide,
/// bundled with any display events that happened since the last prompt.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentPrompt {
    /// Display events to animate before applying the game state.
    #[serde(default)]
    pub display_events: Vec<DisplayEvent>,
    /// The actual prompt data (type + gameView + prompt-specific fields).
    #[serde(flatten)]
    pub inner: AgentPromptInner,
}

/// The actual decision prompt variants.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum AgentPromptInner {
    Mulligan {
        #[serde(rename = "gameView")]
        game_view: GameViewDto,
        #[serde(rename = "handCardIds")]
        hand_card_ids: Vec<String>,
        #[serde(rename = "mulliganCount")]
        mulligan_count: u32,
    },
    /// London Mulligan: choose which cards to put on the bottom of the library.
    MulliganPutBack {
        #[serde(rename = "gameView")]
        game_view: GameViewDto,
        #[serde(rename = "handCardIds")]
        hand_card_ids: Vec<String>,
        /// Card DTOs for display.
        cards: Vec<CardDto>,
        /// Number of cards that must be put back.
        count: usize,
    },
    ChooseAction {
        #[serde(rename = "gameView")]
        game_view: GameViewDto,
        #[serde(rename = "playableCardIds")]
        playable_card_ids: Vec<String>,
        /// All play options with their modes (normal, spectacle, evoke, etc.).
        #[serde(rename = "playableOptions")]
        playable_options: Vec<PlayOptionDto>,
        /// Untapped lands (and creatures/artifacts with mana abilities) that the player can tap.
        #[serde(rename = "tappableLandIds")]
        tappable_land_ids: Vec<String>,
        /// Tapped lands whose mana is still in the pool (can be untapped to undo).
        #[serde(rename = "untappableLandIds")]
        untappable_land_ids: Vec<String>,
        /// Activated abilities on battlefield permanents.
        #[serde(rename = "activatableAbilityIds")]
        activatable_ability_ids: Vec<ActivatableAbilityInfo>,
    },
    ChooseAttackers {
        #[serde(rename = "gameView")]
        game_view: GameViewDto,
        #[serde(rename = "availableAttackerIds")]
        available_attacker_ids: Vec<String>,
        /// Possible defenders: opponent players and their planeswalkers.
        #[serde(rename = "possibleDefenderIds")]
        possible_defender_ids: Vec<DefenderIdDto>,
    },
    ChooseBlockers {
        #[serde(rename = "gameView")]
        game_view: GameViewDto,
        #[serde(rename = "attackerIds")]
        attacker_ids: Vec<String>,
        #[serde(rename = "availableBlockerIds")]
        available_blocker_ids: Vec<String>,
    },
    ChooseTargetPlayer {
        #[serde(rename = "gameView")]
        game_view: GameViewDto,
        #[serde(rename = "validPlayerIds")]
        valid_player_ids: Vec<String>,
        #[serde(rename = "sourceCardId", skip_serializing_if = "Option::is_none")]
        source_card_id: Option<String>,
        /// Whether the targeting effect is hostile (damage/destroy) vs friendly (buff).
        #[serde(default)]
        hostile: bool,
    },
    ChooseTargetCard {
        #[serde(rename = "gameView")]
        game_view: GameViewDto,
        #[serde(rename = "validCardIds")]
        valid_card_ids: Vec<String>,
        #[serde(rename = "sourceCardId", skip_serializing_if = "Option::is_none")]
        source_card_id: Option<String>,
        #[serde(default)]
        hostile: bool,
    },
    ChooseTargetAny {
        #[serde(rename = "gameView")]
        game_view: GameViewDto,
        #[serde(rename = "validPlayerIds")]
        valid_player_ids: Vec<String>,
        #[serde(rename = "validCardIds")]
        valid_card_ids: Vec<String>,
        #[serde(rename = "sourceCardId", skip_serializing_if = "Option::is_none")]
        source_card_id: Option<String>,
        #[serde(default)]
        hostile: bool,
    },
    ChooseTargetCardFromZone {
        #[serde(rename = "gameView")]
        game_view: GameViewDto,
        #[serde(rename = "validCardIds")]
        valid_card_ids: Vec<String>,
        zone: String,
        #[serde(rename = "zoneCards")]
        zone_cards: Vec<CardDto>,
        #[serde(rename = "sourceCardId", skip_serializing_if = "Option::is_none")]
        source_card_id: Option<String>,
    },
    GameOver {
        #[serde(rename = "gameView")]
        game_view: GameViewDto,
    },
    /// Display-only state update — no player decision required.
    /// Emitted after each card play / turn change so the frontend can
    /// animate events one-at-a-time even during the AI's turn.
    StateUpdate {
        #[serde(rename = "gameView")]
        game_view: GameViewDto,
    },
    /// Scry N: player sees `card_ids` (top N of library) and picks which go to bottom.
    Scry {
        #[serde(rename = "gameView")]
        game_view: GameViewDto,
        /// The top N cards the player is looking at (in library order, last = topmost).
        #[serde(rename = "cardIds")]
        card_ids: Vec<String>,
        /// Card DTOs for display.
        #[serde(rename = "cards")]
        cards: Vec<CardDto>,
    },
    /// Surveil N: player sees `card_ids` (top N of library) and picks which go to graveyard.
    Surveil {
        #[serde(rename = "gameView")]
        game_view: GameViewDto,
        #[serde(rename = "cardIds")]
        card_ids: Vec<String>,
        #[serde(rename = "cards")]
        cards: Vec<CardDto>,
    },
    /// Dig N, take K: player sees `card_ids` (top N) and picks up to `num_to_take` to keep.
    Dig {
        #[serde(rename = "gameView")]
        game_view: GameViewDto,
        #[serde(rename = "cardIds")]
        card_ids: Vec<String>,
        #[serde(rename = "cards")]
        cards: Vec<CardDto>,
        #[serde(rename = "numToTake")]
        num_to_take: usize,
        optional: bool,
    },
    /// Discard N cards from hand.
    ChooseDiscard {
        #[serde(rename = "gameView")]
        game_view: GameViewDto,
        /// All card IDs currently in hand.
        #[serde(rename = "handCardIds")]
        hand_card_ids: Vec<String>,
        /// How many cards must be discarded.
        #[serde(rename = "numToDiscard")]
        num_to_discard: usize,
    },
    /// Choose a target spell on the stack (for Counter).
    ChooseTargetSpell {
        #[serde(rename = "gameView")]
        game_view: GameViewDto,
        /// Stack entry IDs (as strings) that can be countered.
        #[serde(rename = "validSpellIds")]
        valid_spell_ids: Vec<String>,
        #[serde(rename = "sourceCardId", skip_serializing_if = "Option::is_none")]
        source_card_id: Option<String>,
    },
    /// Choose whether an optional triggered ability fires.
    ChooseOptionalTrigger {
        #[serde(rename = "gameView")]
        game_view: GameViewDto,
        /// Description of the trigger.
        description: String,
        /// Name of the source card (for displaying card image in modals).
        #[serde(rename = "sourceCardName")]
        source_card_name: Option<String>,
        /// Optional context tag: e.g. "optional_trigger" or "confirm_action".
        #[serde(rename = "promptKind")]
        prompt_kind: Option<String>,
        /// Optional labels for decline/accept buttons.
        #[serde(rename = "optionLabels")]
        option_labels: Option<Vec<String>>,
        /// Optional confirm mode metadata from engine.
        mode: Option<String>,
        /// Optional API metadata from engine.
        api: Option<String>,
    },
    /// Choose N modes for a modal spell (SP$ Charm).
    ChooseMode {
        #[serde(rename = "gameView")]
        game_view: GameViewDto,
        /// Human-readable descriptions for each available mode.
        options: Vec<String>,
        /// Minimum number of modes that must be chosen.
        #[serde(rename = "minChoices")]
        min_choices: usize,
        /// Maximum number of modes that can be chosen.
        #[serde(rename = "maxChoices")]
        max_choices: usize,
        /// Name of the source card (for displaying card image in modals).
        #[serde(rename = "sourceCardName")]
        source_card_name: Option<String>,
    },
    /// Choose whether to pay 2 life instead of mana for a Phyrexian mana shard.
    ChoosePhyrexian {
        #[serde(rename = "gameView")]
        game_view: GameViewDto,
        /// The phyrexian shard string (e.g. "W/P", "U/P").
        #[serde(rename = "phyrexianColor")]
        phyrexian_color: String,
        /// Name of the source card.
        #[serde(rename = "sourceCardName")]
        source_card_name: Option<String>,
    },
    /// Choose whether to pay a kicker cost.
    ChooseKicker {
        #[serde(rename = "gameView")]
        game_view: GameViewDto,
        /// The kicker cost string (e.g. "W", "2 R").
        #[serde(rename = "kickerCost")]
        kicker_cost: String,
        /// Name of the source card (for displaying card image in modals).
        #[serde(rename = "sourceCardName")]
        source_card_name: Option<String>,
    },
    /// Choose whether to pay buyback cost.
    ChooseBuyback {
        #[serde(rename = "gameView")]
        game_view: GameViewDto,
        #[serde(rename = "buybackCost")]
        buyback_cost: String,
        #[serde(rename = "sourceCardName")]
        source_card_name: Option<String>,
    },
    /// Choose how many times to pay multikicker cost.
    ChooseMultikicker {
        #[serde(rename = "gameView")]
        game_view: GameViewDto,
        cost: String,
        #[serde(rename = "maxKicks")]
        max_kicks: u32,
        #[serde(rename = "sourceCardName")]
        source_card_name: Option<String>,
    },
    /// Choose how many times to pay replicate cost.
    ChooseReplicate {
        #[serde(rename = "gameView")]
        game_view: GameViewDto,
        cost: String,
        #[serde(rename = "maxReplicates")]
        max_replicates: u32,
        #[serde(rename = "sourceCardName")]
        source_card_name: Option<String>,
    },
    /// Choose between normal cost and an alternative cost.
    ChooseAlternativeCost {
        #[serde(rename = "gameView")]
        game_view: GameViewDto,
        options: Vec<String>,
        #[serde(rename = "sourceCardName")]
        source_card_name: Option<String>,
    },
    /// Choose a color (for ChooseColorEffect).
    ChooseColor {
        #[serde(rename = "gameView")]
        game_view: GameViewDto,
        #[serde(rename = "validColors")]
        valid_colors: Vec<String>,
        #[serde(rename = "sourceCardName")]
        source_card_name: Option<String>,
    },
    /// Choose a creature/card type (for ChooseType effect).
    ChooseType {
        #[serde(rename = "gameView")]
        game_view: GameViewDto,
        /// Category: "Creature", "Card", "Land", etc.
        #[serde(rename = "typeCategory")]
        type_category: String,
        /// Valid type choices.
        #[serde(rename = "validTypes")]
        valid_types: Vec<String>,
        #[serde(rename = "sourceCardName")]
        source_card_name: Option<String>,
    },
    /// Choose a number (for ChooseNumber effect).
    ChooseNumber {
        #[serde(rename = "gameView")]
        game_view: GameViewDto,
        min: i32,
        max: i32,
        #[serde(rename = "sourceCardName")]
        source_card_name: Option<String>,
    },
    /// Choose a card name (for NameCard effect).
    ChooseCardName {
        #[serde(rename = "gameView")]
        game_view: GameViewDto,
        /// Valid card name choices (for ChooseFromList mode).
        #[serde(rename = "validNames")]
        valid_names: Vec<String>,
        #[serde(rename = "sourceCardName")]
        source_card_name: Option<String>,
    },
    /// Choose damage assignment order for a multi-blocked attacker.
    ChooseDamageAssignmentOrder {
        #[serde(rename = "gameView")]
        game_view: GameViewDto,
        /// The attacker card ID.
        #[serde(rename = "attackerId")]
        attacker_id: String,
        /// The blocker card IDs (to be ordered by the player).
        #[serde(rename = "blockerIds")]
        blocker_ids: Vec<String>,
        /// CardDto info for blockers (so frontend can display them).
        #[serde(rename = "blockerCards")]
        blocker_cards: Vec<CardDto>,
    },
    /// Choose exact combat damage assignment amounts.
    ChooseCombatDamageAssignment {
        #[serde(rename = "gameView")]
        game_view: GameViewDto,
        #[serde(rename = "attackerId")]
        attacker_id: String,
        #[serde(rename = "blockerIds")]
        blocker_ids: Vec<String>,
        /// Defender ID ("player-{i}" or "card-{i}") if defender is a legal assignee.
        #[serde(rename = "defenderId")]
        defender_id: Option<String>,
        #[serde(rename = "totalDamage")]
        total_damage: i32,
        #[serde(rename = "attackerHasDeathtouch")]
        attacker_has_deathtouch: bool,
    },
    /// Pay an attack cost (Propaganda, Ghostly Prison).
    PayCombatCost {
        #[serde(rename = "gameView")]
        game_view: GameViewDto,
        #[serde(rename = "attackerId")]
        attacker_id: String,
        #[serde(rename = "attackerName")]
        attacker_name: String,
        cost: i32,
        description: String,
        #[serde(rename = "tappableLandIds")]
        tappable_land_ids: Vec<String>,
        #[serde(rename = "untappableLandIds")]
        untappable_land_ids: Vec<String>,
        #[serde(rename = "manaPoolTotal")]
        mana_pool_total: i32,
    },
    /// Choose graveyard cards to exile for Delve.
    ChooseDelve {
        #[serde(rename = "gameView")]
        game_view: GameViewDto,
        #[serde(rename = "validCardIds")]
        valid_card_ids: Vec<String>,
        #[serde(rename = "zoneCards")]
        zone_cards: Vec<CardDto>,
        #[serde(rename = "maxCards")]
        max_cards: usize,
        #[serde(rename = "sourceCardName")]
        source_card_name: Option<String>,
    },
    /// Choose creatures to tap for Convoke.
    ChooseConvoke {
        #[serde(rename = "gameView")]
        game_view: GameViewDto,
        #[serde(rename = "validCardIds")]
        valid_card_ids: Vec<String>,
        #[serde(rename = "remainingCost")]
        remaining_cost: String,
        #[serde(rename = "sourceCardName")]
        source_card_name: Option<String>,
    },
    /// Choose artifacts to tap for Improvise.
    ChooseImprovise {
        #[serde(rename = "gameView")]
        game_view: GameViewDto,
        #[serde(rename = "validCardIds")]
        valid_card_ids: Vec<String>,
        #[serde(rename = "remainingCost")]
        remaining_cost: String,
        #[serde(rename = "sourceCardName")]
        source_card_name: Option<String>,
    },
    /// Pay a mana cost interactively (for spells/abilities).
    PayManaCost {
        #[serde(rename = "gameView")]
        game_view: GameViewDto,
        #[serde(rename = "cardId")]
        card_id: String,
        #[serde(rename = "cardName")]
        card_name: String,
        #[serde(rename = "manaCost")]
        mana_cost: String,
        #[serde(rename = "tappableLandIds")]
        tappable_land_ids: Vec<String>,
        #[serde(rename = "untappableLandIds")]
        untappable_land_ids: Vec<String>,
        #[serde(rename = "manaPoolTotal")]
        mana_pool_total: i32,
    },
    /// Specify mana color distribution for combo/any mana production.
    SpecifyManaCombo {
        #[serde(rename = "gameView")]
        game_view: GameViewDto,
        /// Available color letters (e.g. ["W", "U", "B", "R", "G"]).
        #[serde(rename = "availableColors")]
        available_colors: Vec<String>,
        /// Total amount of mana to distribute.
        amount: usize,
        #[serde(rename = "sourceCardName")]
        source_card_name: Option<String>,
    },
    /// Choose which attackers to exert.
    ChooseExertAttackers {
        #[serde(rename = "gameView")]
        game_view: GameViewDto,
        #[serde(rename = "attackerIds")]
        attacker_ids: Vec<String>,
        #[serde(rename = "attackerCards")]
        attacker_cards: Vec<CardDto>,
    },
    /// Choose which attackers to enlist.
    ChooseEnlistAttackers {
        #[serde(rename = "gameView")]
        game_view: GameViewDto,
        #[serde(rename = "attackerIds")]
        attacker_ids: Vec<String>,
        #[serde(rename = "attackerCards")]
        attacker_cards: Vec<CardDto>,
    },
    /// Reorder top cards of library (Ponder-style effects).
    ReorderLibrary {
        #[serde(rename = "gameView")]
        game_view: GameViewDto,
        /// Card IDs to reorder (in current top-first order).
        #[serde(rename = "cardIds")]
        card_ids: Vec<String>,
        /// Card DTOs for display.
        cards: Vec<CardDto>,
        /// Name of the source card.
        #[serde(rename = "sourceCardName")]
        source_card_name: Option<String>,
    },
    /// Explore: choose whether to put the revealed nonland card in graveyard or on top.
    ExploreDecision {
        #[serde(rename = "gameView")]
        game_view: GameViewDto,
        /// Name of the revealed card.
        #[serde(rename = "revealedCardName")]
        revealed_card_name: String,
        /// Card DTO for the revealed card (for display).
        #[serde(rename = "revealedCard")]
        revealed_card: Option<CardDto>,
        /// Name of the exploring creature.
        #[serde(rename = "sourceCardName")]
        source_card_name: Option<String>,
    },
    /// Help pay for a spell with Assist.
    HelpPayAssist {
        #[serde(rename = "gameView")]
        game_view: GameViewDto,
        #[serde(rename = "cardName")]
        card_name: String,
        #[serde(rename = "maxGeneric")]
        max_generic: u32,
    },
    /// Choose card(s) for an effect (ChooseCardEffect, CloneEffect).
    ChooseCardsForEffect {
        #[serde(rename = "gameView")]
        game_view: GameViewDto,
        #[serde(rename = "validCardIds")]
        valid_card_ids: Vec<String>,
        #[serde(rename = "zoneCards")]
        zone_cards: Vec<CardDto>,
        #[serde(rename = "minChoices")]
        min_choices: usize,
        #[serde(rename = "maxChoices")]
        max_choices: usize,
        #[serde(rename = "sourceCardName")]
        source_card_name: Option<String>,
    },
}

impl AgentPromptInner {
    pub fn game_view(&self) -> &GameViewDto {
        match self {
            AgentPromptInner::Mulligan { game_view, .. }
            | AgentPromptInner::MulliganPutBack { game_view, .. }
            | AgentPromptInner::ChooseAction { game_view, .. }
            | AgentPromptInner::ChooseAttackers { game_view, .. }
            | AgentPromptInner::ChooseBlockers { game_view, .. }
            | AgentPromptInner::ChooseTargetPlayer { game_view, .. }
            | AgentPromptInner::ChooseTargetCard { game_view, .. }
            | AgentPromptInner::ChooseTargetAny { game_view, .. }
            | AgentPromptInner::ChooseTargetCardFromZone { game_view, .. }
            | AgentPromptInner::GameOver { game_view }
            | AgentPromptInner::StateUpdate { game_view }
            | AgentPromptInner::Scry { game_view, .. }
            | AgentPromptInner::Surveil { game_view, .. }
            | AgentPromptInner::Dig { game_view, .. }
            | AgentPromptInner::ChooseDiscard { game_view, .. }
            | AgentPromptInner::ChooseTargetSpell { game_view, .. }
            | AgentPromptInner::ChooseMode { game_view, .. }
            | AgentPromptInner::ChooseOptionalTrigger { game_view, .. }
            | AgentPromptInner::ChoosePhyrexian { game_view, .. }
            | AgentPromptInner::ChooseKicker { game_view, .. }
            | AgentPromptInner::ChooseBuyback { game_view, .. }
            | AgentPromptInner::ChooseMultikicker { game_view, .. }
            | AgentPromptInner::ChooseReplicate { game_view, .. }
            | AgentPromptInner::ChooseAlternativeCost { game_view, .. }
            | AgentPromptInner::ChooseColor { game_view, .. }
            | AgentPromptInner::ChooseType { game_view, .. }
            | AgentPromptInner::ChooseNumber { game_view, .. }
            | AgentPromptInner::ChooseCardName { game_view, .. }
            | AgentPromptInner::ChooseDamageAssignmentOrder { game_view, .. }
            | AgentPromptInner::ChooseCombatDamageAssignment { game_view, .. }
            | AgentPromptInner::ChooseExertAttackers { game_view, .. }
            | AgentPromptInner::ChooseEnlistAttackers { game_view, .. }
            | AgentPromptInner::ReorderLibrary { game_view, .. }
            | AgentPromptInner::ExploreDecision { game_view, .. }
            | AgentPromptInner::HelpPayAssist { game_view, .. }
            | AgentPromptInner::ChooseCardsForEffect { game_view, .. }
            | AgentPromptInner::PayCombatCost { game_view, .. }
            | AgentPromptInner::PayManaCost { game_view, .. }
            | AgentPromptInner::ChooseDelve { game_view, .. }
            | AgentPromptInner::ChooseConvoke { game_view, .. }
            | AgentPromptInner::ChooseImprovise { game_view, .. }
            | AgentPromptInner::SpecifyManaCombo { game_view, .. } => game_view,
        }
    }
}

/// Describes a single way to play a card (normal, alternative cost, etc.).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayOptionDto {
    pub card_id: String,
    /// e.g. "normal", "alternative:spectacle", "alternative:evoke", "gainLifeAlt", "foretellExile"
    pub mode: String,
    /// Human-readable label, e.g. "Cast normally", "Cast with Spectacle"
    pub mode_label: String,
}

/// Info about an activatable ability on a battlefield permanent.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivatableAbilityInfo {
    pub card_id: String,
    pub ability_index: usize,
    pub description: String,
    pub is_mana_ability: bool,
}

/// Sent from frontend to game thread: the human player's response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum PlayerAction {
    MulliganDecision {
        keep: bool,
    },
    /// Response to MulliganPutBack: IDs of the cards to put on the bottom.
    MulliganPutBackDecision {
        #[serde(rename = "cardIds")]
        card_ids: Vec<String>,
    },
    PlayCard {
        #[serde(rename = "cardId")]
        card_id: Option<String>,
        /// Optional play mode, e.g. "normal", "alternative:spectacle"
        #[serde(default)]
        mode: Option<String>,
    },
    DeclareAttackers {
        /// Attack assignments: each attacker paired with its defender.
        assignments: Vec<AttackAssignment>,
    },
    DeclareBlockers {
        assignments: Vec<BlockAssignment>,
    },
    TargetPlayer {
        #[serde(rename = "playerId")]
        player_id: Option<String>,
    },
    TargetCard {
        #[serde(rename = "cardId")]
        card_id: Option<String>,
    },
    TargetAny {
        target: TargetAnyChoice,
    },
    TapLand {
        #[serde(rename = "cardId")]
        card_id: String,
    },
    UntapLand {
        #[serde(rename = "cardId")]
        card_id: String,
    },
    ActivateAbility {
        #[serde(rename = "cardId")]
        card_id: String,
        #[serde(rename = "abilityIndex")]
        ability_index: usize,
    },
    /// Response to Scry prompt: IDs of cards the player wants on the bottom.
    ScryDecision {
        #[serde(rename = "bottomCardIds")]
        bottom_card_ids: Vec<String>,
    },
    /// Response to Surveil prompt: IDs of cards the player wants in the graveyard.
    SurveilDecision {
        #[serde(rename = "graveyardCardIds")]
        graveyard_card_ids: Vec<String>,
    },
    /// Response to Dig prompt: IDs of the cards the player wants to take.
    DigDecision {
        #[serde(rename = "chosenCardIds")]
        chosen_card_ids: Vec<String>,
    },
    /// Response to ChooseDiscard prompt: IDs of the cards the player discards.
    DiscardDecision {
        #[serde(rename = "discardedCardIds")]
        discarded_card_ids: Vec<String>,
    },
    /// Response to ChooseTargetSpell prompt: the stack entry ID the player targets.
    TargetSpell {
        #[serde(rename = "spellId")]
        spell_id: Option<String>,
    },
    /// Response to ChooseOptionalTrigger: whether the player accepts.
    OptionalTriggerDecision {
        accept: bool,
    },
    /// Response to ChooseMode prompt: indices (0-based) of chosen modes.
    ModeDecision {
        #[serde(rename = "chosenIndices")]
        chosen_indices: Vec<usize>,
    },
    /// Response to ChoosePhyrexian prompt: whether to pay 2 life.
    PhyrexianDecision {
        #[serde(rename = "payLife")]
        pay_life: bool,
    },
    /// Response to ChooseKicker prompt: whether the player pays the kicker.
    KickerDecision {
        kicked: bool,
    },
    /// Response to ChooseBuyback prompt.
    BuybackDecision {
        #[serde(rename = "buybackPaid")]
        buyback_paid: bool,
    },
    /// Response to ChooseMultikicker prompt: how many times.
    MultikickerDecision {
        #[serde(rename = "kickCount")]
        kick_count: u32,
    },
    /// Response to ChooseReplicate prompt: how many times.
    ReplicateDecision {
        #[serde(rename = "replicateCount")]
        replicate_count: u32,
    },
    /// Response to ChooseAlternativeCost prompt: index of chosen option.
    AlternativeCostDecision {
        #[serde(rename = "chosenIndex")]
        chosen_index: usize,
    },
    /// Response to ChooseColor prompt: the chosen color name.
    ColorDecision {
        color: Option<String>,
    },
    /// Response to ChooseType prompt: the chosen type name.
    TypeDecision {
        #[serde(rename = "chosenType")]
        chosen_type: Option<String>,
    },
    /// Response to ChooseNumber prompt: the chosen number.
    NumberDecision {
        #[serde(rename = "chosenNumber")]
        chosen_number: Option<i32>,
    },
    /// Response to ChooseCardName prompt: the chosen card name.
    CardNameDecision {
        #[serde(rename = "chosenName")]
        chosen_name: Option<String>,
    },
    /// Response to ChooseDamageAssignmentOrder: ordered blocker IDs.
    DamageAssignmentOrderDecision {
        #[serde(rename = "orderedBlockerIds")]
        ordered_blocker_ids: Vec<String>,
    },
    /// Response to ChooseCombatDamageAssignment: exact assignee→damage map.
    CombatDamageAssignmentDecision {
        assignments: Vec<CombatDamageAssignmentEntry>,
    },
    /// Response to ChooseExertAttackers: IDs of attackers to exert.
    ExertDecision {
        #[serde(rename = "chosenAttackerIds")]
        chosen_attacker_ids: Vec<String>,
    },
    /// Response to ChooseEnlistAttackers: IDs of attackers to enlist.
    EnlistDecision {
        #[serde(rename = "chosenAttackerIds")]
        chosen_attacker_ids: Vec<String>,
    },
    /// Response to ReorderLibrary: ordered card IDs (last = top of library).
    ReorderLibraryDecision {
        #[serde(rename = "orderedCardIds")]
        ordered_card_ids: Vec<String>,
    },
    /// Response to ExploreDecision: whether to put in graveyard.
    ExploreResponse {
        #[serde(rename = "putInGraveyard")]
        put_in_graveyard: bool,
    },
    /// Response to HelpPayAssist: amount of generic mana to pay.
    AssistDecision {
        #[serde(rename = "amountToPay")]
        amount_to_pay: u32,
    },
    /// Response to ChooseCardsForEffect prompt: IDs of chosen cards.
    ChooseCardsDecision {
        #[serde(rename = "chosenCardIds")]
        chosen_card_ids: Vec<String>,
    },
    /// Pay the attack cost from the mana pool.
    PayCombatCost,
    /// Decline to pay the attack cost (remove attacker).
    DeclineCombatCost,
    /// Host-only control action: restore engine state to a checkpoint.
    RestoreSnapshot {
        #[serde(rename = "checkpointId")]
        checkpoint_id: u64,
    },
    /// Response to ChooseDelve: IDs of graveyard cards to exile.
    DelveDecision {
        #[serde(rename = "chosenCardIds")]
        chosen_card_ids: Vec<String>,
    },
    /// Response to ChooseConvoke: IDs of creatures to tap.
    ConvokeDecision {
        #[serde(rename = "chosenCardIds")]
        chosen_card_ids: Vec<String>,
    },
    /// Response to ChooseImprovise: IDs of artifacts to tap.
    ImproviseDecision {
        #[serde(rename = "chosenCardIds")]
        chosen_card_ids: Vec<String>,
    },
    /// Response to SpecifyManaCombo: list of color letters chosen.
    ManaComboDecision {
        /// Color letters, e.g. ["W", "W", "U"], totaling the requested amount.
        #[serde(rename = "chosenColors")]
        chosen_colors: Vec<String>,
    },
    /// Confirm mana cost payment from the mana pool.
    PayManaCost,
    /// Cancel casting the spell (mana cost payment).
    CancelManaCost,
    Concede,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockAssignment {
    pub blocker_id: String,
    pub attacker_id: String,
}

/// An attack assignment: attacker paired with its defender.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttackAssignment {
    pub attacker_id: String,
    pub defender_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CombatDamageAssignmentEntry {
    /// "card-{i}" for blockers or defender ID (e.g. "player-{i}" / "card-{i}").
    pub assignee_id: String,
    pub damage: i32,
}

/// A defender identifier sent to/from the frontend.
/// Format: "player-{index}" for players, "card-{index}" for permanents.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DefenderIdDto {
    /// Serialized ID: "player-0", "card-42", etc.
    pub id: String,
    /// Human-readable label for display.
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum TargetAnyChoice {
    Player {
        #[serde(rename = "playerId")]
        player_id: String,
    },
    Card {
        #[serde(rename = "cardId")]
        card_id: String,
    },
    None,
}
