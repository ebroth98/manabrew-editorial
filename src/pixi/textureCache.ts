import { Texture, ImageSource } from "pixi.js";
import { platformFetch } from "@/lib/platformFetch";
import {
  getCardByName,
  getTokenByName,
  getTokenBySetAndNumber,
} from "@/api/scryfall";
import type { ScryfallCard } from "@/types/scryfall";
import { upgradeScryfallUrl } from "@/components/game/game.utils";
import {
  cardImageIdentity,
  clearPendingCardImage,
  getCachedCardImage,
  getPendingCardImage,
  pickImageUrl,
  pickUrisForName,
  setCachedCardImage,
  setPendingCardImage,
  toCachedCardImage,
  type CachedCardImage,
} from "@/lib/cardImageCache";

/** Resolved URLs per identity+size combo. The underlying Scryfall
 *  response is shared across sizes via `cardImageCache`, so this map
 *  only memoizes the cheap size-specific projection. */
const resolvedUrls = new Map<string, string>();
/** In-flight URL projections (per identity+size). Separate from the
 *  identity-level fetch in `cardImageCache` so concurrent callers asking
 *  for the same identity+size share a single `upgradeScryfallUrl` path. */
const pendingUrls = new Map<string, Promise<string | null>>();
const textureCache = new Map<string, Texture>();
const imageElements = new Map<string, HTMLImageElement>();
const blobUrls = new Map<string, string>();
const loadingPromises = new Map<string, Promise<Texture>>();

/** Simple serial queue for Scryfall API calls to avoid 429 rate limiting. */
const apiQueue: Array<() => Promise<void>> = [];
let apiRunning = false;
const API_DELAY_MS = 100; // 10 req/s, safely under Scryfall's limit

async function drainApiQueue(): Promise<void> {
  if (apiRunning) return;
  apiRunning = true;
  while (apiQueue.length > 0) {
    const job = apiQueue.shift()!;
    await job();
    if (apiQueue.length > 0) {
      await new Promise((r) => setTimeout(r, API_DELAY_MS));
    }
  }
  apiRunning = false;
}

function queueApiCall<T>(fn: () => Promise<T>): Promise<T> {
  return new Promise<T>((resolve, reject) => {
    apiQueue.push(async () => {
      try { resolve(await fn()); } catch (e) { reject(e); }
    });
    drainApiQueue();
  });
}

/**
 * Resolve a Scryfall card entry (image_uris + per-face uris) for this
 * identity, reading the shared cache first so the hand sprite that ran
 * the request on draw is the only one to actually hit the network.
 */
async function fetchCardImageEntry(
  name: string,
  setCode: string | undefined,
  cardNumber: string | undefined,
  isToken: boolean | undefined,
): Promise<CachedCardImage | null> {
  const key = cardImageIdentity(name, setCode, cardNumber, isToken);
  const cached = getCachedCardImage(key);
  if (cached) return cached;
  const inflight = getPendingCardImage(key);
  if (inflight) return inflight;

  const task = (async (): Promise<CachedCardImage | null> => {
    // Tokens follow a strict lookup cascade so we never fall through to
    // the normal /cards/named endpoint (which returns the legendary
    // creature with the same name, not the token print):
    //   1. set_code + collector_number — direct token print.
    //   2. Scryfall search with `type:token` — newest matching token print.
    //   3. Only as a last resort, generic /cards/named.
    let card: ScryfallCard | null = null;
    try {
      if (isToken) {
        if (setCode && cardNumber) {
          try {
            card = await queueApiCall(() => getTokenBySetAndNumber(setCode, cardNumber));
          } catch {
            card = null;
          }
        }
        if (!card) card = await queueApiCall(() => getTokenByName(name));
        if (!card) card = await queueApiCall(() => getCardByName(name, setCode));
      } else {
        card = await queueApiCall(() => getCardByName(name, setCode));
      }
    } catch {
      return null;
    }
    if (!card) return null;
    const entry = toCachedCardImage(card);
    setCachedCardImage(key, entry);
    return entry;
  })().finally(() => clearPendingCardImage(key));
  setPendingCardImage(key, task);
  return task;
}

async function resolveScryfallUrl(
  name: string,
  setCode?: string,
  cardNumber?: string,
  isToken?: boolean,
  size: "small" | "normal" | "large" = "normal",
): Promise<string | null> {
  const entry = await fetchCardImageEntry(name, setCode, cardNumber, isToken);
  if (!entry) return null;
  return pickImageUrl(pickUrisForName(entry, name), size) ?? null;
}

import { CARD_BACK_IMAGE_URL as CARD_BACK_URL } from "@/components/game/game.constants";

/**
 * Fetch image via Tauri's native HTTP on desktop (bypasses browser CORS) or
 * the browser's fetch on web, convert to blob URL, then load into an
 * HTMLImageElement for WebGL use.
 */
async function loadImageViaTauri(url: string): Promise<HTMLImageElement> {
  const response = await platformFetch(url);
  if (!response.ok) throw new Error(`HTTP ${response.status} for ${url}`);
  const blob = await response.blob();
  const blobUrl = URL.createObjectURL(blob);
  blobUrls.set(url, blobUrl);

  return new Promise((resolve, reject) => {
    const img = new Image();
    img.onload = () => resolve(img);
    img.onerror = () => reject(new Error(`Failed to decode: ${url}`));
    img.src = blobUrl;
  });
}

function createTextureFromImage(url: string, img: HTMLImageElement): Texture {
  imageElements.set(url, img);
  const source = new ImageSource({ resource: img });
  const tex = new Texture({ source });
  textureCache.set(url, tex);
  return tex;
}

export async function getCardBackTexture(): Promise<Texture> {
  const cached = textureCache.get(CARD_BACK_URL);
  if (cached && !cached.destroyed) return cached;

  try {
    const img = await loadImageViaTauri(CARD_BACK_URL);
    return createTextureFromImage(CARD_BACK_URL, img);
  } catch {
    return Texture.EMPTY;
  }
}

export async function resolveCardImageUrl(
  name: string,
  existingUrl?: string,
  isToken?: boolean,
  setCode?: string,
  cardNumber?: string,
  size: "small" | "normal" | "large" = "normal",
): Promise<string | null> {
  const cacheKey = `${name}:${setCode ?? ""}:${cardNumber ?? ""}:${size}`;

  // 1. Already resolved — instant return
  const cached = resolvedUrls.get(cacheKey);
  if (cached) return cached;

  // 2. Have a direct URL — skip API entirely
  let url = existingUrl ?? null;
  if (url) {
    const upgraded = upgradeScryfallUrl(url, size) ?? url;
    resolvedUrls.set(cacheKey, upgraded);
    return upgraded;
  }

  // 3. Already in-flight for this key — wait on the same promise (no duplicate API call)
  const pending = pendingUrls.get(cacheKey);
  if (pending) return pending;

  // 4. New lookup — queue it and deduplicate
  const promise = resolveScryfallUrl(name, setCode, cardNumber, isToken, size).then((resolved) => {
    pendingUrls.delete(cacheKey);
    if (!resolved) return null;
    const upgraded = upgradeScryfallUrl(resolved, size) ?? resolved;
    resolvedUrls.set(cacheKey, upgraded);
    return upgraded;
  }).catch(() => {
    pendingUrls.delete(cacheKey);
    return null;
  });
  pendingUrls.set(cacheKey, promise);
  return promise;
}

export async function loadCardTexture(
  name: string,
  existingUrl?: string,
  isToken?: boolean,
  setCode?: string,
  cardNumber?: string,
  size: "small" | "normal" | "large" = "normal",
  isFaceDown?: boolean,
): Promise<Texture> {
  if (isFaceDown) {
    return getCardBackTexture();
  }

  const url = await resolveCardImageUrl(name, existingUrl, isToken, setCode, cardNumber, size);
  if (!url) return Texture.EMPTY;

  const cached = textureCache.get(url);
  if (cached && !cached.destroyed) return cached;

  const existing = loadingPromises.get(url);
  if (existing) return existing;

  const promise = loadImageViaTauri(url).then((img) => {
    loadingPromises.delete(url);
    return createTextureFromImage(url, img);
  }).catch((err) => {
    console.warn(`[pixi] ${err.message} (${name})`);
    loadingPromises.delete(url);
    return Texture.EMPTY;
  });
  loadingPromises.set(url, promise);
  return promise;
}

export function clearTextureCache(): void {
  resolvedUrls.clear();
  textureCache.clear();
  imageElements.clear();
  loadingPromises.clear();
  for (const blobUrl of blobUrls.values()) {
    URL.revokeObjectURL(blobUrl);
  }
  blobUrls.clear();
}
