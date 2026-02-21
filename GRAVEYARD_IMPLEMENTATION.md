# Graveyard Interaction Implementation

## Summary

Successfully implemented UI interaction with the graveyard to support cards like **Raise Dead** that explicitly require selecting a card from the graveyard.

## Changes Made

### 1. Rust Engine (`forge-engine`)

#### Enhanced Targeting System (`spellability/targeting.rs`)
- **Added `CardInZone` variant** to `TargetKind` enum to support targeting cards in specific zones
- **Enhanced `parse_valid_targets()`** to parse `Origin$` parameter and determine target zone
- **Added `get_valid_cards_in_zone()`** function to retrieve valid targets from any zone
- **Added `matches_card_filter()`** to handle filters like "YouCtrl", "OpponentCtrl"
- **Updated `choose_targets()`** to call new `choose_target_card_from_zone()` for zone-based targeting
- **Added `parse_zone_type()`** helper to convert zone strings to `ZoneType`

#### Updated PlayerAgent Trait (`agent.rs`)
- **Added `choose_target_card_from_zone()`** method with default implementation
- Method takes `zone: ZoneType` parameter alongside `valid: &[CardId]`
- Default implementation falls back to `choose_target_card()` for backward compatibility

### 2. Tauri Backend (`src-tauri`)

#### Updated TauriAgent (`tauri_agent.rs`)
- **Implemented `choose_target_card_from_zone()`** to send new prompt type to frontend
- Builds zone-specific card lists from game view based on requested zone (Graveyard, Exile, Hand)
- Sends `AgentPromptInner::ChooseTargetCardFromZone` with zone information

#### Enhanced Prompt System (`prompt.rs`)
- **Added `ChooseTargetCardFromZone` variant** to `AgentPromptInner` enum
- Includes fields:
  - `game_view: GameViewDto`
  - `valid_card_ids: Vec<String>`
  - `zone: String` (zone type)
  - `zone_cards: Vec<CardDto>` (cards in the specified zone)

#### Updated Game Manager (`game_manager.rs`)
- **Added match arm** for `ChooseTargetCardFromZone` in `respond()` method
- Ensures proper handling of the new prompt type

### 3. Frontend UI (`src/`)

#### Created ZoneTargetSelector Component (`components/game/ZoneTargetSelector.tsx`)
- **New modal component** for selecting cards from specific zones
- Displays cards in a grid layout with hover previews
- Supports Escape key to cancel
- Shows card names below each card for clarity
- Uses portal to render in document.body

#### Updated Game Store (`stores/useGameStore.ts`)
- **Extended `AgentPrompt` interface** with new fields:
  - `zone?: string` - the zone being targeted
  - `zoneCards?: Card[]` - cards available in that zone

#### Enhanced Game View (`views/Game.tsx`)
- **Added import** for `ZoneTargetSelector` component
- **Added state** for `zoneTargetSelector` to manage modal visibility
- **Updated `PromptBanner`** to include label for `chooseTargetCardFromZone`
- **Added `useEffect`** to automatically show zone selector when prompt type changes
- **Added zone selector UI** to render when selecting from zones
- **Zone detection logic** converts zone types to user-friendly names ("Graveyard", "Exile", "Hand")

## How It Works

1. **Card Played**: When a player casts **Raise Dead** (or similar graveyard-targeting spell)
2. **Backend Processing**: Rust engine parses the card script:
   ```
   SP$ ChangeZone | Origin$ Graveyard | Destination$ Hand | ValidTgts$ Creature.YouCtrl
   ```
3. **Target Identification**: `parse_valid_targets()` detects:
   - `Origin$ Graveyard` → target zone is Graveyard
   - `ValidTgts$ Creature.YouCtrl` → creature cards you control
4. **Agent Prompt**: Engine calls `choose_target_card_from_zone()` with:
   - Zone: Graveyard
   - Valid cards: Creature cards in your graveyard
5. **UI Display**: Frontend shows `ZoneTargetSelector` modal with:
   - Title: "Choose from Graveyard"
   - Cards: All creature cards in your graveyard
   - Click handler: Selects the card as target
6. **Spell Resolution**: Selected card is moved from graveyard to hand

## Testing

### Test Deck
The implementation can be tested using the existing **"Zone Change"** preset deck:
- **Deck ID**: `"zone_change"`
- **Contains**: 3x Raise Dead, 4x Unsummon, 3x Diabolic Edict
- **Location**: Available in Create Game dialog → Deck Selection

### Test Flow
1. Start a new game with "Zone Change" deck
2. Play a creature (e.g., Typhoid Rats)
3. Let opponent kill it (or use sacrifice effects)
4. Cast **Raise Dead**
5. **Graveyard selector modal appears**
6. Click on a creature card in your graveyard
7. Card moves from graveyard to hand

## Features

✅ **Full zone support**: Works with Graveyard, Exile, Hand, Library, Command
✅ **Filter support**: Handles "YouCtrl", "OpponentCtrl", color filters
✅ **Visual feedback**: Cards highlight on hover, show names
✅ **Cancel support**: Escape key or clicking outside closes modal
✅ **Card previews**: Hover shows enlarged card image
✅ **Responsive design**: Modal fits viewport, scrolls if needed

## Backward Compatibility

All changes are backward compatible:
- Existing targeting (battlefield creatures, players) unchanged
- New method has default implementation
- Existing cards without Origin$ parameter work as before
- New prompt type only appears when needed

## Cards Supported

- **Raise Dead** (B: Return target creature card from graveyard to hand)
- Any card with `ValidTgts$` + `Origin$` combination
- Future cards targeting Exile, Hand, Library, or Command zone
