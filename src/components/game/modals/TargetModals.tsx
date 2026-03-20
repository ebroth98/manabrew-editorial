import {
  ZoneViewer,
  LibraryPeekModal,
  type LibraryPeekMode,
  SpellStackModal,
  ChooseCardsModal,
  AbilityPickerModal,
} from "@/components/game/modals";
import { ZoneTargetSelector } from "@/components/game/ZoneTargetSelector";
import type { Card as XMageCard, StackObject, ActivatableAbilityInfo } from "@/types/openmagic";
import type { AgentPrompt } from "@/stores/useGameStore";
import type { PromptType } from "@/types/promptType";
import { PromptType as PT } from "@/types/promptType";

interface TargetModalsProps {
  promptType?: PromptType;
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
  // Card selection
  onChooseCardsDecision: (cardIds: string[]) => void;
  // Exert / Enlist
  onExertDecision: (chosenAttackerIds: string[]) => void;
  onEnlistDecision: (chosenAttackerIds: string[]) => void;
}

export function TargetModals({
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
  onChooseCardsDecision,
  onExertDecision,
  onEnlistDecision,
}: TargetModalsProps) {
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

      {promptType === PT.ChooseCardsForEffect && currentPrompt?.zoneCards != null && (
        <ChooseCardsModal
          cards={currentPrompt.zoneCards}
          minChoices={currentPrompt.minChoices ?? 1}
          maxChoices={currentPrompt.maxChoices ?? 1}
          sourceCardName={currentPrompt.sourceCardName}
          onConfirm={onChooseCardsDecision}
        />
      )}

      {promptType === PT.ChooseExertAttackers && currentPrompt?.attackerCards != null && (
        <ChooseCardsModal
          cards={currentPrompt.attackerCards}
          minChoices={0}
          maxChoices={currentPrompt.attackerCards.length}
          sourceCardName="Exert Attackers"
          description="Choose which attacking creatures to exert. Exerted creatures won't untap during your next untap step."
          onConfirm={onExertDecision}
        />
      )}

      {promptType === PT.ChooseEnlistAttackers && currentPrompt?.attackerCards != null && (
        <ChooseCardsModal
          cards={currentPrompt.attackerCards}
          minChoices={0}
          maxChoices={currentPrompt.attackerCards.length}
          sourceCardName="Enlist Attackers"
          description="Choose which attacking creatures to enlist. Enlisted creatures tap a non-attacking creature to add its power."
          onConfirm={onEnlistDecision}
        />
      )}
    </>
  );
}
