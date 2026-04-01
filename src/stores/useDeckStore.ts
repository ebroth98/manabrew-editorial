import { create } from 'zustand';
import { persist } from 'zustand/middleware';
import type { Deck, Card, DeckFormatId } from '@/types/openmagic';
import { STORAGE_KEYS, DEFAULT_DECK_NAME } from '@/lib/constants';
import { getFormat, BASIC_LAND_NAMES, canBePartners, allowsAnyNumberOfCopies } from '@/lib/formats';

/** Apply a map of name→patch to an array of cards. */
function patchCardsByName(cards: Card[], updates: Map<string, Partial<Card>>): Card[] {
  return cards.map((c) => {
    const patch = updates.get(c.name.toLowerCase());
    return patch ? { ...c, ...patch } : c;
  });
}

function isAttractionCard(card: Card): boolean {
  return card.subtypes?.some((subtype) => subtype.toLowerCase() === 'attraction') ?? false;
}

function isContraptionCard(card: Card): boolean {
  return card.subtypes?.some((subtype) => subtype.toLowerCase() === 'contraption') ?? false;
}

function isSchemeCard(card: Card): boolean {
  return card.types?.some((type) => type.toLowerCase() === 'scheme') ?? false;
}

function isPlaneCard(card: Card): boolean {
  return card.types?.some((type) => type.toLowerCase() === 'plane') ?? false;
}

function normalizeDeck(deck: Deck): Deck {
  const main = [...(deck.cards ?? [])];
  const sideboard = [...(deck.sideboard ?? [])];
  const attractions = [...(deck.attractions ?? [])];
  const contraptions = [...(deck.contraptions ?? [])];
  const schemes = [...(deck.schemes ?? [])];
  const planes = [...(deck.planes ?? [])];
  // Migrate legacy single-commander to commanders array
  const commanders = [...(deck.commanders ?? [])];
  const legacy = (deck as { commander?: Card }).commander;
  if (legacy && !commanders.some((c) => c.name === legacy.name)) {
    commanders.push(legacy);
  }

  for (const cmd of commanders) {
    const idx = main.findIndex((card) => card.name === cmd.name);
    if (idx !== -1) main.splice(idx, 1);
  }

  const remainingSideboard: Card[] = [];
  for (const card of sideboard) {
    if (isAttractionCard(card)) {
      attractions.push(card);
    } else if (isContraptionCard(card)) {
      contraptions.push(card);
    } else if (isSchemeCard(card)) {
      schemes.push(card);
    } else if (isPlaneCard(card)) {
      planes.push(card);
    } else {
      remainingSideboard.push(card);
    }
  }

  const normalized: Deck = {
    ...deck,
    format: deck.format ?? (commanders.length > 0 ? 'commander' : 'constructed'),
    cards: main,
    sideboard: remainingSideboard,
    attractions,
    contraptions,
    schemes,
    planes,
    commanders: commanders.length > 0 ? commanders : undefined,
  };
  // Remove legacy field
  delete (normalized as { commander?: Card }).commander;
  return normalized;
}

function patchDeckCards(deck: Deck, updates: Map<string, Partial<Card>>): Deck {
  const normalized = normalizeDeck(deck);
  return {
    ...normalized,
    cards: patchCardsByName(normalized.cards, updates),
    sideboard: patchCardsByName(normalized.sideboard, updates),
    attractions: patchCardsByName(normalized.attractions ?? [], updates),
    contraptions: patchCardsByName(normalized.contraptions ?? [], updates),
    schemes: patchCardsByName(normalized.schemes ?? [], updates),
    planes: patchCardsByName(normalized.planes ?? [], updates),
    commanders: normalized.commanders ? patchCardsByName(normalized.commanders, updates) : undefined,
    companion: normalized.companion
      ? { ...normalized.companion, ...(updates.get(normalized.companion.name.toLowerCase()) ?? {}) }
      : undefined,
    maybeboard: normalized.maybeboard ? patchCardsByName(normalized.maybeboard, updates) : undefined,
  };
}

export interface SavedDeck {
  id: string;
  deck: Deck;
  savedAt: number;
}

interface DeckState {
  currentDeck: Deck;
  /** ID of the saved deck currently being edited (null if new/unsaved). */
  currentDeckId: string | null;
  pool: Card[];
  savedDecks: SavedDeck[];
  addToMain: (card: Card) => void;
  addToSide: (card: Card) => void;
  addToMaybe: (card: Card) => void;
  removeFromMaybe: (cardId: string) => void;
  addToPool: (card: Card) => void;
  removeFromMain: (cardId: string) => void;
  removeFromSide: (cardId: string) => void;
  setDeckName: (name: string) => void;
  setDeckFormat: (format: DeckFormatId) => void;
  clearDeck: () => void;
  loadDeck: (deck: Deck) => void;
  saveCurrentDeck: () => void;
  saveDraft: () => void;
  loadSavedDeck: (id: string) => void;
  deleteSavedDeck: (id: string) => void;
  setCommander: (card: Card) => void;
  removeCommander: (card?: Card) => void;
  setCompanion: (card: Card) => void;
  removeCompanion: () => void;
  updatePrint: (cardName: string, scryfallCard: import('@/types/scryfall').ScryfallCard) => void;
  /** Patch cards in currentDeck by name with enriched data from Scryfall. */
  enrichDeckCards: (updates: Map<string, Partial<Card>>) => void;
  /** Add a card to a saved deck's main board. */
  addCardToSavedDeck: (id: string, card: Card) => void;
  /** Patch cards in a specific saved deck by name with enriched data from Scryfall. */
  enrichSavedDeck: (id: string, updates: Map<string, Partial<Card>>) => void;
  /** Add a custom tag/section to the current deck. */
  addCustomTag: (tag: string) => void;
  /** Remove a custom tag/section and all card associations. */
  removeCustomTag: (tag: string) => void;
  /** Assign a tag to a card (by name). */
  tagCard: (cardName: string, tag: string) => void;
  /** Remove a tag from a card (by name). */
  untagCard: (cardName: string, tag: string) => void;
  /** Add a label to the current deck. */
  addDeckLabel: (label: string, color?: string) => void;
  /** Remove a label from the current deck. */
  removeDeckLabel: (label: string) => void;
  /** Update the color of an existing label. */
  updateDeckLabelColor: (label: string, color?: string) => void;
  /** Set the card whose art is displayed as the deck cover. Pass undefined to clear. `face` selects front (0, default) or back (1) for DFCs. */
  setCoverCard: (name: string | undefined, face?: 0 | 1) => void;
}

const initialDeck: Deck = {
  name: DEFAULT_DECK_NAME,
  format: 'constructed',
  cards: [],
  sideboard: [],
  attractions: [],
  contraptions: [],
  schemes: [],
  planes: [],
};

export const useDeckStore = create<DeckState>()(
  persist(
    (set) => ({
      currentDeck: initialDeck,
      currentDeckId: null,
      pool: [],
      savedDecks: [],
      addToMain: (card) =>
        set((state) => {
          // Enforce max copy limit based on deck format
          if (!BASIC_LAND_NAMES.has(card.name) && !allowsAnyNumberOfCopies(card.text)) {
            const format = getFormat(state.currentDeck.format ?? 'constructed');
            if (format) {
              const currentCount = state.currentDeck.cards.filter((c) => c.name === card.name).length;
              if (currentCount >= format.deckRules.maxCopies) {
                return state; // silently reject — UI will show toast via DeckBuilder
              }
            }
          }
          return {
            currentDeck: { ...state.currentDeck, cards: [...state.currentDeck.cards, card] },
          };
        }),
      addToSide: (card) =>
        set((state) => {
          const deck = normalizeDeck(state.currentDeck);
          if (isAttractionCard(card)) {
            return {
              currentDeck: { ...deck, attractions: [...(deck.attractions ?? []), card] },
            };
          }
          if (isContraptionCard(card)) {
            return {
              currentDeck: { ...deck, contraptions: [...(deck.contraptions ?? []), card] },
            };
          }
          if (isSchemeCard(card)) {
            return {
              currentDeck: { ...deck, schemes: [...(deck.schemes ?? []), card] },
            };
          }
          if (isPlaneCard(card)) {
            return {
              currentDeck: { ...deck, planes: [...(deck.planes ?? []), card] },
            };
          }
          return {
            currentDeck: { ...deck, sideboard: [...deck.sideboard, card] },
          };
        }),
      addToMaybe: (card) =>
        set((state) => ({
          currentDeck: { ...state.currentDeck, maybeboard: [...(state.currentDeck.maybeboard ?? []), card] },
        })),
      removeFromMaybe: (cardId) =>
        set((state) => {
          const idx = (state.currentDeck.maybeboard ?? []).findIndex((c) => c.id === cardId);
          if (idx === -1) return state;
          const maybeboard = [...(state.currentDeck.maybeboard ?? [])];
          maybeboard.splice(idx, 1);
          return { currentDeck: { ...state.currentDeck, maybeboard } };
        }),
      addToPool: (card) =>
        set((state) => ({
          pool: [...state.pool, card],
        })),
      removeFromMain: (cardId) =>
        set((state) => {
          const index = state.currentDeck.cards.findIndex((c) => c.id === cardId);
          if (index === -1) return state;
          const newCards = [...state.currentDeck.cards];
          newCards.splice(index, 1);
          return { currentDeck: { ...state.currentDeck, cards: newCards } };
        }),
      removeFromSide: (cardId) =>
        set((state) => {
          const deck = normalizeDeck(state.currentDeck);
          const sideIndex = deck.sideboard.findIndex((c) => c.id === cardId);
          if (sideIndex !== -1) {
            const sideboard = [...deck.sideboard];
            sideboard.splice(sideIndex, 1);
            return { currentDeck: { ...deck, sideboard } };
          }
          const attractionIndex = (deck.attractions ?? []).findIndex((c) => c.id === cardId);
          if (attractionIndex !== -1) {
            const attractions = [...(deck.attractions ?? [])];
            attractions.splice(attractionIndex, 1);
            return { currentDeck: { ...deck, attractions } };
          }
          const contraptionIndex = (deck.contraptions ?? []).findIndex((c) => c.id === cardId);
          if (contraptionIndex !== -1) {
            const contraptions = [...(deck.contraptions ?? [])];
            contraptions.splice(contraptionIndex, 1);
            return { currentDeck: { ...deck, contraptions } };
          }
          const schemeIndex = (deck.schemes ?? []).findIndex((c) => c.id === cardId);
          if (schemeIndex !== -1) {
            const schemes = [...(deck.schemes ?? [])];
            schemes.splice(schemeIndex, 1);
            return { currentDeck: { ...deck, schemes } };
          }
          const planeIndex = (deck.planes ?? []).findIndex((c) => c.id === cardId);
          if (planeIndex !== -1) {
            const planes = [...(deck.planes ?? [])];
            planes.splice(planeIndex, 1);
            return { currentDeck: { ...deck, planes } };
          }
          return state;
        }),
      setDeckName: (name) =>
        set((state) => ({
          currentDeck: { ...state.currentDeck, name },
        })),
      setDeckFormat: (format) =>
        set((state) => {
          const deck = normalizeDeck(state.currentDeck);
          if (format === 'constructed' && (deck.commanders?.length ?? 0) > 0) {
            // Move commanders back to main deck
            const movedBack = (deck.commanders ?? []).map((c) => ({ ...c, id: crypto.randomUUID() }));
            return {
              currentDeck: {
                ...deck,
                format,
                cards: [...deck.cards, ...movedBack],
                commanders: undefined,
              },
            };
          }
          return {
            currentDeck: {
              ...deck,
              format,
            },
          };
        }),
      clearDeck: () => set({ currentDeck: { ...initialDeck }, currentDeckId: null }),
      loadDeck: (deck) => set({ currentDeck: normalizeDeck(deck) }),
      setCommander: (card) =>
        set((state) => {
          const deck = normalizeDeck(state.currentDeck);
          const nextMain = [...deck.cards];
          const selectedIndex = nextMain.findIndex((entry) => entry.id === card.id);
          const selectedCard =
            selectedIndex !== -1
              ? nextMain.splice(selectedIndex, 1)[0]
              : { ...card };

          const commanders = [...(deck.commanders ?? [])];

          if (commanders.length >= 1) {
            if (!canBePartners(commanders[0], selectedCard)) {
              // New card can't partner with the existing commander — replace all
              for (const c of commanders.splice(0)) {
                nextMain.push({ ...c, id: crypto.randomUUID() });
              }
            } else if (commanders.length >= 2) {
              // Valid partner pair but already have 2 — replace the second
              const removed = commanders.pop()!;
              nextMain.push({ ...removed, id: crypto.randomUUID() });
            }
          }

          commanders.push(selectedCard!);

          return {
            currentDeck: {
              ...deck,
              format: 'commander',
              cards: nextMain,
              commanders,
            },
          };
        }),
      removeCommander: (card?: Card) =>
        set((state) => {
          const deck = normalizeDeck(state.currentDeck);
          const commanders = deck.commanders ?? [];
          if (commanders.length === 0) return state;

          const toRemove = card
            ? commanders.find((c) => c.name === card.name)
            : commanders[commanders.length - 1];
          if (!toRemove) return state;

          return {
            currentDeck: {
              ...deck,
              cards: [...deck.cards, { ...toRemove, id: crypto.randomUUID() }],
              commanders: commanders.filter((c) => c.name !== toRemove.name),
            },
          };
        }),
      setCompanion: (card) =>
        set((state) => {
          const deck = normalizeDeck(state.currentDeck);
          const nextSide = [...deck.sideboard];
          const idx = nextSide.findIndex((c) => c.id === card.id);
          const selected = idx !== -1 ? nextSide.splice(idx, 1)[0] : { ...card };

          // Move old companion back to sideboard
          if (deck.companion) {
            nextSide.push({ ...deck.companion, id: crypto.randomUUID() });
          }

          return {
            currentDeck: { ...deck, sideboard: nextSide, companion: selected },
          };
        }),
      removeCompanion: () =>
        set((state) => {
          const deck = normalizeDeck(state.currentDeck);
          if (!deck.companion) return state;
          return {
            currentDeck: {
              ...deck,
              sideboard: [...deck.sideboard, { ...deck.companion, id: crypto.randomUUID() }],
              companion: undefined,
            },
          };
        }),
      updatePrint: (cardName, scryfallCard) =>
        set((state) => {
          const updates = new Map<string, Partial<Card>>();
          updates.set(cardName.toLowerCase(), {
            setCode: scryfallCard.set,
            imageUrl: scryfallCard.image_uris?.normal ?? scryfallCard.image_uris?.large ?? undefined,
            cardNumber: scryfallCard.collector_number,
          });
          return {
            currentDeck: patchDeckCards(state.currentDeck, updates),
          };
        }),
      saveCurrentDeck: () =>
        set((state) => {
          // Clear draft flag on full save
          const deckToSave = { ...state.currentDeck, draft: undefined };
          // Match by tracked ID first, then fall back to name match
          const existing = state.currentDeckId
            ? state.savedDecks.find((s) => s.id === state.currentDeckId)
            : state.savedDecks.find((s) => s.deck.name === state.currentDeck.name);
          if (existing) {
            return {
              currentDeckId: existing.id,
              currentDeck: deckToSave,
              savedDecks: state.savedDecks.map((s) =>
                s.id === existing.id ? { ...s, deck: deckToSave, savedAt: Date.now() } : s
              ),
            };
          }
          const newId = crypto.randomUUID();
          const newSaved: SavedDeck = {
            id: newId,
            deck: normalizeDeck(deckToSave),
            savedAt: Date.now(),
          };
          return { currentDeckId: newId, savedDecks: [...state.savedDecks, newSaved] };
        }),
      saveDraft: () =>
        set((state) => {
          const draftDeck = { ...state.currentDeck, draft: true };
          const existing = state.currentDeckId
            ? state.savedDecks.find((s) => s.id === state.currentDeckId)
            : state.savedDecks.find((s) => s.deck.name === state.currentDeck.name);
          if (existing) {
            return {
              currentDeckId: existing.id,
              currentDeck: draftDeck,
              savedDecks: state.savedDecks.map((s) =>
                s.id === existing.id ? { ...s, deck: draftDeck, savedAt: Date.now() } : s
              ),
            };
          }
          const newId = crypto.randomUUID();
          return {
            currentDeckId: newId,
            currentDeck: draftDeck,
            savedDecks: [...state.savedDecks, { id: newId, deck: normalizeDeck(draftDeck), savedAt: Date.now() }],
          };
        }),
      loadSavedDeck: (id) =>
        set((state) => {
          const found = state.savedDecks.find((s) => s.id === id);
          if (!found) return state;
          return { currentDeck: normalizeDeck(found.deck), currentDeckId: id };
        }),
      deleteSavedDeck: (id) =>
        set((state) => ({
          savedDecks: state.savedDecks.filter((s) => s.id !== id),
        })),
      enrichDeckCards: (updates) =>
        set((state) => {
          return {
            currentDeck: patchDeckCards(state.currentDeck, updates),
          };
        }),
      addCardToSavedDeck: (id, card) =>
        set((state) => ({
          savedDecks: state.savedDecks.map((s) =>
            s.id !== id
              ? s
              : { ...s, deck: { ...normalizeDeck(s.deck), cards: [...s.deck.cards, card] }, savedAt: Date.now() },
          ),
        })),
      enrichSavedDeck: (id, updates) =>
        set((state) => ({
          savedDecks: state.savedDecks.map((s) =>
            s.id !== id
              ? s
              : {
                  ...s,
                  deck: patchDeckCards(s.deck, updates),
                }
          ),
        })),
      addCustomTag: (tag) =>
        set((state) => {
          const existing = state.currentDeck.customTags ?? [];
          if (existing.includes(tag)) return state;
          return {
            currentDeck: { ...state.currentDeck, customTags: [...existing, tag] },
          };
        }),
      removeCustomTag: (tag) =>
        set((state) => {
          const customTags = (state.currentDeck.customTags ?? []).filter((t) => t !== tag);
          const cardTags = { ...state.currentDeck.cardTags };
          for (const key of Object.keys(cardTags)) {
            cardTags[key] = cardTags[key].filter((t) => t !== tag);
            if (cardTags[key].length === 0) delete cardTags[key];
          }
          return {
            currentDeck: { ...state.currentDeck, customTags, cardTags },
          };
        }),
      tagCard: (cardName, tag) =>
        set((state) => {
          const key = cardName.toLowerCase();
          const cardTags = { ...state.currentDeck.cardTags };
          const tags = cardTags[key] ?? [];
          if (tags.includes(tag)) return state;
          cardTags[key] = [...tags, tag];
          return {
            currentDeck: { ...state.currentDeck, cardTags },
          };
        }),
      untagCard: (cardName, tag) =>
        set((state) => {
          const key = cardName.toLowerCase();
          const cardTags = { ...state.currentDeck.cardTags };
          const tags = cardTags[key] ?? [];
          cardTags[key] = tags.filter((t) => t !== tag);
          if (cardTags[key].length === 0) delete cardTags[key];
          return {
            currentDeck: { ...state.currentDeck, cardTags },
          };
        }),
      addDeckLabel: (label, color) =>
        set((state) => {
          const existing = state.currentDeck.labels ?? [];
          if (existing.some((l) => l.name.toLowerCase() === label.toLowerCase())) return state;
          return {
            currentDeck: { ...state.currentDeck, labels: [...existing, { name: label, color }] },
          };
        }),
      removeDeckLabel: (label) =>
        set((state) => ({
          currentDeck: {
            ...state.currentDeck,
            labels: (state.currentDeck.labels ?? []).filter((l) => l.name !== label),
          },
        })),
      updateDeckLabelColor: (label, color) =>
        set((state) => ({
          currentDeck: {
            ...state.currentDeck,
            labels: (state.currentDeck.labels ?? []).map((l) =>
              l.name === label ? { ...l, color } : l
            ),
          },
        })),
      setCoverCard: (name, face) =>
        set((state) => ({
          currentDeck: {
            ...state.currentDeck,
            coverCardName: name,
            coverCardFace: name !== undefined ? (face ?? 0) : undefined,
          },
        })),
    }),
    {
      name: STORAGE_KEYS.DECK,
      version: 3,
      migrate: (persistedState: unknown) => {
        if (!persistedState || typeof persistedState !== 'object') return persistedState as DeckState;
        const state = persistedState as {
          currentDeck?: Deck;
          currentDeckId?: string | null;
          savedDecks?: SavedDeck[];
        };
        return {
          ...state,
          currentDeckId: state.currentDeckId ?? null,
          currentDeck: normalizeDeck(state.currentDeck ?? initialDeck),
          savedDecks: (state.savedDecks ?? []).map((saved) => ({
            ...saved,
            deck: normalizeDeck(saved.deck),
          })),
        };
      },
    }
  )
);
