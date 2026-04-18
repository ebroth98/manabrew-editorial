import { useCallback, useEffect, useRef, useState } from "react";
import { Application, type Ticker } from "pixi.js";
import { installPixiPatches } from "./pixiPatches";
import { ArrowLayer, type ArrowDef } from "./ArrowLayer";

installPixiPatches();
import { adaptTheme, type PixiThemeColors } from "./themeAdapter";
import { getGameThemeColors } from "@/components/game/game.theme";
import { usePreferencesStore } from "@/stores/usePreferencesStore";
import type {
  ArrowSpec,
  ArrowEndpoint,
  CastingArrowSpec,
  ScreenPos,
} from "./types";
import type { PixiGameScene } from "./PixiGameScene";

interface PixiArrowsCanvasProps {
  arrowSpecs?: ArrowSpec[];
  castingArrow?: CastingArrowSpec | null;
  /** Ref to the main `PixiGameScene` (the player's own canvas). Drives
   *  the placement-ghost lookup and is the first scene searched for
   *  card endpoints. */
  mainSceneRef: React.MutableRefObject<PixiGameScene | null>;
  /** Per-opponent scene refs keyed by player id. The arrow layer
   *  iterates these after the main scene so opponent permanents
   *  resolve to live sprite positions instead of falling back to DOM
   *  queries. Consumed as a live Map so newly-mounted opponents are
   *  picked up without re-subscribing. */
  opponentSceneRefs?: Map<string, React.MutableRefObject<PixiGameScene | null>>;
  className?: string;
}

/**
 * Transparent full-area Pixi canvas that draws *only* arrows.
 *
 * Sits on top of the React DOM and the main game canvas, with
 * `pointer-events: none`, so arrows can span the entire board (opponent
 * side, middle, my side). Endpoints resolve against:
 *   – the main scene's sprite maps (battlefield + hand) when available,
 *   – DOM query (`data-card-id`, `data-player-id`, `data-stack-object-id`)
 *     with viewport→canvas-local translation, otherwise,
 *   – `window.mousemove` for the free end of the casting arrow.
 *
 * Decoupling from the main canvas keeps event handling unchanged in the
 * play area and lets arrows reach React-rendered opponent permanents.
 */
export function PixiArrowsCanvas({
  arrowSpecs,
  castingArrow,
  mainSceneRef,
  opponentSceneRefs,
  className,
}: PixiArrowsCanvasProps) {
  // Keep a stable ref to the opponent scenes Map so the ticker closure
  // reads the latest live scenes without re-registering on every render.
  const opponentSceneRefsRef = useRef(opponentSceneRefs);
  useEffect(() => {
    opponentSceneRefsRef.current = opponentSceneRefs;
  }, [opponentSceneRefs]);
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const appRef = useRef<Application | null>(null);
  const arrowLayerRef = useRef<ArrowLayer | null>(null);
  const themeRef = useRef<PixiThemeColors | null>(null);

  // Latest inputs accessed inside the ticker callback without re-binding.
  const arrowSpecsRef = useRef<ArrowSpec[]>([]);
  const castingArrowRef = useRef<CastingArrowSpec | null>(null);
  useEffect(() => { arrowSpecsRef.current = arrowSpecs ?? []; }, [arrowSpecs]);
  useEffect(() => { castingArrowRef.current = castingArrow ?? null; }, [castingArrow]);

  const cursorViewportRef = useRef<{ x: number; y: number }>({ x: 0, y: 0 });

  const [ready, setReady] = useState(false);

  const initApp = useCallback(async () => {
    if (!canvasRef.current || appRef.current) return;

    const app = new Application();
    appRef.current = app;
    try {
      await app.init({
        canvas: canvasRef.current,
        preference: "webgl",
        backgroundAlpha: 0,
        antialias: true,
        autoDensity: true,
        // Arrows are thin lines — bump resolution so strokes stay sharp.
        resolution: Math.max(2, window.devicePixelRatio || 1),
        resizeTo: canvasRef.current.parentElement ?? undefined,
      });
    } catch (err) {
      console.error("[pixi-arrows] init failed:", err);
      appRef.current = null;
      return;
    }
    if (!app.renderer) return;

    const arrowLayer = new ArrowLayer();
    arrowLayerRef.current = arrowLayer;
    app.stage.addChild(arrowLayer.graphics);

    themeRef.current = adaptTheme(getGameThemeColors());
    arrowLayer.setTheme(themeRef.current);

    app.ticker.add((ticker: Ticker) => {
      if (!arrowLayerRef.current || !canvasRef.current) return;
      const opponentScenes: PixiGameScene[] = [];
      const map = opponentSceneRefsRef.current;
      if (map) {
        for (const ref of map.values()) {
          if (ref.current) opponentScenes.push(ref.current);
        }
      }
      const resolved = resolveArrows(
        canvasRef.current,
        arrowSpecsRef.current,
        castingArrowRef.current,
        mainSceneRef.current,
        opponentScenes,
        cursorViewportRef.current,
      );
      arrowLayerRef.current.update(resolved, ticker.deltaMS);
    });

    setReady(true);
  }, [mainSceneRef]);

  useEffect(() => {
    initApp();
    return () => {
      arrowLayerRef.current?.destroy();
      arrowLayerRef.current = null;
      appRef.current?.destroy(true);
      appRef.current = null;
      setReady(false);
    };
  }, [initApp]);

  // Re-apply theme whenever preferences change (same subscription pattern
  // PixiGameCanvas uses for its scene).
  useEffect(() => {
    if (!ready) return;
    const apply = () => {
      themeRef.current = adaptTheme(getGameThemeColors());
      arrowLayerRef.current?.setTheme(themeRef.current);
    };
    apply();
    return usePreferencesStore.subscribe(apply);
  }, [ready]);

  // Track cursor globally so the free casting-arrow endpoint follows the
  // mouse even when it's over DOM elements covering the arrows canvas.
  useEffect(() => {
    const onMove = (e: MouseEvent) => {
      cursorViewportRef.current = { x: e.clientX, y: e.clientY };
    };
    window.addEventListener("mousemove", onMove);
    return () => window.removeEventListener("mousemove", onMove);
  }, []);

  return (
    <canvas
      ref={canvasRef}
      className={className}
      style={{
        position: "absolute",
        inset: 0,
        width: "100%",
        height: "100%",
        pointerEvents: "none",
        zIndex: 45,
      }}
    />
  );
}

// ────────────────────────────────────────────────────────────────────────
// Endpoint resolution — pure helpers below
// ────────────────────────────────────────────────────────────────────────

function resolveArrows(
  canvas: HTMLCanvasElement,
  specs: ArrowSpec[],
  casting: CastingArrowSpec | null,
  mainScene: PixiGameScene | null,
  opponentScenes: PixiGameScene[],
  cursorViewport: { x: number; y: number },
): ArrowDef[] {
  if (specs.length === 0 && !casting) return [];
  const canvasRect = canvas.getBoundingClientRect();
  const scenesWithRect: Array<{ scene: PixiGameScene; rect: DOMRect }> = [];
  if (mainScene) {
    scenesWithRect.push({ scene: mainScene, rect: mainScene.canvasElement.getBoundingClientRect() });
  }
  for (const s of opponentScenes) {
    scenesWithRect.push({ scene: s, rect: s.canvasElement.getBoundingClientRect() });
  }

  const toLocal = (viewport: { x: number; y: number }): ScreenPos => ({
    x: viewport.x - canvasRect.left,
    y: viewport.y - canvasRect.top,
  });

  const resolveEndpoint = (ep: ArrowEndpoint): ScreenPos | null => {
    switch (ep.kind) {
      case "card": {
        // Probe each live scene (player first, then opponents) for the
        // sprite before falling through to a DOM query. Each scene
        // reports canvas-local coords that we translate into the
        // arrow-canvas' own viewport.
        for (const { scene, rect } of scenesWithRect) {
          const spr = scene.getCardSpritePosition(ep.id);
          if (spr) {
            return {
              x: spr.x + rect.left - canvasRect.left,
              y: spr.y + rect.top - canvasRect.top,
            };
          }
        }
        return domCenter(`[data-card-id="${CSS.escape(ep.id)}"]`, toLocal);
      }
      case "player":
        return domCenter(`[data-player-id="${CSS.escape(ep.id)}"]`, toLocal);
      case "stack":
        return domCenter(`[data-stack-object-id="${CSS.escape(ep.id)}"]`, toLocal);
      case "placement-ghost": {
        if (!mainScene) return null;
        const rect = scenesWithRect[0]?.rect;
        if (!rect) return null;
        const c = mainScene.getPlacementGhostCenter();
        return {
          x: c.x + rect.left - canvasRect.left,
          y: c.y + rect.top - canvasRect.top,
        };
      }
    }
  };

  const resolved: ArrowDef[] = [];
  for (const spec of specs) {
    const from = resolveEndpoint(spec.from);
    const to = resolveEndpoint(spec.to);
    if (!from || !to) continue;
    resolved.push({
      fromX: from.x, fromY: from.y,
      toX: to.x, toY: to.y,
      type: spec.type,
    });
  }

  if (casting) {
    const from = domCenter(
      `[data-casting-card="${CSS.escape(casting.castingCardId)}"]`,
      toLocal,
    );
    if (from) {
      let to: ScreenPos | null = null;
      if (casting.targetId) {
        to =
          resolveEndpoint({ kind: "card", id: casting.targetId }) ??
          resolveEndpoint({ kind: "player", id: casting.targetId });
      } else {
        to = toLocal(cursorViewport);
      }
      if (to) {
        resolved.push({
          fromX: from.x, fromY: from.y,
          toX: to.x, toY: to.y,
          type: casting.hostile ? "hostile-target" : "friendly-target",
        });
      }
    }
  }

  return resolved;
}

function domCenter(
  selector: string,
  toLocal: (viewport: { x: number; y: number }) => ScreenPos,
): ScreenPos | null {
  const el = document.querySelector(selector);
  if (!el) return null;
  const r = (el as HTMLElement).getBoundingClientRect();
  if (r.width === 0 && r.height === 0) return null;
  return toLocal({ x: r.left + r.width / 2, y: r.top + r.height / 2 });
}
