/**
 * Collapsible bottom-panel section that displays tokens produced by cards in the deck.
 * Renders as a sticky footer panel above the mana curve, matching DeckStats style.
 * Always uses grid card thumbnails for token display.
 *
 * Image resolution: uses token.imageUrl from the deck store (user-selected print)
 * when available, otherwise falls back to a placeholder.
 */

import { useState } from "react";
import { ChevronDown, ChevronRight, Loader2, Palette } from "lucide-react";
import { cn } from "@/lib/utils";
import { GRID_COLS } from "./deckBuilder.utils";
import type { DeckToken } from "@/types/openmagic";

// ─── Scryfall fallback — only used when no stored imageUrl exists ───────────

function ScryFallbackImage({ name, className }: { name: string; className?: string }) {
  return (
    <div
      className={cn("aspect-[2.5/3.5] bg-muted flex items-center justify-center p-2", className)}
    >
      <span className="text-[9px] text-muted-foreground leading-tight text-center">{name}</span>
    </div>
  );
}

// ─── Main TokenSection Component ────────────────────────────────────────────

export interface TokenSectionProps {
  tokens: DeckToken[];
  isLoading: boolean;
  cardSize: number;
  onShowInfo?: (tokenName: string) => void;
  onPickPrint?: (tokenName: string) => void;
}

export function TokenSection({
  tokens,
  isLoading,
  cardSize,
  onShowInfo,
  onPickPrint,
}: TokenSectionProps) {
  const [collapsed, setCollapsed] = useState(true);

  if (!isLoading && tokens.length === 0) return null;

  const gridCols = GRID_COLS[cardSize] ?? "grid-cols-8";

  return (
    <div className="border-t shrink-0">
      {/* ── Toggle header ── */}
      <button
        type="button"
        className="flex items-center gap-1.5 w-full px-3 py-2 hover:bg-muted/30 transition-colors text-left"
        onClick={() => setCollapsed((v) => !v)}
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
          {isLoading ? (
            <Loader2 className="h-3 w-3 inline animate-spin" />
          ) : (
            `${tokens.length} token${tokens.length !== 1 ? "s" : ""}`
          )}
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
      </button>

      {/* ── Expandable content ── */}
      {!collapsed && (
        <div className="px-3 pb-3 max-h-[300px] overflow-y-auto">
          {isLoading ? (
            <div className="flex items-center gap-2 py-2">
              <Loader2 className="h-3.5 w-3.5 animate-spin text-muted-foreground/40" />
              <span className="text-xs text-muted-foreground/40">Loading tokens...</span>
            </div>
          ) : (
            <div className={cn("grid gap-2", gridCols)}>
              {tokens.map((t) => (
                <TokenGridCard
                  key={`${t.name}-${t.setCode ?? ""}-${t.cardNumber ?? ""}`}
                  token={t}
                  onShowInfo={onShowInfo}
                  onPickPrint={onPickPrint}
                />
              ))}
            </div>
          )}
        </div>
      )}
    </div>
  );
}

// ─── Mini pill shown in collapsed header ─────────────────────────────────────

function MiniTokenPill({ token }: { token: DeckToken }) {
  if (token.imageUrl) {
    return (
      <img
        src={token.imageUrl}
        alt={token.name}
        className="h-5 w-[14px] rounded-sm object-cover object-top border border-border/30"
        draggable={false}
      />
    );
  }
  // No stored image — use Scryfall fallback
  return <ScryFallbackPill />;
}

function ScryFallbackPill() {
  return <div className="h-5 w-[14px] rounded-sm bg-muted border border-border/30" />;
}

// ─── Grid card with token image + producer tooltip + print picker ───────────

function TokenGridCard({
  token,
  onShowInfo,
  onPickPrint,
}: {
  token: DeckToken;
  onShowInfo?: (name: string) => void;
  onPickPrint?: (name: string) => void;
}) {
  const [showProducers, setShowProducers] = useState(false);

  return (
    <div
      className="relative group cursor-pointer"
      onMouseEnter={() => setShowProducers(true)}
      onMouseLeave={() => setShowProducers(false)}
      onClick={() => onShowInfo?.(token.name)}
    >
      {/* Image: use stored URL directly, or fall back to Scryfall lookup */}
      {token.imageUrl ? (
        <img
          src={token.imageUrl}
          alt={token.name}
          className="w-full rounded-lg border border-border/50 shadow-sm"
          draggable={false}
        />
      ) : (
        <ScryFallbackImage
          name={token.name}
          className="w-full rounded-lg border border-border/50 shadow-sm"
        />
      )}

      {/* Pick print button — top-right on hover */}
      {onPickPrint && (
        <button
          type="button"
          className="absolute top-1 right-1 z-20 rounded-full p-0.5 shadow transition-colors bg-overlay/70 text-muted-foreground opacity-0 group-hover:opacity-100"
          title="Change printing"
          onClick={(e) => {
            e.stopPropagation();
            onPickPrint(token.name);
          }}
        >
          <Palette className="h-3.5 w-3.5" />
        </button>
      )}

      {/* Producer tooltip on hover */}
      {showProducers && token.producers.length > 0 && (
        <div className="absolute bottom-full left-1/2 -translate-x-1/2 mb-1 z-30 pointer-events-none">
          <div className="bg-popover/95 backdrop-blur-sm border border-border rounded-md px-2 py-1.5 shadow-lg whitespace-nowrap">
            <p className="text-[10px] font-semibold text-muted-foreground mb-0.5">Produced by:</p>
            {token.producers.map((p) => (
              <p key={p} className="text-[10px] text-foreground">
                {p}
              </p>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
