import type { ThemePreset } from "./index";
import { buildGameColors, type BasePalette } from "./buildGameColors";

/** Catppuccin Mocha palette — pastel warm; maps love → red, mauve →
 *  violet/purple, peach → orange, etc. */
const palette: BasePalette = {
  foreground: "#cdd6f4",        // text
  labelMuted: "#6c7086",        // overlay0
  labelGhost: "#a6adc8",        // subtext0
  placeholderFill: "#1e1e2e",   // base
  placeholderStroke: "#45475a", // surface1
  canvasBackground: "#1e1e2e",
  red:      "#f38ba8",          // red
  redDeep:  "#a05571",
  orange:   "#fab387",          // peach
  amber:    "#f9e2af",          // yellow
  yellow:   "#f9e2af",
  green:    "#a6e3a1",          // green
  teal:     "#94e2d5",          // teal
  cyan:     "#89dceb",          // sky
  blue:     "#89b4fa",          // blue
  sky:      "#74c7ec",          // sapphire
  indigo:   "#b4befe",          // lavender
  violet:   "#cba6f7",          // mauve
  purple:   "#cba6f7",
  pink:     "#f5c2e7",          // pink
  slate:    "#6c7086",
  brown:    "#fab387",
  paper:    "#bac2de",          // subtext1
  poison:   "#8cc58a",          // muted sibling of Catppuccin green
  manaW:    "#f5e0dc",          // rosewater
  manaU:    "#89b4fa",
  manaB:    "#45475a",
  manaR:    "#f38ba8",
  manaG:    "#a6e3a1",
  manaC:    "#bac2de",
};

const preset: ThemePreset = {
  id: "catppuccin",
  name: "Catppuccin",
  description: "Pastel, soothing warm tones",
  light: {
    background: "220 23% 95%",
    foreground: "234 16% 30%",
    card: "220 23% 100%",
    "card-foreground": "234 16% 30%",
    popover: "220 23% 100%",
    "popover-foreground": "234 16% 30%",
    primary: "266 85% 58%",
    "primary-foreground": "0 0% 100%",
    secondary: "220 22% 90%",
    "secondary-foreground": "234 16% 30%",
    muted: "220 20% 92%",
    "muted-foreground": "233 10% 48%",
    accent: "220 20% 88%",
    "accent-foreground": "234 16% 30%",
    destructive: "347 87% 44%",
    "destructive-foreground": "0 0% 100%",
    border: "220 18% 84%",
    input: "220 18% 87%",
    ring: "266 85% 58%",
    selection: "266 85% 62%",
    "selection-foreground": "0 0% 100%",
    commander: "41 86% 50%",
    warning: "41 76% 50%",
    overlay: "234 16% 10%",
  },
  dark: {
    background: "240 21% 12%",
    foreground: "226 64% 88%",
    card: "240 21% 15%",
    "card-foreground": "226 64% 88%",
    popover: "240 21% 17%",
    "popover-foreground": "226 64% 88%",
    primary: "267 84% 74%",
    "primary-foreground": "240 21% 12%",
    secondary: "240 18% 20%",
    "secondary-foreground": "226 64% 88%",
    muted: "240 18% 20%",
    "muted-foreground": "227 35% 52%",
    accent: "240 18% 23%",
    "accent-foreground": "226 64% 88%",
    destructive: "347 70% 48%",
    "destructive-foreground": "226 64% 92%",
    border: "240 18% 22%",
    input: "240 18% 20%",
    ring: "267 84% 74%",
    selection: "267 84% 68%",
    "selection-foreground": "226 64% 92%",
    commander: "41 70% 55%",
    warning: "41 65% 50%",
    overlay: "240 21% 4%",
  },
  gameColors: {
    "activeAction.priority": "#cba6f7",
    "activeAction.turnText": "#f9e2af",
    "activeAction.myTurnRing": "#f9e2af",
    "activeAction.opponentTurnRing": "#f9e2af",
    "highlight": "#fab387",
    "hand.playableBorder": "rgba(205, 214, 244, 0.7)",
    "promptAction.default": "#cba6f7",
    "promptAction.passPriority": "#cba6f7",
    "promptAction.passUntilEnd": "#b4befe",
    "promptAction.cancel": "#6c7086",
    "promptAction.pacificAction": "#89b4fa",
    "arrow.attack": "rgba(250, 179, 135, 0.88)",
    "arrow.block": "rgba(243, 139, 168, 0.88)",
    "arrow.hostileTarget": "rgba(243, 139, 168, 0.88)",
    "arrow.friendlyTarget": "rgba(137, 180, 250, 0.88)",
    "cardRing": "#f9e2af",
    ...buildGameColors(palette),
  },
};

export default preset;
