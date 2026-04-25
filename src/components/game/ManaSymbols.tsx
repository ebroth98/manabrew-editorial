import { cn } from "@/lib/utils";

/**
 * Renders mana cost strings as inline Scryfall SVG symbols.
 *
 * Accepts two formats:
 *  - Braced: `{2}{W}{U}`, `{W/U}`, `{X}{R}`
 *  - Space-separated: `2 W U`, `W`, `W/U`
 */

const SIZE_CLASSES = {
  sm: "h-3.5 w-3.5",
  md: "h-4 w-4",
  lg: "h-5 w-5",
} as const;

export type ManaSymbolSize = keyof typeof SIZE_CLASSES;

// Valid mana symbols: single letters, numbers, X, hybrid (W/U, 2/W), phyrexian (W/P), snow (S), tap (T), etc.
const VALID_SYMBOL = /^(?:\d+|[WUBRGCSXYTQEP]|[WUBRG2]\/[WUBRGP]|H[WUBRG])$/i;

/** Parse a mana cost string into individual symbol tokens. */
export function parseManaSymbols(cost: string): string[] {
  if (!cost || cost === "no cost") return [];
  if (cost.includes("{")) {
    const matches = cost.match(/\{[^}]+\}/g);
    if (!matches) return [];
    return matches
      .map((m) => m.slice(1, -1).trim())
      .filter((s) => s.length > 0 && VALID_SYMBOL.test(s));
  }
  return cost.split(/\s+/).filter((s) => s.length > 0 && VALID_SYMBOL.test(s));
}

function symbolUrl(symbol: string): string {
  // Scryfall SVG filenames strip slashes: {W/U} → WU.svg, {2/W} → 2W.svg, {W/P} → WP.svg
  const filename = symbol.replace(/\//g, "");
  return `https://svgs.scryfall.io/card-symbols/${encodeURIComponent(filename)}.svg`;
}

interface ManaSymbolsProps {
  cost: string;
  size?: ManaSymbolSize;
  className?: string;
}

export function ManaSymbols({ cost, size = "md", className }: ManaSymbolsProps) {
  const symbols = parseManaSymbols(cost);
  if (symbols.length === 0) return null;

  const sizeClass = SIZE_CLASSES[size];

  return (
    <span className={cn("inline-flex items-center gap-0.5", className)}>
      {symbols.map((sym, i) => (
        <img
          key={`${sym}-${i}`}
          src={symbolUrl(sym)}
          alt={`{${sym}}`}
          title={`{${sym}}`}
          className={sizeClass}
          loading="lazy"
        />
      ))}
    </span>
  );
}
