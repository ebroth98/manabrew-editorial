import { Button } from "@/components/ui/button";
import { Modal } from "@/components/game/Modal";
import { cn } from "@/lib/utils";
import { useState, useEffect, useCallback, useRef } from "react";
import { useCardImage } from "@/hooks/useCardImage";
import { CardImageThumbnail } from "@/components/game/CardImageThumbnail";

interface ChooseCardNameModalProps {
  validNames: string[];
  cardName?: string;
  onConfirm: (chosenName: string | null) => void;
}

export function ChooseCardNameModal({
  validNames,
  cardName,
  onConfirm,
}: ChooseCardNameModalProps) {
  const { data: imageUrl } = useCardImage(cardName ?? "");
  const [filter, setFilter] = useState("");
  const [textInput, setTextInput] = useState("");
  const inputRef = useRef<HTMLInputElement>(null);

  const hasList = validNames.length > 0;

  useEffect(() => {
    inputRef.current?.focus();
  }, [validNames]);

  const filtered = hasList && filter
    ? validNames.filter((n) => n.toLowerCase().includes(filter.toLowerCase()))
    : validNames;

  const handleTextConfirm = useCallback(() => {
    if (textInput.trim()) {
      onConfirm(textInput.trim());
    }
  }, [textInput, onConfirm]);

  useEffect(() => {
    function handleKey(e: KeyboardEvent) {
      if (e.key === "Enter" && !hasList) {
        e.preventDefault();
        handleTextConfirm();
      }
    }
    window.addEventListener("keydown", handleKey);
    return () => window.removeEventListener("keydown", handleKey);
  }, [hasList, handleTextConfirm]);

  return (
    <Modal maxWidth="max-w-md" maxHeight="" className="outline-none">
      <div role="dialog" aria-modal="true" aria-labelledby="choose-card-name-title">
        <Modal.Header>
          <div className="flex items-center gap-3">
            {imageUrl && (
              <CardImageThumbnail
                imageUrl={imageUrl}
                cardName={cardName ?? "Source card"}
                className="w-[60px] h-[84px] rounded-md object-cover shrink-0 shadow-md"
              />
            )}
            <div>
              <h2 id="choose-card-name-title" className="font-semibold text-base">
                Name a Card
              </h2>
              {cardName && <p className="text-xs text-muted-foreground font-medium">{cardName}</p>}
            </div>
          </div>
        </Modal.Header>

        <Modal.Instructions>
          {hasList ? "Choose a card name from the list." : "Type a card name."}
        </Modal.Instructions>

        {hasList ? (
          <>
            {validNames.length > 10 && (
              <div className="px-4 pb-2">
                <input
                  ref={inputRef}
                  type="text"
                  placeholder="Filter names..."
                  value={filter}
                  onChange={(e) => setFilter(e.target.value)}
                  className="w-full px-3 py-1.5 rounded-md border bg-background text-sm focus:outline-none focus:ring-1 focus:ring-primary"
                />
              </div>
            )}
            <div className="p-4 flex flex-col gap-1.5 max-h-[50vh] overflow-y-auto" role="group" aria-label="Card name choices">
              {filtered.map((name) => (
                <button
                  key={name}
                  onClick={() => onConfirm(name)}
                  className={cn(
                    "w-full text-left px-3 py-2 rounded-md border text-sm font-medium transition-all",
                    "hover:border-primary/50 hover:bg-muted/50",
                    "border-border bg-background",
                  )}
                >
                  {name}
                </button>
              ))}
              {filtered.length === 0 && (
                <p className="text-sm text-muted-foreground">No matching names.</p>
              )}
            </div>
          </>
        ) : (
          <div className="p-4 flex items-center gap-2">
            <input
              ref={inputRef}
              type="text"
              placeholder="Card name..."
              value={textInput}
              onChange={(e) => setTextInput(e.target.value)}
              className="flex-1 px-3 py-2 rounded-md border bg-background text-sm focus:outline-none focus:ring-1 focus:ring-primary"
            />
            <Button size="sm" onClick={handleTextConfirm} disabled={!textInput.trim()}>
              Confirm
            </Button>
          </div>
        )}
      </div>
    </Modal>
  );
}
