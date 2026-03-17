import {
  MulliganModal,
  MulliganBottomModal,
  ChooseModeModal,
  ChooseOptionalTriggerModal,
  ChooseColorModal,
  ChooseTypeModal,
  ChooseNumberModal,
  ChooseCardNameModal,
  DamageOrderModal,
  ReorderLibraryModal,
  SpecifyManaComboModal,
} from "@/components/game/modals";
import type { Card as XMageCard } from "@/types/xmage";
import type { AgentPrompt } from "@/stores/useGameStore";

interface PromptModalsProps {
  promptType?: string;
  currentPrompt: AgentPrompt | null;
  isWaitingForResponse: boolean;
  myHand: XMageCard[];
  // Mulligan callbacks
  onMulliganDecision: (keep: boolean) => void;
  onMulliganPutBackDecision: (cardIds: string[]) => void;
  // Decision callbacks
  onModeDecision: (indices: number[]) => void;
  onOptionalTriggerDecision: (accept: boolean) => void;
  onColorDecision: (color: string) => void;
  onTypeDecision: (chosenType: string | null) => void;
  onNumberDecision: (chosenNumber: number | null) => void;
  onCardNameDecision: (chosenName: string | null) => void;
  onDamageOrderDecision: (orderedBlockerIds: string[]) => void;
  onReorderLibraryDecision: (orderedCardIds: string[]) => void;
  onManaComboDecision: (chosenColors: string[]) => void;
  onExploreDecision: (putInGraveyard: boolean) => void;
  onAssistDecision: (amountToPay: number) => void;
}

export function PromptModals({
  promptType,
  currentPrompt,
  isWaitingForResponse,
  myHand,
  onMulliganDecision,
  onMulliganPutBackDecision,
  onModeDecision,
  onOptionalTriggerDecision,
  onColorDecision,
  onTypeDecision,
  onNumberDecision,
  onCardNameDecision,
  onDamageOrderDecision,
  onReorderLibraryDecision,
  onManaComboDecision,
  onExploreDecision,
  onAssistDecision,
}: PromptModalsProps) {
  return (
    <>
      {promptType === "mulligan" && currentPrompt && (
        <MulliganModal
          handCards={myHand}
          mulliganCount={currentPrompt.mulliganCount ?? 0}
          onKeep={() => onMulliganDecision(true)}
          onMulligan={() => onMulliganDecision(false)}
          isWaitingForResponse={isWaitingForResponse}
        />
      )}

      {promptType === "mulliganPutBack" && currentPrompt?.cards && currentPrompt?.count != null && (
        <MulliganBottomModal
          handCards={currentPrompt.cards}
          count={currentPrompt.count}
          onConfirm={onMulliganPutBackDecision}
        />
      )}

      {promptType === "chooseMode" && currentPrompt?.options && (
        <ChooseModeModal
          options={currentPrompt.options}
          minChoices={currentPrompt.minChoices ?? 1}
          maxChoices={currentPrompt.maxChoices ?? 1}
          cardName={currentPrompt.sourceCardName}
          onConfirm={onModeDecision}
        />
      )}

      {promptType === "chooseOptionalTrigger" && currentPrompt?.description != null && (
        <ChooseOptionalTriggerModal
          description={currentPrompt.description}
          cardName={currentPrompt.sourceCardName}
          promptKind={currentPrompt.promptKind}
          optionLabels={currentPrompt.optionLabels}
          mode={currentPrompt.mode}
          api={currentPrompt.api}
          onConfirm={onOptionalTriggerDecision}
        />
      )}

      {promptType === "chooseColor" && currentPrompt?.validColors != null && (
        <ChooseColorModal
          validColors={currentPrompt.validColors}
          sourceCardName={currentPrompt.sourceCardName}
          onConfirm={onColorDecision}
        />
      )}

      {promptType === "chooseType" && currentPrompt?.validTypes != null && (
        <ChooseTypeModal
          typeCategory={currentPrompt.typeCategory ?? "Creature"}
          validTypes={currentPrompt.validTypes}
          cardName={currentPrompt.sourceCardName}
          onConfirm={onTypeDecision}
        />
      )}

      {promptType === "chooseNumber" && currentPrompt?.min != null && currentPrompt?.max != null && (
        <ChooseNumberModal
          min={currentPrompt.min}
          max={currentPrompt.max}
          cardName={currentPrompt.sourceCardName}
          onConfirm={onNumberDecision}
        />
      )}

      {promptType === "chooseCardName" && currentPrompt?.validNames != null && (
        <ChooseCardNameModal
          validNames={currentPrompt.validNames}
          cardName={currentPrompt.sourceCardName}
          onConfirm={onCardNameDecision}
        />
      )}

      {promptType === "chooseDamageAssignmentOrder" && currentPrompt?.blockerIds != null && (
        <DamageOrderModal
          attackerId={currentPrompt.attackerId}
          blockerIds={currentPrompt.blockerIds}
          blockerCards={currentPrompt.blockerCards ?? []}
          gameViewCards={currentPrompt.gameView?.battlefield}
          onConfirm={onDamageOrderDecision}
        />
      )}

      {promptType === "reorderLibrary" && currentPrompt?.cards != null && (
        <ReorderLibraryModal
          cards={currentPrompt.cards}
          cardName={currentPrompt.sourceCardName}
          onConfirm={onReorderLibraryDecision}
        />
      )}

      {promptType === "specifyManaCombo" && currentPrompt?.availableColors != null && currentPrompt?.amount != null && (
        <SpecifyManaComboModal
          availableColors={currentPrompt.availableColors}
          amount={currentPrompt.amount}
          sourceCardName={currentPrompt.sourceCardName}
          onConfirm={onManaComboDecision}
        />
      )}

      {promptType === "exploreDecision" && currentPrompt?.revealedCardName != null && (
        <ChooseOptionalTriggerModal
          description={`Exploring revealed ${currentPrompt.revealedCardName} (nonland). Put it in graveyard or leave on top of library?`}
          cardName={currentPrompt.revealedCardName}
          promptKind="explore_decision"
          optionLabels={["Put on top of library", "Put in graveyard"]}
          onConfirm={(accept) => onExploreDecision(accept)}
        />
      )}

      {promptType === "helpPayAssist" && currentPrompt?.maxGeneric != null && (
        <ChooseNumberModal
          min={0}
          max={currentPrompt.maxGeneric}
          cardName={currentPrompt.cardName ?? currentPrompt.sourceCardName}
          onConfirm={(n) => onAssistDecision(n ?? 0)}
        />
      )}
    </>
  );
}
