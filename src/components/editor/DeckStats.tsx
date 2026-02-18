import { useDeckStore } from "@/stores/useDeckStore";
import { cn } from "@/lib/utils";

export function DeckStats() {
  const { currentDeck } = useDeckStore();

  const getManaCurve = () => {
    const curve = Array(8).fill(0);
    currentDeck.cards.forEach((card) => {
      // Use pre-calculated CMC if available
      const cmc = card.cmc ?? 0;
      
      const index = Math.min(Math.floor(cmc), 7);
      curve[index]++;
    });
    return curve;
  };

  const curve = getManaCurve();
  const max = Math.max(...curve, 1);

  return (
    <div className="p-4 border-t">
      <h3 className="text-sm font-semibold mb-2">Mana Curve</h3>
      <div className="flex items-end justify-between h-24 gap-1">
        {curve.map((count, i) => (
          <div key={i} className="flex-1 flex flex-col items-center gap-1 group relative">
            <div className="w-full bg-muted rounded-t-sm overflow-hidden flex flex-col justify-end h-full">
              <div 
                className={cn("w-full bg-primary transition-all", count === 0 && "opacity-0")}
                style={{ height: `${(count / max) * 100}%` }}
              />
            </div>
            <span className="text-xs text-muted-foreground">{i === 7 ? '7+' : i}</span>
            {count > 0 && (
              <div className="absolute -top-6 left-1/2 -translate-x-1/2 bg-popover text-popover-foreground text-xs px-1 py-0.5 rounded shadow opacity-0 group-hover:opacity-100 transition-opacity whitespace-nowrap z-10">
                {count} cards
              </div>
            )}
          </div>
        ))}
      </div>
    </div>
  );
}
