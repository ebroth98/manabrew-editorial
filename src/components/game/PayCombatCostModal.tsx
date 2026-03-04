import { createPortal } from "react-dom";

interface PayCombatCostModalProps {
  attackerName: string;
  cost: number;
  description: string;
  manaPoolTotal: number;
  onPay: () => void;
  onDecline: () => void;
}

export function PayCombatCostModal({
  cost,
  description,
  manaPoolTotal,
  onPay,
  onDecline,
}: PayCombatCostModalProps) {
  const canPay = manaPoolTotal >= cost;

  return createPortal(
    <div className="fixed top-4 left-1/2 -translate-x-1/2 z-[9000] pointer-events-none">
      <div className="pointer-events-auto bg-zinc-900/95 border border-zinc-600 rounded-lg p-4 shadow-2xl w-80 backdrop-blur-sm">
        <h2 className="text-sm font-semibold text-zinc-100 mb-1.5">
          Pay Attack Cost
        </h2>
        <p className="text-xs text-zinc-300 mb-3">{description}</p>
        <div className="flex items-center justify-between text-xs text-zinc-400 mb-3">
          <span>Mana available:</span>
          <span
            className={
              canPay ? "text-green-400 font-semibold" : "text-red-400 font-semibold"
            }
          >
            {manaPoolTotal} / {cost}
          </span>
        </div>
        <p className="text-[11px] text-zinc-500 mb-3">
          Tap lands on the battlefield to generate mana, then click Pay.
        </p>
        <div className="flex gap-2">
          <button
            className="flex-1 px-3 py-1.5 rounded bg-green-700 hover:bg-green-600 text-white text-sm font-medium disabled:opacity-40 disabled:cursor-not-allowed transition-colors"
            disabled={!canPay}
            onClick={onPay}
          >
            Pay
          </button>
          <button
            className="flex-1 px-3 py-1.5 rounded bg-zinc-700 hover:bg-zinc-600 text-zinc-200 text-sm font-medium transition-colors"
            onClick={onDecline}
          >
            Decline
          </button>
        </div>
      </div>
    </div>,
    document.body,
  );
}
