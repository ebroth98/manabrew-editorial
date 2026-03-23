import { cn } from "@/lib/utils";
import { PlayerPanel } from "./PlayerPanel";
import { BattlefieldZone } from "../zones";
import { ZoneActionColumn } from "@/components/game/ZoneActionColumn";
import { ZONE_COLUMN_RESERVED_PX } from "../game.constants";
import { useGameThemeColors, withAlpha } from "../game.theme";
import type { OpponentHalfProps } from "../game.types";
import { PromptType } from "@/types/promptType";

const OPPONENT_PLAYER_TILE_RESERVED_PX = 92;

export function OpponentHalf({
  player,
  permanents,
  graveyard,
  exile,
  commandZone,
  isTargetable,
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
}: OpponentHalfProps) {
  const themeColors = useGameThemeColors();

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
          <BattlefieldZone
            cards={permanents}
            label=""
            emptyLabel="No permanents"
            landsAtTop
            onFlipCard={onFlipCard}
            showBackFace={showBackFace}
            className="flex-1"
            minHeight={60}
            leftReserved={
              (zonePanelSide === "left" ? ZONE_COLUMN_RESERVED_PX : 0) +
              OPPONENT_PLAYER_TILE_RESERVED_PX
            }
            rightReserved={zonePanelSide === "right" ? ZONE_COLUMN_RESERVED_PX : 0}
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
          />
        </div>
      </div>
    </div>
  );
}
