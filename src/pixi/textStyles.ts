/**
 * Shared Pixi text styles. Instantiated once and reused across the scene
 * so we don't allocate a new TextStyle per draw call.
 */

import { TextStyle } from "pixi.js";
import { CARD_W } from "@/components/game/game.constants";

const SYSTEM_FONT_FAMILY = "system-ui, -apple-system, sans-serif";
const COLOR_WHITE = "#ffffff";
const COLOR_EMPTY_LABEL = "#666";
const COLOR_GHOST_LABEL = "#888";

export const OVERLAY_LABEL_STYLE = new TextStyle({
  fontFamily: SYSTEM_FONT_FAMILY,
  fontSize: 9,
  fontWeight: "bold",
  fill: COLOR_WHITE,
});

export const GHOST_LABEL_STYLE = new TextStyle({
  fontFamily: SYSTEM_FONT_FAMILY,
  fontSize: 9,
  fill: COLOR_GHOST_LABEL,
  fontWeight: "500",
  wordWrap: true,
  wordWrapWidth: CARD_W - 8,
  align: "center",
});

export const EMPTY_LABEL_STYLE = new TextStyle({
  fontFamily: SYSTEM_FONT_FAMILY,
  fontSize: 12,
  fill: COLOR_EMPTY_LABEL,
  fontStyle: "italic",
});

export const SELECTION_BADGE_STYLE = new TextStyle({
  fontFamily: SYSTEM_FONT_FAMILY,
  fontSize: 10,
  fill: COLOR_WHITE,
});
