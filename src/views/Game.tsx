import { useGameStore } from "@/stores/useGameStore";
import { useGameUIStore } from "@/stores/useGameUIStore";
import { usePreferencesStore } from "@/stores/usePreferencesStore";
import { useShallow } from "zustand/react/shallow";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { createPortal } from "react-dom";
import type { Card as XMageCard, Player, StackObject, ActivatableAbilityInfo } from "@/types/openmagic";
import { Card } from "@/components/game/Card";
import { GameModals } from "@/components/game/GameModals";
import { GameOverScreen } from "@/components/game/GameOverScreen";
import { GameLoadingScreen } from "@/components/game/GameLoadingScreen";
import { ManualTabletopControls } from "@/components/game/ManualTabletopControls";
import { MainActionOverlay, RightActionPanel } from "@/components/game/panels";
import { StackDisplay } from "@/components/game/panels/StackDisplay";
import { CastingArrow } from "@/components/game/CastingArrow";
import { useCastingState } from "@/hooks/useCastingState";
import { ArrowOverlay } from "@/components/game/ArrowOverlay";
import { useGameArrows } from "@/components/game/useGameArrows";
import { PlayModePicker } from "@/components/game/PlayModePicker";
import { HAND_CARD_BASES } from "@/components/game/game.styles";
import { useHandScale } from "@/hooks/useHandScale";
import { useFlashQueue } from "@/hooks/useFlashQueue";
import { useHandDrag } from "@/hooks/useHandDrag";
import { useCardPreview } from "@/hooks/useCardPreview";
import { HoverCardPreview } from "@/components/game/HoverCardPreview";
import { usePromptEffects } from "@/hooks/usePromptEffects";
import { useCombatState } from "@/hooks/useCombatState";
import { useGameEventListeners } from "@/hooks/useGameEventListeners";
import { GameBoard } from "@/components/game/GameBoard";
import { useGameThemeColors, withAlpha } from "@/components/game/game.theme";
import { cn } from "@/lib/utils";
import { Navigate, useLocation } from "react-router-dom";
import { PromptType } from "@/types/promptType";
import { useStackUIStore } from "@/stores/useStackUIStore";
import { applyManualTabletopAction, getSelectedGameRuntime } from "@/game";
import type { HandActionOption } from "@/stores/useGameUIStore";
import type { PlacementGhost } from "@/components/game/zones/FreeBattlefield";
import type { GameRuntime, ManualTabletopApi } from "@/game";

/** Prompt types where hover card preview is allowed (no modal overlay). */
const HOVER_ALLOWED_PROMPTS = new Set<PromptType>([
  PromptType.ChooseAction,
  PromptType.ChooseAttackers,
  PromptType.ChooseBlockers,
  PromptType.ChooseTargetPlayer,
  PromptType.ChooseTargetCard,
  PromptType.ChooseTargetAny,
  PromptType.ChooseTargetCardFromZone,
  PromptType.ChooseTargetSpell,
  PromptType.PayManaCost,
  PromptType.GameOver,
]);

function isManualTabletopApi(
  runtime: GameRuntime,
): runtime is GameRuntime & { api: ManualTabletopApi } {
  return runtime.capabilities.manualTabletop && "applyManualAction" in runtime.api;
}

export default function Game() {
  const USE_STACK_FLASH_PREVIEW = true;
  const gameView = useGameStore(s => s.gameView);
  const currentPrompt = useGameStore(s => s.currentPrompt);
  const isGameActive = useGameStore(s => s.isGameActive);
  const isWaitingForResponse = useGameStore(s => s.isWaitingForResponse);
  const gameLog = useGameStore(s => s.gameLog);
  const snapshots = useGameStore(s => s.snapshots);
  const debugInfo = useGameStore(s => s.debugInfo);
  const isMultiplayer = useGameStore(s => s.isMultiplayer);
  const isHost = useGameStore(s => s.isHost);
  const selectedRuntime = getSelectedGameRuntime();
  const manualApi = isManualTabletopApi(selectedRuntime)
    ? selectedRuntime.api
    : null;
  const {
    passPriority, castSpell, declareAttackers, declareBlockers,
    targetPlayer, targetCard, targetAny,
    mulliganDecision, mulliganPutBackDecision,
    tapLand, untapLand, activateAbility,
    scryDecision, surveilDecision, digDecision, discardDecision,
    targetSpell, modeDecision, optionalTriggerDecision,
    colorDecision, chooseCardsDecision, typeDecision, numberDecision,
    cardNameDecision, respond, payCombatCost, declineCombatCost,
    payManaCost, autoManaCost, cancelManaCost, delveDecision, convokeDecision,
    improviseDecision, manaComboDecision, exploreDecision, exertDecision,
    enlistDecision, reorderLibraryDecision, assistDecision,
    concede, endGame, restoreSnapshot,
  } = useGameStore(useShallow(s => ({
    passPriority: s.passPriority, castSpell: s.castSpell,
    declareAttackers: s.declareAttackers, declareBlockers: s.declareBlockers,
    targetPlayer: s.targetPlayer, targetCard: s.targetCard, targetAny: s.targetAny,
    mulliganDecision: s.mulliganDecision, mulliganPutBackDecision: s.mulliganPutBackDecision,
    tapLand: s.tapLand, untapLand: s.untapLand, activateAbility: s.activateAbility,
    scryDecision: s.scryDecision, surveilDecision: s.surveilDecision,
    digDecision: s.digDecision, discardDecision: s.discardDecision,
    targetSpell: s.targetSpell, modeDecision: s.modeDecision,
    optionalTriggerDecision: s.optionalTriggerDecision,
    colorDecision: s.colorDecision, chooseCardsDecision: s.chooseCardsDecision,
    typeDecision: s.typeDecision, numberDecision: s.numberDecision,
    cardNameDecision: s.cardNameDecision, respond: s.respond,
    payCombatCost: s.payCombatCost, declineCombatCost: s.declineCombatCost,
    payManaCost: s.payManaCost, autoManaCost: s.autoManaCost, cancelManaCost: s.cancelManaCost,
    delveDecision: s.delveDecision, convokeDecision: s.convokeDecision,
    improviseDecision: s.improviseDecision, manaComboDecision: s.manaComboDecision,
    exploreDecision: s.exploreDecision, exertDecision: s.exertDecision,
    enlistDecision: s.enlistDecision, reorderLibraryDecision: s.reorderLibraryDecision,
    assistDecision: s.assistDecision, concede: s.concede,
    endGame: s.endGame, restoreSnapshot: s.restoreSnapshot,
  })));
  const flashDurationMs = usePreferencesStore((s) => s.flashDurationMs);
  const zonePanelSide = usePreferencesStore((s) => s.zonePanelSide);
  const zonePanelOrder = usePreferencesStore((s) => s.zonePanelOrder);
  const handSize = usePreferencesStore((s) => s.handSize);
  const vScale = useHandScale();
  const ghostCardW = Math.round(HAND_CARD_BASES[handSize].cardW * vScale);
  const ghostCardH = Math.round(HAND_CARD_BASES[handSize].cardH * vScale);
  const themeColors = useGameThemeColors();
  const location = useLocation();
  const devExtraOpponents = ((location.state as { devExtraOpponents?: number } | null)?.devExtraOpponents ?? 0);
  const containerRef = useRef<HTMLDivElement>(null);

  const activePrompt = manualApi ? null : currentPrompt;
  const promptType = activePrompt?.type;

  // Accumulating cache of every card we've ever observed across the
  // player's visible zones. Used as a fallback so we can resolve a casting
  // spell that the engine has temporarily removed from every live zone
  // (between hand → in-flight → stack).
  const knownCardsRef = useRef<Map<string, XMageCard>>(new Map());
  useEffect(() => {
    if (!gameView) return;
    const cache = knownCardsRef.current;
    const ingest = (cards: XMageCard[] | undefined) => {
      if (!cards) return;
      for (const c of cards) cache.set(c.id, c);
    };
    ingest(gameView.myHand);
    ingest(gameView.battlefield);
    ingest(gameView.graveyard);
    ingest(gameView.exile);
    ingest(gameView.myCommandZone);
    ingest(gameView.opponentCommandZone);
    ingest(gameView.opponentGraveyard);
    ingest(gameView.opponentExile);
  }, [gameView]);
  const resolveKnownCard = useCallback(
    (id: string) => knownCardsRef.current.get(id),
    [],
  );

  const casting = useCastingState({
    currentPrompt: activePrompt,
    hand: gameView?.myHand ?? [],
    battlefield: gameView?.battlefield ?? [],
    stack: gameView?.stack ?? [],
    resolveKnownCard,
    targetCard,
    targetPlayer,
    targetAny,
  });

  // UI state from Zustand store (modals, panels)
  const {
    abilityPicker: abilityPickerState,
    playModePicker,
    viewingZone,
    isActionPanelCollapsed,
    closeAbilityPicker,
    openPlayModePicker,
    closePlayModePicker,
    openZoneViewer,
    closeZoneViewer,
    toggleActionPanel,
  } = useGameUIStore(useShallow(s => ({
    abilityPicker: s.abilityPicker,
    playModePicker: s.playModePicker,
    viewingZone: s.viewingZone,
    isActionPanelCollapsed: s.isActionPanelCollapsed,
    closeAbilityPicker: s.closeAbilityPicker,
    openPlayModePicker: s.openPlayModePicker,
    closePlayModePicker: s.closePlayModePicker,
    openZoneViewer: s.openZoneViewer,
    closeZoneViewer: s.closeZoneViewer,
    toggleActionPanel: s.toggleActionPanel,
  })));

  /** Map an ActivatableAbilityInfo to a HandActionOption. */
  const toAbilityOption = (a: { cardId: string; abilityIndex: number; description: string; isManaAbility: boolean; cost?: string }): HandActionOption => ({
    kind: "ability" as const,
    cardId: a.cardId,
    abilityIndex: a.abilityIndex,
    label: a.description,
    isManaAbility: a.isManaAbility,
    cost: a.cost,
  });

  const castOptionsByCardId = useMemo(() => {
    const map = new Map<string, HandActionOption[]>();
    for (const o of activePrompt?.playableOptions ?? []) {
      const arr = map.get(o.cardId) ?? [];
      arr.push({ kind: "cast" as const, cardId: o.cardId, mode: o.mode, label: o.modeLabel });
      map.set(o.cardId, arr);
    }
    return map;
  }, [activePrompt?.playableOptions]);

  const abilitiesByCardId = useMemo(() => {
    const map = new Map<string, HandActionOption[]>();
    for (const a of activePrompt?.activatableAbilityIds ?? []) {
      const arr = map.get(a.cardId) ?? [];
      arr.push(toAbilityOption(a));
      map.set(a.cardId, arr);
    }
    return map;
  }, [activePrompt?.activatableAbilityIds]);

  const manaAbilitiesByCardId = useMemo(() => {
    const map = new Map<string, HandActionOption[]>();
    const rawOptions = activePrompt?.manaAbilityOptions ?? [];
    if (rawOptions.length === 0) return map;
    const ANY_COLOR_LETTERS = ["W", "U", "B", "R", "G"];
    const byCard = new Map<string, ActivatableAbilityInfo[]>();
    for (const ab of rawOptions) {
      const arr = byCard.get(ab.cardId) ?? [];
      arr.push(ab);
      byCard.set(ab.cardId, arr);
    }
    for (const [cardId, abilities] of byCard) {
      const expanded: ActivatableAbilityInfo[] = [];
      for (const ab of abilities) {
        const desc = ab.description.toLowerCase();
        const matches = ab.description.matchAll(/\{([WUBRGC])\}/g);
        const letters = Array.from(matches, (m) => m[1]);
        const isAnyColor = desc.includes("any color") || desc.includes("any one color") || desc.includes("mana of any color");
        if (letters.length > 1) {
          letters.forEach((letter) => expanded.push({ ...ab, description: `Add {${letter}}` }));
        } else if (letters.length === 1) {
          expanded.push(ab);
        } else if (isAnyColor) {
          ANY_COLOR_LETTERS.forEach((letter) => expanded.push({ ...ab, description: `Add {${letter}}` }));
        } else {
          expanded.push(ab);
        }
      }
      map.set(cardId, expanded.map(toAbilityOption));
    }
    return map;
  }, [activePrompt?.manaAbilityOptions]);

  const tappableLandIdSet = useMemo(
    () => new Set(activePrompt?.tappableLandIds ?? []),
    [activePrompt?.tappableLandIds],
  );

  const applyManualAction = useCallback(
    async (action: Parameters<typeof applyManualTabletopAction>[1]) => {
      if (!manualApi) return;
      const nextView = await applyManualTabletopAction(manualApi, action);
      if (nextView) useGameStore.setState({ gameView: nextView });
    },
    [manualApi],
  );

  const getManualCardActions = useCallback((card: XMageCard): HandActionOption[] => {
    if (!manualApi) return [];
    const humanPlayerId = gameView?.players[0]?.id;
    const ownsHumanZone = card.controllerId === humanPlayerId || card.ownerId === humanPlayerId;
    const graveyardZone = ownsHumanZone ? "graveyard" : "opponentGraveyard";
    const exileZone = ownsHumanZone ? "exile" : "opponentExile";
    const commandZone = ownsHumanZone ? "command" : "opponentCommand";
    const move = (label: string, toZoneId: string): HandActionOption => ({
      kind: "manual-move",
      cardId: card.id,
      label,
      toZoneId,
    });

    if (card.zoneId === "battlefield") {
      return [
        {
          kind: "manual-tap",
          cardId: card.id,
          label: card.tapped ? "Untap" : "Tap",
          tapped: !card.tapped,
        },
        move("Move to Hand", "hand"),
        move("Move to Graveyard", graveyardZone),
        move("Move to Exile", exileZone),
        move("Move to Command", commandZone),
      ];
    }

    return [
      move("Put onto Battlefield", "battlefield"),
      move("Move to Graveyard", graveyardZone),
      move("Move to Exile", exileZone),
      move("Move to Command", commandZone),
    ];
  }, [manualApi, gameView?.players]);

  const getHandActionOptions = useCallback((card: XMageCard): HandActionOption[] =>
    manualApi
      ? getManualCardActions(card)
      : [...(castOptionsByCardId.get(card.id) ?? []), ...(abilitiesByCardId.get(card.id) ?? [])],
    [manualApi, getManualCardActions, castOptionsByCardId, abilitiesByCardId]);

  const getBattlefieldAbilityOptions = useCallback((card: XMageCard): HandActionOption[] =>
    abilitiesByCardId.get(card.id) ?? [], [abilitiesByCardId]);

  /** All available actions for a card (cast + activated + mana abilities). */
  const getCardActions = useCallback((card: XMageCard): HandActionOption[] => {
    if (manualApi) return getManualCardActions(card);
    if (promptType === PromptType.PayManaCost) {
      return manaAbilitiesByCardId.get(card.id) ?? [];
    }
    if (promptType !== PromptType.ChooseAction) return [];

    const abilities = [...(abilitiesByCardId.get(card.id) ?? [])];
    const manaAbilities = manaAbilitiesByCardId.get(card.id) ?? [];
    const isLandTappable = tappableLandIdSet.has(card.id) && card.types?.includes("Land");

    if (isLandTappable && manaAbilities.length > 0) {
      // Use explicit mana abilities emitted by the engine instead of inventing a generic land tap action.
      abilities.unshift(...manaAbilities);
    }
    return [...(castOptionsByCardId.get(card.id) ?? []), ...abilities];
  }, [manualApi, getManualCardActions, promptType, castOptionsByCardId, abilitiesByCardId, manaAbilitiesByCardId, tappableLandIdSet]);

  // Wraps castSpell: if a card has multiple play modes, show picker first
  const handleCastSpell = (cardId: string) => {
    const options = activePrompt?.playableOptions?.filter((o) => o.cardId === cardId);
    if (options && options.length > 1) {
      const cardName = gameView?.myHand?.find((c) => c.id === cardId)?.name
        ?? gameView?.graveyard?.find((c) => c.id === cardId)?.name
        ?? gameView?.exile?.find((c) => c.id === cardId)?.name
        ?? "Card";
      openPlayModePicker({ cardId, cardName, options });
    } else if (options && options.length === 1) {
      castSpell(cardId, options[0].mode);
    } else {
      castSpell(cardId);
    }
  };

  const handleHandCardAction = (card: XMageCard, e?: React.MouseEvent) => {
    if (manualApi) {
      preview.showSticky(card, e?.clientX, e?.clientY);
      return;
    }
    const actions = getHandActionOptions(card);
    if (actions.length === 0) {
      if (card.isPlayable) {
        handleCastSpell(card.id);
      }
      return;
    }

    if (actions.length === 1) {
      const [action] = actions;
      if (action.kind === "cast") {
        castSpell(card.id, action.mode);
      } else if (action.abilityIndex != null) {
        activateAbility(card.id, action.abilityIndex);
      }
      return;
    }

    // Multiple actions — show the interactive preview without sending anything to the engine
    preview.showSticky(card, e?.clientX, e?.clientY);
  };

  const handleHandCardDragStart = (card: XMageCard, e: React.MouseEvent) => {
    if (manualApi) {
      preview.showSticky(card, e.clientX, e.clientY);
      return;
    }
    const actions = getHandActionOptions(card);
    if (actions.length > 1 || actions.some((action) => action.kind === "ability")) {
      handleHandCardAction(card, e);
      return;
    }
    startHandCardDrag(card, e);
  };

  const handleBattlefieldCardAction = (card: XMageCard, e?: React.MouseEvent) => {
    const abilities = getBattlefieldAbilityOptions(card);
    if (abilities.length === 0) return false;

    if (abilities.length === 1) {
      const ability = abilities[0];
      if (ability.kind === "ability" && ability.abilityIndex != null) {
        activateAbility(card.id, ability.abilityIndex);
        return true;
      }
      return false;
    }

    // Multiple abilities — show the interactive preview without sending anything
    preview.showSticky(card, e?.clientX, e?.clientY);
    return true;
  };

  // Combat state + battlefield/targeting click handlers
  const {
    pendingAttackers,
    pendingAttacker,
    blockAssignments,
    playerIsTargetable,
    handleTargetPlayer,
    handleBattlefieldClick,
    handleAttackerClick,
  } = useCombatState({
    promptType,
    targetCard: casting.wrappedTargetCard,
    targetAny: casting.wrappedTargetAny,
    targetPlayer: casting.wrappedTargetPlayer,
    currentPrompt: activePrompt,
  });

  // Zone viewer helpers (wrap store actions)
  function openZone(title: string, cards: XMageCard[], onClickCard?: (cardId: string) => void) {
    openZoneViewer({ title, cards, onClickCard });
  }
  function openManualZone(title: string, cards: XMageCard[]) {
    openZoneViewer({
      title,
      cards: cards.map((card) => ({ ...card, isPlayable: true })),
      onClickCard: (cardId) => {
        const card = cards.find((candidate) => candidate.id === cardId);
        closeZoneViewer();
        if (!card) return;
        void applyManualAction({
          type: "moveCard",
          cardId,
          fromZoneId: card.zoneId,
          toZoneId: "battlefield",
        });
      },
    });
  }
  function closeZone() {
    closeZoneViewer();
  }
  function openZoneAndCast(title: string, cards: XMageCard[], onClickCard: (cardId: string) => void) {
    openZoneViewer({ title, cards, onClickCard: (cardId) => {
      closeZoneViewer();
      onClickCard(cardId);
    }});
  }

  // Land tap/untap handler — shows interactive preview for multi-ability lands
  const handleTapLand = (card: XMageCard) => {
    if (promptType === PromptType.PayManaCost) {
      const manaAbilities = (activePrompt?.manaAbilityOptions ?? [])
        .filter((a) => a.cardId === card.id)
        .map((ability) => ({
          kind: "ability" as const,
          cardId: ability.cardId,
          abilityIndex: ability.abilityIndex,
          label: ability.description,
          isManaAbility: true,
          cost: ability.cost,
        }));

      if (manaAbilities.length > 1) {
        preview.showSticky(card);
        return;
      }
      if (manaAbilities.length === 1) {
        tapLand(card.id, manaAbilities[0].abilityIndex);
        return;
      }
      tapLand(card.id);
      return;
    }

    if (promptType !== PromptType.ChooseAction) {
      tapLand(card.id);
      return;
    }

    const abilities = (activePrompt?.activatableAbilityIds ?? [])
      .filter((a) => a.cardId === card.id)
      .map((ability) => ({
        kind: "ability" as const,
        cardId: ability.cardId,
        abilityIndex: ability.abilityIndex,
        label: ability.description,
        isManaAbility: ability.isManaAbility,
        cost: ability.cost,
      }));
    const manaAbilities = (activePrompt?.manaAbilityOptions ?? [])
      .filter((a) => a.cardId === card.id);
    const isManaSource = (activePrompt?.tappableLandIds ?? []).includes(card.id);
    const hasManaAbility = isManaSource && card.types.includes("Land");

    // Multiple mana abilities (dual land) — show interactive preview for color choice
    if (manaAbilities.length > 1) {
      preview.showSticky(card);
      return;
    }
    // Single mana ability — tap directly with that ability index
    if (manaAbilities.length === 1 && abilities.length === 0) {
      tapLand(card.id, manaAbilities[0].abilityIndex);
      return;
    }

    // Multiple options — show interactive preview
    if (abilities.length > 1 || (abilities.length >= 1 && hasManaAbility)) {
      preview.showSticky(card);
    } else if (abilities.length === 1) {
      if (abilities[0].abilityIndex != null) {
        activateAbility(card.id, abilities[0].abilityIndex);
      }
    } else {
      tapLand(card.id);
    }
  };

  const handleUntapLand = (card: XMageCard) => {
    untapLand(card.id);
  };

  // Queues for tapping/untapping multiple selected lands across prompt cycles
  const pendingTapQueueRef = useRef<string[]>([]);
  const pendingUntapQueueRef = useRef<string[]>([]);

  /** Start a batch land action: execute the first immediately, queue the rest. */
  const startBatchLandAction = (
    cardIds: string[],
    queueRef: React.MutableRefObject<string[]>,
    action: (id: string) => void,
  ) => {
    if (cardIds.length === 0) return;
    const [first, ...rest] = cardIds;
    queueRef.current = rest;
    action(first);
  };

  const handleTapLands = (cardIds: string[]) =>
    startBatchLandAction(cardIds, pendingTapQueueRef, tapLand);

  const handleUntapLands = (cardIds: string[]) =>
    startBatchLandAction(cardIds, pendingUntapQueueRef, untapLand);

  /** Drain the next item from a land action queue if still valid. Returns true if an action was taken. */
  const drainQueue = (
    queueRef: React.MutableRefObject<string[]>,
    validIds: string[],
    action: (id: string) => void,
  ): boolean => {
    const queue = queueRef.current;
    if (queue.length === 0) return false;
    const valid = new Set(validIds);
    const nextId = queue.find((id) => valid.has(id));
    if (!nextId) {
      queueRef.current = [];
      return false;
    }
    queueRef.current = queue.filter((id) => id !== nextId);
    action(nextId);
    return true;
  };

  // Process pending tap/untap queues when a new prompt arrives
  useEffect(() => {
    if (isWaitingForResponse) return;
    if (!promptType) return;
    if (promptType !== PromptType.ChooseAction && promptType !== PromptType.PayManaCost) {
      pendingTapQueueRef.current = [];
      pendingUntapQueueRef.current = [];
      return;
    }
    if (drainQueue(pendingTapQueueRef, activePrompt?.tappableLandIds ?? [], tapLand)) return;
    drainQueue(pendingUntapQueueRef, activePrompt?.untappableLandIds ?? [], untapLand);
  }, [activePrompt, isWaitingForResponse, promptType, tapLand, untapLand]);

  // Prompt-driven effects: auto-pass, passUntilEot, library peek, zone target, spell stack
  const {
    isAutoPassing,
    isPassingUntilEot,
    activatePassUntilEot,
    libraryPeekModal,
    setLibraryPeekModal,
    zoneTargetSelector,
    setZoneTargetSelector,
    spellStackModalOpen,
    setSpellStackModalOpen,
  } = usePromptEffects({
    currentPrompt: activePrompt,
    isWaitingForResponse,
    passPriority,
    myHand: gameView?.myHand ?? [],
    turn: gameView?.turn ?? 0,
    stackLength: gameView?.stack?.length ?? 0,
  });

  const activatePassUntilEotRef = useRef(activatePassUntilEot);
  activatePassUntilEotRef.current = activatePassUntilEot;

  // Card hover preview with delayed show / auto-dismiss
  // Note: promptType is NOT a dismiss dep — modal prompt types are already guarded
  // by the render condition on CardPreview, and modal states are tracked separately.
  // Including promptType caused hover to break during autopass (rapid prompt changes).
  const preview = useCardPreview(
    [viewingZone, zoneTargetSelector, libraryPeekModal, spellStackModalOpen, abilityPickerState],
  );

  // Hand drag-to-play
  const battlefieldContainerRef = useRef<HTMLDivElement>(null);
  const handContainerRef = useRef<HTMLDivElement>(null);
  const { draggingHandCard, ghostPos, isOverBattlefield, startHandCardDrag } = useHandDrag({
    battlefieldContainerRef,
    handContainerRef,
    onCastSpell: handleCastSpell,
    dismissHover: preview.dismiss,
  });

  const hoveredCardActions = preview.hoveredCard ? getCardActions(preview.hoveredCard) : [];

  /** Handle an action selected from the hover preview. */
  const handlePreviewAction = (action: HandActionOption) => {
    preview.dismiss();
    if (action.kind === "cast") {
      castSpell(action.cardId, action.mode);
    } else if (action.kind === "manual-move" && action.toZoneId) {
      const sourceCard = [
        ...(gameView?.myHand ?? []),
        ...(gameView?.battlefield ?? []),
        ...(gameView?.graveyard ?? []),
        ...(gameView?.exile ?? []),
        ...(gameView?.myCommandZone ?? []),
      ].find((card) => card.id === action.cardId);
      void applyManualAction({
        type: "moveCard",
        cardId: action.cardId,
        fromZoneId: sourceCard?.zoneId ?? "",
        toZoneId: action.toZoneId,
      });
    } else if (action.kind === "manual-tap") {
      void applyManualAction({
        type: "tapCard",
        cardId: action.cardId,
        tapped: action.tapped ?? true,
      });
    } else if (action.abilityIndex != null) {
      if (action.isManaAbility) {
        // Mana abilities use tapLand (ActivateMana) in both ChooseAction and PayManaCost.
        // Extract color from label (e.g. "Add {G}") if present.
        const matches = action.label.match(/\{([WUBRGC])\}/);
        const color = matches ? matches[1] : undefined;
        tapLand(action.cardId, action.abilityIndex, color);
      } else {
        activateAbility(action.cardId, action.abilityIndex);
      }
    }
  };

  // Display flash queue
  const activeFlash = useFlashQueue(flashDurationMs);

  // Debounced priority highlight to avoid rapid border strobing during autopass.
  const [priorityHighlightPlayerId, setPriorityHighlightPlayerId] = useState<string | null>(null);
  useEffect(() => {
    const next = gameView?.priorityPlayerId ?? null;
    if (priorityHighlightPlayerId == null || next == null) {
      setPriorityHighlightPlayerId(next);
      return;
    }
    if (next === priorityHighlightPlayerId) return;
    const timer = setTimeout(() => {
      setPriorityHighlightPlayerId(next);
    }, 160);
    return () => clearTimeout(timer);
  }, [gameView?.priorityPlayerId, priorityHighlightPlayerId]);

  // Set up event listeners on mount
  useGameEventListeners();

  // Keyboard shortcuts
  useEffect(() => {
    function handleKey(e: KeyboardEvent) {
      if (
        e.target instanceof HTMLInputElement ||
        e.target instanceof HTMLTextAreaElement
      )
        return;
      if (manualApi) return;
      if (e.code === "Space") {
        e.preventDefault();
        passPriority();
      } else if (e.code === "F6") {
        e.preventDefault();
        activatePassUntilEotRef.current();
      }
    }
    window.addEventListener("keydown", handleKey);
    return () => window.removeEventListener("keydown", handleKey);
  }, [passPriority]);

  // Targeting / combat arrows — must be called unconditionally (Rules of Hooks)
  const me = gameView?.players?.find((p) => p.isHuman) ?? gameView?.players?.[0];
  const opponents = gameView?.players?.filter((p) => !p.isHuman) ?? [];
  const opponent = opponents[0]; // alias for arrows hook + game-over screen
  // DEV: pad with simulated opponents to test multi-player layout
  const displayOpponents = [
    ...opponents,
    ...Array.from({ length: devExtraOpponents }, (_, i) => ({
      id: `dev-fake-${i}`,
      name: `Dev Opp ${opponents.length + i + 1}`,
      isHuman: false,
      life: 20,
      poison: 0,
      handCount: 7,
      libraryCount: 40,
      graveyardCount: 0,
      exileCount: 0,
      manaPool: {} as Record<string, number>,
    } as Player)),
  ];
  // Stabilize attackerIds so useGameArrows' useEffect doesn't re-run every render
  const attackerIds = useMemo(
    () => activePrompt?.attackerIds ?? [],
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [activePrompt?.attackerIds?.join(",")],
  );
  const combatAssignments = useMemo(
    () => gameView?.combatAssignments ?? [],
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [gameView?.combatAssignments?.map((a) => `${a.blockerId}:${a.attackerId}`).join(",")],
  );

  const arrows = useGameArrows({
    containerRef,
    promptType,
    attackerIds,
    blockAssignments,
    combatAssignments,
    pendingAttackers,
    myPlayerId: me?.id ?? "",
    opponentPlayerId: opponent?.id ?? "",
    stack: gameView?.stack ?? [],
  });

  const hoveredStackObjectId = useStackUIStore((s) => s.hoveredStackObjectId);
  const placementGhost = useMemo((): PlacementGhost | null => {
    const stack = gameView?.stack;
    if (!stack || stack.length === 0) return null;
    const active =
      (hoveredStackObjectId
        ? stack.find((obj) => obj.id === hoveredStackObjectId)
        : null) ?? stack[stack.length - 1];
    const hasTargets = (active.targets ?? []).length > 0;
    if (hasTargets) return null;
    if (!active.isPermanentSpell) return null;
    return { stackObjectId: active.id, cardName: active.name, controllerId: active.controllerId };
  }, [gameView?.stack, hoveredStackObjectId]);

  const visibleCardsById = useMemo(() => {
    if (!gameView) return new Map<string, XMageCard>();
    const cards: XMageCard[] = [
      ...gameView.battlefield,
      ...gameView.myHand,
      ...gameView.graveyard,
      ...gameView.exile,
      ...gameView.opponentGraveyard,
      ...gameView.opponentExile,
      ...(gameView.myCommandZone ?? []),
      ...(gameView.opponentCommandZone ?? []),
    ];
    return new Map(cards.map((c) => [c.id, c]));
  }, [gameView?.battlefield, gameView?.myHand, gameView?.graveyard, gameView?.exile, gameView?.opponentGraveyard, gameView?.opponentExile, gameView?.myCommandZone, gameView?.opponentCommandZone]);

  const stackCardsBySourceId = useMemo(() => {
    if (!gameView?.stack) return new Map<string, XMageCard>();
    const byId = new Map<string, XMageCard>();
    for (const s of gameView.stack) {
      if (byId.has(s.sourceId)) continue;
      byId.set(s.sourceId, {
        id: s.sourceId,
        name: s.name,
        setCode: "",
        cardNumber: "",
        color: "",
        manaCost: "",
        types: [],
        subtypes: [],
        supertypes: [],
        text: s.text,
        isPlayable: false,
        isSelected: false,
        isChoosable: false,
        controllerId: "",
        ownerId: "",
        zoneId: "",
      });
    }
    return byId;
  }, [gameView?.stack]);

  const handleLogCardHover = (cardId: string | null, e?: React.MouseEvent, options: { useAnchor?: boolean; placement?: "auto" | "top-center"; anchorOverride?: DOMRect } = {}) => {
    if (draggingHandCard) {
      preview.dismiss();
      return;
    }
    if (!cardId) {
      preview.dismiss();
      return;
    }
    const card = visibleCardsById.get(cardId) ?? stackCardsBySourceId.get(cardId);
    if (!card) { preview.dismiss(); return; }
    preview.handleMouseEnter(card, e, { ...options, useDelay: true });
  };

  const handleHoverCardGuarded = (card: XMageCard | null, e?: React.MouseEvent, options: { useAnchor?: boolean; placement?: "auto" | "top-center"; anchorOverride?: DOMRect } = {}) => {
    if (draggingHandCard) {
      preview.dismiss();
      return;
    }
    if (card === null) {
      // Use handleMouseLeave so the 250ms grace period allows the user
      // to move the mouse from the card to the preview popup.
      preview.handleMouseLeave();
    } else {
      preview.handleMouseEnter(card, e, { ...options, useDelay: true });
    }
  };

  // Suppress native browser tooltips inside the game view by stripping `title`
  // attributes as they appear. We move the value to `data-title` so it's still
  // accessible to custom tooltip components if needed, but the browser won't
  // show the default tooltip on hover.
  useEffect(() => {
    const el = containerRef.current;
    if (!el) return;
    const strip = (root: Element) => {
      for (const node of root.querySelectorAll("[title]")) {
        const val = node.getAttribute("title");
        if (val) {
          node.setAttribute("data-title", val);
          node.removeAttribute("title");
        }
      }
    };
    strip(el);
    const observer = new MutationObserver((mutations) => {
      for (const m of mutations) {
        if (m.type === "attributes" && m.attributeName === "title" && m.target instanceof Element) {
          const val = m.target.getAttribute("title");
          if (val) {
            m.target.setAttribute("data-title", val);
            m.target.removeAttribute("title");
          }
        }
        if (m.type === "childList") {
          for (const node of m.addedNodes) {
            if (node instanceof Element) strip(node);
          }
        }
      }
    });
    observer.observe(el, { attributes: true, attributeFilter: ["title"], childList: true, subtree: true });
    return () => observer.disconnect();
  }, []);

  useEffect(() => {
    if (draggingHandCard) {
      preview.dismiss();
    }
  }, [draggingHandCard, preview]);

  // If the previewed card leaves all visible zones (e.g. removed from the game),
  // close the preview. We use visibleCardsById so that cards in graveyard, exile,
  // and command zones can still be previewed (e.g. in ZoneViewer modals).
  const hoverableCardIds = useMemo(() => {
    return new Set(visibleCardsById.keys());
  }, [visibleCardsById]);

  useEffect(() => {
    if (!preview.hoveredCard) return;
    if (!hoverableCardIds.has(preview.hoveredCard.id) && !stackCardsBySourceId.has(preview.hoveredCard.id)) {
      preview.dismiss();
    }
  }, [preview, hoverableCardIds, stackCardsBySourceId]);

  const cardNameById = useMemo(() => {
    const byId = new Map<string, string>();
    for (const c of visibleCardsById.values()) {
      byId.set(c.id, c.name);
    }
    for (const [sourceId, c] of stackCardsBySourceId.entries()) {
      if (!byId.has(sourceId)) byId.set(sourceId, c.name);
    }
    return byId;
  }, [visibleCardsById, stackCardsBySourceId]);

  const playerNameById = useMemo(
    () => new Map((gameView?.players ?? []).map((p) => [p.id, p.name] as const)),
    [gameView?.players],
  );

  const resolveStackCard = (stackItem: StackObject): XMageCard =>
    visibleCardsById.get(stackItem.sourceId) ??
    stackCardsBySourceId.get(stackItem.sourceId) ?? {
      id: stackItem.sourceId,
      name: stackItem.name,
      setCode: "",
      cardNumber: "",
      color: "",
      manaCost: "",
      types: [],
      subtypes: [],
      supertypes: [],
      text: stackItem.text,
      isPlayable: false,
      isSelected: false,
      isChoosable: false,
      controllerId: "",
      ownerId: "",
      zoneId: "",
    };

  const activeFlashCard: XMageCard | null = useMemo(() => {
    if (!activeFlash || activeFlash.kind !== "card") return null;
    const knownCard =
      visibleCardsById.get(activeFlash.cardId) ??
      stackCardsBySourceId.get(activeFlash.cardId);
    return {
      id: activeFlash.cardId,
      name: activeFlash.cardName,
      setCode: activeFlash.setCode,
      cardNumber: knownCard?.cardNumber ?? "",
      color: knownCard?.color ?? "",
      manaCost: knownCard?.manaCost ?? "",
      types: knownCard?.types ?? [],
      subtypes: knownCard?.subtypes ?? [],
      supertypes: knownCard?.supertypes ?? [],
      text: knownCard?.text ?? "",
      isPlayable: false,
      isSelected: false,
      isChoosable: false,
      controllerId: knownCard?.controllerId ?? "",
      ownerId: knownCard?.ownerId ?? "",
      zoneId: knownCard?.zoneId ?? "",
    };
  }, [activeFlash, visibleCardsById, stackCardsBySourceId]);

  // Auto-return to play menu when game is over
  useEffect(() => {
    if (!gameView?.gameOver && activePrompt?.type !== PromptType.GameOver) return;
    const timer = setTimeout(() => endGame(), 3000);
    return () => clearTimeout(timer);
  }, [gameView?.gameOver, activePrompt?.type]);

  if (!isGameActive) return <Navigate to="/lobby" replace />;

  // Loading
  if (!gameView) {
    return <GameLoadingScreen debugInfo={debugInfo} />;
  }
  if (!me) {
    return <GameLoadingScreen debugInfo={debugInfo || "Waiting for player state..."} />;
  }

  const battlefieldActivatableIds = new Set(
    promptType === PromptType.ChooseAction
      ? (activePrompt?.activatableAbilityIds ?? []).map((ability) => ability.cardId)
      : [],
  );
  const myPermanents = gameView.battlefield
    .filter((c) => c.controllerId === me.id)
    .map((c) =>
      battlefieldActivatableIds.has(c.id)
        ? { ...c, isChoosable: true }
        : c,
    );
  const opponentPermanentsByPlayer = new Map(
    opponents.map((op) => [
      op.id,
      gameView.battlefield.filter((c) => c.controllerId === op.id),
    ]),
  );

  // Game over overlay
  if (gameView.gameOver || promptType === PromptType.GameOver) {
    return (
      <GameOverScreen
        winnerId={gameView.winnerId}
        me={me}
        opponents={opponents}
        turn={gameView.turn}
        onEndGame={endGame}
      />
    );
  }

  const turnFlashPlayerId =
    activeFlash?.kind === "turn" ? activeFlash.playerId : null;
  const effectivePriorityHighlightPlayerId =
    priorityHighlightPlayerId ?? gameView.priorityPlayerId;
  const shouldRenderStackFlashCard =
    activeFlash?.kind === "card";
  const shouldShowPreStackFlash =
    activeFlashCard?.types.includes("Land") ?? false;

  return (
    <div
      ref={containerRef}
      className="relative flex flex-col h-full min-h-0 gap-1.5 p-1.5 overflow-hidden select-none"
      style={
        {
          "--flash-duration": `${flashDurationMs}ms`,
          "--playable-ring-color": withAlpha(themeColors.cardRing, 0.75),
          "--playable-glow-color": withAlpha(themeColors.cardRing, 0.3),
          "--playable-ring-color-strong": themeColors.cardRing,
          "--playable-glow-color-strong": withAlpha(themeColors.cardRing, 0.6),
        } as React.CSSProperties
      }
    >
      <ArrowOverlay arrows={arrows} />
      <div className="flex gap-1 min-h-0 flex-1 overflow-visible">
        <GameBoard
          me={me}
          opponents={displayOpponents}
          myPermanents={myPermanents}
          opponentPermanentsByPlayer={opponentPermanentsByPlayer}
          myHand={gameView.myHand}
          graveyard={gameView.graveyard}
          exile={gameView.exile}
          myCommandZone={gameView.myCommandZone}
          opponentGraveyard={gameView.opponentGraveyard ?? []}
          opponentExile={gameView.opponentExile ?? []}
          opponentCommandZone={gameView.opponentCommandZone}
          activePlayerId={gameView.activePlayerId}
          priorityPlayerId={effectivePriorityHighlightPlayerId}
          step={gameView.step}
          promptType={promptType}
          currentPrompt={activePrompt}
          pendingAttackers={pendingAttackers}
          pendingAttacker={pendingAttacker}
          blockAssignments={blockAssignments}
          playerIsTargetable={playerIsTargetable}
          turnFlashPlayerId={turnFlashPlayerId}
          showBackFace={preview.showBackFace}
          zonePanelSide={zonePanelSide}
          zonePanelOrder={zonePanelOrder}
          placementGhost={placementGhost}
          isOverBattlefield={isOverBattlefield}
          battlefieldContainerRef={battlefieldContainerRef}
          handContainerRef={handContainerRef}
          draggingCardId={draggingHandCard?.id}
          castingCardId={casting.castingCardId}
          onHandCardDragStart={handleHandCardDragStart}
          onHandCardClick={handleHandCardAction}
          onHoverCard={handleHoverCardGuarded}
          onDismissHoverPreview={preview.dismiss}
          getHandActions={getHandActionOptions}
          onSelectHandAction={handlePreviewAction}
          onFlipCard={preview.flipCard}
          actionableCardIds={
            promptType === PromptType.ChooseAction
              ? (activePrompt?.activatableAbilityIds ?? []).map((ability) => ability.cardId)
              : undefined
          }
          onBattlefieldClick={(card) => {
            if (manualApi) {
              void applyManualAction({
                type: "tapCard",
                cardId: card.id,
                tapped: !card.tapped,
              });
              return;
            }
            if (promptType === PromptType.ChooseAction && handleBattlefieldCardAction(card)) {
              return;
            }
            handleBattlefieldClick(card);
          }}
          onAttackerClick={handleAttackerClick}
          onTargetPlayer={handleTargetPlayer}
          onOpenZone={(title, cards, onClickCard) => {
            if (manualApi) {
              openManualZone(title, cards);
              return;
            }
            openZone(title, cards, onClickCard);
          }}
          onOpenZoneAndCast={(title, cards, onClickCard) =>
            openZoneAndCast(title, cards, (cardId) => {
              handleCastSpell(cardId);
              onClickCard(cardId);
            })
          }
          onTapLand={
            promptType === PromptType.ChooseAction || promptType === PromptType.PayCombatCost || promptType === PromptType.PayManaCost
              ? handleTapLand
              : undefined
          }
          onTapLands={
            promptType === PromptType.ChooseAction || promptType === PromptType.PayManaCost
              ? handleTapLands
              : undefined
          }
          onTapLandAbility={(cardId, abilityIndex, color) => tapLand(cardId, abilityIndex, color)}
          onUntapLand={
            promptType === PromptType.ChooseAction || promptType === PromptType.PayCombatCost || promptType === PromptType.PayManaCost
              ? handleUntapLand
              : undefined
          }
          onUntapLands={
            promptType === PromptType.ChooseAction || promptType === PromptType.PayManaCost
              ? handleUntapLands
              : undefined
          }
        />
      </div>

      <RightActionPanel
        collapsed={isActionPanelCollapsed}
        onToggleCollapse={toggleActionPanel}
        gameLog={gameLog}
        onHoverLogCard={handleLogCardHover}
        resolveCardName={(cardId) => cardNameById.get(cardId) ?? cardId}
        resolvePlayerName={(playerId) => playerNameById.get(playerId) ?? playerId}
        snapshots={snapshots}
        canRestoreSnapshots={
          (!isMultiplayer || isHost) &&
          (promptType === PromptType.ChooseAction ||
            promptType === PromptType.ChooseAttackers ||
            promptType === PromptType.ChooseBlockers)
        }
        onRestoreSnapshot={restoreSnapshot}
      />

      {manualApi && (
        <ManualTabletopControls gameView={gameView} api={manualApi} />
      )}

      {!manualApi && (
        <MainActionOverlay
          promptType={promptType}
          isWaitingForResponse={isWaitingForResponse}
          isAutoPassing={isAutoPassing}
          isPassingUntilEot={isPassingUntilEot}
          availableAttackerIds={activePrompt?.availableAttackerIds ?? []}
          pendingAttackers={pendingAttackers}
          onPassPriority={passPriority}
          onPassUntilEot={activatePassUntilEot}
          onDeclareAttackers={declareAttackers}
          pendingAttacker={pendingAttacker}
          attackerIds={activePrompt?.attackerIds ?? []}
          blockAssignments={blockAssignments}
          onDeclareBlockers={declareBlockers}
          onOpenStack={() => setSpellStackModalOpen(true)}
          onConcede={concede}
          resolveCardName={(cardId) => cardNameById.get(cardId) ?? cardId}
          isMyPriority={gameView.priorityPlayerId === me.id}
          turn={gameView.turn}
          activePlayerName={
            gameView.players.find((p) => p.id === gameView.activePlayerId)?.name ??
            "Unknown"
          }
          isMyTurn={gameView.activePlayerId === me.id}
          step={gameView.step}
          payManaCostInfo={
            promptType === PromptType.PayManaCost && activePrompt?.manaCost != null
              ? {
                  cardName: activePrompt.cardName ?? "Spell",
                  manaCost: activePrompt.manaCost,
                  manaPool: activePrompt.gameView?.players?.find(p => p.isHuman)?.manaPool ?? {},
                  canConfirmFromPool: activePrompt.canConfirmFromPool ?? false,
                }
              : null
          }
          onPayManaCost={payManaCost}
          onAutoManaCost={autoManaCost}
          onCancelManaCost={cancelManaCost}
        />
      )}

      <StackDisplay
        stack={gameView.stack}
        resolveStackCard={resolveStackCard}
        onOpenStack={() => setSpellStackModalOpen(true)}
        flashCard={
          shouldRenderStackFlashCard ? activeFlashCard : null
        }
        flashToken={
          shouldRenderStackFlashCard
            ? `${activeFlash.cardId}:${activeFlash.cardName}:${activeFlash.setCode}`
            : null
        }
        showPreStackFlash={shouldShowPreStackFlash}
        castingCard={casting.castingCard}
        rightPanelCollapsed={isActionPanelCollapsed}
      />

      {casting.showArrow && casting.castingCardId && (
        <CastingArrow castingCardId={casting.castingCardId} targetId={casting.targetId} hostile={casting.arrowHostile} />
      )}

      <GameModals
        promptType={promptType}
        currentPrompt={activePrompt}
        viewingZone={viewingZone}
        onCloseZone={closeZone}
        zoneTargetSelector={zoneTargetSelector}
        onSelectZoneTarget={(cardId) => { casting.wrappedTargetCard(cardId); setZoneTargetSelector(null); }}
        onCancelZoneTarget={() => setZoneTargetSelector(null)}
        libraryPeekModal={libraryPeekModal}
        onLibraryPeekConfirm={(selectedIds) => {
          if (libraryPeekModal!.mode === "scry") scryDecision(selectedIds);
          else if (libraryPeekModal!.mode === "surveil") surveilDecision(selectedIds);
          else if (libraryPeekModal!.mode === "discard") discardDecision(selectedIds);
          else digDecision(selectedIds);
          setLibraryPeekModal(null);
        }}
        spellStackModalOpen={spellStackModalOpen}
        stack={gameView.stack}
        validSpellIds={promptType === PromptType.ChooseTargetSpell ? (activePrompt?.validSpellIds ?? []) : []}
        onTargetSpell={(spellId) => { targetSpell(spellId); setSpellStackModalOpen(false); }}
        onCloseStack={() => setSpellStackModalOpen(false)}
        abilityPickerState={abilityPickerState}
        onSelectAbility={(ability) => {
          if (ability.kind === "cast") {
            castSpell(ability.cardId, ability.mode);
          } else if (ability.abilityIndex === -1) {
            tapLand(abilityPickerState!.cardId);
          } else if (ability.abilityIndex != null) {
            if (promptType === PromptType.PayManaCost && ability.isManaAbility) {
              tapLand(abilityPickerState!.cardId, ability.abilityIndex);
            } else {
              activateAbility(abilityPickerState!.cardId, ability.abilityIndex);
            }
          }
          closeAbilityPicker();
        }}
        onCancelAbilityPicker={closeAbilityPicker}
        onMulliganDecision={mulliganDecision}
        onMulliganPutBackDecision={mulliganPutBackDecision}
        isWaitingForResponse={isWaitingForResponse}
        myHand={gameView.myHand}
        onModeDecision={modeDecision}
        onOptionalTriggerDecision={optionalTriggerDecision}
        onPhyrexianDecision={(payLife) => respond({ type: "phyrexianDecision", payLife })}
        onKickerDecision={(kicked) => respond({ type: "kickerDecision", kicked })}
        onBuybackDecision={(paid) => respond({ type: "buybackDecision", buybackPaid: paid })}
        onMultikickerDecision={(kickCount) => respond({ type: "multikickerDecision", kickCount })}
        onReplicateDecision={(replicateCount) => respond({ type: "replicateDecision", replicateCount })}
        onAlternativeCostDecision={(chosenIndex) => respond({ type: "alternativeCostDecision", chosenIndex })}
        onColorDecision={colorDecision}
        onChooseCardsDecision={chooseCardsDecision}
        onTypeDecision={typeDecision}
        onNumberDecision={numberDecision}
        onCardNameDecision={cardNameDecision}
        onDamageOrderDecision={(orderedBlockerIds) => respond({ type: "damageAssignmentOrderDecision", orderedBlockerIds })}
        onCombatDamageAssignmentDecision={(assignments) => respond({ type: "combatDamageAssignmentDecision", assignments })}
        onPayCombatCost={payCombatCost}
        onDeclineCombatCost={declineCombatCost}
        onDelveDecision={delveDecision}
        onConvokeDecision={convokeDecision}
        onImproviseDecision={improviseDecision}
        onManaComboDecision={manaComboDecision}
        onExploreDecision={exploreDecision}
        onExertDecision={exertDecision}
        onEnlistDecision={enlistDecision}
        onReorderLibraryDecision={reorderLibraryDecision}
        onAssistDecision={assistDecision}
      />

      {playModePicker && (
        <PlayModePicker
          cardName={playModePicker.cardName}
          options={playModePicker.options}
          onSelect={(mode) => {
            castSpell(playModePicker.cardId, mode);
            closePlayModePicker();
          }}
          onCancel={closePlayModePicker}
        />
      )}

      {/* ── Card-play flash overlay ───────────────────────── */}
      {!USE_STACK_FLASH_PREVIEW && activeFlash?.kind === "card" &&
        createPortal(
          <div
            className="fixed inset-0 z-[10000] flex items-center justify-center pointer-events-none bg-black/30 animate-card-flash-backdrop"
            style={
              {
                "--flash-duration": `${flashDurationMs}ms`,
              } as React.CSSProperties
            }
          >
            <div className="animate-card-flash">
              <Card
                card={{
                  id: activeFlash.cardId,
                  name: activeFlash.cardName,
                  setCode: activeFlash.setCode,
                  cardNumber: "",
                  color: "",
                  manaCost: "",
                  types: [],
                  subtypes: [],
                  supertypes: [],
                  text: "",
                  isPlayable: false,
                  isSelected: false,
                  isChoosable: false,
                  controllerId: "",
                  ownerId: "",
                  zoneId: "",
                }}
                className="w-[240px] h-[336px]"
              />
            </div>
          </div>,
          document.body,
        )}

      {/* ── Ghost card while dragging from hand ───────────── */}
      {draggingHandCard &&
        createPortal(
          <div
            className="fixed pointer-events-none z-[9999]"
            style={{ left: ghostPos.x - ghostCardW / 2, top: ghostPos.y - ghostCardH / 2 }}
          >
            <Card
              card={draggingHandCard}
              className={cn("shadow-2xl ring-2 ring-primary playable-card")}
              style={{ width: ghostCardW, height: ghostCardH }}
            />
          </div>,
          document.body,
        )}

      {/* ── Hover card preview ────────────────────────────── */}
      {/* Hide when any overlay modal is open or a modal-based prompt is active.
          Allow-list approach: only show the preview for prompt types that do NOT
          open a modal (battlefield interaction, targeting, inline panel prompts).
          Also hide for hand cards since the hand displays its own actions/preview. */}
      {preview.hoveredCard && preview.hoveredCard.zoneId !== "hand" && !draggingHandCard && !viewingZone && !zoneTargetSelector && !libraryPeekModal && !spellStackModalOpen &&
       !abilityPickerState &&
       (!promptType || HOVER_ALLOWED_PROMPTS.has(promptType)) && (
        <HoverCardPreview
          preview={preview}
          actions={hoveredCardActions}
          onSelectAction={handlePreviewAction}
        />
      )}
    </div>
  );
}
