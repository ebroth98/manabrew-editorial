import type { Card } from "@/types/openmagic";
import type { ScryfallCard } from "@/types/scryfall";
import { getScryfallManaCost } from "@/api/scryfall";
import { chooseImageUrisForCard } from "@/stores/useScryfallStore";

// ─── Constants ────────────────────────────────────────────────────────────────

export const MTG_SUPERTYPES = new Set(["Basic", "Legendary", "Snow", "World", "Ongoing"]);

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

// ─── ScryfallCard → OpenMagic Card (full) ────────────────────────────────────────

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

/** Get the front-face type line, handling DFCs where type_line lives on card_faces. */
function getFrontTypeLine(sc: ScryfallCard): string {
  if (sc.type_line) return sc.type_line.split("//")[0].trim();
  return sc.card_faces?.[0]?.type_line ?? "";
}

/** Get the front-face oracle text, handling DFCs. */
function getFrontOracleText(sc: ScryfallCard): string {
  if (sc.oracle_text) return sc.oracle_text;
  return sc.card_faces?.[0]?.oracle_text ?? "";
}

/** True when a Scryfall card has two separate illustrated faces (transform, modal DFC, etc.). */
function detectIsDoubleFaced(sc: ScryfallCard): boolean {
  return !!(sc.card_faces && sc.card_faces.length >= 2 && sc.card_faces[1]?.image_uris);
}

export function scryfallToOpenMagic(sc: ScryfallCard, id?: string): Card {
  const { supertypes, types, subtypes } = parseTypeLine(getFrontTypeLine(sc));
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
    text: getFrontOracleText(sc),
    imageUrl: chooseImageUrisForCard(sc, { frontOnly: true })?.normal,
    isDoubleFaced: detectIsDoubleFaced(sc) || undefined,
    layout: sc.layout || undefined,
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
