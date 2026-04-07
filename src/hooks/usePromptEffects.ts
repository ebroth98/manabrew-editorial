import { useEffect, useState } from "react";
import { usePreferencesStore } from "@/stores/usePreferencesStore";
import type { AgentPrompt } from "@/stores/useGameStore";
import type { LibraryPeekMode } from "@/components/game/modals";
import type { Card } from "@/types/openmagic";
import { PromptType } from "@/types/promptType";

interface UsePromptEffectsOptions {
  currentPrompt: AgentPrompt | null;
  isWaitingForResponse: boolean;
  passPriority: () => void;
  myHand: Card[];
  turn: number;
  stackLength: number;
}

interface LibraryPeekState {
  mode: LibraryPeekMode;
  cards: Card[];
  numToTake?: number;
  optional?: boolean;
}

interface ZoneTargetState {
  title: string;
  cards: Card[];
  validCardIds: string[];
}

const AUTO_PASS_DELAY_MIN_MS = 250;
const AUTO_PASS_DELAY_MAX_MS = 650;

function getAutoPassDelayMs(): number {
  return Math.floor(
    AUTO_PASS_DELAY_MIN_MS +
      Math.random() * (AUTO_PASS_DELAY_MAX_MS - AUTO_PASS_DELAY_MIN_MS + 1),
  );
}

export function usePromptEffects({
  currentPrompt,
  isWaitingForResponse,
  passPriority,
  myHand,
  turn,
  stackLength,
}: UsePromptEffectsOptions) {
  const promptType = currentPrompt?.type;
  const autoPassEnabled = usePreferencesStore((s) => s.autoPassEnabled);
  const [isAutoPassing, setIsAutoPassing] = useState(false);
  const [passUntilEotTurn, setPassUntilEotTurn] = useState<number | null>(null);

  function activatePassUntilEot() {
    if (passUntilEotTurn !== null) return;
    setPassUntilEotTurn(turn);
    passPriority();
  }

  // Library peek modal state
  const [libraryPeekModal, setLibraryPeekModal] = useState<LibraryPeekState | null>(null);

  // Zone target selector state
  const [zoneTargetSelector, setZoneTargetSelector] = useState<ZoneTargetState | null>(null);

  // Spell stack modal
  const [spellStackModalOpen, setSpellStackModalOpen] = useState(false);

  // Auto-pass: passUntilEot takes precedence, then normal auto-pass
  useEffect(() => {
    setIsAutoPassing(false);
    if (!currentPrompt || isWaitingForResponse) return;

    if (passUntilEotTurn !== null) {
      if (turn > passUntilEotTurn) {
        setPassUntilEotTurn(null);
      } else if (currentPrompt.type === PromptType.ChooseAction && stackLength > 0) {
        setPassUntilEotTurn(null);
      } else if (currentPrompt.type === PromptType.ChooseAction || currentPrompt.type === PromptType.ChooseAttackers) {
        setIsAutoPassing(true);
        const timer = setTimeout(() => passPriority(), getAutoPassDelayMs());
        return () => clearTimeout(timer);
      } else {
        setPassUntilEotTurn(null);
      }
      return;
    }

    if (!autoPassEnabled) return;

    let shouldAutoPass = false;

    if (currentPrompt.type === PromptType.ChooseAction) {
      const hasPlayableCards = (currentPrompt.playableCardIds ?? []).length > 0;
      const hasActivatableAbilities = (currentPrompt.activatableAbilityIds ?? []).length > 0;
      shouldAutoPass = !hasPlayableCards && !hasActivatableAbilities;
    } else if (currentPrompt.type === PromptType.ChooseAttackers) {
      shouldAutoPass = (currentPrompt.availableAttackerIds ?? []).length === 0;
    } else if (currentPrompt.type === PromptType.ChooseBlockers) {
      shouldAutoPass = (currentPrompt.availableBlockerIds ?? []).length === 0;
    }

    if (!shouldAutoPass) return;

    setIsAutoPassing(true);
    const timer = setTimeout(() => passPriority(), getAutoPassDelayMs());

    return () => clearTimeout(timer);
  }, [currentPrompt, isWaitingForResponse, autoPassEnabled, passPriority, passUntilEotTurn, turn, stackLength]);

  // Open library-peek modal for Scry / Surveil / Dig / Discard prompts
  useEffect(() => {
    if (
      (promptType === PromptType.Scry || promptType === PromptType.Surveil || promptType === PromptType.Dig) &&
      currentPrompt?.cards &&
      currentPrompt.cards.length > 0
    ) {
      setLibraryPeekModal({
        mode: promptType as LibraryPeekMode,
        cards: currentPrompt.cards as Card[],
        numToTake: promptType === PromptType.Dig ? currentPrompt.numToTake : undefined,
        optional: promptType === PromptType.Dig ? currentPrompt.optional : undefined,
      });
    } else if (promptType === PromptType.ChooseDiscard && currentPrompt) {
      const promptHand = currentPrompt.gameView?.myHand ?? myHand;
      const handCards = (currentPrompt.handCardIds ?? [])
        .map((id) => promptHand.find((c) => c.id === id))
        .filter((c): c is Card => c !== undefined);
      if (handCards.length > 0) {
        setLibraryPeekModal({
          mode: "discard",
          cards: handCards,
          numToTake: currentPrompt.numToDiscard,
        });
      }
    } else if (
      promptType !== PromptType.Scry &&
      promptType !== PromptType.Surveil &&
      promptType !== PromptType.Dig &&
      promptType !== PromptType.ChooseDiscard
    ) {
      setLibraryPeekModal(null);
    }
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [promptType, currentPrompt]);

  // Handle zone-based targeting prompts
  useEffect(() => {
    if (promptType === PromptType.ChooseTargetCardFromZone && currentPrompt) {
      const zone = currentPrompt.zone;
      const validCardIds = currentPrompt.validCardIds || [];
      const zoneCards = currentPrompt.zoneCards || [];

      if (zone && zoneCards.length > 0) {
        const zoneNames: Record<string, string> = {
          Graveyard: "Graveyard",
          Exile: "Exile",
          Hand: "Hand",
        };
        const title = `Choose from ${zoneNames[zone] || zone}`;
        setZoneTargetSelector({ title, cards: zoneCards, validCardIds });
      }
    } else {
      setZoneTargetSelector(null);
    }
  }, [promptType, currentPrompt]);

  // Auto-open spell-stack modal for counter-targeting prompts
  useEffect(() => {
    setSpellStackModalOpen(promptType === PromptType.ChooseTargetSpell);
  }, [promptType]);

  return {
    isAutoPassing,
    isPassingUntilEot: passUntilEotTurn !== null,
    activatePassUntilEot,
    libraryPeekModal,
    setLibraryPeekModal,
    zoneTargetSelector,
    setZoneTargetSelector,
    spellStackModalOpen,
    setSpellStackModalOpen,
  };
}
