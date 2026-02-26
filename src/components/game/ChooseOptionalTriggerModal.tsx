import { Button } from "@/components/ui/button";
import { Modal } from "@/components/game/Modal";
import { useEffect, useRef, useCallback } from "react";
import { useModalKeyboard } from "@/hooks/useModalKeyboard";
import { useCardImage } from "@/hooks/useCardImage";
import { CardImageThumbnail } from "@/components/game/CardImageThumbnail";
import { MODAL_CARD_IMAGE } from "./game.styles";

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

  useModalKeyboard(
    { onEnter: handleAccept, onEscape: handleDecline },
    [handleAccept, handleDecline],
  );

  return (
    <Modal maxWidth="max-w-md" maxHeight="">
      <div
        ref={dialogRef}
        tabIndex={-1}
        className="outline-none"
        role="dialog"
        aria-modal="true"
        aria-labelledby="optional-trigger-title"
      >
        <Modal.Header>
          <h2 id="optional-trigger-title" className="font-semibold text-base">
            Optional Trigger
          </h2>
          <p className="text-xs text-muted-foreground">
            Do you want this ability to trigger?
          </p>
        </Modal.Header>

        <div className="px-4 py-4 flex gap-3">
          {imageUrl && (
            <CardImageThumbnail
              imageUrl={imageUrl}
              cardName={cardName ?? "Source card"}
              className={MODAL_CARD_IMAGE}
            />
          )}
          <p className="text-sm leading-relaxed self-center">{description || "A triggered ability would trigger. Do you want it to?"}</p>
        </div>

        <Modal.Footer>
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
        </Modal.Footer>
      </div>
    </Modal>
  );
}
