import { ZoneViewer } from "@/components/game/ZoneViewer";
import { ZoneTargetSelector } from "@/components/game/ZoneTargetSelector";
import { LibraryPeekModal, type LibraryPeekMode } from "@/components/game/LibraryPeekModal";
import { SpellStackModal } from "@/components/game/SpellStackModal";
import { ChooseModeModal } from "@/components/game/ChooseModeModal";
import { ChooseOptionalTriggerModal } from "@/components/game/ChooseOptionalTriggerModal";
import { KickerModal, BuybackModal, MultikickerModal, ReplicateModal, AlternativeCostModal } from "@/components/game/cost-modals";
import { ChooseColorModal } from "@/components/game/ChooseColorModal";
import { ChooseCardsModal } from "@/components/game/ChooseCardsModal";
import { ChooseTypeModal } from "@/components/game/ChooseTypeModal";
import { ChooseNumberModal } from "@/components/game/ChooseNumberModal";
import { ChooseCardNameModal } from "@/components/game/ChooseCardNameModal";
import { AbilityPickerModal } from "@/components/game/AbilityPickerModal";
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
  // Decision callbacks
  onModeDecision: (indices: number[]) => void;
  onOptionalTriggerDecision: (accept: boolean) => void;
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
  onModeDecision,
  onOptionalTriggerDecision,
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
}: GameModalsProps) {
  return (
    <>
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
          onConfirm={onOptionalTriggerDecision}
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
    </>
  );
}
