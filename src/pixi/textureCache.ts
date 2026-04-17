import { Texture, ImageSource } from "pixi.js";
import { fetch as tauriFetch } from "@tauri-apps/plugin-http";
import { getCardByName, getTokenBySetAndNumber } from "@/api/scryfall";
import { upgradeScryfallUrl } from "@/components/game/game.utils";

const resolvedUrls = new Map<string, string>();
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

async function resolveScryfallUrl(
  name: string,
  setCode?: string,
  cardNumber?: string,
  isToken?: boolean,
  size: "small" | "normal" | "large" = "normal",
): Promise<string | null> {
  try {
    const card = await queueApiCall(() => {
      if (isToken && setCode && cardNumber) {
        return getTokenBySetAndNumber(setCode, cardNumber);
      }
      return getCardByName(name, setCode);
    });
    if (card.card_faces) {
      const face = card.card_faces.find(
        (f: { name: string }) => f.name.toLowerCase() === name.toLowerCase(),
      ) ?? card.card_faces[0];
      return face?.image_uris?.[size] ?? face?.image_uris?.normal ?? null;
    }
    return card.image_uris?.[size] ?? card.image_uris?.normal ?? null;
  } catch {
    return null;
  }
}

const CARD_BACK_URL = "https://game.scryfall.io/attachments/config/sleeves/standard/back.jpg";

/**
 * Fetch image via Tauri's native HTTP (bypasses browser CORS entirely),
 * convert to blob URL, then load into HTMLImageElement for WebGL use.
 */
async function loadImageViaTauri(url: string): Promise<HTMLImageElement> {
  const response = await tauriFetch(url);
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
