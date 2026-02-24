import { createPortal } from "react-dom";
import { Button } from "@/components/ui/button";
import { useEffect, useRef, useCallback } from "react";
import { useCardImage } from "@/hooks/useCardImage";
import { CardImageThumbnail } from "@/components/game/CardImageThumbnail";

interface ChooseOptionalTriggerModalProps {
  /** Human-readable description of the triggered ability. */
  description: string;
  /** Name of the source card (for displaying card image). */
  cardName?: string;
  onConfirm: (accept: boolean) => void;
}

export function ChooseOptionalTriggerModal({
  description,
  cardName,
  onConfirm,
}: ChooseOptionalTriggerModalProps) {
  const { data: imageUrl } = useCardImage(cardName ?? "");
  const dialogRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    dialogRef.current?.focus();
  }, [description]);

  const handleAccept = useCallback(() => onConfirm(true), [onConfirm]);
  const handleDecline = useCallback(() => onConfirm(false), [onConfirm]);

  // Keyboard: Enter accepts, Escape declines.
  useEffect(() => {
    function handleKey(e: KeyboardEvent) {
      if (e.key === "Enter") {
        e.preventDefault();
        handleAccept();
      } else if (e.key === "Escape") {
        e.preventDefault();
        handleDecline();
      }
    }
    window.addEventListener("keydown", handleKey);
    return () => window.removeEventListener("keydown", handleKey);
  }, [handleAccept, handleDecline]);

  return createPortal(
    <div
      className="fixed inset-0 z-[9000] flex items-center justify-center bg-black/60 backdrop-blur-sm"
      role="dialog"
      aria-modal="true"
      aria-labelledby="optional-trigger-title"
    >
      <div
        ref={dialogRef}
        tabIndex={-1}
        className="bg-card border rounded-xl shadow-2xl flex flex-col w-full max-w-md mx-4 outline-none animate-in fade-in zoom-in-95 duration-200"
        onClick={(e) => e.stopPropagation()}
      >
        {/* Header */}
        <div className="px-4 py-3 border-b">
          <h2
            id="optional-trigger-title"
            className="font-semibold text-base"
          >
            Optional Trigger
          </h2>
          <p className="text-xs text-muted-foreground">
            Do you want this ability to trigger?
          </p>
        </div>

        {/* Description + card image */}
        <div className="px-4 py-4 flex gap-3">
          {imageUrl && (
            <CardImageThumbnail
              imageUrl={imageUrl}
              cardName={cardName ?? "Source card"}
              className="w-[120px] h-[168px] rounded-lg object-cover shrink-0 shadow-md"
            />
          )}
          <p className="text-sm leading-relaxed self-center">{description || "A triggered ability would trigger. Do you want it to?"}</p>
        </div>

        {/* Footer */}
        <div className="flex justify-end gap-2 px-4 py-3 border-t bg-muted/10 rounded-b-xl">
          <Button
            variant="outline"
            size="sm"
            onClick={handleDecline}
            className="min-w-[80px]"
          >
            Decline
          </Button>
          <Button
            size="sm"
            onClick={handleAccept}
            className="min-w-[80px]"
          >
            Accept
          </Button>
        </div>
      </div>
    </div>,
    document.body,
  );
}
