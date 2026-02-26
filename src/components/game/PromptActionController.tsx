import { Button } from "@/components/ui/button";
import type { PromptActionType, CombatAssignment } from "./game.types";
import { PROMPT_BUTTON_COLUMN, PROMPT_HINT, BUTTON_ATTACK, BUTTON_CONFIRM_BLOCKS } from "./game.styles";
import { Sword, TimerOff } from "lucide-react";

interface PromptActionControllerProps {
  promptType?: PromptActionType;
  isWaitingForResponse: boolean;
  isAutoPassing: boolean;
  isPassingUntilEot: boolean;
  isMyTurn: boolean;
  availableAttackerIds: string[];
  pendingAttackers: string[];
  onPassPriority: () => void;
  onPassUntilEot: () => void;
  onDeclareAttackers: (attackerIds: string[]) => void;
  pendingAttacker: string | null;
  blockAssignments: CombatAssignment[];
  onDeclareBlockers: (assignments: CombatAssignment[]) => void;
  onMulliganDecision: (keep: boolean) => void;
  onOpenStack: () => void;
}

export function PromptActionController({
  promptType,
  isWaitingForResponse,
  isAutoPassing,
  isPassingUntilEot,
  isMyTurn,
  availableAttackerIds,
  pendingAttackers,
  onPassPriority,
  onPassUntilEot,
  onDeclareAttackers,
  pendingAttacker,
  blockAssignments,
  onDeclareBlockers,
  onMulliganDecision,
  onOpenStack,
}: PromptActionControllerProps) {
  if (isPassingUntilEot) {
    const label = isMyTurn ? "End Turn (F6)" : "Pass Until Your Turn (F6)";
    return (
      <div className={PROMPT_BUTTON_COLUMN}>
        <p className="text-xs italic text-muted-foreground animate-pulse">
          {isMyTurn ? "Ending turn..." : "Passing until your turn..."}
        </p>
        <Button size="sm" variant="outline" className="flex items-center gap-1" disabled>
          <TimerOff className="h-3.5 w-3.5" />
          {label}
        </Button>
      </div>
    );
  }

  if (isAutoPassing) {
    return <p className="text-xs italic text-muted-foreground animate-pulse">Auto-passing...</p>;
  }

  switch (promptType) {
    case "chooseAction":
      return (
        <div className={PROMPT_BUTTON_COLUMN}>
          <Button size="sm" variant="outline" onClick={onPassPriority} disabled={isWaitingForResponse}>
            Pass (Space)
          </Button>
          <Button
            size="sm"
            variant="outline"
            className="flex items-center gap-1"
            onClick={onPassUntilEot}
            disabled={isWaitingForResponse}
            title="Pass priority to end of turn (F6)"
          >
            <TimerOff className="h-3.5 w-3.5" />
            {isMyTurn ? "End Turn (F6)" : "Pass Until Your Turn (F6)"}
          </Button>
        </div>
      );

    case "chooseAttackers":
      return (
        <div className={PROMPT_BUTTON_COLUMN}>
          <Button size="sm" variant="outline" onClick={onPassPriority} disabled={isWaitingForResponse}>
            No Attackers
          </Button>
          <Button
            size="sm"
            variant="secondary"
            className="flex items-center gap-1"
            disabled={isWaitingForResponse}
            onClick={() => onDeclareAttackers(availableAttackerIds)}
          >
            <Sword className="h-3.5 w-3.5" />
            Attack All
          </Button>
          {pendingAttackers.length > 0 && (
            <Button
              size="sm"
              className={BUTTON_ATTACK}
              disabled={isWaitingForResponse}
              onClick={() => onDeclareAttackers(pendingAttackers)}
            >
              <Sword className="h-3.5 w-3.5" />
              Attack ({pendingAttackers.length})
            </Button>
          )}
        </div>
      );

    case "chooseBlockers":
      return (
        <div className={PROMPT_BUTTON_COLUMN}>
          <Button size="sm" variant="outline" onClick={onPassPriority} disabled={isWaitingForResponse}>
            No Blockers
          </Button>
          {pendingAttacker && (
            <p className="text-xs italic text-muted-foreground">Attacker selected. Click your blocker.</p>
          )}
          {blockAssignments.length > 0 && (
            <Button
              size="sm"
              className={BUTTON_CONFIRM_BLOCKS}
              disabled={isWaitingForResponse}
              onClick={() => onDeclareBlockers(blockAssignments)}
            >
              Confirm Blocks ({blockAssignments.length})
            </Button>
          )}
        </div>
      );

    case "mulligan":
      return (
        <div className={PROMPT_BUTTON_COLUMN}>
          <Button size="sm" onClick={() => onMulliganDecision(true)} disabled={isWaitingForResponse}>
            Keep Hand
          </Button>
          <Button
            size="sm"
            variant="destructive"
            onClick={() => onMulliganDecision(false)}
            disabled={isWaitingForResponse}
          >
            Mulligan
          </Button>
        </div>
      );

    case "chooseTargetSpell":
      return (
        <Button size="sm" onClick={onOpenStack} disabled={isWaitingForResponse}>
          Choose Counter Target
        </Button>
      );

    case "chooseTargetPlayer":
    case "chooseTargetCard":
    case "chooseTargetAny":
    case "chooseTargetCardFromZone":
      return <p className={PROMPT_HINT}>Select a highlighted target on the battlefield or in the selector.</p>;

    case "chooseMode":
    case "chooseOptionalTrigger":
    case "chooseKicker":
    case "chooseBuyback":
    case "chooseMultikicker":
    case "chooseReplicate":
    case "chooseAlternativeCost":
    case "scry":
    case "surveil":
    case "dig":
    case "chooseDiscard":
      return <p className={PROMPT_HINT}>Decision modal is open. Complete the prompt there.</p>;

    default:
      return <p className={PROMPT_HINT}>No action available for this state.</p>;
  }
}
