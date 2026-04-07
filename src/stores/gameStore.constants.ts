import type { AgentPrompt, GameState, DeferredSnapshot } from './gameStore.types';
import type { GameView } from '@/types/openmagic';
import type { GameLogEntry } from '@/types/gameLog';
import { PromptType } from '@/types/promptType';

/** Prompt types the UI knows how to render a modal/interaction for. */
export const HANDLED_PROMPT_TYPES = new Set<PromptType>([
  PromptType.StateUpdate,
  PromptType.GameOver,
  PromptType.Mulligan,
  PromptType.MulliganPutBack,
  PromptType.ChooseAction,
  PromptType.ChooseAttackers,
  PromptType.ChooseBlockers,
  PromptType.ChooseTargetCard,
  PromptType.ChooseTargetCardFromZone,
  PromptType.ChooseTargetPlayer,
  PromptType.ChooseTargetAny,
  PromptType.ChooseTargetSpell,
  PromptType.ChooseMode,
  PromptType.ChooseOptionalTrigger,
  PromptType.ChoosePhyrexian,
  PromptType.ChooseKicker,
  PromptType.ChooseBuyback,
  PromptType.ChooseMultikicker,
  PromptType.ChooseReplicate,
  PromptType.ChooseAlternativeCost,
  PromptType.ChooseColor,
  PromptType.ChooseCardsForEffect,
  PromptType.ChooseType,
  PromptType.ChooseNumber,
  PromptType.ChooseCardName,
  PromptType.ChooseDiscard,
  PromptType.ChooseDamageAssignmentOrder,
  PromptType.ChooseCombatDamageAssignment,
  PromptType.PayCombatCost,
  PromptType.PayManaCost,
  PromptType.ChooseDelve,
  PromptType.ChooseConvoke,
  PromptType.ChooseImprovise,
  PromptType.SpecifyManaCombo,
  PromptType.Scry,
  PromptType.Surveil,
  PromptType.Dig,
  PromptType.ChooseExertAttackers,
  PromptType.ChooseEnlistAttackers,
  PromptType.ReorderLibrary,
  PromptType.ExploreDecision,
  PromptType.HelpPayAssist,
]);

function normalizeGameView(
  nextView: GameView,
  currentView: GameView | null,
): GameView {
  const incoming = (nextView ?? {}) as Partial<GameView>;
  const current = currentView ?? null;

  return {
    gameId: incoming.gameId ?? current?.gameId ?? "",
    turn: incoming.turn ?? current?.turn ?? 0,
    step: incoming.step ?? current?.step ?? "",
    combatAssignments: Array.isArray(incoming.combatAssignments)
      ? incoming.combatAssignments
      : (current?.combatAssignments ?? []),
    activePlayerId: incoming.activePlayerId ?? current?.activePlayerId ?? "",
    priorityPlayerId: incoming.priorityPlayerId ?? current?.priorityPlayerId ?? "",
    players: Array.isArray(incoming.players) ? incoming.players : (current?.players ?? []),
    myHand: Array.isArray(incoming.myHand) ? incoming.myHand : (current?.myHand ?? []),
    battlefield: Array.isArray(incoming.battlefield)
      ? incoming.battlefield
      : (current?.battlefield ?? []),
    stack: Array.isArray(incoming.stack) ? incoming.stack : (current?.stack ?? []),
    exile: Array.isArray(incoming.exile) ? incoming.exile : (current?.exile ?? []),
    graveyard: Array.isArray(incoming.graveyard)
      ? incoming.graveyard
      : (current?.graveyard ?? []),
    opponentGraveyard: Array.isArray(incoming.opponentGraveyard)
      ? incoming.opponentGraveyard
      : (current?.opponentGraveyard ?? []),
    opponentExile: Array.isArray(incoming.opponentExile)
      ? incoming.opponentExile
      : (current?.opponentExile ?? []),
    myCommandZone: Array.isArray(incoming.myCommandZone)
      ? incoming.myCommandZone
      : (current?.myCommandZone ?? []),
    opponentCommandZone: Array.isArray(incoming.opponentCommandZone)
      ? incoming.opponentCommandZone
      : (current?.opponentCommandZone ?? []),
    gameOver: incoming.gameOver ?? current?.gameOver,
    winnerId: incoming.winnerId ?? current?.winnerId ?? null,
    monarchId: incoming.monarchId ?? current?.monarchId ?? null,
    initiativeHolderId:
      incoming.initiativeHolderId ?? current?.initiativeHolderId ?? null,
  };
}

export function applyPrompt(prompt: AgentPrompt, source: string, set: (partial: Partial<GameState>) => void, get: () => GameState) {
  const displayEvents = [...(prompt.displayEvents ?? [])];
  // Don't mutate the original payload (listeners may fire more than once).

  const currentGameView = get().gameView;
  const normalizedGameView = normalizeGameView(prompt.gameView, currentGameView);
  const queueLen = get().deferredQueue.length;
  // stateUpdate prompts only carry a gameView + display events — they should
  // NOT replace the currentPrompt (the active player decision).
  const isStateUpdate = prompt.type === PromptType.StateUpdate;

  // DEV warning: detect prompt types the UI doesn't handle (engine takes a default/arbitrary action)
  if (!isStateUpdate && !HANDLED_PROMPT_TYPES.has(prompt.type)) {
    const cardName = prompt.sourceCardName ?? prompt.cardName ?? prompt.attackerName ?? "unknown";
    const details = JSON.stringify(prompt, null, 2);
    const devMsg = `[DEV] Unhandled prompt "${prompt.type}" for card "${cardName}" — engine takes default action\n${details}`;
    console.warn(devMsg, prompt);
    const devEntry: GameLogEntry = {
      message: devMsg,
      entryType: "warning",
      timestampMs: Date.now(),
    };
    set({ gameLog: [...get().gameLog.slice(-99), devEntry] });
  }

  if (displayEvents.length > 0 && currentGameView !== null) {
    // Enqueue this snapshot — the flash processor will play the events then apply the state.
    const snapshot: DeferredSnapshot = { displayEvents, gameView: normalizedGameView, prompt: isStateUpdate ? null : prompt };
    set({
      deferredQueue: [...get().deferredQueue, snapshot],
      debugInfo: `${source}: ${prompt.type} (queued #${queueLen + 1})`,
    });
  } else if (queueLen > 0 || get().isFlashing) {
    // Flashes are in progress but this prompt has no display events — enqueue with empty events
    // so it gets applied after the current flash sequence finishes.
    const snapshot: DeferredSnapshot = { displayEvents: [], gameView: normalizedGameView, prompt: isStateUpdate ? null : prompt };
    set({
      deferredQueue: [...get().deferredQueue, snapshot],
      debugInfo: `${source}: ${prompt.type} (queued-passthrough #${queueLen + 1})`,
    });
  } else {
    // No display events and no queue — apply immediately
    const updates: Partial<GameState> = {
      gameView: normalizedGameView,
      debugInfo: `${source}: ${prompt.type}`,
      isWaitingForResponse: false,
      currentPrompt: isStateUpdate ? null : prompt,
    };
    set(updates);
  }
}
