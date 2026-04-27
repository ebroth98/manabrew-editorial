import type { Card, Deck } from "@/types/openmagic";

export interface PresetDeckCardEntry {
  name: string;
  count: number;
  set?: string;
}

export interface PresetDeckPayload {
  id: string;
  label: string;
  desc: string;
  color: string;
  coverCardName?: string;
  cards: PresetDeckCardEntry[];
}

function presetCardToCard(entry: PresetDeckCardEntry, presetId: string, index: number): Card {
  return {
    id: `preset:${presetId}:${index}:${entry.name}`,
    name: entry.name,
    setCode: entry.set ?? "",
    cardNumber: "",
    color: "",
    manaCost: "",
    types: [],
    subtypes: [],
    supertypes: [],
    text: "",
    isPlayable: true,
    isSelected: false,
    isChoosable: false,
    controllerId: "",
    ownerId: "",
    zoneId: "",
  };
}

export function presetDeckPayloadToDeck(preset: PresetDeckPayload): Deck {
  let index = 0;
  return {
    id: preset.id,
    name: preset.label,
    description: preset.desc,
    color: preset.color,
    format: "standard",
    coverCardName: preset.coverCardName,
    cards: preset.cards.flatMap((entry) =>
      Array.from({ length: entry.count }, () => presetCardToCard(entry, preset.id, index++)),
    ),
    sideboard: [],
  };
}

export function presetDeckPayloadsToDecks(presets: PresetDeckPayload[]): Deck[] {
  return presets.map(presetDeckPayloadToDeck);
}
