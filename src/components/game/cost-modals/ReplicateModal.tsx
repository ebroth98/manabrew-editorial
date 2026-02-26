import { useState, useEffect } from "react";
import { Modal } from "@/components/game/Modal";
import { Button } from "@/components/ui/button";
import { useCardImage } from "@/hooks/useCardImage";
import { CardImageThumbnail } from "@/components/game/CardImageThumbnail";
import { ManaSymbols } from "@/components/game/ManaSymbols";
import { MODAL_CARD_IMAGE } from "../game.styles";

interface ReplicateModalProps {
  cost: string;
  maxReplicates: number;
  sourceCardName?: string;
  onDecide: (replicateCount: number) => void;
}

export function ReplicateModal({ cost, maxReplicates, sourceCardName, onDecide }: ReplicateModalProps) {
  const { data: imageUrl } = useCardImage(sourceCardName ?? "");
  const [count, setCount] = useState(0);

  useEffect(() => { setCount(0); }, [cost, maxReplicates]);

  return (
    <Modal maxWidth="max-w-md" maxHeight="">
      <Modal.Header>
        <h2 className="font-semibold text-base">Replicate</h2>
      </Modal.Header>
      <div className="px-4 py-4 flex gap-3">
        {imageUrl && (
          <CardImageThumbnail
            imageUrl={imageUrl}
            cardName={sourceCardName ?? "Spell"}
            className={MODAL_CARD_IMAGE}
          />
        )}
        <div className="self-center flex-1">
          <p className="text-sm text-muted-foreground mb-3">
            Pay <ManaSymbols cost={cost} size="lg" /> per copy (max {maxReplicates})
          </p>
          <div className="flex items-center gap-3">
            <Button variant="outline" size="sm" disabled={count <= 0} onClick={() => setCount(c => Math.max(0, c - 1))}>-</Button>
            <span className="text-xl font-bold w-8 text-center">{count}</span>
            <Button variant="outline" size="sm" disabled={count >= maxReplicates} onClick={() => setCount(c => Math.min(maxReplicates, c + 1))}>+</Button>
          </div>
        </div>
      </div>
      <Modal.Footer>
        <Button variant="outline" onClick={() => onDecide(0)}>Skip</Button>
        <Button onClick={() => onDecide(count)}>Confirm ({count}x)</Button>
      </Modal.Footer>
    </Modal>
  );
}
