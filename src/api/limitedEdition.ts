import { getPlatform } from "@/platform";
import type { DraftCard } from "@/types/limited";

export interface EditionSlot {
  label: string;
  count: number;
}

export interface EditionInfo {
  code: string;
  name: string;
  editionType: string;
  date: string | null;
  slots: EditionSlot[];
  foilChance: number;
  foilType: string;
  variants: string[];
  hasReplacementHooks: boolean;
  boosterCovers?: number;
  prerelease?: string | null;
  alias?: string | null;
}

export async function fetchEditionInfo(setCode: string): Promise<EditionInfo | null> {
  if (!setCode) return null;
  try {
    const result = await getPlatform().invoke<EditionInfo | null>("limited_get_edition_info", {
      setCode,
    });
    return result ?? null;
  } catch {
    return null;
  }
}

/**
 * Generate the full card pool for a given set from the engine's cached
 * `EditionsRegistry` — no Scryfall round-trip. The DTO shape matches what
 * `limited_start_sealed` / `limited_start_booster_draft` expect for their
 * `setup.pool` field.
 */
export async function fetchSetPool(setCode: string): Promise<DraftCard[]> {
  return getPlatform().invoke<DraftCard[]>("limited_get_set_pool", { setCode });
}
