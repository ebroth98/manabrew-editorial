import { Modal } from "@/components/game/modals/Modal";
import { useCard } from "@/stores/useScryfallStore";
import { useTheme } from "@/hooks/useTheme";
import { CardImageThumbnail } from "@/components/game/CardImageThumbnail";
import { MODAL_CARD_THUMBNAIL } from "@/components/game/game.styles";
import { DieFaceStatic } from "../DieFaceStatic";

interface ChooseRollToSwapModalProps {
  rolls: number[];
  sides?: number;
  sourceCardName?: string;
  onConfirm: (roll: number | null) => void;
}

/**
 * Pick one die to exchange with a creature's power or toughness. The
 * power/toughness leg is a separate prompt (`ChooseRollSwapValueModal`).
 */
export function ChooseRollToSwapModal({
  rolls,
  sides = 6,
  sourceCardName,
  onConfirm,
}: ChooseRollToSwapModalProps) {
  const cardData = useCard({ name: sourceCardName ?? "" });
  const imageUrl = cardData?.uris.normal;
  const accentColor = useTheme().gameTheme.playerColors.self;

  return (
    <Modal maxWidth="max-w-sm" maxHeight="">
      <div role="dialog" aria-modal="true" aria-labelledby="choose-roll-swap-title">
        <Modal.Header>
          <div className="flex items-center gap-3">
            {imageUrl && (
              <CardImageThumbnail
                imageUrl={imageUrl}
                cardName={sourceCardName ?? "Source card"}
                className={MODAL_CARD_THUMBNAIL}
              />
            )}
            <div>
              <h2 id="choose-roll-swap-title" className="font-semibold text-base">
                Choose a roll to exchange
              </h2>
              {sourceCardName && (
                <p className="text-xs text-muted-foreground font-medium">{sourceCardName}</p>
              )}
            </div>
          </div>
        </Modal.Header>

        <Modal.Instructions>Click a die to exchange with a P/T value.</Modal.Instructions>

        <div className="p-4 flex flex-wrap gap-3 justify-center" role="group">
          {rolls.map((value, index) => (
            <DieFaceStatic
              key={`${value}-${index}`}
              sides={sides}
              value={value}
              size="lg"
              accentColor={accentColor}
              onClick={() => onConfirm(value)}
              ariaLabel={`Swap die ${value}`}
            />
          ))}
        </div>
      </div>
    </Modal>
  );
}
