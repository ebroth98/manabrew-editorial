import type { ThemePreset } from "./index";
import { buildGameColors, type BasePalette } from "./buildGameColors";

/** Rose Pine palette — muted iris, rose, and pine tones.  The canonical
 *  palette has no dedicated green; a desaturated mint approximation is
 *  used for the buff/p1p1 slot so the whole game stays harmonious. */
const palette: BasePalette = {
  foreground: "#e0def4",         // text
  labelMuted: "#6e6a86",         // muted
  labelGhost: "#908caa",         // subtle
  placeholderFill: "#1f1d2e",    // surface
  placeholderStroke: "#26233a",  // overlay
  canvasBackground: "#191724",   // base
  red:      "#eb6f92",           // love
  redDeep:  "#b04567",
  orange:   "#f6c177",           // gold
  amber:    "#f6c177",
  yellow:   "#f6c177",
  green:    "#95b1a7",           // custom desaturated mint
  teal:     "#31748f",           // pine
  cyan:     "#9ccfd8",           // foam
  blue:     "#31748f",
  sky:      "#9ccfd8",
  indigo:   "#c4a7e7",           // iris
  violet:   "#c4a7e7",
  purple:   "#c4a7e7",
  pink:     "#ebbcba",           // rose
  slate:    "#6e6a86",
  brown:    "#f6c177",
  paper:    "#e0def4",
  poison:   "#70978a",           // darker mint sibling of rose-pine green
  manaW:    "#ebbcba",
  manaU:    "#31748f",
  manaB:    "#403d52",
  manaR:    "#eb6f92",
  manaG:    "#95b1a7",
  manaC:    "#908caa",
};

const preset: ThemePreset = {
  id: "rose-pine",
  name: "Rose Pine",
  description: "Warm, muted tones inspired by Rose Pine",
  light: {
    background: "32 57% 95%",
    foreground: "248 19% 25%",
    card: "40 23% 99%",
    "card-foreground": "248 19% 25%",
    popover: "40 23% 99%",
    "popover-foreground": "248 19% 25%",
    primary: "280 40% 48%",
    "primary-foreground": "0 0% 100%",
    secondary: "32 30% 90%",
    "secondary-foreground": "248 19% 25%",
    muted: "32 25% 92%",
    "muted-foreground": "248 10% 48%",
    accent: "32 30% 88%",
    "accent-foreground": "248 19% 25%",
    destructive: "343 76% 52%",
    "destructive-foreground": "0 0% 100%",
    border: "32 20% 84%",
    input: "32 20% 87%",
    ring: "280 40% 48%",
    selection: "280 40% 55%",
    "selection-foreground": "0 0% 100%",
    commander: "35 88% 52%",
    warning: "35 80% 50%",
    overlay: "248 19% 10%",
  },
  dark: {
    background: "249 22% 12%",
    foreground: "245 7% 81%",
    card: "247 23% 15%",
    "card-foreground": "245 7% 81%",
    popover: "246 24% 17%",
    "popover-foreground": "245 7% 81%",
    primary: "280 40% 68%",
    "primary-foreground": "249 22% 12%",
    secondary: "247 16% 20%",
    "secondary-foreground": "245 7% 81%",
    muted: "247 16% 20%",
    "muted-foreground": "245 7% 55%",
    accent: "247 16% 23%",
    "accent-foreground": "245 7% 81%",
    destructive: "343 60% 48%",
    "destructive-foreground": "245 7% 90%",
    border: "247 15% 22%",
    input: "247 15% 20%",
    ring: "280 40% 68%",
    selection: "280 40% 62%",
    "selection-foreground": "245 7% 90%",
    commander: "35 75% 55%",
    warning: "35 70% 50%",
    overlay: "249 22% 5%",
  },
  gameColors: {
    "activeAction.priority": "#c4a7e7",
    "activeAction.turnText": "#f6c177",
    "activeAction.myTurnRing": "#f6c177",
    "activeAction.opponentTurnRing": "#f6c177",
    "highlight": "#ea9a97",
    "hand.playableBorder": "rgba(224, 222, 244, 0.7)",
    "promptAction.default": "#c4a7e7",
    "promptAction.passPriority": "#c4a7e7",
    "promptAction.passUntilEnd": "#907aa9",
    "promptAction.cancel": "#6e6a86",
    "promptAction.pacificAction": "#9ccfd8",
    "arrow.attack": "rgba(246, 193, 119, 0.88)",
    "arrow.block": "rgba(235, 111, 146, 0.88)",
    "arrow.hostileTarget": "rgba(235, 111, 146, 0.88)",
    "arrow.friendlyTarget": "rgba(156, 207, 216, 0.88)",
    "cardRing": "#f6c177",
    ...buildGameColors(palette),
  },
};

export default preset;
