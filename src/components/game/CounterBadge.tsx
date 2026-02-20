import { cn } from "@/lib/utils";

// ---------------------------------------------------------------------------
// Counter configuration registry
// ---------------------------------------------------------------------------

/** Visual configuration for a single counter type. */
export interface CounterConfig {
  /** Symbol shown inside the badge. Keep ≤ 3 chars so it fits at small sizes. */
  label: string;
  /** Tailwind background colour class. */
  bg: string;
  /** Tailwind foreground (text) colour class. */
  fg: string;
  /** Full human-readable name used in tooltips. */
  title: string;
}

/**
 * Known counter types and their visual identity.
 *
 * Keys must match the `CounterType` variant names produced by the Rust engine
 * (derived from `{:?}` formatting, e.g. "P1P1", "M1M1", "Loyalty").
 *
 * To add a new counter type, just add an entry here — no other changes needed.
 */
export const COUNTER_CONFIG: Record<string, CounterConfig> = {
  P1P1:      { label: "+1",  bg: "bg-green-500",   fg: "text-white",      title: "+1/+1"     },
  M1M1:      { label: "−1",  bg: "bg-red-600",     fg: "text-white",      title: "−1/−1"     },
  Loyalty:   { label: "♦",   bg: "bg-blue-500",    fg: "text-white",      title: "Loyalty"   },
  Charge:    { label: "⚡",   bg: "bg-purple-500",  fg: "text-white",      title: "Charge"    },
  Quest:     { label: "◎",   bg: "bg-yellow-400",  fg: "text-gray-900",   title: "Quest"     },
  Study:     { label: "✎",   bg: "bg-cyan-500",    fg: "text-white",      title: "Study"     },
  Lore:      { label: "✦",   bg: "bg-amber-500",   fg: "text-white",      title: "Lore"      },
  Age:       { label: "⌛",   bg: "bg-stone-500",   fg: "text-white",      title: "Age"       },
  Time:      { label: "⏳",   bg: "bg-indigo-500",  fg: "text-white",      title: "Time"      },
  Fade:      { label: "✕",   bg: "bg-slate-500",   fg: "text-white",      title: "Fade"      },
  Level:     { label: "★",   bg: "bg-orange-500",  fg: "text-white",      title: "Level"     },
  Storage:   { label: "▲",   bg: "bg-teal-500",    fg: "text-white",      title: "Storage"   },
  Mining:    { label: "⛏",   bg: "bg-yellow-700",  fg: "text-white",      title: "Mining"    },
  Brick:     { label: "▪",   bg: "bg-orange-800",  fg: "text-white",      title: "Brick"     },
  Depletion: { label: "▼",   bg: "bg-rose-700",    fg: "text-white",      title: "Depletion" },
  Page:      { label: "📄",  bg: "bg-zinc-400",    fg: "text-gray-900",   title: "Page"      },
};

/** Returns the config for a known counter type, or a sensible generic fallback. */
export function getCounterConfig(type: string): CounterConfig {
  return (
    COUNTER_CONFIG[type] ?? {
      label: type.slice(0, 3),
      bg: "bg-gray-600",
      fg: "text-white",
      title: type,
    }
  );
}

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
        cfg.bg, cfg.fg, sz.pill, className,
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
