import { Button } from "@/components/ui/button";
import { Modal } from "@/components/game/Modal";
import { cn } from "@/lib/utils";
import { useState, useEffect, useCallback, useRef } from "react";
import { useCardImage } from "@/hooks/useCardImage";
import { CardImageThumbnail } from "@/components/game/CardImageThumbnail";

interface ChooseNumberModalProps {
  min: number;
  max: number;
  cardName?: string;
  onConfirm: (chosenNumber: number | null) => void;
}

export function ChooseNumberModal({
  min,
  max,
  cardName,
  onConfirm,
}: ChooseNumberModalProps) {
  const { data: imageUrl } = useCardImage(cardName ?? "");
  const range = max - min + 1;
  const useButtons = range <= 20;
  const [inputValue, setInputValue] = useState(String(min));
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (!useButtons) {
      inputRef.current?.focus();
    }
  }, [min, max, useButtons]);

  const handleInputConfirm = useCallback(() => {
    const num = parseInt(inputValue, 10);
    if (!isNaN(num) && num >= min && num <= max) {
      onConfirm(num);
    }
  }, [inputValue, min, max, onConfirm]);

  useEffect(() => {
    function handleKey(e: KeyboardEvent) {
      if (e.key === "Enter" && !useButtons) {
        e.preventDefault();
        handleInputConfirm();
      }
    }
    window.addEventListener("keydown", handleKey);
    return () => window.removeEventListener("keydown", handleKey);
  }, [useButtons, handleInputConfirm]);

  const numbers = useButtons
    ? Array.from({ length: range }, (_, i) => min + i)
    : [];

  return (
    <Modal maxWidth="max-w-sm" maxHeight="" className="outline-none">
      <div role="dialog" aria-modal="true" aria-labelledby="choose-number-title">
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
              <h2 id="choose-number-title" className="font-semibold text-base">
                Choose a Number
              </h2>
              {cardName && <p className="text-xs text-muted-foreground font-medium">{cardName}</p>}
              <p className="text-xs text-muted-foreground">
                Between {min} and {max}
              </p>
            </div>
          </div>
        </Modal.Header>

        <Modal.Instructions>
          {useButtons ? "Click a number." : "Enter a number and confirm."}
        </Modal.Instructions>

        {useButtons ? (
          <div className="p-4 flex flex-wrap gap-2 justify-center" role="group" aria-label="Number choices">
            {numbers.map((num) => (
              <button
                key={num}
                onClick={() => onConfirm(num)}
                className={cn(
                  "w-10 h-10 rounded-md border text-sm font-bold transition-all",
                  "hover:border-primary hover:bg-primary/10",
                  "border-border bg-background",
                )}
              >
                {num}
              </button>
            ))}
          </div>
        ) : (
          <div className="p-4 flex items-center gap-2">
            <input
              ref={inputRef}
              type="number"
              min={min}
              max={max}
              value={inputValue}
              onChange={(e) => setInputValue(e.target.value)}
              className="flex-1 px-3 py-2 rounded-md border bg-background text-sm focus:outline-none focus:ring-1 focus:ring-primary"
            />
            <Button size="sm" onClick={handleInputConfirm}>
              Confirm
            </Button>
          </div>
        )}
      </div>
    </Modal>
  );
}
