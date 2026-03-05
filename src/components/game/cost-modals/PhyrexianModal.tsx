import { Modal } from "@/components/game/Modal";
import { Button } from "@/components/ui/button";
import { useCardImage } from "@/hooks/useCardImage";
import { CardImageThumbnail } from "@/components/game/CardImageThumbnail";
import { ManaSymbols } from "@/components/game/ManaSymbols";
import { MODAL_CARD_IMAGE } from "../game.styles";

interface PhyrexianModalProps {
  phyrexianColor: string;
  sourceCardName?: string;
  onDecide: (payLife: boolean) => void;
}

export function PhyrexianModal({ phyrexianColor, sourceCardName, onDecide }: PhyrexianModalProps) {
  const { data: imageUrl } = useCardImage(sourceCardName ?? "");
  // phyrexianColor is comma-separated, e.g. "W/P" or "B/P, B/P"
  const shards = phyrexianColor.split(",").map((s) => s.trim());
  const lifeCost = shards.length * 2;
  // Build mana cost string for the color payment: "{W}{W}" etc
  const colorShards = shards.map((s) => s.replace(/\/P/g, ""));
  const manaCostStr = colorShards.map((c) => `{${c}}`).join("");
  const phyrexianCostStr = shards.map((s) => `{${s}}`).join("");
  return (
    <Modal maxWidth="max-w-md" maxHeight="">
      <Modal.Header>
        <h2 className="font-semibold text-base">Phyrexian Mana</h2>
      </Modal.Header>
      <div className="px-4 py-4 flex gap-3">
        {imageUrl && (
          <CardImageThumbnail
            imageUrl={imageUrl}
            cardName={sourceCardName ?? "Spell"}
            className={MODAL_CARD_IMAGE}
          />
        )}
        <div className="text-sm text-muted-foreground self-center">
          <p>
            Pay <ManaSymbols cost={phyrexianCostStr} size="lg" /> with{" "}
            <span className="font-bold text-red-500">{lifeCost} life</span> or{" "}
            <ManaSymbols cost={manaCostStr} size="lg" /> mana?
          </p>
        </div>
      </div>
      <Modal.Footer>
        <Button variant="outline" onClick={() => onDecide(false)}>
          Pay Mana <ManaSymbols cost={manaCostStr} size="sm" className="ml-1" />
        </Button>
        <Button variant="destructive" onClick={() => onDecide(true)}>
          Pay {lifeCost} Life
        </Button>
      </Modal.Footer>
    </Modal>
  );
}
