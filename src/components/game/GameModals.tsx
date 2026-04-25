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
  viewingZone: { title: string; cards: XMageCard[]; onClickCard?: (cardId: string) => void } | null;
  onCloseZone: () => void;
  zoneTargetSelector: { title: string; cards: XMageCard[]; validCardIds: string[] } | null;
  onSelectZoneTarget: (cardId: string) => void;
  onCancelZoneTarget: () => void;
  libraryPeekModal: {
    mode: LibraryPeekMode;
    cards: XMageCard[];
    numToTake?: number;
    optional?: boolean;
  } | null;
  onLibraryPeekConfirm: (selectedIds: string[]) => void;
  spellStackModalOpen: boolean;
  stack: StackObject[];
  validSpellIds: string[];
  onTargetSpell: (spellId: string) => void;
  onCloseStack: () => void;
  playerColorMap?: Map<string, string>;
  abilityPickerState: AbilityPickerState | null;
  onSelectAbility: (ability: HandActionOption) => void;
  onCancelAbilityPicker: () => void;
  onModeDecision: (indices: number[]) => void;
  onRevealCardsAcknowledged: () => void;
  onPayCostToPreventEffectDecision: (accept: boolean) => void;
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
  onPayCombatCost: () => void;
  onDeclineCombatCost: () => void;

  onDelveDecision: (cardIds: string[]) => void;
  onConvokeDecision: (cardIds: string[]) => void;
  onImproviseDecision: (cardIds: string[]) => void;
  onManaComboDecision: (chosenColors: string[]) => void;
  onExploreDecision: (putInGraveyard: boolean) => void;
  onExertDecision: (chosenAttackerIds: string[]) => void;
  onEnlistDecision: (chosenAttackerIds: string[]) => void;
  onReorderLibraryDecision: (orderedCardIds: string[]) => void;
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
  playerColorMap,
  abilityPickerState,
  onSelectAbility,
  onCancelAbilityPicker,
  onModeDecision,
  onRevealCardsAcknowledged,
  onPayCostToPreventEffectDecision,
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
        onModeDecision={onModeDecision}
        onRevealCardsAcknowledged={onRevealCardsAcknowledged}
        onPayCostToPreventEffectDecision={onPayCostToPreventEffectDecision}
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
        playerColorMap={playerColorMap}
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
