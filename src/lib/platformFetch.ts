import { fetch as tauriFetch } from "@tauri-apps/plugin-http";
import { getPlatformType } from "@/platform";

type FetchFn = (input: string, init?: RequestInit) => Promise<Response>;

let cached: FetchFn | null = null;

export function platformFetch(input: string, init?: RequestInit): Promise<Response> {
  if (!cached) {
    cached =
      getPlatformType() === "tauri"
        ? (tauriFetch as unknown as FetchFn)
        : (fetch.bind(globalThis) as FetchFn);
  }
  return cached(input, init);
}
