import { Modal } from "@/components/game/Modal";
import { ManaSymbols } from "@/components/game/ManaSymbols";
import { useCardImage } from "@/hooks/useCardImage";
import { CardImageThumbnail } from "@/components/game/CardImageThumbnail";
import { cn } from "@/lib/utils";
import type { ActivatableAbilityInfo } from "@/types/xmage";

function TextWithMana({ text }: { text: string }) {
  const parts = text.split(/(\{[^}]+\}(?:\{[^}]+\})*)/g);
  return (
    <span className="inline-flex items-center gap-0.5 flex-wrap">
      {parts.map((part, i) =>
        part.startsWith("{") ? (
          <ManaSymbols key={i} cost={part} size="sm" />
        ) : (
          <span key={i}>{part}</span>
        ),
      )}
    </span>
  );
}

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
              className="w-[60px] h-[84px] rounded-md object-cover shrink-0 shadow-md"
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
              "hover:border-primary/50 hover:bg-muted/50",
              "border-border bg-background",
            )}
          >
            <TextWithMana text={ability.description} />
          </button>
        ))}
      </div>
    </Modal>
  );
}
