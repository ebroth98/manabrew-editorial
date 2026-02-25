import { useState, useEffect } from "react";
import { Modal } from "@/components/game/Modal";
import { Button } from "@/components/ui/button";
import { useCardImage } from "@/hooks/useCardImage";
import { CardImageThumbnail } from "@/components/game/CardImageThumbnail";
import { ManaSymbols } from "@/components/game/ManaSymbols";

function TextWithMana({ text, manaSize = "md" }: { text: string; manaSize?: "sm" | "md" | "lg" }) {
  const parts = text.split(/(\{[^}]+\}(?:\{[^}]+\})*)/g);
  return (
    <span className="inline-flex items-center gap-0.5 flex-wrap">
      {parts.map((part, i) =>
        part.startsWith("{") ? (
          <ManaSymbols key={i} cost={part} size={manaSize} />
        ) : (
          <span key={i}>{part}</span>
        ),
      )}
    </span>
  );
}

// ── Kicker ─────────────────────────────────────────────────

interface KickerModalProps {
  kickerCost: string;
  sourceCardName?: string;
  onDecide: (kicked: boolean) => void;
}

export function KickerModal({ kickerCost, sourceCardName, onDecide }: KickerModalProps) {
  const { data: imageUrl } = useCardImage(sourceCardName ?? "");
  return (
    <Modal maxWidth="max-w-md" maxHeight="">
      <div className="p-6">
        <h2 className="font-semibold text-base mb-3">Pay Kicker?</h2>
        <div className="flex gap-3 mb-4">
          {imageUrl && (
            <CardImageThumbnail
              imageUrl={imageUrl}
              cardName={sourceCardName ?? "Spell"}
              className="w-[120px] h-[168px] rounded-lg object-cover shrink-0 shadow-md"
            />
          )}
          <p className="text-sm text-muted-foreground self-center">
            Pay additional kicker cost: <ManaSymbols cost={kickerCost} size="lg" />
          </p>
        </div>
        <div className="flex gap-3 justify-end">
          <Button variant="outline" onClick={() => onDecide(false)}>No</Button>
          <Button onClick={() => onDecide(true)}>Pay Kicker</Button>
        </div>
      </div>
    </Modal>
  );
}

// ── Buyback ────────────────────────────────────────────────

interface BuybackModalProps {
  buybackCost: string;
  sourceCardName?: string;
  onDecide: (paid: boolean) => void;
}

export function BuybackModal({ buybackCost, sourceCardName, onDecide }: BuybackModalProps) {
  const { data: imageUrl } = useCardImage(sourceCardName ?? "");
  return (
    <Modal maxWidth="max-w-md" maxHeight="">
      <div className="p-6">
        <h2 className="font-semibold text-base mb-3">Pay Buyback?</h2>
        <div className="flex gap-3 mb-4">
          {imageUrl && (
            <CardImageThumbnail
              imageUrl={imageUrl}
              cardName={sourceCardName ?? "Spell"}
              className="w-[120px] h-[168px] rounded-lg object-cover shrink-0 shadow-md"
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
        <div className="flex gap-3 justify-end">
          <Button variant="outline" onClick={() => onDecide(false)}>No</Button>
          <Button onClick={() => onDecide(true)}>Pay Buyback</Button>
        </div>
      </div>
    </Modal>
  );
}

// ── Multikicker ────────────────────────────────────────────

interface MultikickerModalProps {
  cost: string;
  maxKicks: number;
  sourceCardName?: string;
  onDecide: (kickCount: number) => void;
}

export function MultikickerModal({ cost, maxKicks, sourceCardName, onDecide }: MultikickerModalProps) {
  const { data: imageUrl } = useCardImage(sourceCardName ?? "");
  const [count, setCount] = useState(0);

  useEffect(() => { setCount(0); }, [cost, maxKicks]);

  return (
    <Modal maxWidth="max-w-md" maxHeight="">
      <div className="p-6">
        <h2 className="font-semibold text-base mb-3">Multikicker</h2>
        <div className="flex gap-3 mb-4">
          {imageUrl && (
            <CardImageThumbnail
              imageUrl={imageUrl}
              cardName={sourceCardName ?? "Spell"}
              className="w-[120px] h-[168px] rounded-lg object-cover shrink-0 shadow-md"
            />
          )}
          <div className="self-center flex-1">
            <p className="text-sm text-muted-foreground mb-3">
              Pay <ManaSymbols cost={cost} size="lg" /> per kick (max {maxKicks})
            </p>
            <div className="flex items-center gap-3">
              <Button variant="outline" size="sm" disabled={count <= 0} onClick={() => setCount(c => Math.max(0, c - 1))}>-</Button>
              <span className="text-xl font-bold w-8 text-center">{count}</span>
              <Button variant="outline" size="sm" disabled={count >= maxKicks} onClick={() => setCount(c => Math.min(maxKicks, c + 1))}>+</Button>
            </div>
          </div>
        </div>
        <div className="flex gap-3 justify-end">
          <Button variant="outline" onClick={() => onDecide(0)}>Skip</Button>
          <Button onClick={() => onDecide(count)}>Confirm ({count}x)</Button>
        </div>
      </div>
    </Modal>
  );
}

// ── Replicate ──────────────────────────────────────────────

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
      <div className="p-6">
        <h2 className="font-semibold text-base mb-3">Replicate</h2>
        <div className="flex gap-3 mb-4">
          {imageUrl && (
            <CardImageThumbnail
              imageUrl={imageUrl}
              cardName={sourceCardName ?? "Spell"}
              className="w-[120px] h-[168px] rounded-lg object-cover shrink-0 shadow-md"
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
        <div className="flex gap-3 justify-end">
          <Button variant="outline" onClick={() => onDecide(0)}>Skip</Button>
          <Button onClick={() => onDecide(count)}>Confirm ({count}x)</Button>
        </div>
      </div>
    </Modal>
  );
}

// ── Alternative Cost ───────────────────────────────────────

interface AlternativeCostModalProps {
  options: string[];
  sourceCardName?: string;
  onDecide: (chosenIndex: number) => void;
}

export function AlternativeCostModal({ options, sourceCardName, onDecide }: AlternativeCostModalProps) {
  const { data: imageUrl } = useCardImage(sourceCardName ?? "");
  return (
    <Modal maxWidth="max-w-md" maxHeight="">
      <div className="p-6">
        <h2 className="font-semibold text-base mb-3">Choose Casting Option</h2>
        <div className="flex gap-3 mb-4">
          {imageUrl && (
            <CardImageThumbnail
              imageUrl={imageUrl}
              cardName={sourceCardName ?? "Spell"}
              className="w-[120px] h-[168px] rounded-lg object-cover shrink-0 shadow-md"
            />
          )}
          <div className="flex flex-col gap-2 flex-1">
            {options.map((opt, idx) => (
              <Button
                key={idx}
                variant={idx === 0 ? "outline" : "default"}
                className="text-left justify-start h-auto py-2"
                onClick={() => onDecide(idx)}
              >
                <TextWithMana text={opt} manaSize="sm" />
              </Button>
            ))}
          </div>
        </div>
      </div>
    </Modal>
  );
}
