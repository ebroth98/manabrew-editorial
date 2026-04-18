/**
 * Runtime patches for Pixi v8 bugs we've hit in practice.
 *
 * Side-effect module — import it once at the top of any entry that uses
 * Pixi (PixiGameCanvas / PixiArrowsCanvas). The patches apply to the
 * singleton exports, so a single import protects the whole app.
 */

import { TexturePool } from "pixi.js";

let patched = false;

export function installPixiPatches(): void {
  if (patched) return;
  patched = true;

  // ── TexturePool.returnTexture guard ────────────────────────────────────
  // Pixi v8 crashes with `undefined is not an object (evaluating
  // 'this._texturePool[key].push')` when a texture is returned whose
  // `uid` isn't present in `_poolKeyHash` — e.g. during renderer teardown
  // when the text system's GC manager releases cached text textures after
  // the pool has been cleared. The upstream fix is a one-line guard that
  // checks the pool array exists before pushing. We apply it at runtime.
  //
  // Upstream bug: https://github.com/pixijs/pixijs/blob/main/src/rendering/renderers/shared/texture/TexturePool.ts#L112
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const pool: any = TexturePool;
  const original = pool.returnTexture?.bind(pool);
  if (typeof original !== "function") return;

  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  pool.returnTexture = function patchedReturnTexture(
    renderTexture: unknown,
    resetStyle?: boolean,
  ) {
    try {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const rt = renderTexture as { uid?: number };
      const key = rt?.uid != null ? this._poolKeyHash?.[rt.uid] : undefined;
      if (key == null || !Array.isArray(this._texturePool?.[key])) {
        // The pool has no slot for this texture — nothing to return. This
        // can happen when the renderer is torn down and the GC manager
        // releases text cache entries after the pool has been cleared.
        return;
      }
      original(renderTexture, resetStyle);
    } catch (err) {
      // Swallow so a Pixi internal bug can't crash the React tree mid
      // teardown. We log once for visibility.
      console.warn("[pixi] TexturePool.returnTexture guard swallowed:", err);
    }
  };
}
