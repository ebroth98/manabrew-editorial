import { PromptModals } from "@/components/game/modals/PromptModals";
import { CostModals } from "@/components/game/modals/CostModals";
import { TargetModals } from "@/components/game/modals/TargetModals";
import type { LibraryPeekMode } from "@/components/game/modals";
import type { Card as XMageCard, StackObject } from "@/types/openmagic";
import type { AgentPrompt } from "@/stores/useGameStore";
import type { AbilityPickerState, HandActionOption } from "@/stores/useGameUIStore";
import type { PromptType } from "@/types/promptType";

interface GameModalsProps {
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
  abilityPickerState: AbilityPickerState | null;
  onSelectAbility: (ability: HandActionOption) => void;
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
  onCombatDamageAssignmentDecision: (assignments: { assigneeId: string; damage: number }[]) => void;
  // Pay combat cost
  onPayCombatCost: () => void;
  onDeclineCombatCost: () => void;

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
  onCombatDamageAssignmentDecision,
  onPayCombatCost,
  onDeclineCombatCost,
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
      <PromptModals
        promptType={promptType}
        currentPrompt={currentPrompt}
        isWaitingForResponse={isWaitingForResponse}
        myHand={myHand}
        onMulliganDecision={onMulliganDecision}
        onMulliganPutBackDecision={onMulliganPutBackDecision}
        onModeDecision={onModeDecision}
        onOptionalTriggerDecision={onOptionalTriggerDecision}
        onColorDecision={onColorDecision}
        onTypeDecision={onTypeDecision}
        onNumberDecision={onNumberDecision}
        onCardNameDecision={onCardNameDecision}
        onDamageOrderDecision={onDamageOrderDecision}
        onCombatDamageAssignmentDecision={onCombatDamageAssignmentDecision}
        onReorderLibraryDecision={onReorderLibraryDecision}
        onManaComboDecision={onManaComboDecision}
        onExploreDecision={onExploreDecision}
        onAssistDecision={onAssistDecision}
      />
      <CostModals
        promptType={promptType}
        currentPrompt={currentPrompt}
        onPhyrexianDecision={onPhyrexianDecision}
        onKickerDecision={onKickerDecision}
        onBuybackDecision={onBuybackDecision}
        onMultikickerDecision={onMultikickerDecision}
        onReplicateDecision={onReplicateDecision}
        onAlternativeCostDecision={onAlternativeCostDecision}
        onPayCombatCost={onPayCombatCost}
        onDeclineCombatCost={onDeclineCombatCost}
        onDelveDecision={onDelveDecision}
        onConvokeDecision={onConvokeDecision}
        onImproviseDecision={onImproviseDecision}
      />
      <TargetModals
        promptType={promptType}
        currentPrompt={currentPrompt}
        viewingZone={viewingZone}
        onCloseZone={onCloseZone}
        zoneTargetSelector={zoneTargetSelector}
        onSelectZoneTarget={onSelectZoneTarget}
        onCancelZoneTarget={onCancelZoneTarget}
        libraryPeekModal={libraryPeekModal}
        onLibraryPeekConfirm={onLibraryPeekConfirm}
        spellStackModalOpen={spellStackModalOpen}
        stack={stack}
        validSpellIds={validSpellIds}
        onTargetSpell={onTargetSpell}
        onCloseStack={onCloseStack}
        abilityPickerState={abilityPickerState}
        onSelectAbility={onSelectAbility}
        onCancelAbilityPicker={onCancelAbilityPicker}
        onChooseCardsDecision={onChooseCardsDecision}
        onExertDecision={onExertDecision}
        onEnlistDecision={onEnlistDecision}
      />
    </>
  );
}
