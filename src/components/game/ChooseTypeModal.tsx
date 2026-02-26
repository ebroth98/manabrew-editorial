import { Modal } from "@/components/game/Modal";
import { useState, useEffect, useRef } from "react";
import { useCardImage } from "@/hooks/useCardImage";
import { CardImageThumbnail } from "@/components/game/CardImageThumbnail";
import { MODAL_CARD_THUMBNAIL, MODAL_INPUT, MODAL_PILL_BUTTON } from "./game.styles";

interface ChooseTypeModalProps {
  typeCategory: string;
  validTypes: string[];
  cardName?: string;
  onConfirm: (chosenType: string | null) => void;
}

export function ChooseTypeModal({
  typeCategory,
  validTypes,
  cardName,
  onConfirm,
}: ChooseTypeModalProps) {
  const { data: imageUrl } = useCardImage(cardName ?? "");
  const [filter, setFilter] = useState("");
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    inputRef.current?.focus();
  }, [validTypes]);

  const filtered = filter
    ? validTypes.filter((t) => t.toLowerCase().includes(filter.toLowerCase()))
    : validTypes;

  return (
    <Modal maxWidth="max-w-md" maxHeight="" className="outline-none">
      <div role="dialog" aria-modal="true" aria-labelledby="choose-type-title">
        <Modal.Header>
          <div className="flex items-center gap-3">
            {imageUrl && (
              <CardImageThumbnail
                imageUrl={imageUrl}
                cardName={cardName ?? "Source card"}
                className={MODAL_CARD_THUMBNAIL}
              />
            )}
            <div>
              <h2 id="choose-type-title" className="font-semibold text-base">
                Choose {typeCategory} Type
              </h2>
              {cardName && <p className="text-xs text-muted-foreground font-medium">{cardName}</p>}
            </div>
          </div>
        </Modal.Header>

        <Modal.Instructions>Click a type to choose it.</Modal.Instructions>

        {/* Search filter */}
        {validTypes.length > 10 && (
          <div className="px-4 pb-2">
            <input
              ref={inputRef}
              type="text"
              placeholder="Filter types..."
              value={filter}
              onChange={(e) => setFilter(e.target.value)}
              className={MODAL_INPUT}
            />
          </div>
        )}

        <div className="p-4 flex flex-wrap gap-2 max-h-[50vh] overflow-y-auto" role="group" aria-label="Available types">
          {filtered.map((typeName) => (
            <button
              key={typeName}
              onClick={() => onConfirm(typeName)}
              className={MODAL_PILL_BUTTON}
            >
              {typeName}
            </button>
          ))}
          {filtered.length === 0 && (
            <p className="text-sm text-muted-foreground">No matching types.</p>
          )}
        </div>
      </div>
    </Modal>
  );
}
