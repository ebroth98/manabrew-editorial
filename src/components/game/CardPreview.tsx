import { createPortal } from "react-dom";
import { Loader2 } from "lucide-react";
import type { Card } from "@/types/openmagic";
import { CounterDisplay } from "@/components/game/CounterBadge";
import { ManaSymbols } from "@/components/game/ManaSymbols";
import { TextWithMana } from "@/components/game/TextWithMana";
import { FLASH_CARD_SIZE } from "./game.styles";
import { withAlpha } from "@/themes/gameTheme";
import { useTheme } from "@/hooks/useTheme";
import { upgradeScryfallUrl } from "./game.utils";
import { cn } from "@/lib/utils";
import type { HandActionOption } from "@/stores/useGameUIStore";
import { useEffect } from "react";
import type { CSSProperties } from "react";
import { useCard } from "@/stores/useScryfallStore";

interface CardPreviewProps {
  card: Card;
  mouseX: number;
  mouseY: number;
  anchorRect?: DOMRect | null;
  placement?: "auto" | "top-center";
  showBackFace?: boolean;
  /** Available actions for this card (cast options + activated abilities). */
  actions?: HandActionOption[];
  /** Called when the user selects an action from the preview. */
  onSelectAction?: (action: HandActionOption) => void;
  /** Called to dismiss the preview. */
  onDismiss?: () => void;
  onMouseEnter?: () => void;
  onMouseLeave?: () => void;
  isSticky?: boolean;
}

const { w: CARD_W, h: CARD_H } = FLASH_CARD_SIZE;
const ACTIONS_PANEL_W = 220;

/**
 * Floating card preview rendered into document.body via portal.
 * Positions itself near the cursor or an anchor element, clamped to viewport edges.
 * When actions are available the preview becomes interactive and locks in place
 * until the user clicks an action, presses Escape, or clicks outside.
 */
export function CardPreview({
  card,
  mouseX,
  mouseY,
  anchorRect,
  placement = "auto",
  showBackFace = false,
  actions,
  onSelectAction,
  onDismiss,
  onMouseEnter,
  onMouseLeave,
  isSticky = false,
}: CardPreviewProps) {
  const hasActions = actions && actions.length > 0 && onSelectAction;
  const themeColors = useTheme().gameTheme;
  const ringColor = themeColors.cardRing; // matches battlefield playable color
  const cardD = useCard(card);
  const isLoading = !card.imageUrl && !cardD;
  const imageUrl = upgradeScryfallUrl(card.imageUrl ?? cardD?.uris.large, "large");
  const frontFace = cardD?.info?.card_faces?.[0];
  const backFace = cardD?.info?.card_faces?.[1];
  const doubleFacedData = {
    frontImageUrl: frontFace?.image_uris?.large ?? frontFace?.image_uris?.normal ?? null,
    backImageUrl: backFace?.image_uris?.large ?? backFace?.image_uris?.normal ?? null,
    frontName: frontFace?.name,
    backName: backFace?.name,
  };

  // Dismiss on Escape, outside click, or number key shortcut
  useEffect(() => {
    if (!hasActions || !onDismiss) return;
    function handleKey(e: KeyboardEvent) {
      if (e.key === "Escape") {
        onDismiss!();
        return;
      }
      // Number keys 1-9 activate the corresponding action
      const num = parseInt(e.key);
      if (num >= 1 && num <= actions!.length) {
        e.preventDefault();
        onSelectAction!(actions![num - 1]);
      }
    }
    function handleClick(e: MouseEvent) {
      const target = e.target as HTMLElement;
      if (!target.closest("[data-card-preview]")) {
        onDismiss!();
      }
    }
    window.addEventListener("keydown", handleKey);
    const timer = setTimeout(() => {
      if (isSticky) {
        window.addEventListener("mousedown", handleClick);
      }
    }, 100);
    return () => {
      window.removeEventListener("keydown", handleKey);
      clearTimeout(timer);
      window.removeEventListener("mousedown", handleClick);
    };
  }, [hasActions, isSticky, onDismiss, onSelectAction, actions]);

  // Position the card preview
  const cardWidth = CARD_W;
  const cardHeight = CARD_H;

  let cardLeft: number;
  let top: number;
  let actionsOnRight: boolean;

  if (placement === "top-center" && anchorRect) {
    cardLeft = anchorRect.left + anchorRect.width / 2 - cardWidth / 2;
    top = anchorRect.top - cardHeight - 12;
    // Clamp to screen
    cardLeft = Math.max(8, Math.min(cardLeft, window.innerWidth - cardWidth - 8));
    top = Math.max(8, top);

    const spaceAfterCard = window.innerWidth - (cardLeft + cardWidth);
    actionsOnRight = spaceAfterCard >= ACTIONS_PANEL_W + 16;
  } else {
    // Use anchorRect if available, otherwise fallback to mouse coordinates
    const anchorX = anchorRect ? anchorRect.right : mouseX;
    const anchorY = anchorRect ? anchorRect.top + anchorRect.height / 2 : mouseY;

    const spaceRight = window.innerWidth - anchorX;
    cardLeft =
      spaceRight > cardWidth + 24
        ? anchorX + 16
        : (anchorRect ? anchorRect.left : mouseX) - cardWidth - 16;

    const spaceAfterCard = window.innerWidth - (cardLeft + cardWidth);
    actionsOnRight = spaceAfterCard >= ACTIONS_PANEL_W + 16;

    top = Math.min(Math.max(anchorY - cardHeight / 2, 8), window.innerHeight - cardHeight - 8);
  }

  const hasDoubleFace = !!card.isDoubleFaced && !!doubleFacedData?.backImageUrl;
  const currentImageUrl =
    hasDoubleFace && showBackFace ? doubleFacedData.backImageUrl : imageUrl || cardD?.uris.large;
  const currentCardName =
    hasDoubleFace && showBackFace
      ? doubleFacedData.backName
      : hasDoubleFace && !showBackFace
        ? doubleFacedData.frontName
        : card.name;

  return createPortal(
    <>
      {/* Backdrop dim when interactive */}
      {hasActions && isSticky && (
        <div
          className="fixed inset-0 z-[9998] bg-black/30 animate-in fade-in duration-150"
          onClick={onDismiss}
        />
      )}
      <div
        data-card-preview
        className="fixed z-[9999] select-none pointer-events-auto"
        style={{ left: cardLeft, top }}
        onMouseEnter={onMouseEnter}
        onMouseLeave={onMouseLeave}
      >
        <div className="relative" style={{ width: cardWidth, height: cardHeight }}>
          {/* Card image */}
          <div
            className={cn(
              "w-full h-full rounded-xl shadow-2xl overflow-hidden bg-black transition-shadow duration-200",
              hasActions ? "ring-2" : "ring-1 ring-black/20",
            )}
            style={
              hasActions
                ? ({
                    "--tw-ring-color": ringColor,
                    boxShadow: `0 0 20px ${ringColor}`,
                  } as CSSProperties)
                : undefined
            }
          >
            {isLoading && !currentImageUrl ? (
              <div className="w-full h-full flex flex-col items-center justify-center gap-2 p-4">
                <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
                <span className="text-xs text-muted-foreground text-center">{currentCardName}</span>
              </div>
            ) : currentImageUrl ? (
              <>
                <img
                  src={currentImageUrl}
                  alt={currentCardName}
                  title=""
                  className="w-full h-full object-cover"
                />
                {card.counters && (
                  <CounterDisplay
                    counters={card.counters}
                    size="md"
                    className="absolute bottom-2 left-2 z-10"
                  />
                )}
              </>
            ) : (
              <div className="w-full h-full p-4 flex flex-col gap-2 bg-card">
                <div className="flex justify-between items-start">
                  <span className="font-bold text-sm leading-tight">{currentCardName}</span>
                  {!hasDoubleFace &&
                    (card.effectiveManaCost ? (
                      <div className="flex flex-col items-end">
                        <span className="line-through opacity-50">
                          <ManaSymbols cost={card.manaCost} size="md" />
                        </span>
                        <span
                          className="rounded px-0.5"
                          style={{ backgroundColor: withAlpha(ringColor, 0.2) }}
                        >
                          <ManaSymbols cost={card.effectiveManaCost} size="md" />
                        </span>
                      </div>
                    ) : (
                      <ManaSymbols cost={card.manaCost} size="md" />
                    ))}
                </div>
                {!hasDoubleFace && (
                  <div className="text-xs text-muted-foreground">{card.types?.join(" ")}</div>
                )}
                <div className="flex-1 text-xs text-foreground/80 whitespace-pre-wrap">
                  {hasDoubleFace && showBackFace
                    ? `Back face: ${doubleFacedData!.backName}`
                    : hasDoubleFace && !showBackFace
                      ? `Front face: ${doubleFacedData!.frontName}`
                      : card.text}
                </div>
                {card.counters && <CounterDisplay counters={card.counters} size="md" />}
                {card.power && card.toughness && (
                  <div className="text-right font-bold text-sm">
                    {card.power}/{card.toughness}
                  </div>
                )}
              </div>
            )}
          </div>

          {/* Actions panel */}
          {hasActions && (
            <div
              className="absolute top-0 flex flex-col gap-1.5"
              style={
                actionsOnRight
                  ? { left: cardWidth + 10, width: ACTIONS_PANEL_W }
                  : { right: cardWidth + 10, width: ACTIONS_PANEL_W }
              }
            >
              {/* Curved invisible bridge to maintain hover without blocking cards below */}
              <div
                style={{
                  position: "absolute",
                  top: 0,
                  left: actionsOnRight ? -10 - cardWidth : 0,
                  width: cardWidth + 10 + ACTIONS_PANEL_W,
                  height: cardHeight,
                  backgroundColor: "transparent",
                  borderBottomRightRadius: actionsOnRight ? "100%" : "0",
                  borderBottomLeftRadius: actionsOnRight ? "0" : "100%",
                  zIndex: -1,
                }}
              />

              {actions.map((action, idx) => (
                <button
                  key={idx}
                  onClick={() => onSelectAction(action)}
                  className={cn(
                    "group w-full text-left rounded-lg text-xs font-medium",
                    "bg-popover text-popover-foreground border border-border",
                    "backdrop-blur-md shadow-lg",
                    "transition-all duration-150 ease-out",
                    "hover:scale-[1.02] hover:-translate-y-px hover:shadow-xl",
                    "flex flex-col px-3 py-2",
                  )}
                  onMouseEnter={(e) => {
                    e.currentTarget.style.backgroundColor = withAlpha(ringColor, 0.12);
                    e.currentTarget.style.borderColor = ringColor;
                  }}
                  onMouseLeave={(e) => {
                    e.currentTarget.style.backgroundColor = "";
                    e.currentTarget.style.borderColor = "";
                  }}
                >
                  <span className="flex items-center justify-between w-full mb-0.5">
                    <span className="text-xs font-bold min-w-[22px] h-5 flex items-center justify-center rounded border border-border bg-muted shadow-[0_1px_0_rgba(0,0,0,0.1)]">
                      {idx + 1}
                    </span>
                    {action.cost && (
                      <span className="flex items-center gap-0.5 text-[11px] opacity-90">
                        <TextWithMana text={action.cost} manaSize="sm" />
                      </span>
                    )}
                  </span>
                  <span className="leading-snug text-[13px] font-semibold">
                    <TextWithMana text={action.label} manaSize="sm" />
                  </span>
                </button>
              ))}
            </div>
          )}
        </div>
      </div>
    </>,
    document.body,
  );
}
