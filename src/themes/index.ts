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
