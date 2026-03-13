import { useGameStore } from "@/stores/useGameStore";
import { usePreferencesStore } from "@/stores/usePreferencesStore";
import { Fragment, useEffect, useMemo, useRef, useState } from "react";
import { createPortal } from "react-dom";
import type { Card as XMageCard, Player, ActivatableAbilityInfo } from "@/types/xmage";
import { Card } from "@/components/game/Card";
import { FreeBattlefield } from "@/components/game/FreeBattlefield";
import { CardPreview } from "@/components/game/CardPreview";
import { GameModals } from "@/components/game/GameModals";
import { GameOverScreen } from "@/components/game/GameOverScreen";
import { GameLoadingScreen } from "@/components/game/GameLoadingScreen";
import { RightActionPanel } from "@/components/game/RightActionPanel";
import { ZoneActionColumn } from "@/components/game/ZoneActionColumn";
import { ArrowOverlay } from "@/components/game/ArrowOverlay";
import { useGameArrows } from "@/components/game/useGameArrows";
import { PlayerPanel } from "@/components/game/PlayerPanel";
import { OpponentHalf } from "@/components/game/OpponentHalf";
import { MidPhaseStrip } from "@/components/game/MidPhaseStrip";
import { HandDisplay } from "@/components/game/HandDisplay";
import { PlayModePicker } from "@/components/game/PlayModePicker";
import { ZONE_COLUMN_RESERVED_PX } from "@/components/game/game.constants";
import { BATTLEFIELD_CARD } from "@/components/game/game.styles";
import { useFlashQueue } from "@/hooks/useFlashQueue";
import { useHandDrag } from "@/hooks/useHandDrag";
import { useCardHover } from "@/hooks/useCardHover";
import { usePromptEffects } from "@/hooks/usePromptEffects";
import { useCombatState } from "@/hooks/useCombatState";
import {
  ResizablePanelGroup,
  ResizablePanel,
  ResizableHandle,
} from "@/components/ui/resizable";
import { cn } from "@/lib/utils";
import { Navigate, useLocation } from "react-router-dom";

/** Prompt types where hover card preview is allowed (no modal overlay). */
const HOVER_ALLOWED_PROMPTS = new Set([
  "chooseAction",
  "chooseAttackers",
  "chooseBlockers",
  "chooseTargetPlayer",
  "chooseTargetCard",
  "chooseTargetAny",
  "chooseTargetCardFromZone",
  "chooseTargetSpell",
  "payManaCost",
  "gameOver",
]);

export default function Game() {
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
    setupListeners,
  } = useGameStore();
  const flashDurationMs = usePreferencesStore((s) => s.flashDurationMs);
  const zonePanelSide = usePreferencesStore((s) => s.zonePanelSide);
  const zonePanelOrder = usePreferencesStore((s) => s.zonePanelOrder);
  const location = useLocation();
  const devExtraOpponents = ((location.state as { devExtraOpponents?: number } | null)?.devExtraOpponents ?? 0);
  const containerRef = useRef<HTMLDivElement>(null);

  const promptType = currentPrompt?.type;

  // Ability picker state (for multi-ability lands like Yavimaya Coast)
  const [abilityPickerState, setAbilityPickerState] = useState<{
    cardId: string;
    cardName: string;
    abilities: ActivatableAbilityInfo[];
  } | null>(null);

  // Play mode picker state (for cards with multiple cast modes like Spectacle/Evoke)
  const [playModePicker, setPlayModePicker] = useState<{
    cardId: string;
    cardName: string;
    options: { cardId: string; mode: string; modeLabel: string }[];
  } | null>(null);

  // Wraps castSpell: if a card has multiple play modes, show picker first
  const handleCastSpell = (cardId: string) => {
    const options = currentPrompt?.playableOptions?.filter((o) => o.cardId === cardId);
    if (options && options.length > 1) {
      const cardName = gameView?.myHand?.find((c) => c.id === cardId)?.name
        ?? gameView?.graveyard?.find((c) => c.id === cardId)?.name
        ?? gameView?.exile?.find((c) => c.id === cardId)?.name
        ?? "Card";
      setPlayModePicker({ cardId, cardName, options });
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

  // Zone viewer
  const [viewingZone, setViewingZone] = useState<{
    title: string;
    cards: XMageCard[];
    onClickCard?: (cardId: string) => void;
  } | null>(null);
  function openZone(title: string, cards: XMageCard[], onClickCard?: (cardId: string) => void) {
    setViewingZone({ title, cards, onClickCard });
  }
  function closeZone() {
    setViewingZone(null);
  }

  // Right-side prompt/action panel collapse state
  const [isActionPanelCollapsed, setIsActionPanelCollapsed] = useState(false);

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
  } = useCardHover([viewingZone, zoneTargetSelector, libraryPeekModal, spellStackModalOpen, abilityPickerState]);

  // Hand drag-to-play
  const battlefieldContainerRef = useRef<HTMLDivElement>(null);
  const { draggingHandCard, ghostPos, isOverBattlefield, startHandCardDrag } = useHandDrag({
    battlefieldContainerRef,
    onCastSpell: handleCastSpell,
    dismissHover,
  });

  // Display flash queue
  const activeFlash = useFlashQueue(flashDurationMs);

  // Set up event listeners on mount
  useEffect(() => {
    let cleanup: (() => void) | undefined;
    setupListeners().then((fn) => {
      cleanup = fn;
    });
    return () => {
      cleanup?.();
    };
  }, [setupListeners]);

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
  });

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
    if (!cardId) {
      handleHoverCard(null);
      return;
    }
    const card = visibleCardsById.get(cardId) ?? stackCardsBySourceId.get(cardId) ?? null;
    handleHoverCard(card, e);
  };

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

  // Auto-return to play menu when game is over
  useEffect(() => {
    if (!gameView?.gameOver && currentPrompt?.type !== "gameOver") return;
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
  if (gameView.gameOver || promptType === "gameOver") {
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

  return (
    <div
      ref={containerRef}
      className="relative flex flex-col h-full gap-1.5 p-1.5 overflow-hidden"
      style={
        { "--flash-duration": `${flashDurationMs}ms` } as React.CSSProperties
      }
    >
      <ArrowOverlay arrows={arrows} />

      <div className="flex gap-1 min-h-0 flex-1 overflow-hidden">
        <div className="flex flex-col gap-1 min-h-0 flex-1 overflow-hidden">
          {/* ── Resizable split: opponent (top) / me (bottom) ─── */}
          <ResizablePanelGroup orientation="vertical" className="flex-1 min-h-0">
            <ResizablePanel defaultSize={45} minSize={20}>
              {displayOpponents.length <= 1 ? (
                <OpponentHalf
                  player={displayOpponents[0]!}
                  permanents={opponentPermanentsByPlayer.get(displayOpponents[0]!.id) ?? []}
                  graveyard={gameView.opponentGraveyard ?? []}
                  exile={gameView.opponentExile ?? []}
                  commandZone={gameView.opponentCommandZone ?? undefined}
                  isTargetable={playerIsTargetable(displayOpponents[0]!.id)}
                  onTarget={() => handleTargetPlayer(displayOpponents[0]!.id)}
                  isFlashing={turnFlashPlayerId === displayOpponents[0]?.id}
                  activePlayerId={gameView.activePlayerId}
                  priorityPlayerId={gameView.priorityPlayerId}
                  promptType={promptType}
                  pendingAttacker={pendingAttacker}
                  attackerIds={currentPrompt?.attackerIds}
                  onClickCard={handleBattlefieldClick}
                  onClickAnyCard={handleAttackerClick}
                  onHoverCard={handleHoverCard}
                  onFlipCard={handleFlipCard}
                  showBackFace={showBackFace}
                  onOpenZone={openZone}
                  zonePanelSide={zonePanelSide}
                  zonePanelOrder={zonePanelOrder}
                />
              ) : (
                <ResizablePanelGroup orientation="horizontal">
                  {displayOpponents.map((op, i) => (
                    <Fragment key={op.id}>
                      {i > 0 && <ResizableHandle />}
                      <ResizablePanel>
                        <OpponentHalf
                          player={op}
                          permanents={opponentPermanentsByPlayer.get(op.id) ?? []}
                          graveyard={i === 0 ? (gameView.opponentGraveyard ?? []) : []}
                          exile={i === 0 ? (gameView.opponentExile ?? []) : []}
                          commandZone={i === 0 ? (gameView.opponentCommandZone ?? undefined) : undefined}
                          isTargetable={playerIsTargetable(op.id)}
                          onTarget={() => handleTargetPlayer(op.id)}
                          isFlashing={turnFlashPlayerId === op.id}
                          activePlayerId={gameView.activePlayerId}
                          priorityPlayerId={gameView.priorityPlayerId}
                          promptType={promptType}
                          pendingAttacker={pendingAttacker}
                          attackerIds={currentPrompt?.attackerIds}
                          onClickCard={handleBattlefieldClick}
                          onClickAnyCard={handleAttackerClick}
                          onHoverCard={handleHoverCard}
                          onFlipCard={handleFlipCard}
                          showBackFace={showBackFace}
                          onOpenZone={openZone}
                          zonePanelSide={zonePanelSide}
                          zonePanelOrder={zonePanelOrder}
                        />
                      </ResizablePanel>
                    </Fragment>
                  ))}
                </ResizablePanelGroup>
              )}
            </ResizablePanel>

            <ResizableHandle
              withHandle={false}
              gripOnly
              className="h-8 w-full my-0 flex items-center justify-center overflow-visible"
            >
              <MidPhaseStrip currentStep={gameView.step} />
            </ResizableHandle>

            <ResizablePanel defaultSize={60} minSize={35}>
              <div className="flex flex-col gap-1 h-full overflow-hidden">
                <div className="flex gap-2 flex-1 min-h-0 overflow-hidden">
                  <div
                    ref={battlefieldContainerRef}
                    className="relative flex flex-col flex-1 min-w-0 overflow-hidden"
                  >
                    <div
                      className={cn(
                        "absolute bottom-1 z-20",
                        zonePanelSide === "left" ? "left-1" : "right-1",
                      )}
                    >
                      <ZoneActionColumn
                        libraryCount={me!.libraryCount}
                        graveyardCount={gameView.graveyard.length}
                        exileCount={gameView.exile.length}
                        order={zonePanelOrder}
                        onOpenGraveyard={() => {
                          const hasPlayable = gameView.graveyard.some((c) => c.isPlayable);
                          openZone(
                            "Your Graveyard",
                            gameView.graveyard,
                            hasPlayable && promptType === "chooseAction"
                              ? (cardId) => {
                                  closeZone();
                                  handleCastSpell(cardId);
                                }
                              : undefined,
                          );
                        }}
                        onOpenExile={() => {
                          const hasPlayable = gameView.exile.some((c) => c.isPlayable);
                          openZone(
                            "Your Exile",
                            gameView.exile,
                            hasPlayable && promptType === "chooseAction"
                              ? (cardId) => {
                                  closeZone();
                                  handleCastSpell(cardId);
                                }
                              : undefined,
                          );
                        }}
                        hasPlayableInGraveyard={promptType === "chooseAction" && gameView.graveyard.some((c) => c.isPlayable)}
                        hasPlayableInExile={promptType === "chooseAction" && gameView.exile.some((c) => c.isPlayable)}
                      />
                    </div>
                    <FreeBattlefield
                      cards={myPermanents}
                      className="flex-1"
                      onClickCard={
                        promptType === "chooseAttackers" ||
                        promptType === "chooseBlockers" ||
                        promptType === "chooseTargetCard" ||
                        promptType === "chooseTargetAny"
                          ? handleBattlefieldClick
                          : undefined
                      }
                      onHoverCard={handleHoverCard}
                      onFlipCard={handleFlipCard}
                      showBackFace={showBackFace}
                      pendingCardIds={
                        promptType === "chooseAttackers"
                          ? pendingAttackers
                          : promptType === "chooseBlockers"
                            ? blockAssignments.map((a) => a.blockerId)
                            : undefined
                      }
                      tappableLandIds={
                        promptType === "chooseAction" || promptType === "payCombatCost" || promptType === "payManaCost"
                          ? (currentPrompt?.tappableLandIds ?? [])
                          : undefined
                      }
                      onTapLand={
                        promptType === "chooseAction"
                          ? (card) => {
                              const abilities = (currentPrompt?.activatableAbilityIds ?? [])
                                .filter((a) => a.cardId === card.id);
                              const isManaSource = (currentPrompt?.tappableLandIds ?? []).includes(card.id);
                              const hasManaAbility = isManaSource && !card.types.includes("Land");
                              // If the card has both a mana ability and non-mana abilities
                              // (e.g. Incubation Druid: tap for mana + Adapt), show the
                              // picker so the player can choose which ability to use.
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
                                setAbilityPickerState({
                                  cardId: card.id,
                                  cardName: card.name,
                                  abilities: pickerAbilities,
                                });
                              } else if (abilities.length === 1) {
                                activateAbility(card.id, abilities[0].abilityIndex);
                              } else {
                                tapLand(card.id);
                              }
                            }
                          : promptType === "payCombatCost" || promptType === "payManaCost"
                            ? (card) => tapLand(card.id)
                            : undefined
                      }
                      untappableLandIds={
                        promptType === "chooseAction" || promptType === "payCombatCost" || promptType === "payManaCost"
                          ? (currentPrompt?.untappableLandIds ?? [])
                          : undefined
                      }
                      onUntapLand={
                        promptType === "chooseAction" || promptType === "payCombatCost" || promptType === "payManaCost"
                          ? (card) => untapLand(card.id)
                          : undefined
                      }
                      bottomReserved={130}
                      leftReserved={zonePanelSide === "left" ? ZONE_COLUMN_RESERVED_PX : 0}
                      rightReserved={zonePanelSide === "right" ? ZONE_COLUMN_RESERVED_PX : 0}
                      isDropActive={isOverBattlefield}
                    />
                    <div className="absolute bottom-0 left-1/2 -translate-x-1/2 z-20 w-max max-w-full">
                      <HandDisplay
                        cards={gameView.myHand}
                        onHoverCard={handleHoverCard}
                        onFlipCard={handleFlipCard}
                        showBackFace={showBackFace}
                        onStartDrag={startHandCardDrag}
                      />
                    </div>
                  </div>
                </div>
              </div>
            </ResizablePanel>
          </ResizablePanelGroup>

          <div className="flex items-center gap-2 shrink-0">
            <div className="flex-1 min-w-0">
              <PlayerPanel
                player={me!}
                isOpponent={false}
                isActiveTurn={gameView.activePlayerId === me!.id}
                isPriorityPlayer={gameView.priorityPlayerId === me!.id}
                isTargetable={playerIsTargetable(me!.id)}
                onTarget={() => handleTargetPlayer(me!.id)}
                isFlashing={turnFlashPlayerId === me!.id}
                onOpenCommandZone={() => {
                  if ((gameView.myCommandZone?.length ?? 0) > 0) {
                    openZone("Your Command Zone", gameView.myCommandZone!);
                  }
                }}
                commandZoneCount={gameView.myCommandZone?.length ?? 0}
              />
            </div>
          </div>

        </div>

          <RightActionPanel
          collapsed={isActionPanelCollapsed}
          onToggleCollapse={() => setIsActionPanelCollapsed((prev) => !prev)}
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
          stack={gameView.stack}
          onOpenStack={() => setSpellStackModalOpen(true)}
          onConcede={concede}
          resolveCardName={(cardId) => cardNameById.get(cardId) ?? cardId}
          resolvePlayerName={(playerId) => playerNameById.get(playerId) ?? playerId}
          isMyPriority={gameView.priorityPlayerId === me!.id}
          turn={gameView.turn}
          activePlayerName={
            gameView.players.find((p) => p.id === gameView.activePlayerId)?.name ??
            "Unknown"
          }
          isMyTurn={gameView.activePlayerId === me!.id}
            gameLog={gameLog}
            onHoverLogCard={handleLogCardHover}
            snapshots={snapshots}
            canRestoreSnapshots={
              (!isMultiplayer || isHost) &&
              (promptType === "chooseAction" ||
                promptType === "chooseAttackers" ||
                promptType === "chooseBlockers")
            }
            onRestoreSnapshot={restoreSnapshot}
            payManaCostInfo={
              promptType === "payManaCost" && currentPrompt?.manaCost != null
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
      </div>

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
        validSpellIds={promptType === "chooseTargetSpell" ? (currentPrompt?.validSpellIds ?? []) : []}
        onTargetSpell={(spellId) => { targetSpell(spellId); setSpellStackModalOpen(false); }}
        onCloseStack={() => setSpellStackModalOpen(false)}
        abilityPickerState={abilityPickerState}
        onSelectAbility={(ability) => {
          if (ability.abilityIndex === -1) {
            tapLand(abilityPickerState!.cardId);
          } else {
            activateAbility(abilityPickerState!.cardId, ability.abilityIndex);
          }
          setAbilityPickerState(null);
        }}
        onCancelAbilityPicker={() => setAbilityPickerState(null)}
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
            setPlayModePicker(null);
          }}
          onCancel={() => setPlayModePicker(null)}
        />
      )}

      {/* ── Card-play flash overlay ───────────────────────── */}
      {activeFlash?.kind === "card" &&
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
            style={{ left: ghostPos.x - 35, top: ghostPos.y - 49 }}
          >
            <Card
              card={draggingHandCard}
              className={cn(BATTLEFIELD_CARD, "opacity-70 shadow-2xl ring-2 ring-primary")}
            />
          </div>,
          document.body,
        )}

      {/* ── Hover card preview ────────────────────────────── */}
      {/* Hide when any overlay modal is open or a modal-based prompt is active.
          Allow-list approach: only show the preview for prompt types that do NOT
          open a modal (battlefield interaction, targeting, inline panel prompts). */}
      {hoveredCard && !viewingZone && !zoneTargetSelector && !libraryPeekModal && !spellStackModalOpen &&
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
