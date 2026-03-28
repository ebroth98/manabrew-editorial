// Mirrors Forge's DeckFormat (structural rules) + GameFormat (card legality).
// For our limited card pool, we combine both into a single GameFormat interface.

import type { Card } from "@/types/openmagic";
import type { CardIdentity, DeckSection } from "@/types/server";

export interface GameFormat {
  id: string;
  name: string;
  /** Short label used in badges, e.g. "Constr." / "EDH" */
  shortName: string;
  description: string;
  /** Tailwind color variant key for FormatBadge */
  badgeColor: string;
  deckRules: {
    minDeckSize: number;
    maxDeckSize: number | null; // null = unlimited
    maxCopies: number; // 4 for Constructed, 1 for Commander
    sideboardMax: number;
    startingLife: number;
    requiresCommander: boolean;
  };
  bannedCards: string[];
}

/** Basic land names are exempt from the max-copies rule. */
export const BASIC_LAND_NAMES = new Set([
  "Plains",
  "Island",
  "Swamp",
  "Mountain",
  "Forest",
]);

export const GAME_FORMATS: GameFormat[] = [
  {
    id: "constructed",
    name: "Constructed",
    shortName: "Constr.",
    description: "60+ cards, max 4 copies, 20 life",
    badgeColor: "blue",
    deckRules: {
      minDeckSize: 60,
      maxDeckSize: null,
      maxCopies: 4,
      sideboardMax: 15,
      startingLife: 20,
      requiresCommander: false,
    },
    bannedCards: [],
  },
  {
    id: "commander",
    name: "Commander",
    shortName: "EDH",
    description: "100 cards, singleton, 40 life",
    badgeColor: "purple",
    deckRules: {
      minDeckSize: 100,
      maxDeckSize: 100,
      maxCopies: 1,
      sideboardMax: 10,
      startingLife: 40,
      requiresCommander: true,
    },
    bannedCards: [],
  },
];

export interface DeckValidation {
  legal: boolean;
  errors: string[];
}

export interface DeckValidationInput {
  deckList: CardIdentity[];
  availableCards?: Card[];
  commanderName?: string;
}

export function getFormat(id: string): GameFormat | undefined {
  return GAME_FORMATS.find((f) => f.id === id);
}

/**
 * Validate a deck (as an array of card names, one per copy) against a format.
 * Basic lands are exempt from the per-card copy limit.
 */
export function validateDeck(
  cardNames: string[],
  format: GameFormat
): DeckValidation {
  const errors: string[] = [];
  const { minDeckSize, maxDeckSize, maxCopies } = format.deckRules;

  if (cardNames.length < minDeckSize) {
    errors.push(
      `Deck must have at least ${minDeckSize} cards (has ${cardNames.length})`
    );
  }
  if (maxDeckSize !== null && cardNames.length > maxDeckSize) {
    errors.push(
      `Deck must have at most ${maxDeckSize} cards (has ${cardNames.length})`
    );
  }

  // Count copies and check against limit
  const counts = new Map<string, number>();
  for (const name of cardNames) {
    counts.set(name, (counts.get(name) ?? 0) + 1);
  }
  for (const [name, count] of counts) {
    if (!BASIC_LAND_NAMES.has(name) && count > maxCopies) {
      errors.push(
        `Too many copies of "${name}": ${count} (max ${maxCopies})`
      );
    }
  }

  // Check banned list
  const seenBanned = new Set<string>();
  for (const name of cardNames) {
    if (format.bannedCards.includes(name) && !seenBanned.has(name)) {
      errors.push(`"${name}" is banned in ${format.name}`);
      seenBanned.add(name);
    }
  }

  return { legal: errors.length === 0, errors };
}

function isMainDeckSection(section?: DeckSection): boolean {
  return section === undefined || section === "main" || section === "commander";
}

function getCardByName(cards: Card[], name: string): Card | undefined {
  return cards.find((card) => card.name === name);
}

function getCardIdentity(card?: Card): string[] {
  if (!card) return [];
  if (card.colorIdentity && card.colorIdentity.length > 0) {
    return [...new Set(card.colorIdentity)];
  }
  return [...new Set((card.color ?? "").split("").filter(Boolean))];
}

function isCommanderEligible(card?: Card): boolean {
  if (!card) return false;
  const isLegendaryCreature =
    card.supertypes.includes("Legendary") && card.types.includes("Creature");
  if (isLegendaryCreature) return true;
  return card.text.toLowerCase().includes("can be your commander");
}

function normalizeCommanderSelection(
  deckList: CardIdentity[],
  commanderName?: string,
): CardIdentity[] {
  if (!commanderName) return deckList;
  const alreadyCommander = deckList.some(
    (card) => card.section === "commander" && card.name === commanderName,
  );
  if (alreadyCommander) return deckList;

  let promoted = false;
  return deckList.flatMap((card) => {
    if (!promoted && card.name === commanderName && isMainDeckSection(card.section)) {
      promoted = true;
      return [{ ...card, section: "commander" as const }];
    }
    return [card];
  });
}

export function validateDeckSections(
  input: DeckValidationInput,
  format: GameFormat,
): DeckValidation {
  const availableCards = input.availableCards ?? [];
  const effectiveDeck = normalizeCommanderSelection(input.deckList, input.commanderName);
  const errors: string[] = [];

  const mainDeck = effectiveDeck.filter((card) => isMainDeckSection(card.section));
  const sideboard = effectiveDeck.filter((card) => card.section === "sideboard");
  const commanders = effectiveDeck.filter((card) => card.section === "commander");
  const mainOnly = effectiveDeck.filter(
    (card) => card.section === undefined || card.section === "main",
  );

  const baseValidation = validateDeck(mainDeck.map((card) => card.name), format);
  errors.push(...baseValidation.errors);

  if (sideboard.length > format.deckRules.sideboardMax) {
    errors.push(
      `Sideboard must have at most ${format.deckRules.sideboardMax} cards (has ${sideboard.length})`,
    );
  }

  if (format.deckRules.requiresCommander) {
    if (commanders.length === 0) {
      errors.push("Deck must have exactly 1 commander");
    } else if (commanders.length > 1) {
      errors.push(`Deck must have exactly 1 commander (has ${commanders.length})`);
    }

    if (mainOnly.length !== format.deckRules.minDeckSize - 1) {
      errors.push(
        `Commander deck must have exactly ${format.deckRules.minDeckSize - 1} non-commander cards (has ${mainOnly.length})`,
      );
    }

    const commanderCard = getCardByName(availableCards, commanders[0]?.name ?? "");
    if (commanders.length === 1 && !isCommanderEligible(commanderCard)) {
      errors.push(`"${commanders[0]!.name}" is not a legal commander`);
    }

    const commanderIdentity = new Set(getCardIdentity(commanderCard));
    if (commanders.length === 1 && commanderIdentity.size > 0) {
      const invalidCards = mainOnly
        .map((identity) => getCardByName(availableCards, identity.name))
        .filter((card): card is Card => Boolean(card))
        .filter((card) =>
          getCardIdentity(card).some((color) => !commanderIdentity.has(color)),
        );
      if (invalidCards.length > 0) {
        errors.push(
          `Deck contains cards outside commander color identity: ${invalidCards[0]!.name}`,
        );
      }
    }
  }

  return { legal: errors.length === 0, errors };
}

/**
 * Returns all formats the deck is legal in.
 * Mirrors Forge's GameFormat.Collection.getAllFormatsOfDeck().
 */
export function inferFormats(cardNames: string[]): GameFormat[] {
  return GAME_FORMATS.filter((f) => validateDeck(cardNames, f).legal);
}

export function inferFormatsFromDeck(
  deckList: CardIdentity[],
  availableCards: Card[] = [],
): GameFormat[] {
  return GAME_FORMATS.filter((format) =>
    validateDeckSections({ deckList, availableCards }, format).legal,
  );
}
