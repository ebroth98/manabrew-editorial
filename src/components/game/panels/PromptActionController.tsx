import type { PromptActionType, CombatAssignment } from "../game.types";
import type { ReactElement } from "react";
import { ChooseAction } from "./prompt-actions/ChooseAction";
import { ChooseAttackers } from "./prompt-actions/ChooseAttackers";
import { ChooseBlockers } from "./prompt-actions/ChooseBlockers";
import { ChooseTargetSpell } from "./prompt-actions/ChooseTargetSpell";
import { PayManaCost } from "./prompt-actions/PayManaCost";
import { NoAction } from "./prompt-actions/NoAction";
import { PromptType } from "@/types/promptType";
import type { PromptButtonLayout } from "./PromptActionButton";
import {
  type PromptActionViewKey,
  useGameDevStore,
} from "@/stores/useGameDevStore";

const PROMPT_TO_VIEW_KEY: Record<string, PromptActionViewKey> = {
  [PromptType.ChooseAction]: "chooseAction",
  [PromptType.ChooseAttackers]: "chooseAttackers",
  [PromptType.ChooseBlockers]: "chooseBlockers",
  [PromptType.ChooseTargetSpell]: "chooseTargetSpell",
  [PromptType.PayManaCost]: "payManaCost",

  [PromptType.Mulligan]: "noAction",
  [PromptType.MulliganPutBack]: "noAction",
  [PromptType.ChooseTargetPlayer]: "noAction",
  [PromptType.ChooseTargetCard]: "noAction",
  [PromptType.ChooseTargetAny]: "noAction",
  [PromptType.ChooseTargetCardFromZone]: "noAction",
  [PromptType.ChooseMode]: "noAction",
  [PromptType.ChooseOptionalTrigger]: "noAction",
  [PromptType.ChooseKicker]: "noAction",
  [PromptType.ChooseBuyback]: "noAction",
  [PromptType.ChooseMultikicker]: "noAction",
  [PromptType.ChooseReplicate]: "noAction",
  [PromptType.ChooseAlternativeCost]: "noAction",
  [PromptType.Scry]: "noAction",
  [PromptType.Surveil]: "noAction",
  [PromptType.Dig]: "noAction",
  [PromptType.ChooseDiscard]: "noAction",
  [PromptType.PayCombatCost]: "noAction",
  [PromptType.ChooseColor]: "noAction",
  [PromptType.ChooseType]: "noAction",
  [PromptType.ChooseNumber]: "noAction",
  [PromptType.ChooseCardName]: "noAction",
  [PromptType.ChooseDelve]: "noAction",
  [PromptType.ChooseConvoke]: "noAction",
  [PromptType.ChooseImprovise]: "noAction",
  [PromptType.SpecifyManaCombo]: "noAction",
  [PromptType.ChooseDamageAssignmentOrder]: "noAction",
  [PromptType.ChooseCardsForEffect]: "noAction",
  [PromptType.ChoosePhyrexian]: "noAction",
  [PromptType.ChooseExertAttackers]: "noAction",
  [PromptType.ChooseEnlistAttackers]: "noAction",
  [PromptType.ReorderLibrary]: "noAction",
  [PromptType.ExploreDecision]: "noAction",
  [PromptType.HelpPayAssist]: "noAction",
};

interface PromptActionControllerProps {
  promptType?: PromptActionType;
  isWaitingForResponse: boolean;
  isAutoPassing: boolean;
  isPassingUntilEot: boolean;
  isMyTurn: boolean;
  passToPhaseShort: string;
  availableAttackerIds: string[];
  pendingAttackers: string[];
  onPassPriority: () => void;
  onPassUntilEot: () => void;
  onDeclareAttackers: (attackerIds: string[]) => void;
  pendingAttacker: string | null;
  blockAssignments: CombatAssignment[];
  onDeclareBlockers: (assignments: CombatAssignment[]) => void;
  onOpenStack: () => void;
  buttonLayout?: PromptButtonLayout;
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
  passToPhaseShort,
  availableAttackerIds,
  pendingAttackers,
  onPassPriority,
  onPassUntilEot,
  onDeclareAttackers,
  pendingAttacker,
  blockAssignments,
  onDeclareBlockers,
  onOpenStack,
  buttonLayout = "full",
  payManaCostInfo,
  onPayManaCost,
  onCancelManaCost,
}: PromptActionControllerProps) {
  const promptActionOverride = useGameDevStore((s) => s.promptActionOverride);

  const renderers: Record<PromptActionViewKey, () => ReactElement> = {
    chooseAction: () => (
      <ChooseAction
        buttonLayout={buttonLayout}
        isWaitingForResponse={isWaitingForResponse}
        isMyTurn={isMyTurn}
        passToPhaseShort={passToPhaseShort}
        onPassPriority={onPassPriority}
        onPassUntilEot={onPassUntilEot}
      />
    ),
    chooseAttackers: () => (
      <ChooseAttackers
        buttonLayout={buttonLayout}
        isWaitingForResponse={isWaitingForResponse}
        availableAttackerIds={availableAttackerIds}
        pendingAttackers={pendingAttackers}
        onPassPriority={onPassPriority}
        onDeclareAttackers={onDeclareAttackers}
      />
    ),
    chooseBlockers: () => (
      <ChooseBlockers
        buttonLayout={buttonLayout}
        isWaitingForResponse={isWaitingForResponse}
        pendingAttacker={pendingAttacker}
        blockAssignments={blockAssignments}
        onPassPriority={onPassPriority}
        onDeclareBlockers={onDeclareBlockers}
      />
    ),
    chooseTargetSpell: () => (
      <ChooseTargetSpell
        buttonLayout={buttonLayout}
        isWaitingForResponse={isWaitingForResponse}
        onOpenStack={onOpenStack}
      />
    ),
    payManaCost: () => (
      <PayManaCost
        buttonLayout={buttonLayout}
        isWaitingForResponse={isWaitingForResponse}
        payManaCostInfo={payManaCostInfo}
        onPayManaCost={onPayManaCost}
        onCancelManaCost={onCancelManaCost}
      />
    ),
    passingUntilEot: () => (
      <NoAction
        buttonLayout={buttonLayout}
        label={isMyTurn ? "End Turn" : "Pass Till End"}
      />
    ),
    autoPassing: () => <NoAction buttonLayout={buttonLayout} label="Auto Pass" />,
    noAction: () => <NoAction buttonLayout={buttonLayout} label="No Action" />,
  };

  const runtimeViewKey: PromptActionViewKey = isPassingUntilEot
    ? "passingUntilEot"
    : isAutoPassing
      ? "autoPassing"
      : (promptType ? PROMPT_TO_VIEW_KEY[promptType] : undefined) ?? "noAction";

  const rendered = renderers[promptActionOverride ?? runtimeViewKey]();

  if (promptActionOverride) {
    return (
      <div
        className="contents [&_button]:pointer-events-none"
        aria-disabled="true"
      >
        {rendered}
      </div>
    );
  }

  return rendered;
}
