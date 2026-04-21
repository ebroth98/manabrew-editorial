# UI Style Guidelines

This document defines coding conventions for all React/TypeScript UI code under `src/`.
Follow these rules when creating or modifying components.

---

## 1. File Organization

### Component files
- **One exported component per file.** Internal helpers (sub-components used only inside that file) are fine, but if they grow past ~30 lines, extract them.
- **Max ~200 lines per component file.** If a file grows larger, split it into focused sub-components.
- Files that contain 5+ modal variants (like `CostModal.tsx`) should be split into individual files or grouped in a subdirectory.

### Shared modules
| File | Purpose |
|------|---------|
| `game.types.ts` | Shared TypeScript interfaces and type aliases |
| `game.constants.ts` | Magic numbers, phase definitions, color arrays, sizing values |
| `game.styles.ts` | Reusable Tailwind class-string constants |
| `game.utils.ts` | Pure utility functions (no React) |

When adding a new constant, type, or utility, check these files first — do not duplicate.

---

## 2. Tailwind Class Management

### Use named constants for repeated classes
If the same Tailwind string appears in **2+ files**, extract it to `game.styles.ts`:

```ts
// game.styles.ts
export const BATTLEFIELD_CARD = "w-[70px] h-[98px] shrink-0" as const;
```

```tsx
// BattlefieldZone.tsx
<Card className={cn(BATTLEFIELD_CARD, "hover:z-10")} />
```

### Always use `cn()` for conditional classes
Never use template literals for conditional Tailwind. Use `cn()` from `@/lib/utils`:

```tsx
// ✅ Good
className={cn("text-sm", isActive && "font-bold")}

// ❌ Bad
className={`text-sm ${isActive ? "font-bold" : ""}`}
```

### Never generate Tailwind classes dynamically
Tailwind's JIT compiler cannot detect dynamic class names. Always use full static strings:

```tsx
// ✅ Good
const ringClass = isSelected ? "ring-blue-400" : "ring-gray-300";

// ❌ Bad
const color = "blue";
className={`ring-${color}-400`}
```

### Card sizing constants
Use the standard size constants — don't invent new pixel values:

| Constant | Value | Usage |
|----------|-------|-------|
| `BATTLEFIELD_CARD` | `w-[70px] h-[98px]` | Cards on the battlefield |
| `HAND_CARD` | `w-[80px] h-[112px]` | Cards in hand / zone viewer |
| `MODAL_CARD_SIZE` | `w-[100px] h-[140px]` | Cards inside modal grids |
| `FLASH_CARD_SIZE` | `w-[240px] h-[336px]` | Flash overlay / large preview |

---

## 3. Component Patterns

### Modal structure
All game modals should use the `Modal` compound component:

```tsx
<Modal onClose={onCancel} maxWidth="max-w-md">
  <Modal.Header>
    <h2 className={MODAL_TITLE}>Title</h2>
  </Modal.Header>
  <Modal.Instructions>Instruction text</Modal.Instructions>
  <Modal.Body>{/* content */}</Modal.Body>
  <Modal.Footer>{/* buttons */}</Modal.Footer>
</Modal>
```

- Use `MODAL_CARD_THUMBNAIL` for small card images in headers.
- Use `MODAL_CARD_IMAGE` for larger card images in bodies.
- Use `MODAL_FOOTER_BETWEEN` for footers with left info + right buttons.

### Card image in modal headers
When showing a source card thumbnail alongside a modal title:

```tsx
<Modal.Header>
  <div className="flex items-center gap-3">
    {imageUrl && (
      <CardImageThumbnail
        imageUrl={imageUrl}
        cardName={name}
        className={MODAL_CARD_THUMBNAIL}
      />
    )}
    <div>
      <h2 className="font-semibold text-base">Title</h2>
      <p className="text-xs text-muted-foreground">{subtitle}</p>
    </div>
  </div>
</Modal.Header>
```

### Mana text rendering
Use the shared `TextWithMana` component for any text that may contain `{W}`, `{2}{R}`, etc.:

```tsx
import { TextWithMana } from "@/components/game/TextWithMana";
<TextWithMana text={description} manaSize="sm" />
```

---

## 4. Custom Hooks

### Extract repeated stateful logic into hooks
If 2+ components share the same `useState` + `useEffect` pattern, extract a custom hook:

- **`useModalKeyboard(handlers)`** — for Enter/Escape key handling in modals
- **`useCardSelection(options)`** — for toggle-to-select card sets with min/max constraints
- **`useCardPreview(dismissDeps)`** — unified mouse hover → card preview pattern with sticky & action support
- **`HoverCardPreview`** — standard component to render the preview portal

### Hook file naming
Place hooks in `src/hooks/` if they're app-wide, or co-locate as `useXxx.ts` next to the component if they're component-specific.

---

## 5. Types

### Import types with `type` keyword
Always use `import type` for type-only imports:

```tsx
// ✅ Good
import type { Card as CardType } from "@/types/openmagic";

// ❌ Bad
import { Card as CardType } from "@/types/openmagic";
```

### Props interfaces
- Define props interfaces inline if the component is the only consumer.
- Move to `game.types.ts` if 2+ files reference the same interface.
- Name them `ComponentNameProps` (e.g., `PlayerPanelProps`).

---

## 6. State Management

### Keep state close to where it's used
Don't hoist state to Game.tsx unless multiple sibling components need it. Prefer local `useState` inside the component that owns the behavior.

### Zustand store (`useGameStore`)
Only store data that needs to persist across component unmounts or be accessed from non-React code. UI-only state (hover, modal open, panel collapsed) stays local.

---

## 7. Avoid Over-Engineering

- Don't create abstractions for one-time patterns. Three similar lines are better than a premature abstraction.
- Don't add error boundaries, loading skeletons, or fallback UI unless the user requests it.
- Don't add comments to self-explanatory code. Only comment *why*, not *what*.
- Don't add `aria-*` attributes speculatively — add them when accessibility is specifically requested.

---

## 8. Imports

### Order
1. React / third-party libraries
2. UI components (`@/components/ui/`)
3. Game components (`@/components/game/`)
4. Shared game modules (`./game.types`, `./game.styles`, etc.)
5. Hooks, stores, utils
6. Types (with `import type`)

### Path aliases
Always use `@/` path aliases. Never use `../../` relative paths that escape the current directory.

---

## 9. Colors — NEVER hardcode

**All colours must be theme-driven. No hex / rgb / rgba / hsl literals
anywhere in component or Pixi code.** Every colour the user sees comes
from a preset file under `src/themes/` and flows through one of two
resolvers:

| Surface | Source of truth | Hook / util |
|---------|-----------------|-------------|
| Light / dark app chrome (Radix tokens) | `ThemePreset.light` / `ThemePreset.dark` HSL maps | `useAppTheme()` in `src/hooks/useAppTheme.ts` |
| Game / Pixi board colours | `ThemePreset.gameColors` (dot-path record) | `useGameThemeColors()` in `src/components/game/game.theme.ts` |

### Where colours live

- **Preset files** (`src/themes/<name>.ts`) declare every semantic colour
  for that preset. Default values live in `src/themes/default.ts`; every
  other preset inherits unset keys from the default via the chain in
  `resolveGameThemeColors`.
- **Schema** (`GameThemeColors` in `src/components/game/game.theme.ts`)
  defines the typed shape with semantic groupings (`pointer.*`,
  `mana.*`, `cardStatus.*`, `counter.*`, `pt.*`, `canvas.*`,
  `cardPlaceholder.*`, `textOnTinted` / `textMuted` / `textGhost`, …).
  Adding a new surface means adding it to this interface **and** the
  default preset.
- **Pixi adapter** (`src/pixi/themeAdapter.ts`) converts strings to the
  numeric form Pixi expects. Pixi layers read from `PixiThemeColors`
  and never from literal hex.

### Rules

1. **No `#RRGGBB`, `rgba(…)`, `hsl(…)`, or `0xRRGGBB` literals in
   source files.** Pull every colour from the theme.
2. **No tailwind palette classes that carry semantic colour** (e.g.
   `ring-red-500`, `bg-blue-400`). Use the theme-token utility classes
   instead: `bg-pointer-hostile`, `text-counter-p1p1`, `ring-card-ring`,
   `text-destructive`, `bg-pt-buffed`, etc. Every key in
   `GameThemeColors` has a matching `bg-*` / `text-*` / `ring-*` /
   `border-*` utility generated via `@theme` in `src/index.css`. If a
   colour you need isn't a theme token, add one — don't reach for the
   palette.

   *Narrow identity-palette exception.* Two files intentionally keep
   tailwind palette names to lock in a stable visual identity across
   theme changes:
   - `FormatBadge.tsx` — per-format brand hues (Modern = blue, Pauper
     = rose, …). The colour *is* the identity.
   - `DeckVsSelector.tsx` — Player 1 = blue, Player 2 = red slot
     assignments that must stay distinguishable regardless of preset.

   Any other palette usage should route through the theme.
3. **Absolute fallbacks live only in the merged map** inside
   `resolveGameThemeColors` — never sprinkle fallback literals in
   components. If a preset omits a key, the resolver chain handles it.
4. **Pixi scene classes seed a non-null theme at construction time**
   (`adaptTheme(getGameThemeColors())`) so every draw call can read
   theme values directly — no `this.theme?.X ?? FALLBACK` patterns.
5. **The one narrow exception**: pure `rgba(0, 0, 0, X)` shadow idioms
   in tailwind arbitrary classes (`shadow-[0_10px_30px_rgba(0,0,0,0.35)]`)
   are allowed when the shadow is intentionally physics-black. Any
   coloured shadow must go through the theme.

### When introducing a new colour

1. Add the schema field to `GameThemeColors` (and to `PixiThemeColors`
   via `themeAdapter` if it touches the canvas).
2. Add the default value to `src/themes/default.ts` under the new key.
3. Extend the path-template signatures in `src/themes/index.ts` so
   other preset authors can override via dot-paths.
4. Update any other preset that needs a custom value — presets without
   an override inherit from the default.

If you find yourself about to type a hex literal in a component, stop
and add a semantic theme key instead.

---

## 10. Naming Conventions

| Entity | Convention | Example |
|--------|-----------|---------|
| Component files | PascalCase | `PlayerPanel.tsx` |
| Shared module files | camelCase | `game.styles.ts` |
| Hook files | camelCase, `use` prefix | `useCardSelection.ts` |
| Style constants | UPPER_SNAKE_CASE | `BATTLEFIELD_CARD` |
| Type/Interface | PascalCase | `CombatAssignment` |
| Utility functions | camelCase | `getAvatarColor` |

---

## 11. Testing Checklist

Before committing UI changes:
1. `npx tsc -p tsconfig.app.json --noEmit` — must pass with zero errors
2. `npm run tauri dev` — app must build and render correctly
3. Visual spot-check: verify the changed components look identical to before
