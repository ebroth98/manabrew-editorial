import { useDeckStore } from "@/stores/useDeckStore";
import { cn } from "@/lib/utils";
import { computeCmc, isLand, countColorPips, countGenericMana } from "@/lib/mana";
import type { ManaColor } from "@/lib/mana";
import type { Card } from "@/types/xmage";

// CMC 1–7+ buckets with cool→warm colour progression
const BUCKETS = [
  { label: "1",  bar: "bg-emerald-500 dark:bg-emerald-500" },
  { label: "2",  bar: "bg-teal-500    dark:bg-teal-400"    },
  { label: "3",  bar: "bg-sky-500     dark:bg-sky-400"     },
  { label: "4",  bar: "bg-violet-500  dark:bg-violet-400"  },
  { label: "5",  bar: "bg-amber-500   dark:bg-amber-400"   },
  { label: "6",  bar: "bg-orange-500  dark:bg-orange-400"  },
  { label: "7+", bar: "bg-red-500     dark:bg-red-400"     },
];

const BAR_MAX_PX = 72;

// MTG colour identity for the distribution bars
const COLOR_ROWS: { color: ManaColor; label: string; bar: string; dot: string }[] = [
  { color: "W", label: "W", bar: "bg-yellow-300",  dot: "bg-yellow-300"  },
  { color: "U", label: "U", bar: "bg-blue-500",    dot: "bg-blue-500"    },
  { color: "B", label: "B", bar: "bg-zinc-500",    dot: "bg-zinc-500"    },
  { color: "R", label: "R", bar: "bg-red-500",     dot: "bg-red-500"     },
  { color: "G", label: "G", bar: "bg-green-500",   dot: "bg-green-500"   },
  { color: "C", label: "C", bar: "bg-zinc-400",    dot: "bg-zinc-400"    },
];

/** Resolve CMC for a card. Returns undefined when genuinely unknown. */
function resolveCmc(card: Card): number | undefined {
  // cmc explicitly set (including 0 for zero-cost spells like Ornithopter)
  if (card.cmc !== undefined && card.cmc !== null) return card.cmc;
  // fall back to parsing manaCost string (Scryfall {2}{U} or Forge 2 U)
  if (card.manaCost) return computeCmc(card.manaCost);
  return undefined; // truly unknown — imported card with no cost data
}

interface DeckStatsProps {
  /** Cards to analyse. Defaults to the current deck mainboard. */
  cards?: Card[];
}

export function DeckStats({ cards: propCards }: DeckStatsProps) {
  const { currentDeck } = useDeckStore();
  const cards = propCards ?? currentDeck.cards;

  // Separate lands, spells with known CMC, and spells with unknown CMC
  const lands: Card[] = [];
  const unknown: Card[] = [];
  const spells: { card: Card; cmc: number }[] = [];

  for (const card of cards) {
    if (isLand(card.types)) {
      lands.push(card);
      continue;
    }
    const cmc = resolveCmc(card);
    if (cmc === undefined) {
      unknown.push(card);
    } else {
      spells.push({ card, cmc });
    }
  }

  // Build counts — CMC 1 → bucket 0, CMC 7+ → bucket 6
  // CMC 0 (e.g. Ornithopter) → bucket 0 alongside CMC 1 (shown as "0-1")
  const counts = Array<number>(7).fill(0);
  for (const { cmc } of spells) {
    const idx = Math.min(Math.max(Math.round(cmc) - 1, 0), 6);
    counts[idx]++;
  }

  const max = Math.max(...counts, 1);
  const hasAnything = spells.length > 0;

  // ── Colour pip distribution ──────────────────────────────────────────────
  const pipTotals: Record<ManaColor, number> = { W: 0, U: 0, B: 0, R: 0, G: 0, C: 0 };
  let genericTotal = 0;
  for (const { card } of spells) {
    if (!card.manaCost) continue;
    const pips = countColorPips(card.manaCost);
    for (const c of ["W", "U", "B", "R", "G", "C"] as ManaColor[]) {
      pipTotals[c] += pips[c];
    }
    genericTotal += countGenericMana(card.manaCost);
  }
  const totalPips = Object.values(pipTotals).reduce((a, b) => a + b, 0);
  // All mana: coloured pips + explicit colourless pips + generic numeric costs
  const totalAll = totalPips + genericTotal;
  const activeColors = COLOR_ROWS.filter((r) => pipTotals[r.color] > 0);

  return (
    <div className="px-3 pt-2 pb-3 border-t shrink-0">
      {/* Header */}
      <div className="flex items-center justify-between mb-2">
        <span className="text-xs font-semibold text-muted-foreground uppercase tracking-wide">
          Mana Curve
        </span>
        <div className="flex gap-2 text-xs text-muted-foreground">
          {spells.length > 0 && <span>{spells.length} spells</span>}
          {lands.length > 0  && <span>{lands.length} lands</span>}
          {unknown.length > 0 && (
            <span className="text-amber-500" title="CMC unknown for these cards — search them via Scryfall to fix">
              {unknown.length} unknown
            </span>
          )}
        </div>
      </div>

      {hasAnything ? (
        <>
          {/* Count labels row — always visible */}
          <div className="flex gap-1 mb-0.5">
            {counts.map((count, i) => (
              <div key={i} className="flex-1 text-center">
                <span
                  className={cn(
                    "text-xs font-mono tabular-nums leading-none",
                    count > 0 ? "text-foreground" : "text-transparent select-none"
                  )}
                >
                  {count}
                </span>
              </div>
            ))}
          </div>

          {/* Bar chart — items-end makes bars grow from bottom */}
          <div className="flex items-end gap-1" style={{ height: BAR_MAX_PX }}>
            {counts.map((count, i) => (
              <div
                key={i}
                className={cn(
                  "flex-1 rounded-t-sm transition-all duration-300",
                  BUCKETS[i].bar,
                  count === 0 && "opacity-15"
                )}
                style={{
                  height: count > 0
                    ? `${Math.max((count / max) * BAR_MAX_PX, 3)}px`
                    : "3px",
                }}
              />
            ))}
          </div>

          {/* CMC label row */}
          <div className="flex gap-1 mt-1">
            {BUCKETS.map((b, i) => (
              <div key={i} className="flex-1 text-center">
                <span className="text-xs text-muted-foreground">{b.label}</span>
              </div>
            ))}
          </div>

          {/* ── Colour distribution ───────────────────────────── */}
          {(activeColors.length > 0 || genericTotal > 0) && (
            <div className="mt-3 space-y-1">
              <span className="text-xs font-semibold text-muted-foreground uppercase tracking-wide">
                Colour Distribution
              </span>
              <div className="mt-1.5 space-y-1">
                {/* Generic (uncoloured) mana bar — shown first */}
                {genericTotal > 0 && (
                  <div className="flex items-center gap-2">
                    <span
                      className="inline-block w-3.5 h-3.5 rounded-sm shrink-0 border border-zinc-400/50 bg-zinc-300 dark:bg-zinc-600"
                      title="Generic (uncoloured)"
                    />
                    <div className="flex-1 h-2 rounded-full bg-muted overflow-hidden">
                      <div
                        className="h-full rounded-full transition-all duration-300 bg-zinc-300 dark:bg-zinc-600"
                        style={{ width: `${totalAll > 0 ? (genericTotal / totalAll) * 100 : 0}%` }}
                      />
                    </div>
                    <span className="text-xs font-mono tabular-nums text-muted-foreground w-8 text-right shrink-0">
                      {Math.round(totalAll > 0 ? (genericTotal / totalAll) * 100 : 0)}%
                    </span>
                  </div>
                )}
                {/* Coloured + colourless pip bars */}
                {activeColors.map(({ color, label, bar, dot }) => {
                  const pips = pipTotals[color];
                  const pct = totalAll > 0 ? (pips / totalAll) * 100 : 0;
                  return (
                    <div key={color} className="flex items-center gap-2">
                      {/* Colour dot */}
                      <span
                        className={cn(
                          "inline-block w-3.5 h-3.5 rounded-full shrink-0 border border-black/20",
                          dot
                        )}
                        title={label}
                      />
                      {/* Horizontal bar */}
                      <div className="flex-1 h-2 rounded-full bg-muted overflow-hidden">
                        <div
                          className={cn("h-full rounded-full transition-all duration-300", bar)}
                          style={{ width: `${pct}%` }}
                        />
                      </div>
                      {/* Percentage */}
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
  );
}
