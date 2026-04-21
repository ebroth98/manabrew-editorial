import type { ThemePreset } from "./index";

const preset: ThemePreset = {
  id: "default",
  name: "Default",
  description: "Clean slate-tinted material design",
  light: {
    background: "214 22% 96%",
    foreground: "220 18% 14%",
    card: "0 0% 100%",
    "card-foreground": "220 18% 14%",
    popover: "0 0% 100%",
    "popover-foreground": "220 18% 14%",
    primary: "220 18% 14%",
    "primary-foreground": "0 0% 100%",
    secondary: "214 16% 90%",
    "secondary-foreground": "220 18% 14%",
    muted: "214 16% 93%",
    "muted-foreground": "215 12% 46%",
    accent: "214 16% 90%",
    "accent-foreground": "220 18% 14%",
    destructive: "0 78% 55%",
    "destructive-foreground": "0 0% 100%",
    border: "214 18% 84%",
    input: "214 18% 88%",
    ring: "220 18% 40%",
    selection: "270 60% 65%",
    "selection-foreground": "0 0% 100%",
    commander: "45 93% 47%",
    warning: "38 92% 50%",
    overlay: "0 0% 0%",
  },
  dark: {
    background: "220 7% 8%",
    foreground: "214 12% 92%",
    card: "220 7% 11%",
    "card-foreground": "214 12% 92%",
    popover: "220 7% 13%",
    "popover-foreground": "214 12% 92%",
    primary: "214 12% 84%",
    "primary-foreground": "220 7% 8%",
    secondary: "220 7% 16%",
    "secondary-foreground": "214 12% 92%",
    muted: "220 7% 16%",
    "muted-foreground": "214 8% 52%",
    accent: "220 7% 19%",
    "accent-foreground": "214 12% 92%",
    destructive: "0 68% 48%",
    "destructive-foreground": "214 12% 92%",
    border: "220 7% 20%",
    input: "220 7% 18%",
    ring: "214 12% 68%",
    selection: "270 60% 65%",
    "selection-foreground": "214 12% 92%",
    commander: "45 80% 55%",
    warning: "38 80% 50%",
    overlay: "0 0% 0%",
  },
  gameColors: {
    "activeAction.priority": "#a855f7",
    "activeAction.turnText": "#f59e0b",
    "activeAction.myTurnRing": "#f59e0b",
    "activeAction.opponentTurnRing": "#f59e0b",
    "highlight": "#fb923c",
    "hand.playableBorder": "rgba(255, 255, 255, 0.7)",
    "promptAction.default": "#7c3aed",
    "promptAction.passPriority": "#7c3aed",
    "promptAction.passUntilEnd": "#5b21b6",
    "promptAction.cancel": "#6b7280",
    "promptAction.pacificAction": "#60a5fa",
    "arrow.attack": "rgba(255, 138, 0, 0.88)",
    "arrow.block": "rgba(210, 40, 40, 0.88)",
    "arrow.hostileTarget": "rgba(210, 40, 40, 0.88)",
    "arrow.friendlyTarget": "rgba(90, 150, 255, 0.88)",
    "cardRing": "#f59e0b",

    // ── Targeting pointer colours ─────────────────────────────────────
    // Pointer palette is intentionally binary. The icon glyph already
    // carries the specific semantic (skull = sacrifice, bolt = damage,
    // …), so the glow only signals valence: `hostile` for anything that
    // acts against the target, `friendly` for supportive effects. The
    // mapping from intent → hostile/friendly lives in
    // `intentIsHostile()` in `@/types/promptType`.
    "pointer.hostile":  "rgba(210, 40, 40, 0.88)",
    "pointer.friendly": "rgba(90, 150, 255, 0.88)",

    // ── Mana symbol tints ────────────────────────────────────────────
    // Opaque base colour per mana letter. React + Pixi consumers apply
    // their own alpha on top for pip backgrounds / tap-button fills.
    "mana.W": "#f8f6d8", // white / plains
    "mana.U": "#c1d7e9", // blue / island
    "mana.B": "#bab1ab", // black / swamp
    "mana.R": "#eb9f82", // red / mountain
    "mana.G": "#c4d3ca", // green / forest
    "mana.C": "#cccac7", // colourless

    // ── Generic text colours ─────────────────────────────────────────
    // `textOnTinted` is used where text sits on top of a coloured chip
    // (P/T badges, counter chips, warning pills). `textMuted` and
    // `textGhost` are subdued labels drawn straight on the canvas
    // surface — zone placeholders and ghost-loading card names.
    "textOnTinted": "#ffffff",
    "textMuted":    "#666666",
    "textGhost":    "#888888",

    // ── Canvas-level neutrals ────────────────────────────────────────
    // `background` paints the empty Pixi surface. `shadow` is the drop-
    // shadow ink (nearly always black). `neutral` is the stroke colour
    // used around arrowheads, icons, and similar foreground marks.
    "canvas.background": "#0d1117",
    "canvas.shadow":     "#000000",
    "canvas.neutral":    "#ffffff",

    // ── Card-sprite placeholder ─────────────────────────────────────
    // Colours used while a card's image is still loading.
    "cardPlaceholder.fill":   "#1a1a2e",
    "cardPlaceholder.stroke": "#444466",

    // ── P/T badge backgrounds ────────────────────────────────────────
    "pt.neutral":  "#6b7280", // baseline / no stat change
    "pt.lethal":   "#dc2626", // damage ≥ toughness — pending death
    "pt.buffed":   "#22c55e", // above base P/T
    "pt.debuffed": "#dc2626", // below base P/T

    // ── Generic status signals ────────────────────────────────────────
    // Semantic tokens for UI states that aren't creature-specific.
    // Reserve `pt.*` for actual Power/Toughness badges.
    "success": "#22c55e", // connected, saved, win, good FPS
    "poison":  "#65a30d", // MTG infect-green for poison counters
    "life":    "#dc2626", // heart / life-total indicator

    // ── Card status ring / badge colours ─────────────────────────────
    // Each permanent state draws its own small badge in the pixi card
    // sprite. Colours are semantic, not literal card-border colours.
    "cardStatus.exerted":     "#f97316", // won't untap — warning orange
    "cardStatus.morph":       "#4b5563", // face-down / hidden — slate
    "cardStatus.bestow":      "#14b8a6", // aura mode — teal
    "cardStatus.token":       "#fbbf24", // not a real card — amber
    "cardStatus.transformed": "#a855f7", // DFC back face — purple
    "cardStatus.plotted":     "#6366f1", // plotted in exile — indigo
    "cardStatus.madness":     "#dc2626", // madness-exiled — red
    "cardStatus.warped":      "#0891b2", // warp-exiled — cyan

    // ── Counter chip colours ─────────────────────────────────────────
    // Per counter type; `default` covers any unknown type.
    "counter.default":   "#4b5563",
    "counter.p1p1":      "#22c55e", // +1/+1 — growth green
    "counter.m1m1":      "#dc2626", // -1/-1 — wither red
    "counter.loyalty":   "#3b82f6", // planeswalker loyalty
    "counter.charge":    "#a855f7", // charge / ice etc.
    "counter.quest":     "#facc15",
    "counter.study":     "#06b6d4",
    "counter.lore":      "#f59e0b", // sagas
    "counter.age":       "#78716c",
    "counter.time":      "#6366f1",
    "counter.fade":      "#64748b",
    "counter.level":     "#f97316", // level-up creatures
    "counter.storage":   "#14b8a6",
    "counter.mining":    "#a16207",
    "counter.brick":     "#9a3412",
    "counter.depletion": "#be123c",
    "counter.page":      "#a1a1aa", // book room
  },
};

export default preset;
