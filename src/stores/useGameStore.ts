import { create } from 'zustand';
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type { GameView, Card, ActivatableAbilityInfo } from '@/types/xmage';
import { getFormat } from '@/lib/formats';
import { normalizeGameLogPayload, type GameLogEntry } from '@/types/gameLog';
import { normalizeSnapshotPayload, type GameSnapshotEntry } from '@/types/gameSnapshot';

interface DisplayEvent {
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
  type: string;
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

interface GameConfig {
  formatId: string;
  startingLife: number;
}

/** A snapshot queued for sequential flash-then-apply processing. */
interface DeferredSnapshot {
  displayEvents: DisplayEvent[];
  gameView: GameView;
  /** null for display-only state updates (no player decision). */
  prompt: AgentPrompt | null;
}

interface GameState {
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
  startGame: (deckList: { name: string, setCode: string }[], formatId?: string, commanderName?: string) => Promise<void>;
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
  setupListeners: () => Promise<() => void>;
}

/** Prompt types the UI knows how to render a modal/interaction for. */
const HANDLED_PROMPT_TYPES = new Set([
  "stateUpdate",
  "gameOver",
  "mulligan",
  "mulliganPutBack",
  "chooseAction",
  "chooseAttackers",
  "chooseBlockers",
  "chooseTargetCard",
  "chooseTargetCardFromZone",
  "chooseTargetPlayer",
  "chooseTargetAny",
  "chooseTargetSpell",
  "chooseMode",
  "chooseOptionalTrigger",
  "choosePhyrexian",
  "chooseKicker",
  "chooseBuyback",
  "chooseMultikicker",
  "chooseReplicate",
  "chooseAlternativeCost",
  "chooseColor",
  "chooseCardsForEffect",
  "chooseType",
  "chooseNumber",
  "chooseCardName",
  "chooseDiscard",
  "chooseDamageAssignmentOrder",
  "payCombatCost",
  "payManaCost",
  "chooseDelve",
  "chooseConvoke",
  "chooseImprovise",
  "specifyManaCombo",
  "scry",
  "surveil",
  "dig",
  "chooseExertAttackers",
  "chooseEnlistAttackers",
  "reorderLibrary",
  "exploreDecision",
  "helpPayAssist",
]);

function applyPrompt(prompt: AgentPrompt, source: string, set: (partial: Partial<GameState>) => void, get: () => GameState) {
  const displayEvents = [...(prompt.displayEvents ?? [])];
  // Don't mutate the original payload (listeners may fire more than once).

  const currentGameView = get().gameView;
  const queueLen = get().deferredQueue.length;
  // stateUpdate prompts only carry a gameView + display events — they should
  // NOT replace the currentPrompt (the active player decision).
  const isStateUpdate = prompt.type === "stateUpdate";

  // DEV warning: detect prompt types the UI doesn't handle (engine takes a default/arbitrary action)
  if (!isStateUpdate && !HANDLED_PROMPT_TYPES.has(prompt.type)) {
    const cardName = prompt.sourceCardName ?? prompt.cardName ?? prompt.attackerName ?? "unknown";
    const details = JSON.stringify(prompt, null, 2);
    const devMsg = `[DEV] Unhandled prompt "${prompt.type}" for card "${cardName}" — engine takes default action\n${details}`;
    console.warn(devMsg, prompt);
    const devEntry: import("@/types/gameLog").GameLogEntry = {
      message: devMsg,
      entryType: "warning",
      timestampMs: Date.now(),
    };
    set({ gameLog: [...get().gameLog.slice(-99), devEntry] });
  }

  if (displayEvents.length > 0 && currentGameView !== null) {
    // Enqueue this snapshot — the flash processor will play the events then apply the state.
    const snapshot: DeferredSnapshot = { displayEvents, gameView: prompt.gameView, prompt: isStateUpdate ? null : prompt };
    set({
      deferredQueue: [...get().deferredQueue, snapshot],
      debugInfo: `${source}: ${prompt.type} (queued #${queueLen + 1})`,
    });
  } else if (queueLen > 0 || get().isFlashing) {
    // Flashes are in progress but this prompt has no display events — enqueue with empty events
    // so it gets applied after the current flash sequence finishes.
    const snapshot: DeferredSnapshot = { displayEvents: [], gameView: prompt.gameView, prompt: isStateUpdate ? null : prompt };
    set({
      deferredQueue: [...get().deferredQueue, snapshot],
      debugInfo: `${source}: ${prompt.type} (queued-passthrough #${queueLen + 1})`,
    });
  } else {
    // No display events and no queue — apply immediately
    const updates: Partial<GameState> = {
      gameView: prompt.gameView,
      debugInfo: `${source}: ${prompt.type}`,
      isWaitingForResponse: false,
      currentPrompt: isStateUpdate ? null : prompt,
    };
    set(updates);
  }
}

export const useGameStore = create<GameState>((set, get) => ({
  gameView: null,
  currentPrompt: null,
  gameLog: [],
  snapshots: [],
  isGameActive: false,
  debugInfo: '',
  deferredQueue: [],
  isFlashing: false,
  isWaitingForResponse: false,
  gameConfig: null,
  isMultiplayer: false,
  isHost: false,
  myPlayerSlot: null,

  updateGameView: (view) => set({ gameView: view }),

  setGameConfig: (config) => set({ gameConfig: config }),

  startGame: async (deckList, formatId, commanderName) => {
    try {
      set({ debugInfo: 'Starting game...' });
      const format = formatId ? getFormat(formatId) : undefined;
      const startingLife = format?.deckRules.startingLife ?? 20;
      const gameConfig: GameConfig = { formatId: formatId ?? 'constructed', startingLife };
      set({ gameConfig });
      const result = await invoke('start_game', {
        deckList: deckList,
        startingLife,
        commanderName: commanderName ?? null,
      });
      // Clear old game state so stale gameView/prompts don't bleed into new game
      set({ isGameActive: true, gameLog: [], snapshots: [], gameView: null, currentPrompt: null, deferredQueue: [], isFlashing: false, isWaitingForResponse: false, debugInfo: `Game started: ${result}. Polling...` });
    } catch (e) {
      set({ debugInfo: `Start failed: ${e}` });
      console.error('[store] Failed to start game:', e);
    }
  },

  startMultiplayerGame: async (playerNames, deckLists, enginePlayerIndex, localIsHost, startingLife) => {
    try {
      set({ debugInfo: 'Starting multiplayer game...' });
      await invoke('start_multiplayer_game', {
        playerNames,
        deckLists,
        enginePlayerIndex,
        localIsHost,
        startingLife,
      });
      set({
        isGameActive: true,
        isMultiplayer: true,
        isHost: localIsHost,
        myPlayerSlot: `player-${enginePlayerIndex}`,
        gameLog: [],
        snapshots: [],
        gameView: null,
        currentPrompt: null,
        deferredQueue: [],
        isFlashing: false,
        isWaitingForResponse: false,
        debugInfo: 'Multiplayer game started.',
      });
    } catch (e) {
      set({ debugInfo: `Multiplayer start failed: ${e}` });
      console.error('[store] Failed to start multiplayer game:', e);
    }
  },

  respond: async (action) => {
    try {
      set({ isWaitingForResponse: true, debugInfo: `Responding: ${action.type}` });
      const { myPlayerSlot } = get();
      await invoke('respond', { action, playerSlot: myPlayerSlot });
    } catch (e) {
      set({ isWaitingForResponse: false, debugInfo: `Respond error: ${e}` });
      console.error('Failed to respond:', e);
    }
  },

  castSpell: (cardId, mode?: string) => {
    get().respond({ type: 'playCard', cardId, mode: mode ?? null });
  },

  passPriority: () => {
    if (get().isWaitingForResponse) return;
    const prompt = get().currentPrompt;
    if (!prompt) return;
    switch (prompt.type) {
      case 'chooseAction':
        get().respond({ type: 'playCard', cardId: null });
        break;
      case 'chooseAttackers':
        get().respond({ type: 'declareAttackers', assignments: [] });
        break;
      case 'chooseBlockers':
        get().respond({ type: 'declareBlockers', assignments: [] });
        break;
      default:
        get().respond({ type: 'playCard', cardId: null });
    }
  },

  declareAttackers: (attackerIds, defenderId) => {
    const prompt = get().currentPrompt;
    // Default to first possible defender (the opponent player)
    const defaultDefender = prompt?.possibleDefenderIds?.[0]?.id ?? 'player-1';
    const assignments = attackerIds.map(id => ({
      attackerId: id,
      defenderId: defenderId ?? defaultDefender,
    }));
    get().respond({ type: 'declareAttackers', assignments });
  },

  declareBlockers: (assignments) => {
    get().respond({ type: 'declareBlockers', assignments });
  },

  targetPlayer: (playerId) => {
    get().respond({ type: 'targetPlayer', playerId });
  },

  targetCard: (cardId) => {
    get().respond({ type: 'targetCard', cardId });
  },

  targetAny: (target) => {
    get().respond({ type: 'targetAny', target });
  },

  mulliganDecision: (keep) => {
    get().respond({ type: 'mulliganDecision', keep });
  },

  mulliganPutBackDecision: (cardIds) => {
    get().respond({ type: 'mulliganPutBackDecision', cardIds });
  },

  tapLand: (cardId) => {
    get().respond({ type: 'tapLand', cardId });
  },

  untapLand: (cardId) => {
    get().respond({ type: 'untapLand', cardId });
  },

  activateAbility: (cardId, abilityIndex) => {
    get().respond({ type: 'activateAbility', cardId, abilityIndex });
  },

  scryDecision: (bottomCardIds) => {
    get().respond({ type: 'scryDecision', bottomCardIds });
  },

  surveilDecision: (graveyardCardIds) => {
    get().respond({ type: 'surveilDecision', graveyardCardIds });
  },

  digDecision: (chosenCardIds) => {
    get().respond({ type: 'digDecision', chosenCardIds });
  },

  discardDecision: (discardedCardIds) => {
    get().respond({ type: 'discardDecision', discardedCardIds });
  },

  targetSpell: (spellId) => {
    get().respond({ type: 'targetSpell', spellId });
  },

  modeDecision: (chosenIndices) => {
    get().respond({ type: 'modeDecision', chosenIndices });
  },

  optionalTriggerDecision: (accept) => {
    get().respond({ type: 'optionalTriggerDecision', accept });
  },

  colorDecision: (color) => {
    get().respond({ type: 'colorDecision', color });
  },

  chooseCardsDecision: (chosenCardIds) => {
    get().respond({ type: 'chooseCardsDecision', chosenCardIds });
  },

  typeDecision: (chosenType) => {
    get().respond({ type: 'typeDecision', chosenType });
  },

  numberDecision: (chosenNumber) => {
    get().respond({ type: 'numberDecision', chosenNumber });
  },

  cardNameDecision: (chosenName) => {
    get().respond({ type: 'cardNameDecision', chosenName });
  },

  payCombatCost: () => {
    get().respond({ type: 'payCombatCost' });
  },

  declineCombatCost: () => {
    get().respond({ type: 'declineCombatCost' });
  },

  payManaCost: () => {
    get().respond({ type: 'payManaCost' });
  },

  cancelManaCost: () => {
    get().respond({ type: 'cancelManaCost' });
  },

  delveDecision: (chosenCardIds) => {
    get().respond({ type: 'delveDecision', chosenCardIds });
  },

  convokeDecision: (chosenCardIds) => {
    get().respond({ type: 'convokeDecision', chosenCardIds });
  },

  improviseDecision: (chosenCardIds) => {
    get().respond({ type: 'improviseDecision', chosenCardIds });
  },

  manaComboDecision: (chosenColors) => {
    get().respond({ type: 'manaComboDecision', chosenColors });
  },

  exploreDecision: (putInGraveyard) => {
    get().respond({ type: 'exploreResponse', putInGraveyard });
  },

  exertDecision: (chosenAttackerIds) => {
    get().respond({ type: 'exertDecision', chosenAttackerIds });
  },

  enlistDecision: (chosenAttackerIds) => {
    get().respond({ type: 'enlistDecision', chosenAttackerIds });
  },

  reorderLibraryDecision: (orderedCardIds) => {
    get().respond({ type: 'reorderLibraryDecision', orderedCardIds });
  },

  assistDecision: (amountToPay) => {
    get().respond({ type: 'assistDecision', amountToPay });
  },

  concede: () => {
    get().respond({ type: 'concede' });
  },

  endGame: async () => {
    try {
      await invoke('end_game');
      set({ isGameActive: false, gameView: null, currentPrompt: null, gameLog: [], snapshots: [], deferredQueue: [], isFlashing: false, isWaitingForResponse: false, isMultiplayer: false, isHost: false, myPlayerSlot: null });
    } catch (e) {
      console.error('Failed to end game:', e);
    }
  },

  setMultiplayerState: (isMultiplayer, isHost, myPlayerSlot) => {
    set({ isMultiplayer, isHost, myPlayerSlot });
  },

  restoreSnapshot: async (checkpointId) => {
    const { isMultiplayer, isHost } = get();
    if (isMultiplayer && !isHost) return;
    const promptType = get().currentPrompt?.type;
    const safePrompt =
      promptType === 'chooseAction' ||
      promptType === 'chooseAttackers' ||
      promptType === 'chooseBlockers';
    if (!safePrompt) {
      set({ debugInfo: 'Snapshot restore is only available during priority/combat declaration prompts.' });
      return;
    }
    await invoke('restore_snapshot', { checkpointId });
    set({ debugInfo: `Requested snapshot restore: #${checkpointId}` });
  },

  setupListeners: async () => {
    const unlisteners: UnlistenFn[] = [];

    try {
      const unlisten1 = await listen<AgentPrompt>('game:prompt', (event) => {
        const prompt = event.payload;
        if (get().gameView?.gameOver) return;
        if (prompt && prompt.gameView) {
          applyPrompt(prompt, 'Event', set, get);
        }
      });
      unlisteners.push(unlisten1);

      const unlisten2 = await listen<unknown>('game:log', (event) => {
        const entry = normalizeGameLogPayload(event.payload);
        set((state) => ({
          gameLog: [...state.gameLog.slice(-199), entry],
        }));
      });
      unlisteners.push(unlisten2);

      const unlistenSnapshot = await listen<unknown>('game:snapshot', (event) => {
        const snapshot = normalizeSnapshotPayload(event.payload);
        if (!snapshot.gameView) return;
        set((state) => ({
          snapshots: [...state.snapshots.filter((s) => s.checkpointId !== snapshot.checkpointId).slice(-199), snapshot],
        }));
      });
      unlisteners.push(unlistenSnapshot);

      // Remote prompt listener: receives prompts relayed via the server for non-host players
      const unlisten3 = await listen<{ kind: string; forPlayer: string; prompt: AgentPrompt }>('game:remote_prompt', (event) => {
        const { forPlayer, prompt } = event.payload;
        const { myPlayerSlot } = get();
        if (forPlayer === myPlayerSlot) {
          // This prompt is for us — render it fully.
          applyPrompt(prompt, 'Remote', set, get);
        } else {
          // Keep shared turn/priority in sync even when the prompt is for another player.
          // Do not apply full foreign-perspective view (would leak/flip local actionability).
          const current = get().gameView;
          if (current && prompt?.gameView) {
            const iHavePriority = prompt.gameView.priorityPlayerId === myPlayerSlot;
            set({
              gameView: {
                ...current,
                turn: prompt.gameView.turn,
                step: prompt.gameView.step,
                activePlayerId: prompt.gameView.activePlayerId,
                priorityPlayerId: prompt.gameView.priorityPlayerId,
                gameOver: prompt.gameView.gameOver,
                winnerId: prompt.gameView.winnerId,
              },
              // Never keep a stale actionable prompt when priority is not ours.
              currentPrompt: iHavePriority ? get().currentPrompt : null,
              isWaitingForResponse: iHavePriority ? get().isWaitingForResponse : false,
              debugInfo: `Remote sync: ${prompt.type}`,
            });
          }
        }
      });
      unlisteners.push(unlisten3);

      const unlisten4 = await listen<{ reason: string; message: string }>('game:forced_end', (event) => {
        const message = event.payload?.message ?? 'Forced game exit';
        set({
          isGameActive: false,
          gameView: null,
          currentPrompt: null,
          deferredQueue: [],
          isFlashing: false,
          isWaitingForResponse: false,
          isMultiplayer: false,
          isHost: false,
          myPlayerSlot: null,
          snapshots: [],
          debugInfo: `Game ended: ${message}`,
        });
      });
      unlisteners.push(unlisten4);
    } catch (e) {
      console.error('[store] Failed to setup listeners:', e);
    }

    return () => {
      unlisteners.forEach((fn) => fn());
    };
  },
}));
