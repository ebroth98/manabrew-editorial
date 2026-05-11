import type { Card as CardType, StackObject } from "@/types/manabrew";
import { PROMPT_LABELS } from "./game.constants";

export function getInitials(name: string): string {
  return name
    .split(" ")
    .map((w) => w[0] ?? "")
    .join("")
    .toUpperCase()
    .slice(0, 2);
}

export function getPromptLabel(promptType?: string): string {
  if (!promptType) return "Waiting for your next decision";
  return PROMPT_LABELS[promptType] ?? promptType;
}

export function isCreature(card: CardType): boolean {
  return card.types?.some((t) => t.toLowerCase() === "creature") ?? false;
}

export function isLethalDamage(card: CardType, queuedDamage = 0): boolean {
  if (!card.toughness) return false;
  const total = (card.damage ?? 0) + queuedDamage;
  if (total <= 0) return false;
  const toughness = parseInt(card.toughness, 10);
  return !isNaN(toughness) && total >= toughness;
}

export type ScryfallImageSize = "small" | "normal" | "large" | "png";

/** Upgrade a Scryfall image URL to a higher resolution if it matches the Scryfall pattern. */
export function upgradeScryfallUrl(
  url: string | undefined,
  size: ScryfallImageSize,
): string | undefined {
  if (!url || !url.includes("cards.scryfall.io")) return url;
  // 1. Swap the size component (small/normal/large/png)
  const newUrl = url.replace(/\/(small|normal|large|png)\//, `/${size}/`);
  // 2. Swap extension if moving to/from PNG
  if (size === "png") {
    return newUrl.replace(/\.jpg(\?.*)?$/, ".png$1");
  } else {
    return newUrl.replace(/\.png(\?.*)?$/, ".jpg$1");
  }
}

export function stackObjectToCardStub(obj: StackObject): CardType {
  return {
    id: obj.sourceId,
    name: obj.name,
    setCode: "",
    cardNumber: "",
    color: "",
    manaCost: "",
    types: [],
    subtypes: [],
    supertypes: [],
    text: obj.text,
    isPlayable: false,
    isSelected: false,
    isChoosable: false,
    controllerId: "",
    ownerId: "",
    zoneId: "",
  };
}
