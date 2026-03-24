import { ManaSymbols } from "@/components/game/ManaSymbols";
import { MANA_COLORS } from "../game.constants";

export function ManaPool({ pool }: { pool: Record<string, number> }) {
  const total = Object.values(pool).reduce((a, b) => a + b, 0);
  if (total === 0)
    return <span className="text-xs text-muted-foreground italic">Empty</span>;
  return (
    <div className="flex flex-row items-center gap-1 flex-nowrap">
      {MANA_COLORS.flatMap(({ key }) => {
        const count = pool[key] ?? 0;
        if (count === 0) return [];
        return [
          <span
            key={key}
            className="inline-flex flex-row items-center gap-0.5 flex-nowrap"
          >
            <span className="text-[11px] font-extrabold text-white leading-none tabular-nums">
              {count}
            </span>
            <ManaSymbols cost={key} size="lg" />
          </span>,
        ];
      })}
    </div>
  );
}
