import { ManaSymbols } from "@/components/game/ManaSymbols";
import type { ManaSymbolSize } from "@/components/game/ManaSymbols";

interface TextWithManaProps {
  text: string;
  manaSize?: ManaSymbolSize;
}

/**
 * Renders a text string with inline mana symbols. Any `{W}`, `{2}{R}`, etc.
 * sequences are replaced with Scryfall SVG symbols; plain text is rendered as-is.
 */
export function TextWithMana({ text, manaSize = "sm" }: TextWithManaProps) {
  const parts = text.split(/(\{[^}]+\}(?:\{[^}]+\})*)/g);
  return (
    <span className="inline-flex items-center gap-0.5 flex-wrap">
      {parts.map((part, i) =>
        part.startsWith("{") ? (
          <ManaSymbols key={i} cost={part} size={manaSize} />
        ) : (
          <span key={i}>{part}</span>
        ),
      )}
    </span>
  );
}
