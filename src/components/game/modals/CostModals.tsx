import {
  PayCombatCostModal,
  ChooseCardsModal,
  PromptModalController,
} from "@/components/game/modals";
import {
  KickerModal,
  BuybackModal,
  MultikickerModal,
  ReplicateModal,
  AlternativeCostModal,
  PhyrexianModal,
} from "@/components/game/cost-modals";
import type { AgentPrompt } from "@/stores/useGameStore";
import type { PromptType } from "@/types/promptType";
import { PromptType as PT } from "@/types/promptType";

interface CostModalsProps {
  promptType?: PromptType;
  currentPrompt: AgentPrompt | null;
  // Cost decision callbacks
  onPhyrexianDecision: (payLife: boolean) => void;
  onKickerDecision: (kicked: boolean) => void;
  onBuybackDecision: (paid: boolean) => void;
  onMultikickerDecision: (kickCount: number) => void;
  onReplicateDecision: (replicateCount: number) => void;
  onAlternativeCostDecision: (chosenIndex: number) => void;
  onPayCombatCost: () => void;
  onDeclineCombatCost: () => void;
  onDelveDecision: (cardIds: string[]) => void;
  onConvokeDecision: (cardIds: string[]) => void;
  onImproviseDecision: (cardIds: string[]) => void;
}

export function CostModals({
  promptType,
  currentPrompt,
  onPhyrexianDecision,
  onKickerDecision,
  onBuybackDecision,
  onMultikickerDecision,
  onReplicateDecision,
  onAlternativeCostDecision,
  onPayCombatCost,
  onDeclineCombatCost,
  onDelveDecision,
  onConvokeDecision,
  onImproviseDecision,
}: CostModalsProps) {
  const isActiveCostModal =
    (promptType === PT.ChoosePhyrexian && currentPrompt?.phyrexianColor != null) ||
    (promptType === PT.ChooseKicker && currentPrompt?.kickerCost != null) ||
    (promptType === PT.ChooseBuyback && currentPrompt?.buybackCost != null) ||
    (promptType === PT.ChooseMultikicker && currentPrompt?.cost != null) ||
    (promptType === PT.ChooseReplicate && currentPrompt?.cost != null) ||
    (promptType === PT.ChooseAlternativeCost && currentPrompt?.options != null) ||
    (promptType === PT.PayCombatCost && currentPrompt?.description != null) ||
    (promptType === PT.ChooseDelve && currentPrompt?.zoneCards != null) ||
    (promptType === PT.ChooseConvoke && currentPrompt?.validCardIds != null) ||
    (promptType === PT.ChooseImprovise && currentPrompt?.validCardIds != null);

  return (
    <PromptModalController
      isActive={isActiveCostModal}
      promptStateKey={currentPrompt}
    >
      {promptType === PT.ChoosePhyrexian && currentPrompt?.phyrexianColor != null && (
        <PhyrexianModal
          phyrexianColor={currentPrompt.phyrexianColor}
          sourceCardName={currentPrompt.sourceCardName}
          onDecide={onPhyrexianDecision}
        />
      )}

      {promptType === PT.ChooseKicker && currentPrompt?.kickerCost != null && (
        <KickerModal
          kickerCost={currentPrompt.kickerCost}
          sourceCardName={currentPrompt.sourceCardName}
          onDecide={onKickerDecision}
        />
      )}

      {promptType === PT.ChooseBuyback && currentPrompt?.buybackCost != null && (
        <BuybackModal
          buybackCost={currentPrompt.buybackCost}
          sourceCardName={currentPrompt.sourceCardName}
          onDecide={onBuybackDecision}
        />
      )}

      {promptType === PT.ChooseMultikicker && currentPrompt?.cost != null && (
        <MultikickerModal
          cost={currentPrompt.cost}
          maxKicks={currentPrompt.maxKicks ?? 0}
          sourceCardName={currentPrompt.sourceCardName}
          onDecide={onMultikickerDecision}
        />
      )}

      {promptType === PT.ChooseReplicate && currentPrompt?.cost != null && (
        <ReplicateModal
          cost={currentPrompt.cost}
          maxReplicates={currentPrompt.maxReplicates ?? 0}
          sourceCardName={currentPrompt.sourceCardName}
          onDecide={onReplicateDecision}
        />
      )}

      {promptType === PT.ChooseAlternativeCost && currentPrompt?.options != null && (
        <AlternativeCostModal
          options={currentPrompt.options}
          sourceCardName={currentPrompt.sourceCardName}
          onDecide={onAlternativeCostDecision}
        />
      )}

      {promptType === PT.PayCombatCost && currentPrompt?.description != null && (
        <PayCombatCostModal
          attackerName={currentPrompt.attackerName ?? "Creature"}
          cost={currentPrompt.cost != null ? Number(currentPrompt.cost) : 0}
          description={currentPrompt.description}
          manaPool={currentPrompt.gameView?.players?.[0]?.manaPool ?? {}}
          onPay={onPayCombatCost}
          onDecline={onDeclineCombatCost}
        />
      )}

      {promptType === PT.ChooseDelve && currentPrompt?.zoneCards != null && (
        <ChooseCardsModal
          cards={currentPrompt.zoneCards}
          minChoices={0}
          maxChoices={currentPrompt.maxCards ?? 0}
          sourceCardName={currentPrompt.sourceCardName}
          onConfirm={onDelveDecision}
        />
      )}

      {promptType === PT.ChooseConvoke && currentPrompt?.validCardIds != null && (
        <ChooseCardsModal
          cards={currentPrompt.gameView?.battlefield?.filter(
            (c) => currentPrompt.validCardIds?.includes(c.id)
          ) ?? []}
          minChoices={0}
          maxChoices={currentPrompt.validCardIds?.length ?? 0}
          sourceCardName={currentPrompt.sourceCardName}
          description={currentPrompt.remainingCost ? `Remaining cost: ${currentPrompt.remainingCost}` : undefined}
          onConfirm={onConvokeDecision}
        />
      )}

      {promptType === PT.ChooseImprovise && currentPrompt?.validCardIds != null && (
        <ChooseCardsModal
          cards={currentPrompt.gameView?.battlefield?.filter(
            (c) => currentPrompt.validCardIds?.includes(c.id)
          ) ?? []}
          minChoices={0}
          maxChoices={currentPrompt.validCardIds?.length ?? 0}
          sourceCardName={currentPrompt.sourceCardName}
          description={currentPrompt.remainingCost ? `Remaining cost: ${currentPrompt.remainingCost}` : undefined}
          onConfirm={onImproviseDecision}
        />
      )}
    </PromptModalController>
  );
}
