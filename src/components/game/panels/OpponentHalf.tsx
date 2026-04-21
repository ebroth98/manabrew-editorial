import { useMemo } from "react";
import { cn } from "@/lib/utils";
import { PlayerPanel } from "./PlayerPanel";
import { BattlefieldZone } from "../zones";
import { ZoneActionColumn } from "@/components/game/ZoneActionColumn";
import { ZONE_COLUMN_RESERVED_PX } from "../game.constants";
import { useGameThemeColors, withAlpha } from "../game.theme";
import type { OpponentHalfProps } from "../game.types";
import { PromptType } from "@/types/promptType";
import { usePreferencesStore } from "@/stores/usePreferencesStore";
import { PixiGameCanvas } from "@/pixi/PixiGameCanvas";
import type { BattlefieldState, GameCanvasCallbacks } from "@/pixi/types";

const OPPONENT_PLAYER_TILE_RESERVED_PX = 92;
/** Options passed to the opponent's Pixi scene — no hand, no drag, and
 *  mirrored so lands sit at the top like the React BattlefieldZone's
 *  `landsAtTop` mode. */
const OPPONENT_SCENE_OPTIONS = {
  mirrored: true,
  showHand: false,
  allowDrag: false,
} as const;

export function OpponentHalf({
  player,
  permanents,
  graveyard,
  exile,
  commandZone,
  isTargetable,
  isSelectedTarget,
  onTarget,
  isFlashing,
  activePlayerId,
  priorityPlayerId,
  promptType,
  pendingAttacker,
  attackerIds,
  onClickCard,
  onClickAnyCard,
  onHoverCard,
  onFlipCard,
  showBackFace,
  onOpenZone,
  zonePanelSide,
  zonePanelOrder,
  placementGhost,
  hostileTargeting,
  manaAbilityOptions,
  onTapLandAbility,
  pixiSceneRef,
}: OpponentHalfProps) {
  const themeColors = useGameThemeColors();
  const pixiEnabled = usePreferencesStore((s) => s.pixiEnabled);

  const leftReserved =
    (zonePanelSide === "left" ? ZONE_COLUMN_RESERVED_PX : 0) +
    OPPONENT_PLAYER_TILE_RESERVED_PX;
  const rightReserved = zonePanelSide === "right" ? ZONE_COLUMN_RESERVED_PX : 0;

  const canTarget =
    promptType === PromptType.ChooseTargetCard ||
    promptType === PromptType.ChooseTargetAny;
  const canPickForBlockers = promptType === PromptType.ChooseBlockers;

  const pixiBattlefield = useMemo<BattlefieldState>(() => ({
    cards: permanents,
    // ChooseBlockers prompt: opponent's attackers get the ring + pending
    // pile so the local player can see who's swinging at them.
    attackingCardIds: canPickForBlockers ? attackerIds ?? [] : undefined,
    pendingCardIds:
      canPickForBlockers && pendingAttacker ? [pendingAttacker] : undefined,
    hostileTargeting,
    manaAbilityOptions,
  }), [
    permanents,
    canPickForBlockers,
    attackerIds,
    pendingAttacker,
    hostileTargeting,
    manaAbilityOptions,
  ]);

  const pixiCallbacks: GameCanvasCallbacks = useMemo(() => ({
    onClickCard: (c) => {
      if (canTarget) onClickCard(c);
      else if (canPickForBlockers) onClickAnyCard(c);
    },
    onClickAnyCard: (c) => {
      if (canPickForBlockers) onClickAnyCard(c);
    },
    onHoverCard: (c, bounds, opts) => {
      // Pixi provides DOMRect-shaped bounds (canvas-local + screen offsets
      // already applied). Synthesize a minimal anchorOverride so the React
      // preview uses it verbatim.
      if (c && bounds) {
        const rect = new DOMRect(bounds.x, bounds.y, bounds.width, bounds.height);
        onHoverCard(c, undefined, {
          anchorOverride: rect,
          useAnchor: opts?.useAnchor,
          placement: opts?.placement,
        });
      } else {
        onHoverCard(null);
      }
    },
    onFlipCard,
    // Opponent sprites never drive a cast / tap-land / target-player flow.
  }), [canTarget, canPickForBlockers, onClickCard, onClickAnyCard, onHoverCard, onFlipCard]);

  return (
    <div
      className={cn(
        "flex flex-col h-full min-h-0 overflow-visible rounded-lg border border-transparent",
      )}
      style={
        priorityPlayerId === player.id
          ? {
              borderColor: themeColors.activeAction.active,
              boxShadow: `inset 0 0 0 1px ${withAlpha(themeColors.activeAction.active, 0.85)}`,
            }
          : undefined
      }
    >
      <div className="flex gap-2 flex-1 min-h-0 overflow-visible">
        <div className="relative flex flex-col gap-1 flex-1 min-w-0 min-h-0 overflow-visible">
          <div
            className={cn(
              "absolute bottom-1 z-20",
              zonePanelSide === "left" ? "left-1" : "right-1",
            )}
          >
            <ZoneActionColumn
              libraryCount={player.libraryCount}
              graveyardCount={graveyard.length}
              exileCount={exile.length}
              order={zonePanelOrder}
              onOpenGraveyard={() => onOpenZone(`${player.name}'s Graveyard`, graveyard)}
              onOpenExile={() => onOpenZone(`${player.name}'s Exile`, exile)}
            />
          </div>
          <div className="absolute top-[-12px] left-[-12px] z-30 max-w-[calc(100%-8px)]">
            <PlayerPanel
              player={player}
              isOpponent
              verticalAlign="top"
              isActiveTurn={activePlayerId === player.id}
              isPriorityPlayer={priorityPlayerId === player.id}
              isTargetable={isTargetable}
              isSelectedTarget={isSelectedTarget}
              onTarget={onTarget}
              isFlashing={isFlashing}
              onOpenCommandZone={
                (commandZone?.length ?? 0) > 0
                  ? () => onOpenZone(`${player.name}'s Command Zone`, commandZone!)
                  : undefined
              }
              commandZoneCount={commandZone?.length ?? 0}
            />
          </div>
          {pixiEnabled && (
            <div className="absolute inset-0 z-10 rounded-lg overflow-hidden">
              <PixiGameCanvas
                battlefield={pixiBattlefield}
                sceneRef={pixiSceneRef}
                callbacks={pixiCallbacks}
                leftReserved={leftReserved}
                bottomReserved={0}
                externalBlockers={[]}
                sceneOptions={OPPONENT_SCENE_OPTIONS}
              />
            </div>
          )}
          <BattlefieldZone
            cards={permanents}
            label=""
            emptyLabel="No permanents"
            landsAtTop
            onFlipCard={onFlipCard}
            showBackFace={showBackFace}
            className={cn("flex-1", pixiEnabled && "invisible")}
            minHeight={60}
            leftReserved={leftReserved}
            rightReserved={rightReserved}
            onClickCard={
              promptType === PromptType.ChooseTargetCard ||
              promptType === PromptType.ChooseTargetAny
                ? onClickCard
                : undefined
            }
            onClickAnyCard={
              promptType === PromptType.ChooseBlockers ? onClickAnyCard : undefined
            }
            onHoverCard={onHoverCard}
            pendingCardIds={
              promptType === PromptType.ChooseBlockers && pendingAttacker
                ? [pendingAttacker]
                : undefined
            }
            attackingCardIds={
              promptType === PromptType.ChooseBlockers ? (attackerIds ?? []) : undefined
            }
            placementGhost={placementGhost}
            hostileTargeting={hostileTargeting}
            manaAbilityOptions={manaAbilityOptions}
            onTapLandAbility={onTapLandAbility}
          />
        </div>
      </div>
    </div>
  );
}
