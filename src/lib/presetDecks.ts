import type { Card, Deck, DeckFormatId } from "@/types/openmagic";

export interface PresetDeckCardEntry {
  name: string;
  count: number;
  set?: string;
  cardNumber?: string;
  manaCost?: string;
  colors?: string[];
  colorIdentity?: string[];
  cmc?: number;
  types?: string[];
  subtypes?: string[];
  supertypes?: string[];
  text?: string;
  imageUrl?: string;
  layout?: string;
  power?: string;
  toughness?: string;
}

export interface PresetDeckPayload {
  id: string;
  label: string;
  desc: string;
  color: string;
  format?: DeckFormatId;
  commander?: string;
  coverCardName?: string;
  cards: PresetDeckCardEntry[];
}

function presetCardToCard(entry: PresetDeckCardEntry, presetId: string, index: number): Card {
  return {
    id: `preset:${presetId}:${index}:${entry.name}`,
    name: entry.name,
    setCode: entry.set ?? "",
    cardNumber: entry.cardNumber ?? "",
    color: entry.colors ? entry.colors.join("") : "",
    colorIdentity: entry.colorIdentity ?? [],
    manaCost: entry.manaCost ?? "",
    cmc: entry.cmc,
    types: entry.types ?? [],
    subtypes: entry.subtypes ?? [],
    supertypes: entry.supertypes ?? [],
    power: entry.power,
    toughness: entry.toughness,
    text: entry.text ?? "",
    imageUrl: entry.imageUrl,
    layout: entry.layout,
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
  const cards = preset.cards.flatMap((entry) =>
    Array.from({ length: entry.count }, () => presetCardToCard(entry, preset.id, index++)),
  );
  // Commander goes in `commanders[]`, not the main 99 — strip it out of cards.
  let commanders: Card[] | undefined;
  if (preset.commander) {
    const commanderEntry: PresetDeckCardEntry = { name: preset.commander, count: 1 };
    commanders = [presetCardToCard(commanderEntry, preset.id, index++)];
    const cmdIdx = cards.findIndex((c) => c.name === preset.commander);
    if (cmdIdx !== -1) cards.splice(cmdIdx, 1);
  }
  return {
    id: preset.id,
    name: preset.label,
    description: preset.desc,
    color: preset.color,
    format: preset.format ?? "standard",
    coverCardName: preset.coverCardName ?? preset.commander,
    cards,
    sideboard: [],
    commanders,
  };
}

export function presetDeckPayloadsToDecks(presets: PresetDeckPayload[]): Deck[] {
  return presets.map(presetDeckPayloadToDeck);
}
