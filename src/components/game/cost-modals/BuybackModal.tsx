import { Modal } from "../modals/Modal";
import { Button } from "@/components/ui/button";
import { useCard } from "@/stores/useScryfallStore";
import { CardImageThumbnail } from "@/components/game/CardImageThumbnail";
import { ManaSymbols } from "@/components/game/ManaSymbols";
import { MODAL_CARD_IMAGE } from "../game.styles";

interface BuybackModalProps {
  buybackCost: string;
  sourceCardName?: string;
  onDecide: (paid: boolean) => void;
}

export function BuybackModal({ buybackCost, sourceCardName, onDecide }: BuybackModalProps) {
  const cardData = useCard({ name: sourceCardName ?? "" });
  const imageUrl = cardData?.uris.normal;
  return (
    <Modal maxWidth="max-w-md" maxHeight="">
      <Modal.Header>
        <h2 className="font-semibold text-base">Pay Buyback?</h2>
      </Modal.Header>
      <div className="px-4 py-4 flex gap-3">
        {imageUrl && (
          <CardImageThumbnail
            imageUrl={imageUrl}
            cardName={sourceCardName ?? "Spell"}
            className={MODAL_CARD_IMAGE}
          />
        )}
        <div className="self-center">
          <p className="text-sm text-muted-foreground">
            Pay additional buyback cost: <ManaSymbols cost={buybackCost} size="lg" />
          </p>
          <p className="text-xs text-muted-foreground mt-1">
            If paid, this spell returns to your hand instead of going to the graveyard.
          </p>
        </div>
      </div>
      <Modal.Footer>
        <Button variant="outline" onClick={() => onDecide(false)}>
          No
        </Button>
        <Button onClick={() => onDecide(true)}>Pay Buyback</Button>
      </Modal.Footer>
    </Modal>
  );
}
