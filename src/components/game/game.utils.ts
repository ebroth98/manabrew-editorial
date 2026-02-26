import type { Card as CardType, StackObject } from "@/types/xmage";
import { AVATAR_COLORS, PROMPT_LABELS } from "./game.constants";

export function getAvatarColor(name: string): string {
  const hash = name.split("").reduce((acc, c) => acc + c.charCodeAt(0), 0);
  return AVATAR_COLORS[hash % AVATAR_COLORS.length];
}

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

export function isLethalDamage(card: CardType): boolean {
  if (!card.damage || !card.toughness) return false;
  const toughness = parseInt(card.toughness, 10);
  return !isNaN(toughness) && card.damage >= toughness;
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
