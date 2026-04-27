import { Modal } from "./Modal";
import { TextWithMana } from "@/components/game/TextWithMana";
import { useCard } from "@/stores/useScryfallStore";
import { CardImageThumbnail } from "@/components/game/CardImageThumbnail";
import { cn } from "@/lib/utils";
import type { HandActionOption } from "@/stores/useGameUIStore";
import { MODAL_CARD_THUMBNAIL } from "../game.styles";

interface AbilityPickerModalProps {
  cardName: string;
  abilities: HandActionOption[];
  onSelect: (ability: HandActionOption) => void;
  onCancel: () => void;
}

export function AbilityPickerModal({
  cardName,
  abilities,
  onSelect,
  onCancel,
}: AbilityPickerModalProps) {
  const cardData = useCard({ name: cardName });
  const imageUrl = cardData?.uris.normal;
  const hasCastOption = abilities.some((ability) => ability.kind === "cast");

  return (
    <Modal maxWidth="max-w-md" maxHeight="" onClose={onCancel}>
      <Modal.Header>
        <div className="flex items-center gap-3">
          {imageUrl && (
            <CardImageThumbnail
              imageUrl={imageUrl}
              cardName={cardName}
              className={MODAL_CARD_THUMBNAIL}
            />
          )}
          <div>
            <h2 className="font-semibold text-base">
              {hasCastOption ? "Choose Action" : "Activate Ability"}
            </h2>
            <p className="text-xs text-muted-foreground font-medium">{cardName}</p>
          </div>
        </div>
      </Modal.Header>

      <Modal.Instructions>Click an option to continue.</Modal.Instructions>

      <div
        className="p-4 flex flex-col gap-2 max-h-[60vh] overflow-y-auto"
        role="group"
        aria-label="Available abilities"
      >
        {abilities.map((ability, idx) => (
          <button
            key={idx}
            onClick={() => onSelect(ability)}
            className={cn(
              "w-full text-left px-4 py-3 rounded-lg border text-sm font-medium transition-all",
              "hover:border-primary/50 hover:bg-muted/50 border-border bg-background",
            )}
          >
            <TextWithMana text={ability.label} />
          </button>
        ))}
      </div>
    </Modal>
  );
}
