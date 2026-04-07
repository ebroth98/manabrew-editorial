import { create } from 'zustand';
import { devtools } from 'zustand/middleware';
import { toast } from 'sonner';
import { getPlatform } from '@/platform';
import { getFormat } from '@/lib/formats';
import type { GameState, GameConfig } from './gameStore.types';

export type { AgentPrompt, GameConfig, GameState, DisplayEvent, DeferredSnapshot } from './gameStore.types';

function formatMissingCardsMessage(deckLabel: string, missingCards: string[]): string {
  const preview = missingCards.slice(0, 8).join(', ');
  const extra = missingCards.length > 8 ? ` (+${missingCards.length - 8} more)` : '';
  return `${deckLabel} contains cards not available in the web engine bundle: ${preview}${extra}`;
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
      const gameConfig: GameConfig = { formatId: formatId ?? 'constructed', startingLife };
      set({ gameConfig });
      const platform = getPlatform();
      const playerAvailability = await platform.game.validateDeckAvailability(deckList);
      if (!playerAvailability.supported) {
        throw new Error(formatMissingCardsMessage('Selected deck', playerAvailability.missingCards));
      }
      if (opponentDeckList) {
        const opponentAvailability = await platform.game.validateDeckAvailability(opponentDeckList);
        if (!opponentAvailability.supported) {
          throw new Error(formatMissingCardsMessage('Opponent deck', opponentAvailability.missingCards));
        }
      }
      const result = await platform.game.startGame({
        deckList: deckList,
        startingLife,
        commanderName: commanderName ?? null,
        opponentDeckList: opponentDeckList ?? null,
      });
      // Clear old game state so stale gameView/prompts don't bleed into new game
      set({ isGameActive: true, gameLog: [], snapshots: [], gameView: null, currentPrompt: null, deferredQueue: [], isFlashing: false, isWaitingForResponse: false, debugInfo: `Game started: ${result}. Polling...` });
    } catch (e) {
      set({ debugInfo: `Start failed: ${e}` });
      console.error('[store] Failed to start game:', e);
      toast.error(e instanceof Error ? e.message : 'Failed to start game');
    }
  },

  startMultiplayerGame: async (playerNames, deckLists, commanderNames, enginePlayerIndex, localIsHost, startingLife) => {
    try {
      set({ debugInfo: 'Starting multiplayer game...' });
      const platform = getPlatform();
      for (const [index, deckList] of deckLists.entries()) {
        const availability = await platform.game.validateDeckAvailability(deckList);
        if (!availability.supported) {
          throw new Error(formatMissingCardsMessage(`Player ${index + 1} deck`, availability.missingCards));
        }
      }
      await platform.game.startMultiplayerGame({
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
      const platform = getPlatform();
      await platform.game.respond({ action, playerSlot: myPlayerSlot });
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

  tapLand: (cardId, abilityIndex) => {
    get().respond({ type: 'tapLand', cardId, abilityIndex: abilityIndex ?? null });
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
      const platform = getPlatform();
      await platform.game.endGame();
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
    const platform = getPlatform();
    await platform.game.restoreSnapshot({ checkpointId });
    set({ debugInfo: `Requested snapshot restore: #${checkpointId}` });
  },
}), { name: "game", enabled: import.meta.env.DEV }));
