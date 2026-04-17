import { create } from 'zustand';
import { devtools } from 'zustand/middleware';
import { toast } from 'sonner';
import {
  BroadcastRoomHost,
  getSelectedGameRuntime,
  resetSelectedGameRuntime,
  selectGameRuntime,
  startManualRoomSync,
  stopManualRoomSync as stopActiveManualRoomSync,
} from '@/game';
import { getFormat } from '@/lib/formats';
import { applyPrompt } from './gameStore.constants';
import type { GameState, GameConfig } from './gameStore.types';
import type { AgentPrompt } from './gameStore.types';
import type { Card, Deck, GameView } from '@/types/openmagic';
import type { GameRuntime, ManualTabletopApi } from '@/game';

export type { AgentPrompt, GameConfig, GameState, DisplayEvent, DeferredSnapshot } from './gameStore.types';

function formatMissingCardsMessage(deckLabel: string, missingCards: string[]): string {
  const preview = missingCards.slice(0, 8).join(', ');
  const extra = missingCards.length > 8 ? ` (+${missingCards.length - 8} more)` : '';
  return `${deckLabel} contains cards not available in the web engine bundle: ${preview}${extra}`;
}

function isManualTabletopApi(
  runtime: GameRuntime,
): runtime is GameRuntime & { api: ManualTabletopApi } {
  return runtime.capabilities.manualTabletop && 'applyManualAction' in runtime.api;
}

function manualZoneCard(card: Card, playerId: string, zoneId: string): Card {
  return {
    ...card,
    id: `manual-card-${crypto.randomUUID()}`,
    controllerId: playerId,
    ownerId: playerId,
    zoneId,
    isPlayable: false,
    isSelected: false,
    isChoosable: false,
    tapped: false,
  };
}

function seedManualDeck(
  gameView: GameView,
  deck: Deck,
): { gameView: GameView; libraries: Record<string, Card[]> } {
  const playerId = gameView.players[0]?.id ?? 'player-0';
  const openingHandSize = Math.min(7, deck.cards.length);
  const hand = deck.cards
    .slice(0, openingHandSize)
    .map((card) => manualZoneCard(card, playerId, 'hand'));
  const library = deck.cards
    .slice(openingHandSize)
    .map((card) => manualZoneCard(card, playerId, 'library'));
  const commandZone = (deck.commanders ?? []).map((card) =>
    manualZoneCard(card, playerId, 'command'),
  );

  return {
    gameView: {
      ...gameView,
      myHand: hand,
      myCommandZone: commandZone,
      players: gameView.players.map((player) =>
        player.id === playerId
          ? {
              ...player,
              handCount: hand.length,
              libraryCount: library.length,
            }
          : player,
      ),
    },
    libraries: {
      [playerId]: library,
    },
  };
}

export const useGameStore = create<GameState>()(devtools((set, get) => ({
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

  startGame: async (deckList, formatId, commanderName, opponentDeckList) => {
    try {
      set({ debugInfo: 'Starting game...' });
      const format = formatId ? getFormat(formatId) : undefined;
      const startingLife = format?.deckRules.startingLife ?? 20;
      const gameConfig: GameConfig = { formatId: formatId ?? 'standard', startingLife };
      set({ gameConfig });
      const runtime = getSelectedGameRuntime();
      const playerAvailability = await runtime.api.validateDeckAvailability(deckList);
      if (!playerAvailability.supported) {
        throw new Error(formatMissingCardsMessage('Selected deck', playerAvailability.missingCards));
      }
      if (opponentDeckList) {
        const opponentAvailability = await runtime.api.validateDeckAvailability(opponentDeckList);
        if (!opponentAvailability.supported) {
          throw new Error(formatMissingCardsMessage('Opponent deck', opponentAvailability.missingCards));
        }
      }
      const result = await runtime.api.startGame({
        deckList: deckList,
        startingLife,
        commanderName: commanderName ?? null,
        opponentDeckList: opponentDeckList ?? null,
      });
      // Clear old game state so stale gameView/prompts don't bleed into new game
      set({ isGameActive: true, gameLog: [], snapshots: [], gameView: null, currentPrompt: null, deferredQueue: [], isFlashing: false, isWaitingForResponse: false, debugInfo: `Game started: ${result}. Polling...` });
      if (runtime.capabilities.manualTabletop) {
        const prompt = await runtime.api.getPrompt();
        if (prompt && (prompt as AgentPrompt).gameView) {
          applyPrompt(prompt as AgentPrompt, 'Manual', set, get);
        }
      }
    } catch (e) {
      set({ debugInfo: `Start failed: ${e}` });
      console.error('[store] Failed to start game:', e);
      toast.error(e instanceof Error ? e.message : 'Failed to start game');
    }
  },

  startManualTabletopGame: async (deck?: Deck) => {
    selectGameRuntime('manual-tabletop');
    await get().startGame([], deck?.format ?? 'standard', undefined, []);
    if (!deck) return;

    const runtime = getSelectedGameRuntime();
    if (!isManualTabletopApi(runtime)) return;
    const gameView = runtime.api.getGameView();
    if (!gameView) return;

    await runtime.api.applyManualAction({
      type: 'replaceState',
      ...seedManualDeck(gameView, deck),
    });
    const prompt = await runtime.api.getPrompt();
    if (prompt && (prompt as AgentPrompt).gameView) {
      applyPrompt(prompt as AgentPrompt, 'Manual', set, get);
    }
  },

  startManualRoomHost: async (localPlayerSlot: string) => {
    const runtime = getSelectedGameRuntime();
    if (!isManualTabletopApi(runtime)) {
      throw new Error('Manual room host requires the manual tabletop runtime.');
    }
    const roomHost = new BroadcastRoomHost({
      localPlayerSlot,
      mode: 'authoritative-host',
      seats: [
        {
          kind: 'local-human',
          playerSlot: localPlayerSlot,
          displayName: 'You',
        },
      ],
    });
    startManualRoomSync({ roomHost, api: runtime.api });
    const gameView = runtime.api.getGameView();
    if (gameView) {
      await roomHost.broadcastManualState(gameView);
    }
  },

  startManualRoomClient: async (localPlayerSlot: string) => {
    selectGameRuntime('manual-tabletop');
    const runtime = getSelectedGameRuntime();
    if (!isManualTabletopApi(runtime)) {
      throw new Error('Manual room client requires the manual tabletop runtime.');
    }
    const roomHost = new BroadcastRoomHost({
      localPlayerSlot,
      mode: 'relay-client',
      seats: [
        {
          kind: 'local-human',
          playerSlot: localPlayerSlot,
          displayName: 'You',
        },
      ],
    });
    startManualRoomSync({ roomHost, api: runtime.api });
    set({
      isGameActive: true,
      isMultiplayer: true,
      isHost: false,
      myPlayerSlot: localPlayerSlot,
      debugInfo: 'Manual room client connected. Waiting for table state...',
    });
  },

  stopManualRoomSync: () => {
    stopActiveManualRoomSync();
  },

  startMultiplayerGame: async (playerNames, deckLists, commanderNames, enginePlayerIndex, localIsHost, startingLife) => {
    try {
      set({ debugInfo: 'Starting multiplayer game...' });
      resetSelectedGameRuntime();
      const runtime = getSelectedGameRuntime();
      for (const [index, deckList] of deckLists.entries()) {
        const availability = await runtime.api.validateDeckAvailability(deckList);
        if (!availability.supported) {
          throw new Error(formatMissingCardsMessage(`Player ${index + 1} deck`, availability.missingCards));
        }
      }
      await runtime.api.startMultiplayerGame({
        playerNames,
        deckLists,
        commanderNames,
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
      toast.error(e instanceof Error ? e.message : 'Failed to start multiplayer game');
    }
  },

  respond: async (action) => {
    try {
      set({ isWaitingForResponse: true, debugInfo: `Responding: ${action.type}` });
      const { myPlayerSlot } = get();
      const runtime = getSelectedGameRuntime();
      await runtime.api.respond({ action, playerSlot: myPlayerSlot });
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

  tapLand: (cardId, abilityIndex, color) => {
    get().respond({ type: 'tapLand', cardId, abilityIndex: abilityIndex ?? null, color: color ?? null });
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

  payManaCost: (auto = false) => {
    get().respond({ type: 'payManaCost', auto });
  },

  autoManaCost: () => {
    get().respond({ type: 'payManaCost', auto: true });
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
    const runtime = getSelectedGameRuntime();
    if (runtime.capabilities.concedeBehavior === 'end-session') {
      void get().endGame();
      return;
    }
    get().respond({ type: 'concede' });
  },

  endGame: async () => {
    try {
      const runtime = getSelectedGameRuntime();
      await runtime.api.endGame();
      stopActiveManualRoomSync();
      resetSelectedGameRuntime();
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
    const runtime = getSelectedGameRuntime();
    await runtime.api.restoreSnapshot({ checkpointId });
    set({ debugInfo: `Requested snapshot restore: #${checkpointId}` });
  },
}), { name: "game", enabled: import.meta.env.DEV }));
