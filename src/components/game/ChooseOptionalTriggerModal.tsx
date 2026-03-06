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
  /** Prompt context (optional_trigger or confirm_action). */
  promptKind?: string;
  /** Optional labels for [decline, accept] buttons. */
  optionLabels?: string[];
  /** Optional mode metadata for confirm_action prompts. */
  mode?: string;
  /** Optional API metadata for confirm_action prompts. */
  api?: string;
  onConfirm: (accept: boolean) => void;
}

export function ChooseOptionalTriggerModal({
  description,
  cardName,
  promptKind,
  optionLabels,
  mode,
  api,
  onConfirm,
}: ChooseOptionalTriggerModalProps) {
  const { data: imageUrl } = useCardImage(cardName ?? "");
  const dialogRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    dialogRef.current?.focus();
  }, [description]);

  const handleAccept = useCallback(() => onConfirm(true), [onConfirm]);
  const handleDecline = useCallback(() => onConfirm(false), [onConfirm]);
  const declineLabel = optionLabels?.[0] ?? "Decline";
  const acceptLabel = optionLabels?.[1] ?? "Accept";
  const isGenericConfirm = promptKind === "confirm_action";
  const title = isGenericConfirm ? "Confirm Action" : "Optional Trigger";
  const subtitle = isGenericConfirm
    ? "Confirm whether to apply this optional action."
    : "Do you want this ability to trigger?";
  const metaBits = [mode, api].filter(Boolean).join(" • ");

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
            {title}
          </h2>
          <p className="text-xs text-muted-foreground">
            {subtitle}
          </p>
          {metaBits && <p className="text-[11px] text-muted-foreground">{metaBits}</p>}
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
            {declineLabel}
          </Button>
          <Button
            size="sm"
            onClick={handleAccept}
            className="min-w-[80px]"
          >
            {acceptLabel}
          </Button>
        </Modal.Footer>
      </div>
    </Modal>
  );
}
