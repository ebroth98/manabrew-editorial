import { useGameStore } from "@/stores/useGameStore";
import { useGameUIStore } from "@/stores/useGameUIStore";
import { usePreferencesStore } from "@/stores/usePreferencesStore";
import { useEffect, useMemo, useRef, useState } from "react";
import { createPortal } from "react-dom";
import type { Card as XMageCard, Player, StackObject } from "@/types/openmagic";
import { Card } from "@/components/game/Card";
import { CardPreview } from "@/components/game/CardPreview";
import { GameModals } from "@/components/game/GameModals";
import { GameOverScreen } from "@/components/game/GameOverScreen";
import { GameLoadingScreen } from "@/components/game/GameLoadingScreen";
import { MainActionOverlay, RightActionPanel } from "@/components/game/panels";
import { StackDisplay } from "@/components/game/panels/StackDisplay";
import { ArrowOverlay } from "@/components/game/ArrowOverlay";
import { useGameArrows } from "@/components/game/useGameArrows";
import { PlayModePicker } from "@/components/game/PlayModePicker";
import { HAND_CARD } from "@/components/game/game.styles";
import { useFlashQueue } from "@/hooks/useFlashQueue";
import { useHandDrag } from "@/hooks/useHandDrag";
import { useCardHover } from "@/hooks/useCardHover";
import { usePromptEffects } from "@/hooks/usePromptEffects";
import { useCombatState } from "@/hooks/useCombatState";
import { useGameEventListeners } from "@/hooks/useGameEventListeners";
import { GameBoard } from "@/components/game/GameBoard";
import { useGameThemeColors, withAlpha } from "@/components/game/game.theme";
import { cn } from "@/lib/utils";
import { Navigate, useLocation } from "react-router-dom";
import { PromptType } from "@/types/promptType";
import { useStackUIStore } from "@/stores/useStackUIStore";
import type { PlacementGhost } from "@/components/game/zones/FreeBattlefield";

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

export default function Game() {
  const USE_STACK_FLASH_PREVIEW = true;
  const {
    gameView,
    currentPrompt,
    isGameActive,
    isWaitingForResponse,
    gameLog,
    snapshots,
    debugInfo,
    passPriority,
    castSpell,
    declareAttackers,
    declareBlockers,
    targetPlayer,
    targetCard,
    targetAny,
    mulliganDecision,
    mulliganPutBackDecision,
    tapLand,
    untapLand,
    activateAbility,
    scryDecision,
    surveilDecision,
    digDecision,
    discardDecision,
    targetSpell,
    modeDecision,
    optionalTriggerDecision,
    colorDecision,
    chooseCardsDecision,
    typeDecision,
    numberDecision,
    cardNameDecision,
    respond,
    payCombatCost,
    declineCombatCost,
    payManaCost,
    cancelManaCost,
    delveDecision,
    convokeDecision,
    improviseDecision,
    manaComboDecision,
    exploreDecision,
    exertDecision,
    enlistDecision,
    reorderLibraryDecision,
    assistDecision,
    concede,
    endGame,
    restoreSnapshot,
    isMultiplayer,
    isHost,
  } = useGameStore();
  const flashDurationMs = usePreferencesStore((s) => s.flashDurationMs);
  const zonePanelSide = usePreferencesStore((s) => s.zonePanelSide);
  const zonePanelOrder = usePreferencesStore((s) => s.zonePanelOrder);
  const themeColors = useGameThemeColors();
  const location = useLocation();
  const devExtraOpponents = ((location.state as { devExtraOpponents?: number } | null)?.devExtraOpponents ?? 0);
  const containerRef = useRef<HTMLDivElement>(null);

  const promptType = currentPrompt?.type;

  // UI state from Zustand store (modals, panels)
  const {
    abilityPicker: abilityPickerState,
    playModePicker,
    viewingZone,
    isActionPanelCollapsed,
    openAbilityPicker,
    closeAbilityPicker,
    openPlayModePicker,
    closePlayModePicker,
    openZoneViewer,
    closeZoneViewer,
    toggleActionPanel,
  } = useGameUIStore();

  // Wraps castSpell: if a card has multiple play modes, show picker first
  const handleCastSpell = (cardId: string) => {
    const options = currentPrompt?.playableOptions?.filter((o) => o.cardId === cardId);
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
    targetCard,
    targetAny,
    targetPlayer,
    currentPrompt,
  });

  // Zone viewer helpers (wrap store actions)
  function openZone(title: string, cards: XMageCard[], onClickCard?: (cardId: string) => void) {
    openZoneViewer({ title, cards, onClickCard });
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

  // Land tap/untap handler with ability picker support
  const handleTapLand = (card: XMageCard) => {
    if (promptType !== PromptType.ChooseAction) {
      tapLand(card.id);
      return;
    }

    const abilities = (currentPrompt?.activatableAbilityIds ?? [])
      .filter((a) => a.cardId === card.id);
    const isManaSource = (currentPrompt?.tappableLandIds ?? []).includes(card.id);
    const hasManaAbility = isManaSource && card.types.includes("Land");

    // If the card has both a mana ability and non-mana abilities, show picker
    if (abilities.length > 1 || (abilities.length >= 1 && hasManaAbility)) {
      const pickerAbilities = hasManaAbility
        ? [
            {
              cardId: card.id,
              abilityIndex: -1,
              description: "{T}: Tap for mana",
              isManaAbility: true,
            },
            ...abilities,
          ]
        : abilities;
      openAbilityPicker({
        cardId: card.id,
        cardName: card.name,
        abilities: pickerAbilities,
      });
    } else if (abilities.length === 1) {
      activateAbility(card.id, abilities[0].abilityIndex);
    } else {
      tapLand(card.id);
    }
  };

  const handleUntapLand = (card: XMageCard) => {
    untapLand(card.id);
  };

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
    currentPrompt,
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
  const {
    hoveredCard,
    mousePos,
    showBackFace,
    dismissHover,
    handleFlipCard,
    handleHoverCard,
  } = useCardHover(
    [viewingZone, zoneTargetSelector, libraryPeekModal, spellStackModalOpen, abilityPickerState],
  );

  // Hand drag-to-play
  const battlefieldContainerRef = useRef<HTMLDivElement>(null);
  const handContainerRef = useRef<HTMLDivElement>(null);
  const { draggingHandCard, ghostPos, isOverBattlefield, startHandCardDrag } = useHandDrag({
    battlefieldContainerRef,
    handContainerRef,
    onCastSpell: handleCastSpell,
    dismissHover,
  });

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
  const me = gameView?.players.find((p) => p.isHuman) ?? gameView?.players[0];
  const opponents = gameView?.players.filter((p) => !p.isHuman) ?? [];
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
    () => currentPrompt?.attackerIds ?? [],
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [currentPrompt?.attackerIds?.join(",")],
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
  }, [gameView]);

  const stackCardsBySourceId = useMemo(() => {
    if (!gameView) return new Map<string, XMageCard>();
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
  }, [gameView]);

  const handleLogCardHover = (cardId: string | null, e?: React.MouseEvent) => {
    if (draggingHandCard) {
      handleHoverCard(null);
      return;
    }
    if (!cardId) {
      handleHoverCard(null);
      return;
    }
    const card = visibleCardsById.get(cardId) ?? stackCardsBySourceId.get(cardId) ?? null;
    handleHoverCard(card, e);
  };

  const handleHoverCardGuarded = (card: XMageCard | null, e?: React.MouseEvent) => {
    if (draggingHandCard) {
      handleHoverCard(null);
      return;
    }
    handleHoverCard(card, e);
  };

  useEffect(() => {
    if (draggingHandCard) {
      handleHoverCard(null);
    }
  }, [draggingHandCard, handleHoverCard]);

  // If the previewed card leaves all visible zones (e.g. removed from the game),
  // close the preview. We use visibleCardsById so that cards in graveyard, exile,
  // and command zones can still be previewed (e.g. in ZoneViewer modals).
  const hoverableCardIds = useMemo(() => {
    return new Set(visibleCardsById.keys());
  }, [visibleCardsById]);

  useEffect(() => {
    if (!hoveredCard) return;
    if (!hoverableCardIds.has(hoveredCard.id) && !stackCardsBySourceId.has(hoveredCard.id)) {
      dismissHover();
    }
  }, [hoveredCard, hoverableCardIds, stackCardsBySourceId, dismissHover]);

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
    if (!gameView?.gameOver && currentPrompt?.type !== PromptType.GameOver) return;
    const timer = setTimeout(() => endGame(), 3000);
    return () => clearTimeout(timer);
  }, [gameView?.gameOver, currentPrompt?.type]);

  if (!isGameActive) return <Navigate to="/lobby" replace />;

  // Loading
  if (!gameView) {
    return <GameLoadingScreen debugInfo={debugInfo} />;
  }

  const myPermanents = gameView.battlefield.filter(
    (c) => c.controllerId === me!.id,
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
        me={me!}
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
      className="relative flex flex-col h-full min-h-0 gap-1.5 p-1.5 overflow-visible"
      style={
        {
          "--flash-duration": `${flashDurationMs}ms`,
          "--playable-ring-color": withAlpha(themeColors.activeAction.active, 0.75),
          "--playable-glow-color": withAlpha(themeColors.activeAction.active, 0.3),
          "--playable-ring-color-strong": themeColors.activeAction.active,
          "--playable-glow-color-strong": withAlpha(themeColors.activeAction.active, 0.6),
        } as React.CSSProperties
      }
    >
      <ArrowOverlay arrows={arrows} />
      <div className="flex gap-1 min-h-0 flex-1 overflow-visible">
        <GameBoard
          me={me!}
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
          currentPrompt={currentPrompt}
          pendingAttackers={pendingAttackers}
          pendingAttacker={pendingAttacker}
          blockAssignments={blockAssignments}
          playerIsTargetable={playerIsTargetable}
          turnFlashPlayerId={turnFlashPlayerId}
          showBackFace={showBackFace}
          zonePanelSide={zonePanelSide}
          zonePanelOrder={zonePanelOrder}
          placementGhost={placementGhost}
          isOverBattlefield={isOverBattlefield}
          battlefieldContainerRef={battlefieldContainerRef}
          handContainerRef={handContainerRef}
          draggingCardId={draggingHandCard?.id}
          onHandCardDragStart={startHandCardDrag}
          onHoverCard={handleHoverCardGuarded}
          onFlipCard={handleFlipCard}
          onBattlefieldClick={handleBattlefieldClick}
          onAttackerClick={handleAttackerClick}
          onTargetPlayer={handleTargetPlayer}
          onOpenZone={openZone}
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
          onUntapLand={
            promptType === PromptType.ChooseAction || promptType === PromptType.PayCombatCost || promptType === PromptType.PayManaCost
              ? handleUntapLand
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

      <MainActionOverlay
        promptType={promptType}
        isWaitingForResponse={isWaitingForResponse}
        isAutoPassing={isAutoPassing}
        isPassingUntilEot={isPassingUntilEot}
        availableAttackerIds={currentPrompt?.availableAttackerIds ?? []}
        pendingAttackers={pendingAttackers}
        onPassPriority={passPriority}
        onPassUntilEot={activatePassUntilEot}
        onDeclareAttackers={declareAttackers}
        pendingAttacker={pendingAttacker}
        attackerIds={currentPrompt?.attackerIds ?? []}
        blockAssignments={blockAssignments}
        onDeclareBlockers={declareBlockers}
        onOpenStack={() => setSpellStackModalOpen(true)}
        onConcede={concede}
        resolveCardName={(cardId) => cardNameById.get(cardId) ?? cardId}
        isMyPriority={gameView.priorityPlayerId === me!.id}
        turn={gameView.turn}
        activePlayerName={
          gameView.players.find((p) => p.id === gameView.activePlayerId)?.name ??
          "Unknown"
        }
        isMyTurn={gameView.activePlayerId === me!.id}
        step={gameView.step}
        payManaCostInfo={
          promptType === PromptType.PayManaCost && currentPrompt?.manaCost != null
            ? {
                cardName: currentPrompt.cardName ?? "Spell",
                manaCost: currentPrompt.manaCost,
                manaPool: currentPrompt.gameView?.players?.[0]?.manaPool ?? {},
              }
            : null
        }
        onPayManaCost={payManaCost}
        onCancelManaCost={cancelManaCost}
      />

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
      />

      <GameModals
        promptType={promptType}
        currentPrompt={currentPrompt}
        viewingZone={viewingZone}
        onCloseZone={closeZone}
        zoneTargetSelector={zoneTargetSelector}
        onSelectZoneTarget={(cardId) => { targetCard(cardId); setZoneTargetSelector(null); }}
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
        validSpellIds={promptType === PromptType.ChooseTargetSpell ? (currentPrompt?.validSpellIds ?? []) : []}
        onTargetSpell={(spellId) => { targetSpell(spellId); setSpellStackModalOpen(false); }}
        onCloseStack={() => setSpellStackModalOpen(false)}
        abilityPickerState={abilityPickerState}
        onSelectAbility={(ability) => {
          if (ability.abilityIndex === -1) {
            tapLand(abilityPickerState!.cardId);
          } else {
            activateAbility(abilityPickerState!.cardId, ability.abilityIndex);
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
            style={{ left: ghostPos.x - 40, top: ghostPos.y - 56 }}
          >
            <Card
              card={draggingHandCard}
              className={cn(HAND_CARD, "shadow-2xl ring-2 ring-primary playable-card")}
            />
          </div>,
          document.body,
        )}

      {/* ── Hover card preview ────────────────────────────── */}
      {/* Hide when any overlay modal is open or a modal-based prompt is active.
          Allow-list approach: only show the preview for prompt types that do NOT
          open a modal (battlefield interaction, targeting, inline panel prompts). */}
      {hoveredCard && !draggingHandCard && !viewingZone && !zoneTargetSelector && !libraryPeekModal && !spellStackModalOpen &&
       !abilityPickerState &&
       (!promptType || HOVER_ALLOWED_PROMPTS.has(promptType)) && (
        <CardPreview
          card={hoveredCard}
          mouseX={mousePos.x}
          mouseY={mousePos.y}
          showBackFace={showBackFace}
        />
      )}
    </div>
  );
}
