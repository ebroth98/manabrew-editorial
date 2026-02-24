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

/** Parse a mana cost string into individual symbol tokens. */
export function parseManaSymbols(cost: string): string[] {
  if (!cost) return [];
  if (cost.includes("{")) {
    const matches = cost.match(/\{[^}]+\}/g);
    if (!matches) return [];
    return matches.map((m) => m.slice(1, -1));
  }
  return cost.split(/\s+/).filter(Boolean);
}

function symbolUrl(symbol: string): string {
  return `https://svgs.scryfall.io/card-symbols/${encodeURIComponent(symbol)}.svg`;
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
    <span className={`inline-flex items-center gap-0.5 ${className ?? ""}`}>
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
