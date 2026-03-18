import { Button } from "@/components/ui/button";
import { TextWithMana } from "@/components/game/TextWithMana";
import { ManaPool } from "./ManaPool";
import type { PromptActionType, CombatAssignment } from "../game.types";
import { PROMPT_BUTTON_COLUMN, PROMPT_HINT, BUTTON_ATTACK, BUTTON_CONFIRM_BLOCKS } from "../game.styles";
import { Sword, TimerOff } from "lucide-react";
import { PromptType } from "@/types/promptType";

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
  onOpenStack: () => void;
  // Pay mana cost
  payManaCostInfo?: { cardName: string; manaCost: string; manaPool: Record<string, number> } | null;
  onPayManaCost?: () => void;
  onCancelManaCost?: () => void;
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
  onOpenStack,
  payManaCostInfo,
  onPayManaCost,
  onCancelManaCost,
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
    case PromptType.ChooseAction:
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

    case PromptType.ChooseAttackers:
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

    case PromptType.ChooseBlockers:
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

    case PromptType.Mulligan:
    case PromptType.MulliganPutBack:
      return <p className={PROMPT_HINT}>Mulligan decision is open. Complete the prompt there.</p>;

    case PromptType.ChooseTargetSpell:
      return (
        <Button size="sm" onClick={onOpenStack} disabled={isWaitingForResponse}>
          Choose Counter Target
        </Button>
      );

    case PromptType.ChooseTargetPlayer:
    case PromptType.ChooseTargetCard:
    case PromptType.ChooseTargetAny:
    case PromptType.ChooseTargetCardFromZone:
      return <p className={PROMPT_HINT}>Select a highlighted target on the battlefield or in the selector.</p>;

    case PromptType.PayManaCost:
      return (
        <div className={PROMPT_BUTTON_COLUMN}>
          {payManaCostInfo && (
            <>
              <p className="text-xs text-muted-foreground">
                Cast <span className="font-semibold text-foreground">{payManaCostInfo.cardName}</span>{" "}
                for <TextWithMana text={payManaCostInfo.manaCost} manaSize="sm" />
              </p>
              <div className="flex items-center justify-between text-xs text-muted-foreground">
                <span>Mana pool:</span>
                <ManaPool pool={payManaCostInfo.manaPool} />
              </div>
              <p className="text-[11px] text-muted-foreground/70">
                Tap lands to generate mana, then click Pay.
              </p>
            </>
          )}
          <Button size="sm" onClick={onPayManaCost} disabled={isWaitingForResponse}>
            Pay
          </Button>
          <Button size="sm" variant="outline" onClick={onCancelManaCost} disabled={isWaitingForResponse}>
            Cancel
          </Button>
        </div>
      );

    case PromptType.ChooseMode:
    case PromptType.ChooseOptionalTrigger:
    case PromptType.ChooseKicker:
    case PromptType.ChooseBuyback:
    case PromptType.ChooseMultikicker:
    case PromptType.ChooseReplicate:
    case PromptType.ChooseAlternativeCost:
    case PromptType.Scry:
    case PromptType.Surveil:
    case PromptType.Dig:
    case PromptType.ChooseDiscard:
    case PromptType.PayCombatCost:
    case PromptType.ChooseColor:
    case PromptType.ChooseType:
    case PromptType.ChooseNumber:
    case PromptType.ChooseCardName:
    case PromptType.ChooseDelve:
    case PromptType.ChooseConvoke:
    case PromptType.ChooseImprovise:
    case PromptType.SpecifyManaCombo:
    case PromptType.ChooseDamageAssignmentOrder:
    case PromptType.ChooseCardsForEffect:
    case PromptType.ChoosePhyrexian:
    case PromptType.ChooseExertAttackers:
    case PromptType.ChooseEnlistAttackers:
    case PromptType.ReorderLibrary:
    case PromptType.ExploreDecision:
    case PromptType.HelpPayAssist:
      return <p className={PROMPT_HINT}>Decision modal is open. Complete the prompt there.</p>;

    default:
      return <p className={PROMPT_HINT}>No action available for this state.</p>;
  }
}
