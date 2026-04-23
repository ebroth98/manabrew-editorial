/**
 * Standalone Pixi canvas for the horizontal phase strip.
 * Intended to be positioned at the border between opponent and player halves.
 */

import { useRef, useEffect, useCallback, useState } from "react";
import { Application } from "pixi.js";
import { installPixiPatches } from "./pixiPatches";
installPixiPatches();
import { PhaseStripLayer, type PhaseStripState, type PhaseStripCallbacks } from "./PhaseStripLayer";
import { adaptTheme } from "./themeAdapter";
import { getGameThemeColors } from "@/components/game/game.theme";
import { usePreferencesStore } from "@/stores/usePreferencesStore";

interface Props {
  state: PhaseStripState;
  callbacks: PhaseStripCallbacks;
  className?: string;
}

export function PixiPhaseStripCanvas({ state, callbacks, className }: Props) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const appRef = useRef<Application | null>(null);
  const stripRef = useRef<PhaseStripLayer | null>(null);
  const callbacksRef = useRef(callbacks);
  callbacksRef.current = callbacks;

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
        resolution: Math.max(2, window.devicePixelRatio || 1),
      });
    } catch {
      appRef.current = null;
      return;
    }
    if (!app.renderer) { appRef.current = null; return; }

    const theme = adaptTheme(getGameThemeColors());
    const strip = new PhaseStripLayer(theme);
    stripRef.current = strip;
    app.stage.addChild(strip.container);

    strip.setCallbacks({
      onToggleSelfPhase: (id) => callbacksRef.current.onToggleSelfPhase?.(id),
      onToggleOpponentPhase: (oppId, id) => callbacksRef.current.onToggleOpponentPhase?.(oppId, id),
    });

    const parent = canvasRef.current.parentElement;
    if (parent) {
      app.renderer.resize(parent.clientWidth, parent.clientHeight);
      strip.resize(parent.clientWidth, parent.clientHeight);
    }

    app.ticker.add(() => strip.tick());
    setReady(true);
  }, []);

  useEffect(() => {
    initApp();
    return () => {
      stripRef.current?.destroy();
      stripRef.current = null;
      appRef.current?.destroy(true);
      appRef.current = null;
      setReady(false);
    };
  }, [initApp]);

  // Resize
  useEffect(() => {
    const parent = canvasRef.current?.parentElement;
    const app = appRef.current;
    const strip = stripRef.current;
    if (!parent || !app || !strip) return;
    const observer = new ResizeObserver((entries) => {
      for (const entry of entries) {
        const { width, height } = entry.contentRect;
        if (width > 0 && height > 0) {
          app.renderer.resize(width, height);
          strip.resize(width, height);
        }
      }
    });
    observer.observe(parent);
    return () => observer.disconnect();
  }, [ready]);

  // Theme
  useEffect(() => {
    if (!stripRef.current) return;
    const unsub = usePreferencesStore.subscribe(() => {
      const theme = adaptTheme(getGameThemeColors());
      stripRef.current?.setTheme(theme);
    });
    return unsub;
  }, [ready]);

  // State
  useEffect(() => {
    stripRef.current?.update(state);
  }, [state, ready]);

  return (
    <div className={className} style={{ position: "relative", width: "100%", height: "100%" }}>
      <canvas
        ref={canvasRef}
        style={{ width: "100%", height: "100%", display: "block" }}
      />
    </div>
  );
}
