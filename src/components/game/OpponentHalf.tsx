import { cn } from "@/lib/utils";
import { PlayerPanel } from "./PlayerPanel";
import { BattlefieldZone } from "./BattlefieldZone";
import { ZoneActionColumn } from "@/components/game/ZoneActionColumn";
import { ZONE_COLUMN_RESERVED_PX } from "./game.constants";
import type { OpponentHalfProps } from "./game.types";

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
}: OpponentHalfProps) {
  return (
    <div className="flex flex-col gap-1 h-full overflow-hidden">
      <PlayerPanel
        player={player}
        isOpponent
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
      <div className="flex gap-2 flex-1 min-h-0 overflow-hidden">
        <div className="relative flex flex-col gap-1 flex-1 min-w-0 overflow-hidden">
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
          <BattlefieldZone
            cards={permanents}
            label=""
            emptyLabel="No permanents"
            landsAtTop
            onFlipCard={onFlipCard}
            showBackFace={showBackFace}
            className="flex-1"
            minHeight={60}
            leftReserved={zonePanelSide === "left" ? ZONE_COLUMN_RESERVED_PX : 0}
            rightReserved={zonePanelSide === "right" ? ZONE_COLUMN_RESERVED_PX : 0}
            onClickCard={
              promptType === "chooseTargetCard" ||
              promptType === "chooseTargetAny"
                ? onClickCard
                : undefined
            }
            onClickAnyCard={
              promptType === "chooseBlockers" ? onClickAnyCard : undefined
            }
            onHoverCard={onHoverCard}
            pendingCardIds={
              promptType === "chooseBlockers" && pendingAttacker
                ? [pendingAttacker]
                : undefined
            }
            attackingCardIds={
              promptType === "chooseBlockers" ? (attackerIds ?? []) : undefined
            }
          />
        </div>
      </div>
    </div>
  );
}
