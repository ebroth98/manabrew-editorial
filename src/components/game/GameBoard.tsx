import { Fragment, useCallback, useMemo, useRef, useState } from "react";
import type { Card, Player } from "@/types/openmagic";
import type { AgentPrompt } from "@/stores/useGameStore";
import { usePreferencesStore, type ZonePanelItem } from "@/stores/usePreferencesStore";
import { PixiGameCanvas } from "@/pixi/PixiGameCanvas";
import { PixiPhaseStripCanvas } from "@/pixi/PixiPhaseStripCanvas";
import type { BattlefieldState, GameCanvasCallbacks, ScreenBounds, PlayerColumnState, PlayerColumnCallbacks } from "@/pixi/types";
import { usePhaseStopStore } from "@/stores/usePhaseStopStore";
import type { PixiGameScene } from "@/pixi/PixiGameScene";
import type { PromptType } from "@/types/promptType";
import { PromptType as PT } from "@/types/promptType";
import { OpponentHalf } from "@/components/game/panels";
import { FreeBattlefield, HandDisplay } from "@/components/game/zones";
import type { PlacementGhost } from "@/components/game/zones/FreeBattlefield";
import { ZONE_COLUMN_RESERVED_PX } from "@/components/game/game.constants";
import { useHandScale } from "@/hooks/useHandScale";
import { HAND_CARD_BASES } from "@/components/game/game.styles";
import type { HandActionOption } from "@/stores/useGameUIStore";
import {
  ResizablePanelGroup,
  ResizablePanel,
  ResizableHandle,
} from "@/components/ui/resizable";
import { cn } from "@/lib/utils";

// Footprint of the bottom-right action cluster (PASS, Pass-Until-End,
// phase buttons). Matches MainActionOverlay's `w-[300px]` + its visible
// vertical extent so the battlefield doesn't auto-place or let the user
// drag cards underneath it.
// Only reserve space for the bottom-most mana pool strip — the prompt
// overlay now sits higher and doesn't block card placement.
const PASS_BUTTON_RESERVED = { width: 312, height: 50 } as const;

interface GameBoardProps {
  // Core game state
  me: Player;
  opponents: Player[];
  myPermanents: Card[];
  opponentPermanentsByPlayer: Map<string, Card[]>;
  myHand: Card[];
  graveyard: Card[];
  exile: Card[];
  myCommandZone?: Card[];
  opponentGraveyard: Card[];
  opponentExile: Card[];
  opponentCommandZone?: Card[];
  activePlayerId: string;
  priorityPlayerId: string;
  step: string;

  // Prompt state
  promptType?: PromptType;
  currentPrompt: AgentPrompt | null;

  // Combat state
  pendingAttackers: string[];
  pendingAttacker: string | null;
  selectedAttackDefenderId?: string | null;
  blockAssignments: { blockerId: string; attackerId: string }[];
  playerIsTargetable: (playerId: string) => boolean;

  // Flash state
  turnFlashPlayerId: string | null;

  // Hover state
  showBackFace: boolean;

  // Preferences
  zonePanelSide: "left" | "right";
  zonePanelOrder: ZonePanelItem[];

  // Stack placement preview
  placementGhost?: PlacementGhost | null;

  // Battlefield drag state
  isOverBattlefield: boolean;
  battlefieldContainerRef: React.RefObject<HTMLDivElement | null>;
  handContainerRef: React.RefObject<HTMLDivElement | null>;
  draggingCardId?: string;
  castingCardId?: string | null;

  // Callbacks
  onHandCardDragStart: (card: Card, e: React.MouseEvent) => void;
  onHandCardClick: (card: Card, e?: React.MouseEvent) => void;
  onHoverCard: (card: Card | null, e?: React.MouseEvent, options?: { useAnchor?: boolean; placement?: "auto" | "top-center"; anchorOverride?: DOMRect }) => void;
  onDismissHoverPreview?: () => void;
  getHandActions?: (card: Card) => HandActionOption[];
  onSelectHandAction?: (action: HandActionOption) => void;
  onFlipCard: () => void;
  onBattlefieldClick: (card: Card) => void;
  actionableCardIds?: string[];
  onAttackerClick: (card: Card) => void;
  onTargetPlayer: (playerId: string) => void;
  onOpenZone: (title: string, cards: Card[], onClickCard?: (cardId: string) => void) => void;
  onOpenZoneAndCast: (title: string, cards: Card[], onClickCard: (cardId: string) => void) => void;
  onTapLand?: (card: Card) => void;
  onTapLands?: (cardIds: string[]) => void;
  onTapLandAbility?: (cardId: string, abilityIndex: number, color?: string) => void;
  onUntapLand?: (card: Card) => void;
  onUntapLands?: (cardIds: string[]) => void;

  /** Out-ref populated with the live Pixi scene so Game.tsx can share it
   *  with the full-board PixiArrowsCanvas. */
  pixiSceneRef?: React.MutableRefObject<PixiGameScene | null>;

  /** Canvas-local keep-out rects (e.g. the StackDisplay panel when it is
   *  mounted) so battlefield cards beneath them move into a free cell. */
  pixiExternalBlockers?: ScreenBounds[];

  /** Per-opponent Pixi scene refs, keyed by player id. Each opponent's
   *  canvas writes into its ref once the scene is live, so the full-board
   *  arrow layer can resolve opponent sprite positions without DOM
   *  fallbacks. Provided by `Game.tsx` which maintains the ref bag. */
  getOpponentPixiSceneRef?: (playerId: string) => React.MutableRefObject<PixiGameScene | null>;

  /** Mulligan-bottom selection overlay applied to the in-game hand so
   *  the player picks cards to send to the bottom of the library
   *  directly from the real hand fan instead of a separate modal. */
  handSelectionMode?: boolean;
  handSelectedIds?: Set<string>;
  onHandCardToggle?: (cardId: string) => void;

  /** True while the mulligan flow owns the hand (keep/mulligan prompt or
   *  put-back prompt). Hides the Pixi hand so only the React fan shows
   *  — prevents the "two hands stacked on top of each other" look the
   *  player would otherwise see. */
  mulliganActive?: boolean;
}

export function GameBoard({
  me,
  opponents,
  myPermanents,
  opponentPermanentsByPlayer,
  myHand,
  graveyard,
  exile,
  myCommandZone,
  opponentGraveyard,
  opponentExile,
  opponentCommandZone,
  activePlayerId,
  priorityPlayerId,
  step,
  promptType,
  currentPrompt,
  pendingAttackers,
  pendingAttacker,
  selectedAttackDefenderId,
  blockAssignments,
  playerIsTargetable,
  turnFlashPlayerId,
  showBackFace,
  zonePanelSide,
  zonePanelOrder,
  placementGhost,
  isOverBattlefield,
  battlefieldContainerRef,
  handContainerRef,
  draggingCardId,
  castingCardId,
  onHandCardDragStart,
  onHandCardClick,
  onHoverCard,
  onDismissHoverPreview,
  getHandActions,
  onSelectHandAction,
  onFlipCard,
  onBattlefieldClick,
  actionableCardIds,
  onAttackerClick,
  onTargetPlayer,
  onOpenZone,
  onOpenZoneAndCast,
  onTapLand,
  onTapLands,
  onTapLandAbility,
  onUntapLand,
  onUntapLands,
  pixiSceneRef,
  pixiExternalBlockers,
  getOpponentPixiSceneRef,
  handSelectionMode,
  handSelectedIds,
  onHandCardToggle,
  mulliganActive,
}: GameBoardProps) {
  const selfStops = usePhaseStopStore((s) => s.selfStops);
  const toggleSelfStop = usePhaseStopStore((s) => s.toggleSelfStop);

  const handSize = usePreferencesStore((s) => s.handSize);
  const pixiEnabled = usePreferencesStore((s) => s.pixiEnabled);
  const vScale = useHandScale();
  const handBottomReserved = Math.round(HAND_CARD_BASES[handSize].containerH * vScale * 0.5);
  const hostileTargeting = currentPrompt?.hostile ?? false;
  const showChooseActionManaSources =
    promptType === PT.ChooseAction &&
    activePlayerId === me.id &&
    priorityPlayerId === me.id &&
    (step === "main1" || step === "main2") &&
    (currentPrompt?.gameView.stack?.length ?? 0) === 0;

  const pixiBattlefield = useMemo((): BattlefieldState => ({
    cards: myPermanents,
    pendingCardIds: promptType === PT.ChooseAttackers ? pendingAttackers : promptType === PT.ChooseBlockers ? blockAssignments.map((a) => a.blockerId) : undefined,
    attackingCardIds: currentPrompt?.attackerIds,
    tappableLandIds: (promptType === PT.ChooseAction || promptType === PT.PayCombatCost || promptType === PT.PayManaCost) ? (currentPrompt?.tappableLandIds ?? []) : undefined,
    untappableLandIds: (promptType === PT.ChooseAction || promptType === PT.PayCombatCost || promptType === PT.PayManaCost) ? (currentPrompt?.untappableLandIds ?? []) : undefined,
    manaAbilityOptions: (promptType === PT.ChooseAction || promptType === PT.PayManaCost) ? (currentPrompt?.manaAbilityOptions ?? []) : undefined,
    hostileTargeting,
  }), [myPermanents, promptType, pendingAttackers, blockAssignments, currentPrompt, hostileTargeting]);

  const pixiHand = useMemo((): import("@/pixi/types").HandState => ({
    cards: myHand,
    draggingCardId,
    castingCardId,
  }), [myHand, draggingCardId, castingCardId]);

  const pixiCallbacks = useMemo((): GameCanvasCallbacks => ({
    onClickCard: (promptType === PT.ChooseAction || promptType === PT.ChooseAttackers || promptType === PT.ChooseBlockers || promptType === PT.ChooseTargetCard || promptType === PT.ChooseTargetAny) ? onBattlefieldClick : undefined,
    onHoverCard: (card, bounds) => {
      if (!card) { onHoverCard(null); return; }
      if (bounds) {
        const syntheticEvent = {
          clientX: bounds.x + bounds.width / 2,
          clientY: bounds.y,
          buttons: 0,
          currentTarget: document.createElement("div"),
          shiftKey: false, altKey: false, ctrlKey: false, metaKey: false,
        } as unknown as React.MouseEvent;
        onHoverCard(card, syntheticEvent, {
          useAnchor: true,
          anchorOverride: {
            left: bounds.x, right: bounds.x + bounds.width,
            top: bounds.y, bottom: bounds.y + bounds.height,
            width: bounds.width, height: bounds.height,
            x: bounds.x, y: bounds.y,
            toJSON: () => ({}),
          } as DOMRect,
        });
      } else {
        onHoverCard(null);
      }
    },
    onStartDrag: (card, screenPos) => {
      onHandCardDragStart(card, { clientX: screenPos.x, clientY: screenPos.y, preventDefault: () => {} } as React.MouseEvent);
    },
    onClickCard_Hand: (card) => onHandCardClick(card),
    onDismissHoverPreview,
    onTapLand,
    onTapLands,
    onTapLandAbility,
    onUntapLand,
    onUntapLands,
    onFlipCard,
    onAttackerClick,
  }), [promptType, onBattlefieldClick, onHoverCard, onDismissHoverPreview, onHandCardDragStart, onHandCardClick, onTapLand, onTapLands, onTapLandAbility, onUntapLand, onUntapLands, onFlipCard, onAttackerClick]);

  const totalCmdDmg = Object.values(me.commanderDamage ?? {}).reduce(
    (a, b) => a + b,
    0,
  );

  const pixiPlayerColumn = useMemo((): PlayerColumnState => ({
    playerName: me.name,
    playerId: me.id,
    life: me.life,
    handCount: me.handCount,
    poison: me.poison,
    energyCounters: me.energyCounters ?? 0,
    commanderDamage: totalCmdDmg,
    manaPool: me.manaPool,
    libraryCount: me.libraryCount,
    graveyardCount: graveyard.length,
    exileCount: exile.length,
    commandZoneCount: myCommandZone?.length ?? 0,
    isActiveTurn: activePlayerId === me.id,
    isPriorityPlayer: priorityPlayerId === me.id,
    isTargetable: playerIsTargetable(me.id),
    currentStep: step,
    hasPlayableInGraveyard: promptType === PT.ChooseAction && graveyard.some((c) => c.isPlayable),
    hasPlayableInExile: promptType === PT.ChooseAction && exile.some((c) => c.isPlayable),
    enabledPhases: selfStops,
    isInteractive: true,
    playerSeat: "self",
  }), [me, graveyard, exile, myCommandZone, activePlayerId, promptType, playerIsTargetable, totalCmdDmg, step, selfStops]);

  const pixiPlayerColumnCallbacks = useMemo((): PlayerColumnCallbacks => ({
    onOpenGraveyard: () => {
      const hasPlayable = graveyard.some((c) => c.isPlayable);
      if (hasPlayable && promptType === PT.ChooseAction) {
        onOpenZoneAndCast("Your Graveyard", graveyard, () => {});
      } else {
        onOpenZone("Your Graveyard", graveyard);
      }
    },
    onOpenExile: () => {
      const hasPlayable = exile.some((c) => c.isPlayable);
      if (hasPlayable && promptType === PT.ChooseAction) {
        onOpenZoneAndCast("Your Exile", exile, () => {});
      } else {
        onOpenZone("Your Exile", exile);
      }
    },
    onOpenCommandZone: (myCommandZone?.length ?? 0) > 0 ? () => {
      const hasPlayable = myCommandZone!.some((c) => c.isPlayable);
      if (hasPlayable && promptType === PT.ChooseAction) {
        onOpenZoneAndCast("Your Command Zone", myCommandZone!, () => {});
      } else {
        onOpenZone("Your Command Zone", myCommandZone!);
      }
    } : undefined,
    onTargetPlayer: playerIsTargetable(me.id) ? () => onTargetPlayer(me.id) : undefined,
    onTogglePhase: toggleSelfStop,
  }), [graveyard, exile, myCommandZone, promptType, onOpenZone, onOpenZoneAndCast, onTargetPlayer, playerIsTargetable, me.id, toggleSelfStop]);

  const opponentStopsMap = usePhaseStopStore((s) => s.opponentStops);
  const toggleOpponentStop = usePhaseStopStore((s) => s.toggleOpponentStop);

  const pixiPhaseStrip = useMemo((): import("@/pixi/PhaseStripLayer").PhaseStripState => {
    // Build per-opponent enabled phases map
    const oppEnabled = new Map<string, Set<string>>();
    for (const op of opponents) {
      oppEnabled.set(op.id, opponentStopsMap.get(op.id) ?? new Set(["end"]));
    }
    return {
      currentStep: step,
      isActiveTurn: activePlayerId === me.id,
      activePlayerId,
      myPlayerId: me.id,
      selfEnabledPhases: selfStops,
      opponentEnabledPhases: oppEnabled,
      opponents: opponents.map((op, i) => ({ id: op.id, index: i })),
      isInteractive: true,
    };
  }, [step, activePlayerId, me.id, selfStops, opponents, opponentStopsMap]);

  const pixiPhaseStripCallbacks = useMemo((): import("@/pixi/PhaseStripLayer").PhaseStripCallbacks => ({
    onToggleSelfPhase: toggleSelfStop,
    onToggleOpponentPhase: toggleOpponentStop,
  }), [toggleSelfStop, toggleOpponentStop]);

  // ── Resizable split via custom drag handle on phase strip left edge ──
  const [splitPct, setSplitPct] = useState(45); // opponent % of total height
  const boardRef = useRef<HTMLDivElement>(null);

  const onGripPointerDown = useCallback((e: React.PointerEvent) => {
    e.preventDefault();
    const el = boardRef.current;
    if (!el) return;
    (e.target as HTMLElement).setPointerCapture(e.pointerId);
    const onMove = (ev: PointerEvent) => {
      const rect = el.getBoundingClientRect();
      const y = ev.clientY - rect.top;
      const pct = Math.max(20, Math.min(80, (y / rect.height) * 100));
      setSplitPct(pct);
    };
    const onUp = () => {
      window.removeEventListener("pointermove", onMove);
      window.removeEventListener("pointerup", onUp);
    };
    window.addEventListener("pointermove", onMove);
    window.addEventListener("pointerup", onUp);
  }, []);

  return (
    <div ref={boardRef} className="game-board-surface relative flex flex-col min-h-0 flex-1 overflow-visible">
      {/* ── Split: opponent (top) / phase strip / me (bottom) ─── */}

      {/* Opponent half */}
      <div style={{ flex: `${splitPct} 1 0%` }} className="min-h-0 overflow-visible">
          {opponents.length <= 1 ? (
            <OpponentHalf
              player={opponents[0]!}
              opponentIndex={0}
              permanents={opponentPermanentsByPlayer.get(opponents[0]!.id) ?? []}
              graveyard={opponentGraveyard}
              exile={opponentExile}
              commandZone={opponentCommandZone}
              isTargetable={playerIsTargetable(opponents[0]!.id)}
              isSelectedTarget={selectedAttackDefenderId === opponents[0]!.id}
              onTarget={() => onTargetPlayer(opponents[0]!.id)}
              isFlashing={turnFlashPlayerId === opponents[0]?.id}
              activePlayerId={activePlayerId}
              priorityPlayerId={priorityPlayerId}
              step={step}
              promptType={promptType}
              pendingAttacker={pendingAttacker}
              attackerIds={currentPrompt?.attackerIds}
              onClickCard={onBattlefieldClick}
              onClickAnyCard={onAttackerClick}
              onHoverCard={(card, e, opts) => onHoverCard(card, e, { useAnchor: true, ...opts })}
              onFlipCard={onFlipCard}
              showBackFace={showBackFace}
              onOpenZone={onOpenZone}
              zonePanelSide={zonePanelSide}
              zonePanelOrder={zonePanelOrder}
              placementGhost={placementGhost?.controllerId === opponents[0]!.id ? placementGhost : null}
              hostileTargeting={hostileTargeting}
              manaAbilityOptions={currentPrompt?.manaAbilityOptions}
              onTapLandAbility={onTapLandAbility}
              pixiSceneRef={getOpponentPixiSceneRef?.(opponents[0]!.id)}
            />
          ) : (
            <ResizablePanelGroup orientation="horizontal">
              {opponents.map((op, i) => (
                <Fragment key={op.id}>
                  {i > 0 && <ResizableHandle />}
                  <ResizablePanel className="overflow-visible">
                    <OpponentHalf
                      player={op}
                      opponentIndex={i}
                      permanents={opponentPermanentsByPlayer.get(op.id) ?? []}
                      graveyard={i === 0 ? opponentGraveyard : []}
                      exile={i === 0 ? opponentExile : []}
                      commandZone={i === 0 ? opponentCommandZone : undefined}
                      isTargetable={playerIsTargetable(op.id)}
                      isSelectedTarget={selectedAttackDefenderId === op.id}
                      onTarget={() => onTargetPlayer(op.id)}
                      isFlashing={turnFlashPlayerId === op.id}
                      activePlayerId={activePlayerId}
                      priorityPlayerId={priorityPlayerId}
                      step={step}
                      promptType={promptType}
                      pendingAttacker={pendingAttacker}
                      attackerIds={currentPrompt?.attackerIds}
                      onClickCard={onBattlefieldClick}
                      onClickAnyCard={onAttackerClick}
                      onHoverCard={(card, e, opts) => onHoverCard(card, e, { useAnchor: true, ...opts })}
                      onFlipCard={onFlipCard}
                      showBackFace={showBackFace}
                      onOpenZone={onOpenZone}
                      zonePanelSide={zonePanelSide}
                      zonePanelOrder={zonePanelOrder}
                      placementGhost={placementGhost?.controllerId === op.id ? placementGhost : null}
                      hostileTargeting={hostileTargeting}
                      manaAbilityOptions={currentPrompt?.manaAbilityOptions}
                      onTapLandAbility={onTapLandAbility}
                      pixiSceneRef={getOpponentPixiSceneRef?.(op.id)}
                    />
                  </ResizablePanel>
                </Fragment>
              ))}
            </ResizablePanelGroup>
          )}
      </div>

      {/* Phase strip — the center line with resize grip on the left */}
      <div className="h-16 w-full shrink-0 relative">
        {/* Resize grip — overlaid on the left, above the phase strip */}
        <div
          className="absolute left-2 top-0 h-full w-10 cursor-row-resize z-20 flex items-center justify-center"
          onPointerDown={onGripPointerDown}
        >
          <div className="flex flex-col items-center gap-[3px]">
            <div className="w-4 h-[2px] rounded-full bg-white/25" />
            <div className="w-6 h-[2px] rounded-full bg-white/35" />
            <div className="w-4 h-[2px] rounded-full bg-white/25" />
          </div>
        </div>
        {/* Phase strip — full width, centered */}
        <div className="absolute inset-0">
          <PixiPhaseStripCanvas
            state={pixiPhaseStrip}
            callbacks={pixiPhaseStripCallbacks}
          />
        </div>
      </div>

      {/* Player half */}
      <div style={{ flex: `${100 - splitPct} 1 0%` }} className="min-h-0 overflow-visible">
          <div className="flex flex-col h-full overflow-visible">
            <div className="flex flex-1 min-h-0 overflow-visible">
              <div
                ref={battlefieldContainerRef}
                className={cn(
                  "relative flex flex-col flex-1 min-w-0 overflow-visible",
                )}
              >
                {pixiEnabled && (
                  <div className="absolute inset-0 z-10 overflow-hidden">
                    <PixiGameCanvas
                      battlefield={pixiBattlefield}
                      hand={mulliganActive ? undefined : pixiHand}
                      sceneRef={pixiSceneRef}
                      placementGhostName={placementGhost?.controllerId === me.id ? placementGhost.cardName : null}
                      isDropActive={isOverBattlefield}
                      callbacks={pixiCallbacks}
                      bottomReserved={handBottomReserved}
                      leftReserved={ZONE_COLUMN_RESERVED_PX}
                      playerColumn={pixiPlayerColumn}
                      playerColumnCallbacks={pixiPlayerColumnCallbacks}
                      getHandActions={getHandActions}
                      onSelectHandAction={(_card, action) => onSelectHandAction?.(action)}
                      bottomRightReserved={PASS_BUTTON_RESERVED}
                      externalBlockers={pixiExternalBlockers}
                    />
                  </div>
                )}
                <FreeBattlefield
                  cards={myPermanents}
                  className={cn("flex-1", pixiEnabled && "invisible")}
                  onClickCard={
                    promptType === PT.ChooseAction ||
                    promptType === PT.ChooseAttackers ||
                    promptType === PT.ChooseBlockers ||
                    promptType === PT.ChooseTargetCard ||
                    promptType === PT.ChooseTargetAny
                      ? onBattlefieldClick
                      : undefined
                  }
                  onHoverCard={(card, e, opts) => onHoverCard(card, e, { useAnchor: true, ...opts })}
                  onFlipCard={onFlipCard}
                  showBackFace={showBackFace}
                  pendingCardIds={
                    promptType === PT.ChooseAttackers
                      ? pendingAttackers
                      : promptType === PT.ChooseBlockers
                        ? blockAssignments.map((a) => a.blockerId)
                        : undefined
                  }
                  actionableCardIds={actionableCardIds}
                  tappableLandIds={
                    showChooseActionManaSources ||
                    promptType === PT.PayCombatCost ||
                    promptType === PT.PayManaCost
                      ? (currentPrompt?.tappableLandIds ?? [])
                      : undefined
                  }
                  onTapLand={onTapLand}
                  onTapLands={onTapLands}
                  manaAbilityOptions={
                    showChooseActionManaSources ||
                    promptType === PT.PayManaCost
                      ? (currentPrompt?.manaAbilityOptions ?? [])
                      : undefined
                  }
                  onTapLandAbility={onTapLandAbility}
                  untappableLandIds={
                    showChooseActionManaSources ||
                    promptType === PT.PayCombatCost ||
                    promptType === PT.PayManaCost
                      ? (currentPrompt?.untappableLandIds ?? [])
                      : undefined
                  }
                  onUntapLand={onUntapLand}
                  onUntapLands={onUntapLands}
                  bottomReserved={handBottomReserved}
                  leftReserved={ZONE_COLUMN_RESERVED_PX}
                  rightReserved={0}
                  isDropActive={isOverBattlefield}
                  placementGhost={placementGhost?.controllerId === me.id ? placementGhost : null}
                  hostileTargeting={hostileTargeting}
                />
                <div
                  ref={handContainerRef}
                  className={cn(
                    "absolute bottom-0 left-1/2 -translate-x-1/2 z-20 w-max max-w-full",
                    // Pixi normally owns the hand and hides the React
                    // fan, but during the mulligan flow we swap: Pixi
                    // skips the hand entirely (above) and the React fan
                    // takes over so click-to-toggle / the keep prompt
                    // have a single surface.
                    pixiEnabled && !mulliganActive && "invisible pointer-events-none",
                  )}
                >
                  <HandDisplay
                    cards={myHand}
                    onHoverCard={onHoverCard}
                    onClickCard={onHandCardClick}
                    onFlipCard={onFlipCard}
                    showBackFace={showBackFace}
                    onStartDrag={onHandCardDragStart}
                    draggingCardId={draggingCardId}
                    castingCardId={castingCardId}
                    getActions={getHandActions}
                    onSelectAction={onSelectHandAction}
                    selectionMode={handSelectionMode}
                    selectedIds={handSelectedIds}
                    onCardToggle={onHandCardToggle}
                  />
                </div>
              </div>
            </div>
          </div>
      </div>
    </div>
  );
}
