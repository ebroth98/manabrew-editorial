import { cn } from "@/lib/utils";
import type { GameThemeColors } from "@/components/game/game.theme";

// ---------------------------------------------------------------------------
// Counter configuration registry
// ---------------------------------------------------------------------------

type CounterColorKey = keyof GameThemeColors["counter"];

/** Visual configuration for a single counter type. */
export interface CounterConfig {
  /** Symbol shown inside the badge. Keep ≤ 3 chars so it fits at small sizes. */
  label: string;
  /** Theme colour key for this counter's background tint. */
  colorKey: CounterColorKey;
  /** Full human-readable name used in tooltips. */
  title: string;
}

/**
 * Known counter types and their visual identity.
 *
 * Keys must match the `CounterType` variant names produced by the Rust engine
 * (derived from `{:?}` formatting, e.g. "P1P1", "M1M1", "Loyalty").
 *
 * Colour comes from `theme.counter.<colorKey>` — see `src/themes/default.ts`
 * for the canonical palette. To add a new counter type, add an entry here
 * and a matching `counter.<name>` key to the theme.
 */
export const COUNTER_CONFIG: Record<string, CounterConfig> = {
  P1P1:      { label: "+1",  colorKey: "p1p1",      title: "+1/+1"     },
  M1M1:      { label: "−1",  colorKey: "m1m1",      title: "−1/−1"     },
  Loyalty:   { label: "♦",   colorKey: "loyalty",   title: "Loyalty"   },
  Charge:    { label: "⚡",   colorKey: "charge",    title: "Charge"    },
  Quest:     { label: "◎",   colorKey: "quest",     title: "Quest"     },
  Study:     { label: "✎",   colorKey: "study",     title: "Study"     },
  Lore:      { label: "✦",   colorKey: "lore",      title: "Lore"      },
  Age:       { label: "⌛",   colorKey: "age",       title: "Age"       },
  Time:      { label: "⏳",   colorKey: "time",      title: "Time"      },
  Fade:      { label: "✕",   colorKey: "fade",      title: "Fade"      },
  Level:     { label: "★",   colorKey: "level",     title: "Level"     },
  Storage:   { label: "▲",   colorKey: "storage",   title: "Storage"   },
  Mining:    { label: "⛏",   colorKey: "mining",    title: "Mining"    },
  Brick:     { label: "▪",   colorKey: "brick",     title: "Brick"     },
  Depletion: { label: "▼",   colorKey: "depletion", title: "Depletion" },
  Page:      { label: "📄",  colorKey: "page",      title: "Page"      },
};

/** Returns the config for a known counter type, or a sensible generic fallback. */
export function getCounterConfig(type: string): CounterConfig {
  return (
    COUNTER_CONFIG[type] ?? {
      label: type.slice(0, 3),
      colorKey: "default",
      title: type,
    }
  );
}

/** Static `bg-counter-*` class per counter colour key — Tailwind JIT
 *  needs the full class name in source, so we can't string-build it. */
const COUNTER_BG_CLASS: Record<CounterColorKey, string> = {
  default:   "bg-counter-default",
  p1p1:      "bg-counter-p1p1",
  m1m1:      "bg-counter-m1m1",
  loyalty:   "bg-counter-loyalty",
  charge:    "bg-counter-charge",
  quest:     "bg-counter-quest",
  study:     "bg-counter-study",
  lore:      "bg-counter-lore",
  age:       "bg-counter-age",
  time:      "bg-counter-time",
  fade:      "bg-counter-fade",
  level:     "bg-counter-level",
  storage:   "bg-counter-storage",
  mining:    "bg-counter-mining",
  brick:     "bg-counter-brick",
  depletion: "bg-counter-depletion",
  page:      "bg-counter-page",
};

// ---------------------------------------------------------------------------
// Size tokens
// ---------------------------------------------------------------------------

export type CounterSize = "sm" | "md" | "lg";

interface SizeTokens {
  pill: string;   // outer element classes
  symbol: string; // symbol text size
  count: string;  // count text size
}

const SIZE_TOKENS: Record<CounterSize, SizeTokens> = {
  sm: { pill: "h-4 min-w-[1rem] px-0.5 gap-px",  symbol: "text-[8px]",  count: "text-[7px]"  },
  md: { pill: "h-5 min-w-[1.25rem] px-1 gap-0.5", symbol: "text-[10px]", count: "text-[9px]"  },
  lg: { pill: "h-6 min-w-[1.5rem] px-1.5 gap-1",  symbol: "text-xs",     count: "text-[10px]" },
};

// ---------------------------------------------------------------------------
// CounterBadge — one badge per counter type
// ---------------------------------------------------------------------------

export interface CounterBadgeProps {
  type: string;
  count: number;
  size?: CounterSize;
  className?: string;
}

/**
 * Renders a single pill-shaped counter badge showing its symbol and, when
 * count > 1, the quantity.  Returns null for zero-count counters.
 */
export function CounterBadge({ type, count, size = "sm", className }: CounterBadgeProps) {
  if (count <= 0) return null;

  const cfg = getCounterConfig(type);
  const sz = SIZE_TOKENS[size];

  return (
    <span
      className={cn(
        "inline-flex items-center justify-center rounded-full font-bold leading-none",
        "select-none shadow-sm ring-1 ring-black/20",
        COUNTER_BG_CLASS[cfg.colorKey],
        "text-text-on-tinted",
        sz.pill, className,
      )}
      title={`${count} ${cfg.title} counter${count !== 1 ? "s" : ""}`}
    >
      <span className={sz.symbol}>{cfg.label}</span>
      {count > 1 && (
        <span className={cn(sz.count, "opacity-90")}>{count}</span>
      )}
    </span>
  );
}

// ---------------------------------------------------------------------------
// CounterDisplay — row of badges for all counters on a card
// ---------------------------------------------------------------------------

export interface CounterDisplayProps {
  /** Map of counter-type name → count, as received from the engine. */
  counters: Record<string, number>;
  size?: CounterSize;
  className?: string;
}

/**
 * Renders all non-zero counters for a card as a compact, wrapping row of
 * `CounterBadge` elements.  Returns null when there are no counters.
 */
export function CounterDisplay({ counters, size = "sm", className }: CounterDisplayProps) {
  const entries = Object.entries(counters).filter(([, n]) => n > 0);
  if (entries.length === 0) return null;

  return (
    <div className={cn("flex flex-wrap gap-0.5", className)}>
      {entries.map(([type, count]) => (
        <CounterBadge key={type} type={type} count={count} size={size} />
      ))}
    </div>
  );
}
