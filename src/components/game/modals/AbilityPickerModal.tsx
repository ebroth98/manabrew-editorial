import { Modal } from "./Modal";
import { TextWithMana } from "@/components/game/TextWithMana";
import { useCardImage } from "@/hooks/useCardImage";
import { CardImageThumbnail } from "@/components/game/CardImageThumbnail";
import { cn } from "@/lib/utils";
import type { ActivatableAbilityInfo } from "@/types/openmagic";
import { MODAL_CARD_THUMBNAIL } from "../game.styles";

interface AbilityPickerModalProps {
  cardName: string;
  abilities: ActivatableAbilityInfo[];
  onSelect: (ability: ActivatableAbilityInfo) => void;
  onCancel: () => void;
}

export function AbilityPickerModal({
  cardName,
  abilities,
  onSelect,
  onCancel,
}: AbilityPickerModalProps) {
  const { data: imageUrl } = useCardImage(cardName);

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
            <h2 className="font-semibold text-base">Activate Ability</h2>
            <p className="text-xs text-muted-foreground font-medium">{cardName}</p>
          </div>
        </div>
      </Modal.Header>

      <Modal.Instructions>
        Click an ability to activate it.
      </Modal.Instructions>

      <div className="p-4 flex flex-col gap-2 max-h-[60vh] overflow-y-auto" role="group" aria-label="Available abilities">
        {abilities.map((ability, idx) => (
          <button
            key={idx}
            onClick={() => onSelect(ability)}
            className={cn(
              "w-full text-left px-4 py-3 rounded-lg border text-sm font-medium transition-all",
              "hover:border-primary/50 hover:bg-muted/50 border-border bg-background",
            )}
          >
            <TextWithMana text={ability.description} />
          </button>
        ))}
      </div>
    </Modal>
  );
}
