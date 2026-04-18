import {
  ChooseModeModal,
  ChooseOptionalTriggerModal,
  ChooseColorModal,
  ChooseTypeModal,
  ChooseNumberModal,
  ChooseCardNameModal,
  DamageOrderModal,
  VAssignCombatDamageModal,
  ReorderLibraryModal,
  RevealCardsModal,
  SpecifyManaComboModal,
  PromptModalController,
} from "@/components/game/modals";
import type { AgentPrompt } from "@/stores/useGameStore";
import type { PromptType } from "@/types/promptType";
import { PromptType as PT } from "@/types/promptType";

interface PromptModalsProps {
  promptType?: PromptType;
  currentPrompt: AgentPrompt | null;
  // Decision callbacks
  onModeDecision: (indices: number[]) => void;
  onRevealCardsAcknowledged: () => void;
  onPayCostToPreventEffectDecision: (accept: boolean) => void;
  onOptionalTriggerDecision: (accept: boolean) => void;
  onColorDecision: (color: string) => void;
  onTypeDecision: (chosenType: string | null) => void;
  onNumberDecision: (chosenNumber: number | null) => void;
  onCardNameDecision: (chosenName: string | null) => void;
  onDamageOrderDecision: (orderedBlockerIds: string[]) => void;
  onCombatDamageAssignmentDecision: (assignments: { assigneeId: string; damage: number }[]) => void;
  onReorderLibraryDecision: (orderedCardIds: string[]) => void;
  onManaComboDecision: (chosenColors: string[]) => void;
  onExploreDecision: (putInGraveyard: boolean) => void;
  onAssistDecision: (amountToPay: number) => void;
}

export function PromptModals({
  promptType,
  currentPrompt,
  onModeDecision,
  onRevealCardsAcknowledged,
  onPayCostToPreventEffectDecision,
  onOptionalTriggerDecision,
  onColorDecision,
  onTypeDecision,
  onNumberDecision,
  onCardNameDecision,
  onDamageOrderDecision,
  onCombatDamageAssignmentDecision,
  onReorderLibraryDecision,
  onManaComboDecision,
  onExploreDecision,
  onAssistDecision,
}: PromptModalsProps) {
  const isActivePromptModal =
    (promptType === PT.RevealCards && currentPrompt?.cards != null && currentPrompt?.message != null) ||
    // Mulligan + MulliganPutBack render in the same bottom-right slot
    // as Pass Priority (see `PromptActionController`), and the real
    // in-game hand drives card selection via `HandDisplayCool`'s
    // selection mode — they intentionally do NOT open a modal here.
    (promptType === PT.ChooseMode && currentPrompt?.options != null) ||
    (promptType === PT.ChooseOptionalTrigger && currentPrompt?.description != null) ||
    (promptType === PT.PayCostToPreventEffect && currentPrompt?.description != null) ||
    (promptType === PT.ChooseColor && currentPrompt?.validColors != null) ||
    (promptType === PT.ChooseType && currentPrompt?.validTypes != null) ||
    (promptType === PT.ChooseNumber && currentPrompt?.min != null && currentPrompt?.max != null) ||
    (promptType === PT.ChooseCardName && currentPrompt?.validNames != null) ||
    (promptType === PT.ChooseDamageAssignmentOrder && currentPrompt?.blockerIds != null) ||
    (promptType === PT.ChooseCombatDamageAssignment
      && currentPrompt?.attackerId != null
      && currentPrompt?.blockerIds != null
      && currentPrompt?.totalDamage != null
      && currentPrompt?.gameView != null) ||
    (promptType === PT.ReorderLibrary && currentPrompt?.cards != null) ||
    (promptType === PT.SpecifyManaCombo && currentPrompt?.availableColors != null && currentPrompt?.amount != null) ||
    (promptType === PT.ExploreDecision && currentPrompt?.revealedCardName != null) ||
    (promptType === PT.HelpPayAssist && currentPrompt?.maxGeneric != null);

  return (
    <PromptModalController
      isActive={isActivePromptModal}
      promptStateKey={currentPrompt}
    >
      {promptType === PT.RevealCards && currentPrompt?.cards && currentPrompt?.message != null && (
        <RevealCardsModal
          cards={currentPrompt.cards}
          message={currentPrompt.message}
          onConfirm={onRevealCardsAcknowledged}
        />
      )}

      {promptType === PT.ChooseMode && currentPrompt?.options && (
        <ChooseModeModal
          options={currentPrompt.options}
          minChoices={currentPrompt.minChoices ?? 1}
          maxChoices={currentPrompt.maxChoices ?? 1}
          cardName={currentPrompt.sourceCardName}
          onConfirm={onModeDecision}
        />
      )}

      {promptType === PT.ChooseOptionalTrigger && currentPrompt?.description != null && (
        <ChooseOptionalTriggerModal
          description={currentPrompt.description}
          cardName={currentPrompt.sourceCardName}
          cards={currentPrompt.cards}
          promptKind={currentPrompt.promptKind}
          optionLabels={currentPrompt.optionLabels}
          mode={currentPrompt.mode}
          api={currentPrompt.api}
          onConfirm={onOptionalTriggerDecision}
        />
      )}

      {promptType === PT.PayCostToPreventEffect && currentPrompt?.description != null && (
        <ChooseOptionalTriggerModal
          description={currentPrompt.description}
          cardName={currentPrompt.sourceCardName}
          promptKind="confirm_payment"
          optionLabels={["Decline", "Accept"]}
          mode={currentPrompt.costKind}
          api={currentPrompt.api}
          onConfirm={onPayCostToPreventEffectDecision}
        />
      )}

      {promptType === PT.ChooseColor && currentPrompt?.validColors != null && (
        <ChooseColorModal
          validColors={currentPrompt.validColors}
          sourceCardName={currentPrompt.sourceCardName}
          onConfirm={onColorDecision}
        />
      )}

      {promptType === PT.ChooseType && currentPrompt?.validTypes != null && (
        <ChooseTypeModal
          typeCategory={currentPrompt.typeCategory ?? "Creature"}
          validTypes={currentPrompt.validTypes}
          cardName={currentPrompt.sourceCardName}
          onConfirm={onTypeDecision}
        />
      )}

      {promptType === PT.ChooseNumber && currentPrompt?.min != null && currentPrompt?.max != null && (
        <ChooseNumberModal
          min={currentPrompt.min}
          max={currentPrompt.max}
          cardName={currentPrompt.sourceCardName}
          onConfirm={onNumberDecision}
        />
      )}

      {promptType === PT.ChooseCardName && currentPrompt?.validNames != null && (
        <ChooseCardNameModal
          validNames={currentPrompt.validNames}
          cardName={currentPrompt.sourceCardName}
          onConfirm={onCardNameDecision}
        />
      )}

      {promptType === PT.ChooseDamageAssignmentOrder && currentPrompt?.blockerIds != null && (
        <DamageOrderModal
          attackerId={currentPrompt.attackerId}
          blockerIds={currentPrompt.blockerIds}
          blockerCards={currentPrompt.blockerCards ?? []}
          gameViewCards={currentPrompt.gameView?.battlefield}
          onConfirm={onDamageOrderDecision}
        />
      )}

      {promptType === PT.ChooseCombatDamageAssignment
        && currentPrompt?.attackerId
        && currentPrompt?.blockerIds
        && currentPrompt?.totalDamage != null
        && currentPrompt?.gameView && (
          <VAssignCombatDamageModal
            attackerId={currentPrompt.attackerId}
            blockerIds={currentPrompt.blockerIds}
            defenderId={currentPrompt.defenderId}
            totalDamage={currentPrompt.totalDamage}
            attackerHasDeathtouch={currentPrompt.attackerHasDeathtouch}
            gameView={currentPrompt.gameView}
            onConfirm={onCombatDamageAssignmentDecision}
          />
      )}

      {promptType === PT.ReorderLibrary && currentPrompt?.cards != null && (
        <ReorderLibraryModal
          cards={currentPrompt.cards}
          cardName={currentPrompt.sourceCardName}
          onConfirm={onReorderLibraryDecision}
        />
      )}

      {promptType === PT.SpecifyManaCombo && currentPrompt?.availableColors != null && currentPrompt?.amount != null && (
        <SpecifyManaComboModal
          availableColors={currentPrompt.availableColors}
          amount={currentPrompt.amount}
          sourceCardName={currentPrompt.sourceCardName}
          onConfirm={onManaComboDecision}
        />
      )}

      {promptType === PT.ExploreDecision && currentPrompt?.revealedCardName != null && (
        <ChooseOptionalTriggerModal
          description={`Exploring revealed ${currentPrompt.revealedCardName} (nonland). Put it in graveyard or leave on top of library?`}
          cardName={currentPrompt.revealedCardName}
          promptKind="explore_decision"
          optionLabels={["Put on top of library", "Put in graveyard"]}
          onConfirm={(accept) => onExploreDecision(accept)}
        />
      )}

      {promptType === PT.HelpPayAssist && currentPrompt?.maxGeneric != null && (
        <ChooseNumberModal
          min={0}
          max={currentPrompt.maxGeneric}
          cardName={currentPrompt.cardName ?? currentPrompt.sourceCardName}
          onConfirm={(n) => onAssistDecision(n ?? 0)}
        />
      )}
    </PromptModalController>
  );
}
