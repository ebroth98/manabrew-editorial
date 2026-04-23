import { useCallback, useMemo } from "react";
import { cn } from "@/lib/utils";
import { BattlefieldZone } from "../zones";
import { ZONE_COLUMN_RESERVED_PX } from "../game.constants";
import type { OpponentHalfProps } from "../game.types";
import { PromptType } from "@/types/promptType";
import { usePreferencesStore } from "@/stores/usePreferencesStore";
import { PixiGameCanvas } from "@/pixi/PixiGameCanvas";
import { usePhaseStopStore } from "@/stores/usePhaseStopStore";
import type {
  BattlefieldState,
  GameCanvasCallbacks,
  PlayerColumnState,
  PlayerColumnCallbacks,
} from "@/pixi/types";

/** Options passed to the opponent's Pixi scene — no hand, no drag, and
 *  mirrored so lands sit at the top like the React BattlefieldZone's
 *  `landsAtTop` mode. */
const DEFAULT_OPPONENT_STOPS = new Set(["end"]);

const OPPONENT_SCENE_OPTIONS = {
  mirrored: true,
  showHand: false,
  allowDrag: false,
} as const;

export function OpponentHalf({
  player,
  opponentIndex,
  permanents,
  graveyard,
  exile,
  commandZone,
  isTargetable,
  isSelectedTarget: _isSelectedTarget,
  onTarget,
  isFlashing: _isFlashing,
  activePlayerId,
  priorityPlayerId,
  step,
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
  zonePanelOrder: _zonePanelOrder,
  placementGhost,
  hostileTargeting,
  manaAbilityOptions,
  onTapLandAbility,
  pixiSceneRef,
}: OpponentHalfProps) {
  const opponentEnabledPhases = usePhaseStopStore((s) => s.opponentStops.get(player.id)) ?? DEFAULT_OPPONENT_STOPS;
  const toggleOpponentPhase = useCallback(
    (phaseId: string) => usePhaseStopStore.getState().toggleOpponentStop(player.id, phaseId),
    [player.id],
  );

  const pixiEnabled = usePreferencesStore((s) => s.pixiEnabled);

  const leftReserved = ZONE_COLUMN_RESERVED_PX;
  const rightReserved = zonePanelSide === "right" ? ZONE_COLUMN_RESERVED_PX : 0;

  const canTarget =
    promptType === PromptType.ChooseTargetCard ||
    promptType === PromptType.ChooseTargetAny;
  const canPickForBlockers = promptType === PromptType.ChooseBlockers;

  const totalCmdDmg = Object.values(player.commanderDamage ?? {}).reduce(
    (a, b) => a + b,
    0,
  );

  const pixiBattlefield = useMemo<BattlefieldState>(() => ({
    cards: permanents,
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
  }), [canTarget, canPickForBlockers, onClickCard, onClickAnyCard, onHoverCard, onFlipCard]);

  const pixiPlayerColumn = useMemo((): PlayerColumnState => ({
    playerName: player.name,
    playerId: player.id,
    life: player.life,
    handCount: player.handCount,
    poison: player.poison,
    energyCounters: player.energyCounters ?? 0,
    commanderDamage: totalCmdDmg,
    manaPool: player.manaPool,
    libraryCount: player.libraryCount,
    graveyardCount: graveyard.length,
    exileCount: exile.length,
    commandZoneCount: commandZone?.length ?? 0,
    currentStep: step,
    isActiveTurn: activePlayerId === player.id,
    isPriorityPlayer: priorityPlayerId === player.id,
    isTargetable,
    hasPlayableInGraveyard: false,
    hasPlayableInExile: false,
    enabledPhases: opponentEnabledPhases,
    isInteractive: true,
    playerSeat: (["opponent1", "opponent2", "opponent3"] as const)[opponentIndex] ?? "opponent1",
  }), [player, graveyard, exile, commandZone, activePlayerId, priorityPlayerId, step, isTargetable, totalCmdDmg, opponentEnabledPhases]);

  const pixiPlayerColumnCallbacks = useMemo((): PlayerColumnCallbacks => ({
    onOpenGraveyard: () => onOpenZone(`${player.name}'s Graveyard`, graveyard),
    onOpenExile: () => onOpenZone(`${player.name}'s Exile`, exile),
    onOpenCommandZone: (commandZone?.length ?? 0) > 0
      ? () => onOpenZone(`${player.name}'s Command Zone`, commandZone!)
      : undefined,
    onTargetPlayer: isTargetable ? onTarget : undefined,
    onTogglePhase: toggleOpponentPhase,
  }), [player.name, graveyard, exile, commandZone, onOpenZone, isTargetable, onTarget, toggleOpponentPhase]);

  return (
    <div
      className={cn(
        "flex flex-col h-full min-h-0 overflow-visible",
      )}
    >
      <div className="flex gap-2 flex-1 min-h-0 overflow-visible">
        <div className="relative flex flex-col gap-1 flex-1 min-w-0 min-h-0 overflow-visible">
          {pixiEnabled && (
            <div className="absolute inset-0 z-10 overflow-hidden">
              <PixiGameCanvas
                battlefield={pixiBattlefield}
                sceneRef={pixiSceneRef}
                callbacks={pixiCallbacks}
                leftReserved={leftReserved}
                bottomReserved={0}
                externalBlockers={[]}
                playerColumn={pixiPlayerColumn}
                playerColumnCallbacks={pixiPlayerColumnCallbacks}
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
