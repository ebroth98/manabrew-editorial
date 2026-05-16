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
  LibraryPeekModal,
  PromptModalController,
} from "@/components/game/modals";
import {
  DiceRollFeedback,
  FirstPlayerRollFeedback,
  ChooseRollToIgnoreModal,
  ChooseRollToSwapModal,
  ChooseRollToModifyModal,
  ChooseDiceToRerollModal,
  ChooseRollSwapValueModal,
} from "@/components/game/dice";
import type { AgentPrompt } from "@/stores/useGameStore";
import type { PromptType } from "@/types/promptType";
import type { DeckCard, GameCard } from "@/types/manabrew";
import { PromptType as PT } from "@/types/promptType";

interface PromptModalsProps {
  promptType?: PromptType;
  currentPrompt: AgentPrompt | null;
  sourceDeckCard?: DeckCard;
  revealedDeckCard?: DeckCard;
  // Decision callbacks
  onModeDecision: (indices: number[]) => void;
  onRevealCardsAcknowledged: () => void;
  onPayCostToPreventEffectDecision: (accept: boolean) => void;
  onOptionalTriggerDecision: (accept: boolean) => void;
  onColorDecision: (color: string) => void;
  onTypeDecision: (chosenType: string | null) => void;
  onNumberDecision: (chosenNumber: number | null) => void;
  onCardNameDecision: (chosenName: string | null) => void;
  onScryDecision: (bottomCardIds: string[]) => void;
  onSurveilDecision: (graveyardCardIds: string[]) => void;
  onDigDecision: (chosenCardIds: string[]) => void;
  onDiscardDecision: (discardedCardIds: string[]) => void;
  onDamageOrderDecision: (orderedBlockerIds: string[]) => void;
  onCombatDamageAssignmentDecision: (assignments: { assigneeId: string; damage: number }[]) => void;
  onReorderLibraryDecision: (orderedCardIds: string[]) => void;
  onManaComboDecision: (chosenColors: string[]) => void;
  onExploreDecision: (putInGraveyard: boolean) => void;
  onAssistDecision: (amountToPay: number) => void;
  // Dice rolls
  onDiceRolledAcknowledged: () => void;
  onRollToIgnoreDecision: (roll: number | null) => void;
  onRollToSwapDecision: (roll: number | null) => void;
  onRollToModifyDecision: (roll: number | null) => void;
  onDiceToRerollDecision: (rolls: number[]) => void;
  onRollSwapValueDecision: (choice: "power" | "toughness" | null) => void;
  onFirstPlayerRollAcknowledged: () => void;
}

export function PromptModals({
  promptType,
  currentPrompt,
  sourceDeckCard,
  revealedDeckCard,
  onModeDecision,
  onRevealCardsAcknowledged,
  onPayCostToPreventEffectDecision,
  onOptionalTriggerDecision,
  onColorDecision,
  onTypeDecision,
  onNumberDecision,
  onCardNameDecision,
  onScryDecision,
  onSurveilDecision,
  onDigDecision,
  onDiscardDecision,
  onDamageOrderDecision,
  onCombatDamageAssignmentDecision,
  onReorderLibraryDecision,
  onManaComboDecision,
  onExploreDecision,
  onAssistDecision,
  onDiceRolledAcknowledged,
  onRollToIgnoreDecision,
  onRollToSwapDecision,
  onRollToModifyDecision,
  onDiceToRerollDecision,
  onRollSwapValueDecision,
  onFirstPlayerRollAcknowledged,
}: PromptModalsProps) {
  const isActivePromptModal =
    (promptType === PT.RevealCards &&
      currentPrompt?.cards != null &&
      currentPrompt?.message != null) ||
    // Mulligan + MulliganPutBack render in the same bottom-right slot
    // as Pass Priority (see `PromptActionController`), and the real
    // in-game Pixi hand drives card selection via its selection mode —
    // they intentionally do NOT open a modal here.
    (promptType === PT.ChooseMode && currentPrompt?.options != null) ||
    (promptType === PT.ChooseOptionalTrigger && currentPrompt?.description != null) ||
    (promptType === PT.PayCostToPreventEffect && currentPrompt?.description != null) ||
    (promptType === PT.ChooseColor && currentPrompt?.validColors != null) ||
    (promptType === PT.ChooseType && currentPrompt?.validTypes != null) ||
    (promptType === PT.ChooseNumber && currentPrompt?.min != null && currentPrompt?.max != null) ||
    (promptType === PT.ChooseCardName && currentPrompt?.validNames != null) ||
    ((promptType === PT.Scry || promptType === PT.Surveil || promptType === PT.Dig) &&
      currentPrompt?.cards != null) ||
    (promptType === PT.ChooseDiscard &&
      currentPrompt?.handCardIds != null &&
      currentPrompt?.gameView != null) ||
    (promptType === PT.ChooseDamageAssignmentOrder && currentPrompt?.blockerIds != null) ||
    (promptType === PT.ChooseCombatDamageAssignment &&
      currentPrompt?.attackerId != null &&
      currentPrompt?.blockerIds != null &&
      currentPrompt?.totalDamage != null &&
      currentPrompt?.gameView != null) ||
    (promptType === PT.ReorderLibrary && currentPrompt?.cards != null) ||
    (promptType === PT.SpecifyManaCombo &&
      currentPrompt?.availableColors != null &&
      currentPrompt?.amount != null) ||
    (promptType === PT.ExploreDecision && currentPrompt?.revealedCardName != null) ||
    (promptType === PT.HelpPayAssist && currentPrompt?.maxGeneric != null) ||
    (promptType === PT.FirstPlayerRoll &&
      currentPrompt?.sides != null &&
      currentPrompt?.firstPlayerRolls != null &&
      currentPrompt?.winnerPlayerId != null) ||
    (promptType === PT.DiceRolled &&
      currentPrompt?.sides != null &&
      currentPrompt?.finalResults != null) ||
    (promptType === PT.ChooseRollToIgnore && currentPrompt?.rolls != null) ||
    (promptType === PT.ChooseRollToSwap && currentPrompt?.rolls != null) ||
    (promptType === PT.ChooseRollToModify && currentPrompt?.rolls != null) ||
    (promptType === PT.ChooseDiceToReroll && currentPrompt?.rolls != null) ||
    (promptType === PT.ChooseRollSwapValue &&
      currentPrompt?.currentResult != null &&
      currentPrompt?.power != null &&
      currentPrompt?.toughness != null);

  return (
    <PromptModalController isActive={isActivePromptModal} promptStateKey={currentPrompt}>
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
          sourceCard={sourceDeckCard}
          sourceLabel={currentPrompt.sourceCardName}
          onConfirm={onModeDecision}
        />
      )}

      {promptType === PT.ChooseOptionalTrigger && currentPrompt?.description != null && (
        <ChooseOptionalTriggerModal
          description={currentPrompt.description}
          sourceCard={sourceDeckCard}
          sourceCardId={currentPrompt.sourceCardId}
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
          sourceCard={sourceDeckCard}
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
          sourceCard={sourceDeckCard}
          onConfirm={onTypeDecision}
        />
      )}

      {promptType === PT.ChooseNumber &&
        currentPrompt?.min != null &&
        currentPrompt?.max != null && (
          <ChooseNumberModal
            min={currentPrompt.min}
            max={currentPrompt.max}
            sourceCard={sourceDeckCard}
            onConfirm={onNumberDecision}
          />
        )}

      {promptType === PT.ChooseCardName &&
        currentPrompt?.validNames != null && (
          <ChooseCardNameModal
            validNames={currentPrompt.validNames}
            sourceCard={sourceDeckCard}
            onConfirm={onCardNameDecision}
          />
        )}

      {promptType === PT.Scry && currentPrompt?.cards != null && (
        <LibraryPeekModal mode="scry" cards={currentPrompt.cards} onConfirm={onScryDecision} />
      )}

      {promptType === PT.Surveil && currentPrompt?.cards != null && (
        <LibraryPeekModal
          mode="surveil"
          cards={currentPrompt.cards}
          onConfirm={onSurveilDecision}
        />
      )}

      {promptType === PT.Dig && currentPrompt?.cards != null && (
        <LibraryPeekModal
          mode="dig"
          cards={currentPrompt.cards}
          numToTake={currentPrompt.numToTake}
          optional={currentPrompt.optional}
          onConfirm={onDigDecision}
        />
      )}

      {promptType === PT.ChooseDiscard &&
        currentPrompt?.handCardIds != null &&
        currentPrompt?.gameView != null && (
          <LibraryPeekModal
            mode="discard"
            cards={currentPrompt.handCardIds
              .map((id) => currentPrompt.gameView.myHand.find((card) => card.id === id))
              .filter((card): card is GameCard => card != null)}
            numToTake={currentPrompt.numToDiscard}
            onConfirm={onDiscardDecision}
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

      {promptType === PT.ChooseCombatDamageAssignment &&
        currentPrompt?.attackerId &&
        currentPrompt?.blockerIds &&
        currentPrompt?.totalDamage != null &&
        currentPrompt?.gameView && (
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
          sourceCard={sourceDeckCard}
          onConfirm={onReorderLibraryDecision}
        />
      )}

      {promptType === PT.SpecifyManaCombo &&
        currentPrompt?.availableColors != null &&
        currentPrompt?.amount != null && (
          <SpecifyManaComboModal
            availableColors={currentPrompt.availableColors}
            amount={currentPrompt.amount}
            sourceCardName={currentPrompt.sourceCardName}
            onConfirm={onManaComboDecision}
          />
        )}

      {promptType === PT.ExploreDecision &&
        currentPrompt?.revealedCardName != null &&
        revealedDeckCard && (
          <ChooseOptionalTriggerModal
            description={`Exploring revealed ${currentPrompt.revealedCardName} (nonland). Put it in graveyard or leave on top of library?`}
            sourceCard={revealedDeckCard}
            promptKind="explore_decision"
            optionLabels={["Put on top of library", "Put in graveyard"]}
            onConfirm={(accept) => onExploreDecision(accept)}
          />
        )}

      {promptType === PT.HelpPayAssist && currentPrompt?.maxGeneric != null && (
        <ChooseNumberModal
          min={0}
          max={currentPrompt.maxGeneric}
          sourceCard={sourceDeckCard}
          onConfirm={(n) => onAssistDecision(n ?? 0)}
        />
      )}

      {promptType === PT.FirstPlayerRoll &&
        currentPrompt?.sides != null &&
        currentPrompt?.firstPlayerRolls != null &&
        currentPrompt?.winnerPlayerId != null && (
          <FirstPlayerRollFeedback
            sides={currentPrompt.sides}
            rolls={currentPrompt.firstPlayerRolls}
            winnerPlayerId={currentPrompt.winnerPlayerId}
            players={
              currentPrompt.gameView?.players?.map((p) => ({
                id: p.id,
                isHuman: p.isHuman,
              })) ?? []
            }
            onAcknowledge={onFirstPlayerRollAcknowledged}
          />
        )}

      {promptType === PT.DiceRolled &&
        currentPrompt?.sides != null &&
        currentPrompt?.finalResults != null && (
          <DiceRollFeedback
            sides={currentPrompt.sides}
            naturalResults={currentPrompt.naturalResults ?? currentPrompt.finalResults}
            finalResults={currentPrompt.finalResults}
            ignoredRolls={currentPrompt.ignoredRolls}
            playerId={currentPrompt.playerId}
            players={
              currentPrompt.gameView?.players?.map((p) => ({
                id: p.id,
                isHuman: p.isHuman,
              })) ?? []
            }
            sourceCard={sourceDeckCard}
            onAcknowledge={onDiceRolledAcknowledged}
          />
        )}

      {promptType === PT.ChooseRollToIgnore && currentPrompt?.rolls != null && (
        <ChooseRollToIgnoreModal
          rolls={currentPrompt.rolls}
          sides={currentPrompt.sides}
          sourceCard={sourceDeckCard}
          onConfirm={onRollToIgnoreDecision}
        />
      )}

      {promptType === PT.ChooseRollToSwap && currentPrompt?.rolls != null && (
        <ChooseRollToSwapModal
          rolls={currentPrompt.rolls}
          sides={currentPrompt.sides}
          sourceCard={sourceDeckCard}
          onConfirm={onRollToSwapDecision}
        />
      )}

      {promptType === PT.ChooseRollToModify && currentPrompt?.rolls != null && (
        <ChooseRollToModifyModal
          rolls={currentPrompt.rolls}
          sides={currentPrompt.sides}
          sourceCard={sourceDeckCard}
          onConfirm={onRollToModifyDecision}
        />
      )}

      {promptType === PT.ChooseDiceToReroll && currentPrompt?.rolls != null && (
        <ChooseDiceToRerollModal
          rolls={currentPrompt.rolls}
          sides={currentPrompt.sides}
          sourceCard={sourceDeckCard}
          onConfirm={onDiceToRerollDecision}
        />
      )}

      {promptType === PT.ChooseRollSwapValue &&
        currentPrompt?.currentResult != null &&
        currentPrompt?.power != null &&
        currentPrompt?.toughness != null && (
          <ChooseRollSwapValueModal
            currentResult={currentPrompt.currentResult}
            power={currentPrompt.power}
            toughness={currentPrompt.toughness}
            sourceCard={sourceDeckCard}
            onConfirm={onRollSwapValueDecision}
          />
        )}
    </PromptModalController>
  );
}
