import { ManaSymbols } from "@/components/game/ManaSymbols";
import { MANA_COLORS } from "./game.constants";

export function ManaPool({ pool }: { pool: Record<string, number> }) {
  const total = Object.values(pool).reduce((a, b) => a + b, 0);
  if (total === 0)
    return <span className="text-xs text-muted-foreground italic">Empty</span>;
  return (
    <div className="flex gap-1.5 flex-wrap items-center">
      {MANA_COLORS.flatMap(({ key }, i, arr) => {
        const count = pool[key] ?? 0;
        if (count === 0) return [];
        const items = [
          <span key={key} className="inline-flex items-center gap-0.5">
            <ManaSymbols cost={key} size="md" />
            {count > 1 && (
              <span className="text-xs font-bold text-foreground">{count}</span>
            )}
          </span>,
        ];
        const hasMore = arr.slice(i + 1).some(({ key: k }) => (pool[k] ?? 0) > 0);
        if (hasMore) {
          items.push(
            <span key={`sep-${key}`} className="text-muted-foreground/40 text-xs select-none">|</span>
          );
        }
        return items;
      })}
    </div>
  );
}
