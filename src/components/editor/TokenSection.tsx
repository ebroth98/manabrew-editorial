/**
 * Collapsible bottom-panel section that displays tokens referenced by cards in the deck.
 * Renders as a sticky footer panel above the mana curve, matching DeckStats style.
 * Always uses grid card thumbnails for token display.
 *
 * Image resolution: uses token DeckCard URIs from the deck store.
 */

import { useState, type MouseEvent } from "react";
import { ChevronDown, ChevronRight, Palette, X } from "lucide-react";
import { CARD_WIDTH_MAP } from "./deckBuilder.utils";
import { ScryfallImg } from "@/components/ScryfallImg";
import type { DeckCard } from "@/types/manabrew";

// ─── Main TokenSection Component ────────────────────────────────────────────

export interface TokenSectionProps {
  tokens: DeckCard[];
  cardSize: number;
  onShowInfo?: (tokenName: string) => void;
  onPickPrint?: (tokenName: string) => void;
  onRemoveToken?: (tokenName: string) => void;
  onHover?: (token: DeckCard, e: MouseEvent) => void;
  onLeave?: () => void;
}

export function TokenSection({
  tokens,
  cardSize,
  onShowInfo,
  onPickPrint,
  onRemoveToken,
  onHover,
  onLeave,
}: TokenSectionProps) {
  const [collapsed, setCollapsed] = useState(tokens.length === 0);

  const cardWidth = CARD_WIDTH_MAP[cardSize] ?? 115;

  return (
    <div className="border-t shrink-0">
      {/* ── Toggle header ── */}
      <div className="flex items-center gap-1.5 w-full px-3 py-2 hover:bg-muted/30 transition-colors">
        <div
          role="button"
          tabIndex={0}
          className="flex items-center gap-1.5 flex-1 text-left cursor-pointer"
          onClick={() => setCollapsed((v) => !v)}
          onKeyDown={(e) => {
            if (e.key === "Enter" || e.key === " ") {
              e.preventDefault();
              setCollapsed((v) => !v);
            }
          }}
        >
          {collapsed ? (
            <ChevronRight className="h-3 w-3 text-muted-foreground shrink-0" />
          ) : (
            <ChevronDown className="h-3 w-3 text-muted-foreground shrink-0" />
          )}
          <span className="text-xs font-semibold text-muted-foreground uppercase tracking-wide">
            Tokens
          </span>
          <span className="text-xs text-muted-foreground/70">
            {tokens.length} token{tokens.length !== 1 ? "s" : ""}
          </span>

          {/* Mini preview thumbnails when collapsed */}
          {collapsed && tokens.length > 0 && (
            <div className="ml-auto flex items-center gap-1 shrink-0">
              {tokens.slice(0, 6).map((t) => (
                <MiniTokenPill key={t.name} token={t} />
              ))}
              {tokens.length > 6 && (
                <span className="text-[10px] text-muted-foreground/50">+{tokens.length - 6}</span>
              )}
            </div>
          )}
        </div>
      </div>

      {/* ── Expandable content ── */}
      {!collapsed && (
        <div className="px-3 pb-3 max-h-[300px] overflow-y-auto">
          {tokens.length === 0 ? (
            <div className="text-xs text-muted-foreground py-3">
              No tokens produced by this deck.
            </div>
          ) : (
            <div className="flex flex-wrap gap-2">
              {tokens.map((t) => (
                <div
                  key={`${t.name}-${t.setCode}-${t.cardNumber}`}
                  className="shrink-0"
                  style={{ width: cardWidth }}
                >
                  <TokenGridCard
                    token={t}
                    onShowInfo={onShowInfo}
                    onPickPrint={onPickPrint}
                    onRemove={onRemoveToken}
                    onHover={onHover}
                    onLeave={onLeave}
                  />
                </div>
              ))}
            </div>
          )}
        </div>
      )}
    </div>
  );
}

// ─── Mini pill shown in collapsed header ─────────────────────────────────────

function MiniTokenPill({ token }: { token: DeckCard }) {
  return (
    <ScryfallImg
      src={token.uris.small}
      alt={token.name}
      className="h-5 w-[14px] rounded-sm object-cover object-top border border-border/30"
      draggable={false}
    />
  );
}

// ─── Grid card with token image + print picker ──────────────────────────────

function TokenGridCard({
  token,
  onShowInfo,
  onPickPrint,
  onRemove,
  onHover,
  onLeave,
}: {
  token: DeckCard;
  onShowInfo?: (name: string) => void;
  onPickPrint?: (name: string) => void;
  onRemove?: (name: string) => void;
  onHover?: (token: DeckCard, e: MouseEvent) => void;
  onLeave?: () => void;
}) {
  return (
    <div
      className="relative group cursor-pointer"
      onClick={() => onShowInfo?.(token.name)}
      onMouseEnter={(e) => onHover?.(token, e)}
      onMouseLeave={() => onLeave?.()}
    >
      <ScryfallImg
        src={token.uris.normal}
        alt={token.name}
        className="w-full rounded-lg border border-border/50 shadow-sm"
        draggable={false}
      />

      {/* Action buttons — top-right on hover */}
      <div className="absolute top-1 right-1 z-20 flex gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
        {onPickPrint && (
          <button
            type="button"
            className="rounded-full p-0.5 shadow bg-overlay/70 text-muted-foreground hover:text-foreground transition-colors"
            title="Change printing"
            onClick={(e) => {
              e.stopPropagation();
              onPickPrint(token.name);
            }}
          >
            <Palette className="h-3.5 w-3.5" />
          </button>
        )}
        {onRemove && (
          <button
            type="button"
            className="rounded-full p-0.5 shadow bg-overlay/70 text-muted-foreground hover:text-destructive transition-colors"
            title="Remove token"
            onClick={(e) => {
              e.stopPropagation();
              onRemove(token.name);
            }}
          >
            <X className="h-3.5 w-3.5" />
          </button>
        )}
      </div>
    </div>
  );
}
