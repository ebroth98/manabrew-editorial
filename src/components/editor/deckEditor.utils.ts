import type { LucideIcon } from "lucide-react";
import { Plus, Minus, Tag } from "lucide-react";
import type React from "react";

export interface OverlayAction {
  label: string;
  icon: LucideIcon;
  onClick: () => void;
  variant?: "primary" | "ghost";
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
