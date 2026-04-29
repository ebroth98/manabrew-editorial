import { getPlatform } from "@/platform";

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
