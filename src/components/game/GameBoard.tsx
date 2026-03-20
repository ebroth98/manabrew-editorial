import { Fragment } from "react";
import type { Card, Player } from "@/types/openmagic";
import type { AgentPrompt } from "@/stores/useGameStore";
import type { ZonePanelItem } from "@/stores/usePreferencesStore";
import type { PromptType } from "@/types/promptType";
import { PromptType as PT } from "@/types/promptType";
import { OpponentHalf, PlayerPanel } from "@/components/game/panels";
import { MidPhaseStrip } from "@/components/game/MidPhaseStrip";
import { FreeBattlefield, HandDisplay } from "@/components/game/zones";
import type { PlacementGhost } from "@/components/game/zones/FreeBattlefield";
import { ZoneActionColumn } from "@/components/game/ZoneActionColumn";
import { ZONE_COLUMN_RESERVED_PX } from "@/components/game/game.constants";
import { useGameThemeColors, withAlpha } from "@/components/game/game.theme";
import {
  ResizablePanelGroup,
  ResizablePanel,
  ResizableHandle,
} from "@/components/ui/resizable";
import { cn } from "@/lib/utils";

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

  // Callbacks
  onHandCardDragStart: (card: Card, e: React.MouseEvent) => void;
  onHoverCard: (card: Card | null, e?: React.MouseEvent) => void;
  onFlipCard: () => void;
  onBattlefieldClick: (card: Card) => void;
  onAttackerClick: (card: Card) => void;
  onTargetPlayer: (playerId: string) => void;
  onOpenZone: (title: string, cards: Card[], onClickCard?: (cardId: string) => void) => void;
  onOpenZoneAndCast: (title: string, cards: Card[], onClickCard: (cardId: string) => void) => void;
  onTapLand?: (card: Card) => void;
  onUntapLand?: (card: Card) => void;
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
  onHandCardDragStart,
  onHoverCard,
  onFlipCard,
  onBattlefieldClick,
  onAttackerClick,
  onTargetPlayer,
  onOpenZone,
  onOpenZoneAndCast,
  onTapLand,
  onUntapLand,
}: GameBoardProps) {
  const themeColors = useGameThemeColors();

  return (
    <div className="game-board-surface flex flex-col gap-1 min-h-0 flex-1 overflow-visible">
      {/* ── Resizable split: opponent (top) / me (bottom) ─── */}
      <ResizablePanelGroup orientation="vertical" className="flex-1 min-h-0">
        <ResizablePanel defaultSize={45} minSize={20} className="overflow-visible">
          {opponents.length <= 1 ? (
            <OpponentHalf
              player={opponents[0]!}
              permanents={opponentPermanentsByPlayer.get(opponents[0]!.id) ?? []}
              graveyard={opponentGraveyard}
              exile={opponentExile}
              commandZone={opponentCommandZone}
              isTargetable={playerIsTargetable(opponents[0]!.id)}
              onTarget={() => onTargetPlayer(opponents[0]!.id)}
              isFlashing={turnFlashPlayerId === opponents[0]?.id}
              activePlayerId={activePlayerId}
              priorityPlayerId={priorityPlayerId}
              promptType={promptType}
              pendingAttacker={pendingAttacker}
              attackerIds={currentPrompt?.attackerIds}
              onClickCard={onBattlefieldClick}
              onClickAnyCard={onAttackerClick}
              onHoverCard={onHoverCard}
              onFlipCard={onFlipCard}
              showBackFace={showBackFace}
              onOpenZone={onOpenZone}
              zonePanelSide={zonePanelSide}
              zonePanelOrder={zonePanelOrder}
              placementGhost={placementGhost?.controllerId === opponents[0]!.id ? placementGhost : null}
            />
          ) : (
            <ResizablePanelGroup orientation="horizontal">
              {opponents.map((op, i) => (
                <Fragment key={op.id}>
                  {i > 0 && <ResizableHandle />}
                  <ResizablePanel className="overflow-visible">
                    <OpponentHalf
                      player={op}
                      permanents={opponentPermanentsByPlayer.get(op.id) ?? []}
                      graveyard={i === 0 ? opponentGraveyard : []}
                      exile={i === 0 ? opponentExile : []}
                      commandZone={i === 0 ? opponentCommandZone : undefined}
                      isTargetable={playerIsTargetable(op.id)}
                      onTarget={() => onTargetPlayer(op.id)}
                      isFlashing={turnFlashPlayerId === op.id}
                      activePlayerId={activePlayerId}
                      priorityPlayerId={priorityPlayerId}
                      promptType={promptType}
                      pendingAttacker={pendingAttacker}
                      attackerIds={currentPrompt?.attackerIds}
                      onClickCard={onBattlefieldClick}
                      onClickAnyCard={onAttackerClick}
                      onHoverCard={onHoverCard}
                      onFlipCard={onFlipCard}
                      showBackFace={showBackFace}
                      onOpenZone={onOpenZone}
                      zonePanelSide={zonePanelSide}
                      zonePanelOrder={zonePanelOrder}
                      placementGhost={placementGhost?.controllerId === op.id ? placementGhost : null}
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
          <MidPhaseStrip currentStep={step} />
        </ResizableHandle>

        <ResizablePanel defaultSize={60} minSize={35}>
          <div className="flex flex-col gap-1 h-full overflow-visible">
            <div className="flex gap-2 flex-1 min-h-0 overflow-visible">
              <div
                ref={battlefieldContainerRef}
                className={cn(
                  "relative flex flex-col flex-1 min-w-0 overflow-visible rounded-lg border border-transparent",
                )}
                style={
                  priorityPlayerId === me.id
                    ? {
                        borderColor: themeColors.activeAction.active,
                        boxShadow: `inset 0 0 0 1px ${withAlpha(themeColors.activeAction.active, 0.85)}`,
                      }
                    : undefined
                }
              >
                <div className="absolute bottom-12 left-0 z-30">
                  <ZoneActionColumn
                    libraryCount={me.libraryCount}
                    graveyardCount={graveyard.length}
                    exileCount={exile.length}
                    order={zonePanelOrder}
                    onOpenGraveyard={() => {
                      const hasPlayable = graveyard.some((c) => c.isPlayable);
                      if (hasPlayable && promptType === PT.ChooseAction) {
                        onOpenZoneAndCast("Your Graveyard", graveyard, (_cardId) => {
                          // Parent will close zone and call handleCastSpell
                        });
                      } else {
                        onOpenZone("Your Graveyard", graveyard);
                      }
                    }}
                    onOpenExile={() => {
                      const hasPlayable = exile.some((c) => c.isPlayable);
                      if (hasPlayable && promptType === PT.ChooseAction) {
                        onOpenZoneAndCast("Your Exile", exile, (_cardId) => {
                          // Parent will close zone and call handleCastSpell
                        });
                      } else {
                        onOpenZone("Your Exile", exile);
                      }
                    }}
                    hasPlayableInGraveyard={
                      promptType === PT.ChooseAction && graveyard.some((c) => c.isPlayable)
                    }
                    hasPlayableInExile={
                      promptType === PT.ChooseAction && exile.some((c) => c.isPlayable)
                    }
                  />
                </div>
                <div className="absolute bottom-[-12px] left-[-12px] z-30 max-w-[calc(100%-8px)]">
                  <PlayerPanel
                    player={me}
                    isOpponent={false}
                    verticalAlign="bottom"
                    isActiveTurn={activePlayerId === me.id}
                    isPriorityPlayer={priorityPlayerId === me.id}
                    isTargetable={playerIsTargetable(me.id)}
                    onTarget={() => onTargetPlayer(me.id)}
                    isFlashing={turnFlashPlayerId === me.id}
                    onOpenCommandZone={() => {
                      if ((myCommandZone?.length ?? 0) > 0) {
                        onOpenZone("Your Command Zone", myCommandZone!);
                      }
                    }}
                    commandZoneCount={myCommandZone?.length ?? 0}
                  />
                </div>
                <FreeBattlefield
                  cards={myPermanents}
                  className="flex-1"
                  onClickCard={
                    promptType === PT.ChooseAttackers ||
                    promptType === PT.ChooseBlockers ||
                    promptType === PT.ChooseTargetCard ||
                    promptType === PT.ChooseTargetAny
                      ? onBattlefieldClick
                      : undefined
                  }
                  onHoverCard={onHoverCard}
                  onFlipCard={onFlipCard}
                  showBackFace={showBackFace}
                  pendingCardIds={
                    promptType === PT.ChooseAttackers
                      ? pendingAttackers
                      : promptType === PT.ChooseBlockers
                        ? blockAssignments.map((a) => a.blockerId)
                        : undefined
                  }
                  tappableLandIds={
                    promptType === PT.ChooseAction ||
                    promptType === PT.PayCombatCost ||
                    promptType === PT.PayManaCost
                      ? (currentPrompt?.tappableLandIds ?? [])
                      : undefined
                  }
                  onTapLand={onTapLand}
                  untappableLandIds={
                    promptType === PT.ChooseAction ||
                    promptType === PT.PayCombatCost ||
                    promptType === PT.PayManaCost
                      ? (currentPrompt?.untappableLandIds ?? [])
                      : undefined
                  }
                  onUntapLand={onUntapLand}
                  bottomReserved={130}
                  leftReserved={ZONE_COLUMN_RESERVED_PX}
                  rightReserved={0}
                  isDropActive={isOverBattlefield}
                  placementGhost={placementGhost?.controllerId === me.id ? placementGhost : null}
                />
                <div ref={handContainerRef} className="absolute bottom-0 left-1/2 -translate-x-1/2 z-20 w-max max-w-full">
                  <HandDisplay
                    cards={myHand}
                    onHoverCard={onHoverCard}
                    onFlipCard={onFlipCard}
                    showBackFace={showBackFace}
                    onStartDrag={onHandCardDragStart}
                    draggingCardId={draggingCardId}
                  />
                </div>
              </div>
            </div>
          </div>
        </ResizablePanel>
      </ResizablePanelGroup>
    </div>
  );
}
