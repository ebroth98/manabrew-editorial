import type { ThemePreset } from "./index";
import { buildGameColors, type BasePalette } from "./buildGameColors";

/** Kanagawa palette — wave-inspired; muted autumn-meets-spring hues. */
const palette: BasePalette = {
  foreground: "#dcd7ba",         // fujiWhite
  labelMuted: "#727169",         // fujiGray
  labelGhost: "#9e9b93",
  placeholderFill: "#1f1f28",    // sumiInk1
  placeholderStroke: "#454658",
  canvasBackground: "#1f1f28",
  red:      "#c34043",           // autumnRed
  redDeep:  "#8b2d30",
  orange:   "#dca561",           // carpYellow (warm orange-ish)
  amber:    "#e6c384",           // boatYellow2
  yellow:   "#e6c384",
  green:    "#76946a",           // autumnGreen
  teal:     "#7aa89f",
  cyan:     "#7fb4ca",           // springBlue
  blue:     "#7e9cd8",           // crystalBlue
  sky:      "#7fb4ca",
  indigo:   "#957fb8",           // oniViolet
  violet:   "#957fb8",
  purple:   "#957fb8",
  pink:     "#d27e99",           // sakuraPink
  slate:    "#54546d",
  brown:    "#dca561",
  paper:    "#b8b4d0",           // waveAqua2
  poison:   "#5a7250",           // darker moss sibling of kanagawa green
  manaW:    "#dcd7ba",
  manaU:    "#7e9cd8",
  manaB:    "#454658",
  manaR:    "#c34043",
  manaG:    "#76946a",
  manaC:    "#9e9b93",
};

const preset: ThemePreset = {
  id: "kanagawa",
  name: "Kanagawa",
  description: "Japanese ink-wash inspired, deep indigo and warm accents",
  light: {
    background: "220 18% 95%",
    foreground: "228 20% 20%",
    card: "220 18% 100%",
    "card-foreground": "228 20% 20%",
    popover: "220 18% 100%",
    "popover-foreground": "228 20% 20%",
    primary: "220 50% 48%",
    "primary-foreground": "0 0% 100%",
    secondary: "220 16% 90%",
    "secondary-foreground": "228 20% 20%",
    muted: "220 14% 92%",
    "muted-foreground": "228 12% 46%",
    accent: "220 14% 88%",
    "accent-foreground": "228 20% 20%",
    destructive: "348 72% 50%",
    "destructive-foreground": "0 0% 100%",
    border: "220 14% 82%",
    input: "220 14% 86%",
    ring: "220 50% 48%",
    selection: "270 50% 55%",
    "selection-foreground": "0 0% 100%",
    commander: "30 80% 48%",
    warning: "30 75% 48%",
    overlay: "228 20% 8%",
  },
  dark: {
    background: "228 20% 12%",
    foreground: "39 14% 74%",
    card: "228 18% 15%",
    "card-foreground": "39 14% 74%",
    popover: "228 18% 17%",
    "popover-foreground": "39 14% 74%",
    primary: "220 50% 62%",
    "primary-foreground": "228 20% 12%",
    secondary: "228 16% 20%",
    "secondary-foreground": "39 14% 74%",
    muted: "228 16% 20%",
    "muted-foreground": "228 10% 46%",
    accent: "228 16% 23%",
    "accent-foreground": "39 14% 74%",
    destructive: "348 60% 48%",
    "destructive-foreground": "39 14% 85%",
    border: "228 14% 22%",
    input: "228 14% 20%",
    ring: "220 50% 62%",
    selection: "270 50% 58%",
    "selection-foreground": "39 14% 85%",
    commander: "30 70% 52%",
    warning: "30 65% 50%",
    overlay: "228 20% 4%",
  },
  gameColors: {
    "activeAction.priority": "#957fb8",
    "activeAction.turnText": "#dca561",
    "activeAction.myTurnRing": "#dca561",
    "activeAction.opponentTurnRing": "#dca561",
    "highlight": "#ff5d62",
    "hand.playableBorder": "rgba(192, 181, 162, 0.7)",
    "promptAction.default": "#7e9cd8",
    "promptAction.passPriority": "#7e9cd8",
    "promptAction.passUntilEnd": "#658594",
    "promptAction.cancel": "#727169",
    "promptAction.pacificAction": "#7fb4ca",
    "arrow.attack": "rgba(220, 165, 97, 0.88)",
    "arrow.block": "rgba(255, 93, 98, 0.88)",
    "arrow.hostileTarget": "rgba(255, 93, 98, 0.88)",
    "arrow.friendlyTarget": "rgba(127, 180, 202, 0.88)",
    "cardRing": "#dca561",
    ...buildGameColors(palette),
  },
};

export default preset;
