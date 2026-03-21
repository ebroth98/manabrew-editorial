import type { GameView, Card, ActivatableAbilityInfo } from '@/types/openmagic';
import type { GameLogEntry } from '@/types/gameLog';
import type { GameSnapshotEntry } from '@/types/gameSnapshot';
import type { PromptType } from '@/types/promptType';

export interface DisplayEvent {
  kind: string;
  cardId?: string;
  cardName?: string;
  setCode?: string;
  playerId?: string;
  activePlayerId?: string;
  activePlayerName?: string;
  turnNumber?: number;
}

export interface AgentPrompt {
  type: PromptType;
  gameView: GameView;
  displayEvents?: DisplayEvent[];
  playableCardIds?: string[];
  /** All play options with modes (normal, spectacle, evoke, etc.) */
  playableOptions?: { cardId: string; mode: string; modeLabel: string }[];
  handCardIds?: string[];
  availableAttackerIds?: string[];
  attackerIds?: string[];
  availableBlockerIds?: string[];
  validPlayerIds?: string[];
  validCardIds?: string[];
  tappableLandIds?: string[];
  untappableLandIds?: string[];
  zone?: string;
  zoneCards?: Card[];
  /** IDs of library cards revealed for scry / surveil / dig */
  cardIds?: string[];
  /** Card DTOs for the revealed library cards */
  cards?: Card[];
  /** dig: maximum number of cards the player may take */
  numToTake?: number;
  /** dig: whether taking 0 cards is allowed */
  optional?: boolean;
  /** chooseDiscard: how many must be discarded */
  numToDiscard?: number;
  /** chooseTargetSpell: stack entry IDs that can be countered */
  validSpellIds?: string[];
  /** chooseMode: human-readable descriptions for each available mode */
  options?: string[];
  /** chooseMode: minimum number of modes that must be chosen */
  minChoices?: number;
  /** chooseMode: maximum number of modes that can be chosen */
  maxChoices?: number;
  /** chooseOptionalTrigger: trigger description text */
  description?: string;
  /** chooseOptionalTrigger: context tag (optional_trigger | confirm_action) */
  promptKind?: string;
  /** chooseOptionalTrigger: optional labels for decline/accept buttons */
  optionLabels?: string[];
  /** chooseOptionalTrigger/confirmAction: optional mode metadata */
  mode?: string;
  /** chooseOptionalTrigger/confirmAction: optional API metadata */
  api?: string;
  /** choosePhyrexian: the phyrexian shard string (e.g. "W/P") */
  phyrexianColor?: string;
  /** chooseKicker: the kicker cost string */
  kickerCost?: string;
  /** Source card name for displaying card image in modals */
  sourceCardName?: string;
  /** chooseBuyback: the buyback cost string */
  buybackCost?: string;
  /** chooseMultikicker / chooseReplicate: the cost per iteration */
  cost?: string;
  /** chooseMultikicker: max number of kicks */
  maxKicks?: number;
  /** chooseReplicate: max number of replicates */
  maxReplicates?: number;
  /** chooseColor: valid color choices */
  validColors?: string[];
  /** chooseType: category of type to choose */
  typeCategory?: string;
  /** chooseType: valid type choices */
  validTypes?: string[];
  /** chooseNumber: minimum value */
  min?: number;
  /** chooseNumber: maximum value */
  max?: number;
  /** chooseCardName: valid card name choices */
  validNames?: string[];
  /** chooseAction: activated abilities on battlefield permanents */
  activatableAbilityIds?: ActivatableAbilityInfo[];
  /** mulligan: how many mulligans taken so far */
  mulliganCount?: number;
  /** mulliganPutBack: how many cards must be put on the bottom */
  count?: number;
  /** chooseAttackers: possible defenders (players and planeswalkers) */
  possibleDefenderIds?: { id: string; label: string }[];
  /** chooseDamageAssignmentOrder: the attacker card ID */
  attackerId?: string;
  /** chooseDamageAssignmentOrder: blocker IDs to order */
  blockerIds?: string[];
  /** chooseDamageAssignmentOrder: blocker CardDto info */
  blockerCards?: Card[];
  /** chooseCombatDamageAssignment: defender id ("player-{i}" or "card-{i}") */
  defenderId?: string | null;
  /** chooseCombatDamageAssignment: total damage to assign */
  totalDamage?: number;
  /** chooseCombatDamageAssignment: attacker has deathtouch */
  attackerHasDeathtouch?: boolean;
  /** payManaCost: the card being cast */
  cardId?: string;
  /** payManaCost: card display name */
  cardName?: string;
  /** payManaCost: mana cost string (e.g. "{2}{R}") */
  manaCost?: string;
  /** chooseDelve/chooseConvoke: remaining cost string */
  remainingCost?: string;
  /** chooseDelve: max cards to exile */
  maxCards?: number;
  /** payCombatCost: attacker card ID */
  attackerIdForCost?: string;
  /** payCombatCost: attacker display name */
  attackerName?: string;
  /** payCombatCost: mana pool total available */
  manaPoolTotal?: number;
  /** specifyManaCombo: available color letters */
  availableColors?: string[];
  /** specifyManaCombo: total mana to distribute */
  amount?: number;
  /** exploreDecision: name of the revealed card */
  revealedCardName?: string;
  /** exploreDecision: card DTO for the revealed card */
  revealedCard?: Card;
  /** chooseExertAttackers / chooseEnlistAttackers: attacker card IDs */
  attackerCardIds?: string[];
  /** chooseExertAttackers / chooseEnlistAttackers: attacker card DTOs */
  attackerCards?: Card[];
  /** reorderLibrary: cards to reorder */
  reorderCards?: Card[];
  /** reorderLibrary: card IDs to reorder */
  reorderCardIds?: string[];
  /** helpPayAssist: max generic mana the assisting player can pay */
  maxGeneric?: number;
}

export interface GameConfig {
  formatId: string;
  startingLife: number;
}

/** A snapshot queued for sequential flash-then-apply processing. */
export interface DeferredSnapshot {
  displayEvents: DisplayEvent[];
  gameView: GameView;
  /** null for display-only state updates (no player decision). */
  prompt: AgentPrompt | null;
}

export interface GameState {
  gameView: GameView | null;
  currentPrompt: AgentPrompt | null;
  gameLog: GameLogEntry[];
  snapshots: GameSnapshotEntry[];
  isGameActive: boolean;
  debugInfo: string;
  /** Queue of deferred snapshots waiting for flash animation. */
  deferredQueue: DeferredSnapshot[];
  /** True while Game.tsx is processing flash animations. */
  isFlashing: boolean;
  /** True after respond() is called and before the next prompt arrives — prevents double-submit. */
  isWaitingForResponse: boolean;
  gameConfig: GameConfig | null;
  /** True if this is a networked multiplayer game. */
  isMultiplayer: boolean;
  /** True if this client is the host (runs the engine). */
  isHost: boolean;
  /** This player's slot identifier, e.g. "player-0", "player-1". */
  myPlayerSlot: string | null;
  updateGameView: (view: GameView) => void;
  setGameConfig: (config: GameConfig) => void;
  // Actions
  startGame: (deckList: { name: string, setCode: string }[], formatId?: string, commanderName?: string, opponentDeckList?: { name: string, setCode: string }[]) => Promise<void>;
  startMultiplayerGame: (
    playerNames: string[],
    deckLists: { name: string, setCode: string }[][],
    enginePlayerIndex: number,
    localIsHost: boolean,
    startingLife: number
  ) => Promise<void>;
  respond: (action: Record<string, unknown>) => Promise<void>;
  castSpell: (cardId: string, mode?: string) => void;
  passPriority: () => void;
  declareAttackers: (attackerIds: string[], defenderId?: string) => void;
  declareBlockers: (assignments: { blockerId: string; attackerId: string }[]) => void;
  targetPlayer: (playerId: string | null) => void;
  targetCard: (cardId: string | null) => void;
  targetAny: (target: { kind: string; playerId?: string; cardId?: string }) => void;
  mulliganDecision: (keep: boolean) => void;
  mulliganPutBackDecision: (cardIds: string[]) => void;
  tapLand: (cardId: string) => void;
  untapLand: (cardId: string) => void;
  activateAbility: (cardId: string, abilityIndex: number) => void;
  scryDecision: (bottomCardIds: string[]) => void;
  surveilDecision: (graveyardCardIds: string[]) => void;
  digDecision: (chosenCardIds: string[]) => void;
  discardDecision: (discardedCardIds: string[]) => void;
  targetSpell: (spellId: string | null) => void;
  modeDecision: (chosenIndices: number[]) => void;
  optionalTriggerDecision: (accept: boolean) => void;
  colorDecision: (color: string | null) => void;
  chooseCardsDecision: (chosenCardIds: string[]) => void;
  typeDecision: (chosenType: string | null) => void;
  numberDecision: (chosenNumber: number | null) => void;
  cardNameDecision: (chosenName: string | null) => void;
  payCombatCost: () => void;
  declineCombatCost: () => void;
  payManaCost: () => void;
  cancelManaCost: () => void;
  delveDecision: (chosenCardIds: string[]) => void;
  convokeDecision: (chosenCardIds: string[]) => void;
  improviseDecision: (chosenCardIds: string[]) => void;
  manaComboDecision: (chosenColors: string[]) => void;
  exploreDecision: (putInGraveyard: boolean) => void;
  exertDecision: (chosenAttackerIds: string[]) => void;
  enlistDecision: (chosenAttackerIds: string[]) => void;
  reorderLibraryDecision: (orderedCardIds: string[]) => void;
  assistDecision: (amountToPay: number) => void;
  concede: () => void;
  endGame: () => Promise<void>;
  setMultiplayerState: (isMultiplayer: boolean, isHost: boolean, myPlayerSlot: string | null) => void;
  restoreSnapshot: (checkpointId: number) => Promise<void>;
}
