// Unified deck-source dispatcher. Parses URLs from multiple providers and
// routes fetches to the matching provider-specific module. Both the CLI and
// the in-app importer consume this.

import {
  fetchArchidektDeck,
  fetchArchidektResult,
  parseArchidektUrl,
  type ArchidektDeck,
  type ArchidektSearchResult,
  type RequestOptions,
} from "./archidekt";
import { fetchMoxfieldDeck, fetchMoxfieldResult, parseMoxfieldUrl } from "./moxfield";

export type DeckSource = "archidekt" | "moxfield";

export interface ParsedDeckUrl {
  source: DeckSource;
  id: string;
}

export function parseDeckUrl(input: string): ParsedDeckUrl | null {
  const trimmed = input.trim();
  if (!trimmed) return null;
  const mox = parseMoxfieldUrl(trimmed);
  if (mox) return { source: "moxfield", id: mox };
  const arc = parseArchidektUrl(trimmed);
  if (arc) return { source: "archidekt", id: arc };
  return null;
}

export async function fetchDeckBySource(
  source: DeckSource,
  id: string,
  opts: RequestOptions = {},
): Promise<ArchidektDeck> {
  switch (source) {
    case "archidekt":
      return fetchArchidektDeck(id, opts);
    case "moxfield":
      return fetchMoxfieldDeck(id, opts);
  }
}

export async function fetchResultBySource(
  source: DeckSource,
  id: string,
  opts: RequestOptions = {},
): Promise<ArchidektSearchResult> {
  switch (source) {
    case "archidekt":
      return fetchArchidektResult(id, opts);
    case "moxfield":
      return fetchMoxfieldResult(id, opts);
  }
}
