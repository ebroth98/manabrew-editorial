import type { PromptActionType, CombatAssignment } from "../game.types";
import type { ReactElement } from "react";
import { ChooseAction } from "./prompt-actions/ChooseAction";
import { ChooseAttackers } from "./prompt-actions/ChooseAttackers";
import { ChooseBlockers } from "./prompt-actions/ChooseBlockers";
import { ChooseTargetSpell } from "./prompt-actions/ChooseTargetSpell";
import { PayManaCost } from "./prompt-actions/PayManaCost";
import { PromptRequired } from "./prompt-actions/PromptRequired";
import { NoAction } from "./prompt-actions/NoAction";
import { PromptType } from "@/types/promptType";
import type { PromptButtonLayout } from "./PromptActionButton";
import {
  type PromptActionViewKey,
  useGameDevStore,
} from "@/stores/useGameDevStore";
import { useGameUIStore } from "@/stores/useGameUIStore";

const PROMPT_TO_VIEW_KEY: Record<string, PromptActionViewKey> = {
  [PromptType.ChooseAction]: "chooseAction",
  [PromptType.ChooseAttackers]: "chooseAttackers",
  [PromptType.ChooseBlockers]: "chooseBlockers",
  [PromptType.ChooseTargetSpell]: "chooseTargetSpell",
  [PromptType.PayManaCost]: "payManaCost",

  [PromptType.Mulligan]: "promptRequired",
  [PromptType.MulliganPutBack]: "promptRequired",
  [PromptType.ChooseTargetPlayer]: "noAction",
  [PromptType.ChooseTargetCard]: "noAction",
  [PromptType.ChooseTargetAny]: "noAction",
  [PromptType.ChooseTargetCardFromZone]: "noAction",
  [PromptType.ChooseMode]: "promptRequired",
  [PromptType.ChooseOptionalTrigger]: "promptRequired",
  [PromptType.ChooseKicker]: "promptRequired",
  [PromptType.ChooseBuyback]: "promptRequired",
  [PromptType.ChooseMultikicker]: "promptRequired",
  [PromptType.ChooseReplicate]: "promptRequired",
  [PromptType.ChooseAlternativeCost]: "promptRequired",
  [PromptType.Scry]: "noAction",
  [PromptType.Surveil]: "noAction",
  [PromptType.Dig]: "noAction",
  [PromptType.ChooseDiscard]: "noAction",
  [PromptType.PayCombatCost]: "promptRequired",
  [PromptType.ChooseColor]: "promptRequired",
  [PromptType.ChooseType]: "promptRequired",
  [PromptType.ChooseNumber]: "promptRequired",
  [PromptType.ChooseCardName]: "promptRequired",
  [PromptType.ChooseDelve]: "promptRequired",
  [PromptType.ChooseConvoke]: "promptRequired",
  [PromptType.ChooseImprovise]: "promptRequired",
  [PromptType.SpecifyManaCombo]: "promptRequired",
  [PromptType.ChooseDamageAssignmentOrder]: "promptRequired",
  [PromptType.ChooseCombatDamageAssignment]: "promptRequired",
  [PromptType.ChooseCardsForEffect]: "promptRequired",
  [PromptType.ChoosePhyrexian]: "promptRequired",
  [PromptType.ChooseExertAttackers]: "promptRequired",
  [PromptType.ChooseEnlistAttackers]: "promptRequired",
  [PromptType.ReorderLibrary]: "promptRequired",
  [PromptType.ExploreDecision]: "promptRequired",
  [PromptType.HelpPayAssist]: "promptRequired",
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
  const promptModalHidden = useGameUIStore((s) => s.promptModalHidden);
  const showPromptModal = useGameUIStore((s) => s.showPromptModal);

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
    promptRequired: () => (
      <PromptRequired
        buttonLayout={buttonLayout}
        isWaitingForResponse={isWaitingForResponse}
        hidden={promptModalHidden}
        onOpenPrompt={showPromptModal}
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
