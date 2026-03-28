import type { Card } from "@/types/openmagic";
import type { ScryfallCard } from "@/types/scryfall";
import { getScryfallImageUrl, getScryfallManaCost } from "@/api/scryfall";

// ─── Constants ────────────────────────────────────────────────────────────────

export const MTG_SUPERTYPES = new Set([
  "Basic",
  "Legendary",
  "Snow",
  "World",
  "Ongoing",
]);

// ─── Type Line Parsing ────────────────────────────────────────────────────────

export interface ParsedTypeLine {
  supertypes: string[];
  types: string[];
  subtypes: string[];
}

export function parseTypeLine(typeLine: string): ParsedTypeLine {
  const [mainPart = "", subPart = ""] = typeLine.split("—").map((s) => s.trim());
  const mainTokens = mainPart.split(/\s+/).filter(Boolean);
  return {
    supertypes: mainTokens.filter((t) => MTG_SUPERTYPES.has(t)),
    types: mainTokens.filter((t) => !MTG_SUPERTYPES.has(t)),
    subtypes: subPart ? subPart.split(/\s+/).filter(Boolean) : [],
  };
}

// ─── ScryfallCard → XMage Card (full) ────────────────────────────────────────

const DEFAULT_CARD_FIELDS: Pick<
  Card,
  "isPlayable" | "isSelected" | "isChoosable" | "controllerId" | "ownerId" | "zoneId"
> = {
  isPlayable: true,
  isSelected: false,
  isChoosable: true,
  controllerId: "",
  ownerId: "",
  zoneId: "",
};

export function scryfallToXMage(sc: ScryfallCard, id?: string): Card {
  const { supertypes, types, subtypes } = parseTypeLine(sc.type_line);
  return {
    ...DEFAULT_CARD_FIELDS,
    id: id ?? crypto.randomUUID(),
    name: sc.name,
    setCode: sc.set,
    cardNumber: sc.collector_number,
    color: sc.colors ? sc.colors.join("") : "",
    colorIdentity: sc.color_identity ?? [],
    manaCost: getScryfallManaCost(sc) ?? "",
    cmc: sc.cmc,
    types,
    subtypes,
    supertypes,
    power: sc.power,
    toughness: sc.toughness,
    text: sc.oracle_text || "",
    imageUrl: getScryfallImageUrl(sc),
  };
}

// ─── ScryfallCard → Partial<Card> (for enrichment) ───────────────────────────

export function scryfallCardToPartial(sc: ScryfallCard): Partial<Card> {
  const { supertypes, types, subtypes } = parseTypeLine(sc.type_line);
  const imageUrl = getScryfallImageUrl(sc);
  return {
    manaCost: getScryfallManaCost(sc) ?? "",
    cmc: sc.cmc,
    types,
    subtypes,
    supertypes,
    color: (sc.colors ?? []).join(""),
    colorIdentity: sc.color_identity ?? [],
    power: sc.power,
    toughness: sc.toughness,
    setCode: sc.set,
    cardNumber: sc.collector_number,
    ...(imageUrl ? { imageUrl } : {}),
  };
}

// ─── Default empty Card ──────────────────────────────────────────────────────

export function createEmptyCard(name: string): Card {
  return {
    ...DEFAULT_CARD_FIELDS,
    id: crypto.randomUUID(),
    name,
    setCode: "",
    cardNumber: "",
    color: "",
    manaCost: "",
    types: [],
    subtypes: [],
    supertypes: [],
    text: "",
  };
}
