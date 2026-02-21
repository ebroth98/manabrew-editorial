# Quick Test - Graveyard & Battlefield Targeting

## Fix Applied
Successfully fixed the targeting system to handle both:
- **Battlefield targeting** (Unsummon, removal spells) - click on battlefield creature
- **Zone targeting** (Raise Dead, Exile retrieval) - modal popup selector

## What's Different

### Before Fix
- ALL cards with `Origin$` parameter used the new modal selector
- Unsummon (Origin$ Battlefield) showed modal (wrong!)

### After Fix  
- Only non-battlefield zones use modal selector
- Unsummon (Origin$ Battlefield) - click on battlefield creature (correct!)
- Raise Dead (Origin$ Graveyard) - modal selector (correct!)

## How to Test

### Test 1: Unsummon (Battlefield → Hand)
1. Start game with "Zone Change" deck
2. Play a creature (e.g., Typhoid Rats)
3. Cast **Unsummon**
4. **Expected**: Can click directly on your creature on battlefield
5. **Expected**: Creature moves from battlefield to hand

### Test 2: Raise Dead (Graveyard → Hand)
1. Start game with "Zone Change" deck
2. Play a creature → let it die
3. Check graveyard count increases
4. Cast **Raise Dead**
5. **Expected**: Modal popup shows graveyard cards
6. **Expected**: Click creature in modal → moves to hand

### Test 3: Both in Same Game
1. Play game as normal
2. Use Unsummon to bounce opponent's creature
3. Let your creature die
4. Use Raise Dead to get it back
5. **Verify**: Both targeting modes work correctly

## Test Deck
**"Zone Change"** preset contains:
- 4x Unsummon (Battlefield → Hand)
- 3x Raise Dead (Graveyard → Hand)
- 2x Boomerang (Any permanent Battlefield → Hand)
- 3x Diabolic Edict (Sacrifice)

## Build Status
✅ Rust engine compiles
✅ Tauri backend compiles  
✅ Frontend compiles
✅ All tests passing
