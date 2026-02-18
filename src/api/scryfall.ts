import type { ScryfallListResponse } from "@/types/scryfall";

export async function searchCards(query: string, page: number = 1): Promise<ScryfallListResponse> {
  const url = `https://api.scryfall.com/cards/search?q=${encodeURIComponent(query)}&page=${page}&order=cmc&unique=cards`;
  const response = await fetch(url);
  if (!response.ok) {
    throw new Error('Failed to fetch cards from Scryfall');
  }
  return response.json();
}
