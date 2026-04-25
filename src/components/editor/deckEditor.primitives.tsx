/**
 * Shared primitive components for the deck editor views.
 * Extracted to eliminate duplication across DeckListView card components.
 */

import { Button } from "@/components/ui/button";
import { Plus, Minus, Tag, ChevronDown, ChevronRight } from "lucide-react";
import { cn } from "@/lib/utils";
import { useCardImage } from "@/hooks/useCardImage";
import type { LucideIcon } from "lucide-react";

// ─── Card Count Badge ────────────────────────────────────────────────────────

export function CardCountBadge({ count, className }: { count: number; className?: string }) {
  if (count <= 1) return null;
  return (
    <div
      className={cn(
        "absolute top-1 left-1 bg-overlay/80 text-white text-[10px] font-bold rounded-full w-5 h-5 flex items-center justify-center border border-white/20",
        className,
      )}
      style={{ zIndex: 10 }}
    >
      {count}
    </div>
  );
}

// ─── Card Thumbnail (image or fallback) ──────────────────────────────────────

export function CardThumbnail({
  imageUrl,
  name,
  className,
  fallbackClassName,
  fallbackStyle,
}: {
  imageUrl?: string;
  name: string;
  className?: string;
  fallbackClassName?: string;
  fallbackStyle?: React.CSSProperties;
}) {
  const { data: fetchedUrl } = useCardImage(name, imageUrl);
  const resolvedUrl = imageUrl ?? fetchedUrl ?? null;

  if (resolvedUrl) {
    return (
      <img
        src={resolvedUrl}
        alt={name}
        className={cn("w-full rounded-lg border border-border/50 shadow-sm", className)}
        draggable={false}
      />
    );
  }
  return (
    <div
      className={cn(
        "w-full aspect-[2.5/3.5] rounded-lg border border-border bg-muted flex items-center justify-center p-2",
        fallbackClassName,
      )}
      style={fallbackStyle}
    >
      <span className="text-[9px] text-muted-foreground leading-tight text-center">{name}</span>
    </div>
  );
}

// ─── Card Hover Overlay ──────────────────────────────────────────────────────

export interface OverlayAction {
  label: string;
  icon: LucideIcon;
  onClick: () => void;
  variant?: "primary" | "ghost";
}

export function CardHoverOverlay({
  actions,
  rounded = "rounded-lg",
  onMouseEnter,
  onMouseLeave,
}: {
  actions: OverlayAction[];
  rounded?: string;
  onMouseEnter?: (e: React.MouseEvent) => void;
  onMouseLeave?: (e: React.MouseEvent) => void;
}) {
  return (
    <div
      className={cn(
        "absolute inset-0 bg-overlay/60 opacity-0 group-hover:opacity-100 transition-opacity flex flex-col items-center justify-center gap-1 pointer-events-none group-hover:pointer-events-auto",
        rounded,
      )}
      onMouseEnter={onMouseEnter}
      onMouseLeave={onMouseLeave}
    >
      {actions.map((a) => {
        const Icon = a.icon;
        return (
          <Button
            key={a.label}
            size="sm"
            variant={a.variant === "primary" ? "secondary" : "ghost"}
            className={cn(
              "h-6 w-4/5 text-xs",
              a.variant !== "primary" && "text-white/80 hover:text-white hover:bg-white/10",
            )}
            onClick={(e) => {
              e.stopPropagation();
              a.onClick();
            }}
          >
            <Icon className="h-3 w-3 mr-1" /> {a.label}
          </Button>
        );
      })}
    </div>
  );
}

/** Build the standard Add + Remove/Untag actions array for card overlays. */
export function buildCardActions(
  onAddOne: () => void,
  onRemoveOne: () => void,
  onUntag?: () => void,
): OverlayAction[] {
  const actions: OverlayAction[] = [
    { label: "Add", icon: Plus, onClick: onAddOne, variant: "primary" },
  ];
  if (onUntag) {
    actions.push({ label: "Untag", icon: Tag, onClick: onUntag });
  } else {
    actions.push({ label: "Remove", icon: Minus, onClick: onRemoveOne });
  }
  return actions;
}

// ─── Card Click Handler ──────────────────────────────────────────────────────

export function handleCardClick(
  e: React.MouseEvent,
  cardName: string,
  onSelect?: (cardName: string, addToSelection: boolean) => void,
  onShowInfo?: () => void,
) {
  e.stopPropagation();
  if (e.shiftKey && onSelect) {
    onSelect(cardName, true);
  } else if (onShowInfo) {
    onShowInfo();
  }
}

// ─── Collapsible Section Header ──────────────────────────────────────────────

export function CollapsibleHeader({
  label,
  count,
  collapsed,
  onToggle,
  extraContent,
}: {
  label: string;
  count: number;
  collapsed: boolean;
  onToggle: () => void;
  extraContent?: React.ReactNode;
}) {
  return (
    <div className="flex items-center gap-1 mb-1.5">
      <button
        type="button"
        className="flex items-center gap-1 flex-1 text-left hover:text-foreground text-muted-foreground"
        onClick={onToggle}
      >
        {collapsed ? (
          <ChevronRight className="h-3 w-3 shrink-0" />
        ) : (
          <ChevronDown className="h-3 w-3 shrink-0" />
        )}
        <span className="text-xs font-semibold uppercase tracking-wide">{label}</span>
        <span className="text-xs text-muted-foreground/60 ml-1">({count})</span>
      </button>
      {extraContent}
    </div>
  );
}

// ─── Empty Drop Zone ─────────────────────────────────────────────────────────

export function EmptyDropZone({ message = "Drop cards here" }: { message?: string }) {
  return (
    <div className="border border-dashed border-border/40 rounded py-3 text-center">
      <p className="text-[10px] text-muted-foreground/40">{message}</p>
    </div>
  );
}
