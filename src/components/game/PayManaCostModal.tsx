import { createPortal } from "react-dom";
import { TextWithMana } from "@/components/game/TextWithMana";
import { ManaPool } from "@/components/game/ManaPool";

interface PayManaCostModalProps {
  cardName: string;
  manaCost: string;
  manaPool: Record<string, number>;
  onPay: () => void;
  onCancel: () => void;
}

export function PayManaCostModal({
  cardName,
  manaCost,
  manaPool,
  onPay,
  onCancel,
}: PayManaCostModalProps) {
  return createPortal(
    <div className="fixed top-4 left-1/2 -translate-x-1/2 z-[9000] pointer-events-none">
      <div className="pointer-events-auto bg-zinc-900/95 border border-zinc-600 rounded-lg p-4 shadow-2xl w-80 backdrop-blur-sm">
        <h2 className="text-sm font-semibold text-zinc-100 mb-1.5">
          Pay Mana Cost
        </h2>
        <p className="text-xs text-zinc-300 mb-3">
          Cast <span className="font-semibold">{cardName}</span> for{" "}
          <TextWithMana text={manaCost} manaSize="sm" />
        </p>
        <div className="flex items-center justify-between text-xs text-zinc-400 mb-3">
          <span>Mana in pool:</span>
          <ManaPool pool={manaPool} />
        </div>
        <p className="text-[11px] text-zinc-500 mb-3">
          Tap lands on the battlefield to generate mana, then click Pay.
        </p>
        <div className="flex gap-2">
          <button
            className="flex-1 px-3 py-1.5 rounded bg-green-700 hover:bg-green-600 text-white text-sm font-medium transition-colors"
            onClick={onPay}
          >
            Pay
          </button>
          <button
            className="flex-1 px-3 py-1.5 rounded bg-zinc-700 hover:bg-zinc-600 text-zinc-200 text-sm font-medium transition-colors"
            onClick={onCancel}
          >
            Cancel
          </button>
        </div>
      </div>
    </div>,
    document.body,
  );
}
