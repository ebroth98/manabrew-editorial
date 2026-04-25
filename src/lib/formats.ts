// Mirrors Forge's DeckFormat (structural rules) + GameFormat (card legality).
// For our limited card pool, we combine both into a single GameFormat interface.

import type { Card } from "@/types/openmagic";
import type { CardIdentity, DeckSection } from "@/types/server";

export interface GameFormat {
  id: string;
  name: string;
  /** Short label used in badges, e.g. "STD" / "CMD" */
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
export const BASIC_LAND_NAMES = new Set(["Plains", "Island", "Swamp", "Mountain", "Forest"]);

/**
 * Returns true when a card's oracle text explicitly declares that a deck may
 * contain any number of copies (e.g. Relentless Rats, Shadowborn Apostle,
 * Dragon's Approach, Rat Colony …).
 *
 * Matches phrases like:
 *   "A deck can have any number of cards named …"
 *   "You may have any number of cards named …"
 */
export function allowsAnyNumberOfCopies(oracleText: string | undefined): boolean {
  if (!oracleText) return false;
  return /any number of cards named/i.test(oracleText);
}

export const GAME_FORMATS: GameFormat[] = [
  // ── 60-card Constructed formats ─────────────────────────────────────
  {
    id: "standard",
    name: "Standard",
    shortName: "STD",
    description: "60+ cards, max 4 copies, 20 life, rotating sets",
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
    id: "pioneer",
    name: "Pioneer",
    shortName: "PIO",
    description: "60+ cards, max 4 copies, 20 life, Return to Ravnica forward",
    badgeColor: "amber",
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
    id: "modern",
    name: "Modern",
    shortName: "MOD",
    description: "60+ cards, max 4 copies, 20 life, 8th Edition forward",
    badgeColor: "emerald",
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
    id: "legacy",
    name: "Legacy",
    shortName: "LEG",
    description: "60+ cards, max 4 copies, 20 life, all sets, banned list",
    badgeColor: "rose",
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
    id: "vintage",
    name: "Vintage",
    shortName: "VIN",
    description: "60+ cards, max 4 copies, 20 life, all sets, restricted list",
    badgeColor: "slate",
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
    id: "pauper",
    name: "Pauper",
    shortName: "PAU",
    description: "60+ cards, max 4 copies, 20 life, commons only",
    badgeColor: "zinc",
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
  // ── Singleton / Commander variants ──────────────────────────────────
  {
    id: "commander",
    name: "Commander",
    shortName: "CMD",
    description: "100 cards, singleton, 40 life, requires commander",
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
  {
    id: "brawl",
    name: "Brawl",
    shortName: "BRL",
    description: "60 cards, singleton, 25 life, Standard-legal commander",
    badgeColor: "teal",
    deckRules: {
      minDeckSize: 60,
      maxDeckSize: 60,
      maxCopies: 1,
      sideboardMax: 0,
      startingLife: 25,
      requiresCommander: true,
    },
    bannedCards: [],
  },
  {
    id: "oathbreaker",
    name: "Oathbreaker",
    shortName: "OAT",
    description: "60 cards, singleton, 20 life, planeswalker commander",
    badgeColor: "orange",
    deckRules: {
      minDeckSize: 60,
      maxDeckSize: 60,
      maxCopies: 1,
      sideboardMax: 0,
      startingLife: 20,
      requiresCommander: true,
    },
    bannedCards: [],
  },
  // ── Limited formats ─────────────────────────────────────────────────
  {
    id: "draft",
    name: "Draft",
    shortName: "DFT",
    description: "40+ cards, no copy limit, 20 life",
    badgeColor: "sky",
    deckRules: {
      minDeckSize: 40,
      maxDeckSize: null,
      maxCopies: Infinity,
      sideboardMax: Infinity,
      startingLife: 20,
      requiresCommander: false,
    },
    bannedCards: [],
  },
  {
    id: "sealed",
    name: "Sealed",
    shortName: "SLD",
    description: "40+ cards, no copy limit, 20 life",
    badgeColor: "indigo",
    deckRules: {
      minDeckSize: 40,
      maxDeckSize: null,
      maxCopies: Infinity,
      sideboardMax: Infinity,
      startingLife: 20,
      requiresCommander: false,
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
 * Basic lands and cards whose text explicitly allows any number of copies are
 * exempt from the per-card copy limit.
 *
 * @param anyNumberNames - optional set of card names that carry the
 *   "any number of copies" exemption (derived from oracle text by the caller).
 */
export function validateDeck(
  cardNames: string[],
  format: GameFormat,
  anyNumberNames?: ReadonlySet<string>,
): DeckValidation {
  const errors: string[] = [];
  const { minDeckSize, maxDeckSize, maxCopies } = format.deckRules;

  if (cardNames.length < minDeckSize) {
    errors.push(`Deck must have at least ${minDeckSize} cards (has ${cardNames.length})`);
  }
  if (maxDeckSize !== null && cardNames.length > maxDeckSize) {
    errors.push(`Deck must have at most ${maxDeckSize} cards (has ${cardNames.length})`);
  }

  // Count copies and check against limit
  const counts = new Map<string, number>();
  for (const name of cardNames) {
    counts.set(name, (counts.get(name) ?? 0) + 1);
  }
  for (const [name, count] of counts) {
    if (!BASIC_LAND_NAMES.has(name) && !anyNumberNames?.has(name) && count > maxCopies) {
      errors.push(`Too many copies of "${name}": ${count} (max ${maxCopies})`);
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

// ─── Partner utilities ───────────────────────────────────────────────────────

/** Returns true if the card has the generic "Partner" keyword (not "Partner with"). */
export function hasPartner(card?: Card): boolean {
  if (!card) return false;
  if (card.keywords?.some((k) => /^partner$/i.test(k.trim()))) return true;
  return /(?:^|\n)partner(?!\s+with)/im.test(card.text);
}

/**
 * Returns the specific partner name this card must pair with ("Partner with Xxx"),
 * or null if it doesn't have the specific-partner ability.
 */
export function getPartnerWithName(card?: Card): string | null {
  if (!card) return null;
  const fromKeywords = card.keywords?.find((k) => /^partner with /i.test(k));
  if (fromKeywords) return fromKeywords.replace(/^partner with /i, "").trim();
  const match = card.text.match(/partner with ([^\n(]+)/i);
  return match ? match[1].trim() : null;
}

function hasFriendsForever(card?: Card): boolean {
  if (!card) return false;
  if (card.keywords?.some((k) => /^friends forever$/i.test(k.trim()))) return true;
  return /friends forever/i.test(card.text);
}

function hasChooseBackground(card?: Card): boolean {
  if (!card) return false;
  return card.text.toLowerCase().includes("choose a background");
}

function isBackgroundCard(card?: Card): boolean {
  if (!card) return false;
  return card.subtypes?.some((s) => s.toLowerCase() === "background") ?? false;
}

/**
 * Returns true if two cards are a legal pair of partner commanders.
 * Handles: generic Partner, "Partner with [Name]", Friends forever, and Background.
 */
export function canBePartners(a: Card, b: Card): boolean {
  // Generic Partner: both must have Partner
  if (hasPartner(a) && hasPartner(b)) return true;
  // Friends forever: both must have Friends forever
  if (hasFriendsForever(a) && hasFriendsForever(b)) return true;
  // Partner with: each must specifically name the other
  const pwA = getPartnerWithName(a);
  const pwB = getPartnerWithName(b);
  if (
    pwA &&
    pwB &&
    pwA.toLowerCase() === b.name.toLowerCase() &&
    pwB.toLowerCase() === a.name.toLowerCase()
  )
    return true;
  // Background: one has "Choose a Background" text, the other is a Background
  if (hasChooseBackground(a) && isBackgroundCard(b)) return true;
  if (hasChooseBackground(b) && isBackgroundCard(a)) return true;
  return false;
}

export function isCommanderEligible(card?: Card): boolean {
  if (!card) return false;
  const isLegendary = card.supertypes.includes("Legendary");
  if (isLegendary && card.types.includes("Creature")) return true;
  if (
    isLegendary &&
    card.subtypes?.some((s) => ["vehicle", "spacecraft"].includes(s.toLowerCase()))
  )
    return true;
  // Also allow legendary planeswalkers that say "can be your commander"
  // and backgrounds (for "choose a background")
  const hasCommanderText = card.text.toLowerCase().includes("can be your commander");
  if (hasCommanderText) return true;
  const isBackground = card.subtypes?.some((s) => s.toLowerCase() === "background") ?? false;
  if (isBackground) return true;
  return false;
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

  // Build the set of card names whose text allows unlimited copies
  const anyNumberNames = new Set(
    availableCards.filter((c) => allowsAnyNumberOfCopies(c.text)).map((c) => c.name),
  );

  const baseValidation = validateDeck(
    mainDeck.map((card) => card.name),
    format,
    anyNumberNames,
  );
  errors.push(...baseValidation.errors);

  if (sideboard.length > format.deckRules.sideboardMax) {
    errors.push(
      `Sideboard must have at most ${format.deckRules.sideboardMax} cards (has ${sideboard.length})`,
    );
  }

  if (format.deckRules.requiresCommander) {
    if (commanders.length === 0) {
      errors.push("Deck must have at least 1 commander");
    } else if (commanders.length > 2) {
      errors.push(`Deck can have at most 2 commanders (has ${commanders.length})`);
    }

    const expectedMainSize = format.deckRules.minDeckSize - commanders.length;
    if (mainOnly.length !== expectedMainSize) {
      errors.push(
        `Commander deck must have exactly ${expectedMainSize} non-commander cards (has ${mainOnly.length})`,
      );
    }

    // Validate each commander's eligibility
    for (const cmd of commanders) {
      const commanderCard = getCardByName(availableCards, cmd.name);
      if (!isCommanderEligible(commanderCard)) {
        errors.push(`"${cmd.name}" is not a legal commander`);
      }
    }

    // Validate partner legality when there are 2 commanders
    if (commanders.length === 2) {
      const cmd1 = getCardByName(availableCards, commanders[0].name);
      const cmd2 = getCardByName(availableCards, commanders[1].name);
      if (cmd1 && cmd2 && !canBePartners(cmd1, cmd2)) {
        errors.push(
          `"${commanders[0].name}" and "${commanders[1].name}" cannot be paired — both commanders must have a compatible partner ability`,
        );
      }
    }

    // Combined color identity of all commanders
    const commanderIdentity = new Set(
      commanders.flatMap((cmd) => getCardIdentity(getCardByName(availableCards, cmd.name))),
    );
    if (commanderIdentity.size > 0) {
      const invalidCards = mainOnly
        .map((identity) => getCardByName(availableCards, identity.name))
        .filter((card): card is Card => Boolean(card))
        .filter((card) => getCardIdentity(card).some((color) => !commanderIdentity.has(color)));
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
  return GAME_FORMATS.filter(
    (format) => validateDeckSections({ deckList, availableCards }, format).legal,
  );
}
