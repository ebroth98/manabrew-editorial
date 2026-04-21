import { useDeckStore } from "@/stores/useDeckStore";
import { cn } from "@/lib/utils";
import { computeCmc, isLand, countColorPips, countGenericMana } from "@/lib/mana";
import type { ManaColor } from "@/lib/mana";
import type { Card } from "@/types/openmagic";
import { useState } from "react";
import { ChevronDown, ChevronRight } from "lucide-react";
import { ManaSymbols } from "@/components/game/ManaSymbols";

// CMC 1–7+ buckets — cool→warm progression using theme counter / signal
// tokens so the curve retones with the active preset.
const BUCKETS = [
  { label: "1",  bar: "bg-pt-buffed"       }, // +1 growth green
  { label: "2",  bar: "bg-counter-storage" }, // teal
  { label: "3",  bar: "bg-counter-study"   }, // cyan
  { label: "4",  bar: "bg-counter-charge"  }, // purple
  { label: "5",  bar: "bg-warning"         }, // amber
  { label: "6",  bar: "bg-counter-level"   }, // orange
  { label: "7+", bar: "bg-pt-lethal"       }, // red
];

const BAR_MAX_PX = 72;

const COLOR_ROWS: { color: ManaColor; label: string; bar: string }[] = [
  { color: "W", label: "W", bar: "bg-mana-w" },
  { color: "U", label: "U", bar: "bg-mana-u" },
  { color: "B", label: "B", bar: "bg-mana-b" },
  { color: "R", label: "R", bar: "bg-mana-r" },
  { color: "G", label: "G", bar: "bg-mana-g" },
  { color: "C", label: "C", bar: "bg-mana-c" },
];

/** Resolve CMC for a card. Returns undefined when genuinely unknown. */
function resolveCmc(card: Card): number | undefined {
  if (card.cmc !== undefined && card.cmc !== null) return card.cmc;
  if (card.manaCost) return computeCmc(card.manaCost);
  return undefined;
}

interface DeckStatsProps {
  cards?: Card[];
}

export function DeckStats({ cards: propCards }: DeckStatsProps) {
  const { currentDeck } = useDeckStore();
  const cards = propCards ?? currentDeck.cards;
  const [collapsed, setCollapsed] = useState(true);

  const lands: Card[] = [];
  const unknown: Card[] = [];
  const spells: { card: Card; cmc: number }[] = [];

  for (const card of cards) {
    if (isLand(card.types)) { lands.push(card); continue; }
    const cmc = resolveCmc(card);
    if (cmc === undefined) unknown.push(card);
    else spells.push({ card, cmc });
  }

  const counts = Array<number>(7).fill(0);
  for (const { cmc } of spells) {
    const idx = Math.min(Math.max(Math.round(cmc) - 1, 0), 6);
    counts[idx]++;
  }

  const max = Math.max(...counts, 1);
  const hasAnything = spells.length > 0;

  const pipTotals: Record<ManaColor, number> = { W: 0, U: 0, B: 0, R: 0, G: 0, C: 0 };
  let genericTotal = 0;
  for (const { card } of spells) {
    if (!card.manaCost) continue;
    const pips = countColorPips(card.manaCost);
    for (const c of ["W", "U", "B", "R", "G", "C"] as ManaColor[]) pipTotals[c] += pips[c];
    genericTotal += countGenericMana(card.manaCost);
  }
  const totalPips = Object.values(pipTotals).reduce((a, b) => a + b, 0);
  const totalAll = totalPips + genericTotal;
  const activeColors = COLOR_ROWS.filter((r) => pipTotals[r.color] > 0);

  // Mini inline curve preview shown in the collapsed header
  const curvePreview = hasAnything ? counts : null;

  return (
    <div className="border-t shrink-0">
      {/* ── Toggle header ── */}
      <button
        type="button"
        className="flex items-center gap-1.5 w-full px-3 py-2 hover:bg-muted/30 transition-colors text-left"
        onClick={() => setCollapsed((v) => !v)}
      >
        {collapsed
          ? <ChevronRight className="h-3 w-3 text-muted-foreground shrink-0" />
          : <ChevronDown  className="h-3 w-3 text-muted-foreground shrink-0" />
        }
        <span className="text-xs font-semibold text-muted-foreground uppercase tracking-wide">
          Mana Curve
        </span>

        {/* Summary pills — always visible */}
        <div className="flex gap-1.5 ml-1 text-xs text-muted-foreground/70">
          {spells.length > 0 && <span>{spells.length} spells</span>}
          {lands.length > 0  && <span>{lands.length} lands</span>}
          {unknown.length > 0 && (
            <span className="text-warning" title="CMC unknown">{unknown.length} ?</span>
          )}
        </div>

        {/* Mini sparkline curve when collapsed */}
        {collapsed && curvePreview && (
          <div className="ml-auto flex items-end gap-px h-4 shrink-0">
            {curvePreview.map((count, i) => (
              <div
                key={i}
                className={cn("w-2 rounded-t-sm", BUCKETS[i].bar, count === 0 && "opacity-20")}
                style={{ height: count > 0 ? `${Math.max((count / max) * 16, 2)}px` : "2px" }}
              />
            ))}
          </div>
        )}
      </button>

      {/* ── Expandable content ── */}
      {!collapsed && (
        <div className="px-3 pb-3">
          {hasAnything ? (
            <>
              {/* Count labels */}
              <div className="flex gap-1 mb-0.5">
                {counts.map((count, i) => (
                  <div key={i} className="flex-1 text-center">
                    <span className={cn(
                      "text-xs font-mono tabular-nums leading-none",
                      count > 0 ? "text-foreground" : "text-transparent select-none"
                    )}>
                      {count}
                    </span>
                  </div>
                ))}
              </div>

              {/* Bar chart */}
              <div className="flex items-end gap-1" style={{ height: BAR_MAX_PX }}>
                {counts.map((count, i) => (
                  <div
                    key={i}
                    className={cn(
                      "flex-1 rounded-t-sm transition-all duration-300",
                      BUCKETS[i].bar,
                      count === 0 && "opacity-15"
                    )}
                    style={{ height: count > 0 ? `${Math.max((count / max) * BAR_MAX_PX, 3)}px` : "3px" }}
                  />
                ))}
              </div>

              {/* CMC labels */}
              <div className="flex gap-1 mt-1">
                {BUCKETS.map((b, i) => (
                  <div key={i} className="flex-1 text-center">
                    <span className="text-xs text-muted-foreground">{b.label}</span>
                  </div>
                ))}
              </div>

              {/* Colour distribution */}
              {(activeColors.length > 0 || genericTotal > 0) && (
                <div className="mt-3 space-y-1">
                  <span className="text-xs font-semibold text-muted-foreground uppercase tracking-wide">
                    Colour Distribution
                  </span>
                  <div className="mt-1.5 space-y-1">
                    {genericTotal > 0 && (
                      <div className="flex items-center gap-2">
                        <span className="inline-block w-3.5 h-3.5 rounded-sm shrink-0 border border-border bg-muted-foreground/40" title="Generic" />
                        <div className="flex-1 h-2 rounded-full bg-muted overflow-hidden">
                          <div
                            className="h-full rounded-full transition-all duration-300 bg-muted-foreground/60"
                            style={{ width: `${totalAll > 0 ? (genericTotal / totalAll) * 100 : 0}%` }}
                          />
                        </div>
                        <span className="text-xs font-mono tabular-nums text-muted-foreground w-8 text-right shrink-0">
                          {Math.round(totalAll > 0 ? (genericTotal / totalAll) * 100 : 0)}%
                        </span>
                      </div>
                    )}
                    {activeColors.map(({ color, bar }) => {
                      const pips = pipTotals[color];
                      const pct = totalAll > 0 ? (pips / totalAll) * 100 : 0;
                      return (
                        <div key={color} className="flex items-center gap-2">
                          <ManaSymbols cost={`{${color}}`} size="sm" />
                          <div className="flex-1 h-2 rounded-full bg-muted overflow-hidden">
                            <div className={cn("h-full rounded-full transition-all duration-300", bar)} style={{ width: `${pct}%` }} />
                          </div>
                          <span className="text-xs font-mono tabular-nums text-muted-foreground w-8 text-right shrink-0">
                            {Math.round(pct)}%
                          </span>
                        </div>
                      );
                    })}
                  </div>
                </div>
              )}
            </>
          ) : (
            <p className="text-xs text-muted-foreground italic text-center py-3">
              {cards.length === 0 ? "No cards in deck." : "Add non-land cards to see the curve."}
            </p>
          )}
        </div>
      )}
    </div>
  );
}
