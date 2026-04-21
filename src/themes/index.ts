/**
 * App theme presets. Each preset provides HSL values (without `hsl()` wrapper)
 * for both light and dark modes. These override the CSS variables in index.css.
 */

export interface ThemeColors {
  background: string;
  foreground: string;
  card: string;
  "card-foreground": string;
  popover: string;
  "popover-foreground": string;
  primary: string;
  "primary-foreground": string;
  secondary: string;
  "secondary-foreground": string;
  muted: string;
  "muted-foreground": string;
  accent: string;
  "accent-foreground": string;
  destructive: string;
  "destructive-foreground": string;
  border: string;
  input: string;
  ring: string;
  selection: string;
  "selection-foreground": string;
  commander: string;
  warning: string;
  overlay: string;
}

export interface GameThemePresetColors {
  "activeAction.priority": string;
  "activeAction.turnText": string;
  "activeAction.myTurnRing": string;
  "activeAction.opponentTurnRing": string;
  "highlight": string;
  "hand.playableBorder": string;
  "promptAction.default": string;
  "promptAction.passPriority": string;
  "promptAction.passUntilEnd": string;
  "promptAction.cancel": string;
  "promptAction.pacificAction": string;
  "arrow.attack": string;
  "arrow.block": string;
  "arrow.hostileTarget": string;
  "arrow.friendlyTarget": string;
  "cardRing": string;
  /** Optional per-intent pointer colour overrides; keys are
   *  `pointer.<intent>` where `<intent>` is a `TargetingIntent` value.
   *  Presets that don't define these fall through to the default
   *  preset's entries. */
  [pointerOverride: `pointer.${string}`]: string | undefined;
  /** Optional mana-letter tint overrides (`mana.W` ... `mana.C`). */
  [manaOverride: `mana.${string}`]: string | undefined;
  /** Optional card-status badge colour overrides (`cardStatus.exerted`, etc.). */
  [cardStatusOverride: `cardStatus.${string}`]: string | undefined;
  /** Optional per-counter-type badge colour overrides (`counter.p1p1`, etc.). */
  [counterOverride: `counter.${string}`]: string | undefined;
  /** Optional P/T badge state overrides (`pt.neutral`, etc.). */
  [ptOverride: `pt.${string}`]: string | undefined;
  /** Optional override for the generic text-on-tinted-bg colour. */
  textOnTinted?: string;
  /** Optional override for the positive-state indicator colour (green). */
  success?: string;
  /** Optional override for the poison counter / skull colour (MTG infect green). */
  poison?: string;
  /** Optional override for the life / heart indicator colour (red). */
  life?: string;
  /** Optional override for the subdued "empty zone" label colour. */
  textMuted?: string;
  /** Optional override for the ghost-placeholder card-name colour. */
  textGhost?: string;
  /** Optional canvas-level neutral overrides (`canvas.background`, etc.). */
  [canvasOverride: `canvas.${string}`]: string | undefined;
  /** Optional placeholder card colour overrides (`cardPlaceholder.fill`, etc.). */
  [placeholderOverride: `cardPlaceholder.${string}`]: string | undefined;
}

export interface ThemePreset {
  id: string;
  name: string;
  description: string;
  light: ThemeColors;
  dark: ThemeColors;
  gameColors: GameThemePresetColors;
}

import defaultPreset from "./default";
import rosePinePreset from "./rose-pine";
import nordPreset from "./nord";
import catppuccinPreset from "./catppuccin";
import solarizedPreset from "./solarized";
import draculaPreset from "./dracula";
import gruvboxPreset from "./gruvbox";
import tokyoNightPreset from "./tokyo-night";
import oneDarkPreset from "./one-dark";
import monokaiPreset from "./monokai";
import everforestPreset from "./everforest";
import kanagawaPreset from "./kanagawa";

export const THEME_PRESETS: ThemePreset[] = [
  defaultPreset,
  nordPreset,
  rosePinePreset,
  catppuccinPreset,
  draculaPreset,
  tokyoNightPreset,
  oneDarkPreset,
  gruvboxPreset,
  monokaiPreset,
  solarizedPreset,
  everforestPreset,
  kanagawaPreset,
];
