import { ZoneViewer } from "@/components/game/ZoneViewer";
import { ZoneTargetSelector } from "@/components/game/ZoneTargetSelector";
import { LibraryPeekModal, type LibraryPeekMode } from "@/components/game/LibraryPeekModal";
import { SpellStackModal } from "@/components/game/SpellStackModal";
import { ChooseModeModal } from "@/components/game/ChooseModeModal";
import { ChooseOptionalTriggerModal } from "@/components/game/ChooseOptionalTriggerModal";
import { KickerModal, BuybackModal, MultikickerModal, ReplicateModal, AlternativeCostModal, PhyrexianModal } from "@/components/game/cost-modals";
import { ChooseColorModal } from "@/components/game/ChooseColorModal";
import { ChooseCardsModal } from "@/components/game/ChooseCardsModal";
import { ChooseTypeModal } from "@/components/game/ChooseTypeModal";
import { ChooseNumberModal } from "@/components/game/ChooseNumberModal";
import { ChooseCardNameModal } from "@/components/game/ChooseCardNameModal";
import { DamageOrderModal } from "@/components/game/DamageOrderModal";
import { ReorderLibraryModal } from "@/components/game/ReorderLibraryModal";
import { PayCombatCostModal } from "@/components/game/PayCombatCostModal";
import { PayManaCostModal } from "@/components/game/PayManaCostModal";
import { AbilityPickerModal } from "@/components/game/AbilityPickerModal";
import { SpecifyManaComboModal } from "@/components/game/SpecifyManaComboModal";
import { MulliganModal } from "@/components/game/MulliganModal";
import { MulliganBottomModal } from "@/components/game/MulliganBottomModal";
import type { Card as XMageCard, StackObject, ActivatableAbilityInfo } from "@/types/xmage";
import type { AgentPrompt } from "@/stores/useGameStore";

interface GameModalsProps {
  promptType?: string;
  currentPrompt: AgentPrompt | null;
  // Zone viewer
  viewingZone: { title: string; cards: XMageCard[]; onClickCard?: (cardId: string) => void } | null;
  onCloseZone: () => void;
  // Zone target selector
  zoneTargetSelector: { title: string; cards: XMageCard[]; validCardIds: string[] } | null;
  onSelectZoneTarget: (cardId: string) => void;
  onCancelZoneTarget: () => void;
  // Library peek
  libraryPeekModal: { mode: LibraryPeekMode; cards: XMageCard[]; numToTake?: number; optional?: boolean } | null;
  onLibraryPeekConfirm: (selectedIds: string[]) => void;
  // Spell stack
  spellStackModalOpen: boolean;
  stack: StackObject[];
  validSpellIds: string[];
  onTargetSpell: (spellId: string) => void;
  onCloseStack: () => void;
  // Ability picker
  abilityPickerState: { cardId: string; cardName: string; abilities: ActivatableAbilityInfo[] } | null;
  onSelectAbility: (ability: ActivatableAbilityInfo) => void;
  onCancelAbilityPicker: () => void;
  // Mulligan callbacks
  onMulliganDecision: (keep: boolean) => void;
  onMulliganPutBackDecision: (cardIds: string[]) => void;
  isWaitingForResponse: boolean;
  myHand: XMageCard[];
  // Decision callbacks
  onModeDecision: (indices: number[]) => void;
  onOptionalTriggerDecision: (accept: boolean) => void;
  onPhyrexianDecision: (payLife: boolean) => void;
  onKickerDecision: (kicked: boolean) => void;
  onBuybackDecision: (paid: boolean) => void;
  onMultikickerDecision: (kickCount: number) => void;
  onReplicateDecision: (replicateCount: number) => void;
  onAlternativeCostDecision: (chosenIndex: number) => void;
  onColorDecision: (color: string) => void;
  onChooseCardsDecision: (cardIds: string[]) => void;
  onTypeDecision: (chosenType: string | null) => void;
  onNumberDecision: (chosenNumber: number | null) => void;
  onCardNameDecision: (chosenName: string | null) => void;
  onDamageOrderDecision: (orderedBlockerIds: string[]) => void;
  // Pay combat cost
  onPayCombatCost: () => void;
  onDeclineCombatCost: () => void;
  // Pay mana cost
  onPayManaCost: () => void;
  onCancelManaCost: () => void;
  // Delve / Convoke
  onDelveDecision: (cardIds: string[]) => void;
  onConvokeDecision: (cardIds: string[]) => void;
  onImproviseDecision: (cardIds: string[]) => void;
  // Specify mana combo
  onManaComboDecision: (chosenColors: string[]) => void;
  // Explore
  onExploreDecision: (putInGraveyard: boolean) => void;
  // Exert / Enlist
  onExertDecision: (chosenAttackerIds: string[]) => void;
  onEnlistDecision: (chosenAttackerIds: string[]) => void;
  // Reorder library
  onReorderLibraryDecision: (orderedCardIds: string[]) => void;
  // Assist
  onAssistDecision: (amountToPay: number) => void;
}

export function GameModals({
  promptType,
  currentPrompt,
  viewingZone,
  onCloseZone,
  zoneTargetSelector,
  onSelectZoneTarget,
  onCancelZoneTarget,
  libraryPeekModal,
  onLibraryPeekConfirm,
  spellStackModalOpen,
  stack,
  validSpellIds,
  onTargetSpell,
  onCloseStack,
  abilityPickerState,
  onSelectAbility,
  onCancelAbilityPicker,
  onMulliganDecision,
  onMulliganPutBackDecision,
  isWaitingForResponse,
  myHand,
  onModeDecision,
  onOptionalTriggerDecision,
  onPhyrexianDecision,
  onKickerDecision,
  onBuybackDecision,
  onMultikickerDecision,
  onReplicateDecision,
  onAlternativeCostDecision,
  onColorDecision,
  onChooseCardsDecision,
  onTypeDecision,
  onNumberDecision,
  onCardNameDecision,
  onDamageOrderDecision,
  onPayCombatCost,
  onDeclineCombatCost,
  onPayManaCost,
  onCancelManaCost,
  onDelveDecision,
  onConvokeDecision,
  onImproviseDecision,
  onManaComboDecision,
  onExploreDecision,
  onExertDecision,
  onEnlistDecision,
  onReorderLibraryDecision,
  onAssistDecision,
}: GameModalsProps) {
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

      {viewingZone && (
        <ZoneViewer
          title={viewingZone.title}
          cards={viewingZone.cards}
          onClose={onCloseZone}
          onClickCard={viewingZone.onClickCard}
        />
      )}

      {zoneTargetSelector && (
        <ZoneTargetSelector
          title={zoneTargetSelector.title}
          cards={zoneTargetSelector.cards}
          validCardIds={zoneTargetSelector.validCardIds}
          onSelect={onSelectZoneTarget}
          onCancel={onCancelZoneTarget}
        />
      )}

      {libraryPeekModal && (
        <LibraryPeekModal
          mode={libraryPeekModal.mode}
          cards={libraryPeekModal.cards}
          numToTake={libraryPeekModal.numToTake}
          optional={libraryPeekModal.optional}
          onConfirm={onLibraryPeekConfirm}
        />
      )}

      {spellStackModalOpen && stack.length > 0 && (
        <SpellStackModal
          stack={stack}
          validSpellIds={validSpellIds}
          onTarget={onTargetSpell}
          onCancel={onCloseStack}
        />
      )}

      {abilityPickerState && (
        <AbilityPickerModal
          cardName={abilityPickerState.cardName}
          abilities={abilityPickerState.abilities}
          onSelect={onSelectAbility}
          onCancel={onCancelAbilityPicker}
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

      {promptType === "choosePhyrexian" && currentPrompt?.phyrexianColor != null && (
        <PhyrexianModal
          phyrexianColor={currentPrompt.phyrexianColor}
          sourceCardName={currentPrompt.sourceCardName}
          onDecide={onPhyrexianDecision}
        />
      )}

      {promptType === "chooseKicker" && currentPrompt?.kickerCost != null && (
        <KickerModal
          kickerCost={currentPrompt.kickerCost}
          sourceCardName={currentPrompt.sourceCardName}
          onDecide={onKickerDecision}
        />
      )}

      {promptType === "chooseBuyback" && currentPrompt?.buybackCost != null && (
        <BuybackModal
          buybackCost={currentPrompt.buybackCost}
          sourceCardName={currentPrompt.sourceCardName}
          onDecide={onBuybackDecision}
        />
      )}

      {promptType === "chooseMultikicker" && currentPrompt?.cost != null && (
        <MultikickerModal
          cost={currentPrompt.cost}
          maxKicks={currentPrompt.maxKicks ?? 0}
          sourceCardName={currentPrompt.sourceCardName}
          onDecide={onMultikickerDecision}
        />
      )}

      {promptType === "chooseReplicate" && currentPrompt?.cost != null && (
        <ReplicateModal
          cost={currentPrompt.cost}
          maxReplicates={currentPrompt.maxReplicates ?? 0}
          sourceCardName={currentPrompt.sourceCardName}
          onDecide={onReplicateDecision}
        />
      )}

      {promptType === "chooseAlternativeCost" && currentPrompt?.options != null && (
        <AlternativeCostModal
          options={currentPrompt.options}
          sourceCardName={currentPrompt.sourceCardName}
          onDecide={onAlternativeCostDecision}
        />
      )}

      {promptType === "chooseColor" && currentPrompt?.validColors != null && (
        <ChooseColorModal
          validColors={currentPrompt.validColors}
          sourceCardName={currentPrompt.sourceCardName}
          onConfirm={onColorDecision}
        />
      )}

      {promptType === "chooseCardsForEffect" && currentPrompt?.zoneCards != null && (
        <ChooseCardsModal
          cards={currentPrompt.zoneCards}
          minChoices={currentPrompt.minChoices ?? 1}
          maxChoices={currentPrompt.maxChoices ?? 1}
          sourceCardName={currentPrompt.sourceCardName}
          onConfirm={onChooseCardsDecision}
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
      {promptType === "payCombatCost" && currentPrompt?.description != null && (
        <PayCombatCostModal
          attackerName={currentPrompt.attackerName ?? "Creature"}
          cost={currentPrompt.cost != null ? Number(currentPrompt.cost) : 0}
          description={currentPrompt.description}
          manaPool={currentPrompt.gameView?.players?.[0]?.manaPool ?? {}}
          onPay={onPayCombatCost}
          onDecline={onDeclineCombatCost}
        />
      )}
      {promptType === "chooseDelve" && currentPrompt?.zoneCards != null && (
        <ChooseCardsModal
          cards={currentPrompt.zoneCards}
          minChoices={0}
          maxChoices={currentPrompt.maxCards ?? 0}
          sourceCardName={currentPrompt.sourceCardName}
          onConfirm={onDelveDecision}
        />
      )}

      {promptType === "chooseConvoke" && currentPrompt?.validCardIds != null && (
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

      {promptType === "chooseImprovise" && currentPrompt?.validCardIds != null && (
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

      {promptType === "payManaCost" && currentPrompt?.manaCost != null && (
        <PayManaCostModal
          cardName={currentPrompt.cardName ?? "Spell"}
          manaCost={currentPrompt.manaCost}
          manaPool={currentPrompt.gameView?.players?.[0]?.manaPool ?? {}}
          onPay={onPayManaCost}
          onCancel={onCancelManaCost}
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

      {promptType === "exploreDecision" && currentPrompt?.revealedCardName != null && (
        <ChooseOptionalTriggerModal
          description={`Exploring revealed ${currentPrompt.revealedCardName} (nonland). Put it in graveyard or leave on top of library?`}
          cardName={currentPrompt.revealedCardName}
          promptKind="explore_decision"
          optionLabels={["Put on top of library", "Put in graveyard"]}
          onConfirm={(accept) => onExploreDecision(accept)}
        />
      )}

      {promptType === "chooseExertAttackers" && currentPrompt?.attackerCards != null && (
        <ChooseCardsModal
          cards={currentPrompt.attackerCards}
          minChoices={0}
          maxChoices={currentPrompt.attackerCards.length}
          sourceCardName="Exert Attackers"
          description="Choose which attacking creatures to exert. Exerted creatures won't untap during your next untap step."
          onConfirm={onExertDecision}
        />
      )}

      {promptType === "chooseEnlistAttackers" && currentPrompt?.attackerCards != null && (
        <ChooseCardsModal
          cards={currentPrompt.attackerCards}
          minChoices={0}
          maxChoices={currentPrompt.attackerCards.length}
          sourceCardName="Enlist Attackers"
          description="Choose which attacking creatures to enlist. Enlisted creatures tap a non-attacking creature to add its power."
          onConfirm={onEnlistDecision}
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
