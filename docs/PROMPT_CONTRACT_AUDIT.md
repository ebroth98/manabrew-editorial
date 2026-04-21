# Prompt Contract Audit

This document tracks whether the frontend can honor the same player decisions
from both supported engines.

The target architecture is one shared UI contract:

- Rust engine emits `AgentPromptInner` variants through `forge-agent-interface`.
- Java Forge emits Forge-native controller decisions through `forge-harness`.
- Java-specific prompt shapes are normalized before they reach React.
- React renders and answers backend-neutral prompt types only.

Java Forge is the reference surface because it already contains the mature rules
and prompt model. The end goal is not only that Java can drive the current UI;
Rust must also emit equivalent prompts for cards that currently resolve through
default agent behavior. A card is not truly supported if it "works" only because
the Rust agent silently picked the first mode, first card, minimum number, or a
default yes/no answer.

## Status Summary

The Rust engine and React UI have a shared prompt vocabulary. Java Forge is not
yet equivalent: the interactive Java bridge currently exposes only the core
gameplay subset and lets the rest fall back to deterministic controller choices.

Rust also has prompt gaps even when gameplay advances. Several `PlayerAgent`
methods still have default implementations that choose deterministically instead
of sending a prompt. Those defaults are useful for AI and parity runs, but they
should be audited carefully before calling the corresponding card support
interactive-complete.

Current Java bridge coverage:

- priority actions: covered by `chooseAction`
- mulligan keep/redraw: covered by `mulligan`
- London mulligan put-back: covered by `mulliganPutBack`
- discard: covered by `chooseDiscard`
- attacker declaration: covered by `chooseAttackers`
- generic card choice: covered by `chooseCardsForEffect`
- mode choice: covered by `chooseMode`
- optional trigger / confirmation: covered by `chooseOptionalTrigger`
- color/type/card-name/number choices: covered by existing scalar prompts

The main project risk is not React knowing about Java. The risk is Java Forge
making player decisions through `DeterministicController` defaults instead of
publishing a normalized prompt.

## Contract Coverage

| Decision concept | UI prompt contract | Rust prompt | React UI | Java harness | Java normalizer | Priority |
| --- | --- | --- | --- | --- | --- | --- |
| State update | `stateUpdate` | covered | covered | covered via snapshot/prompt relay | covered | done |
| Game over | `gameOver` | covered | covered | partial via snapshot only | partial | medium |
| Mulligan keep/redraw | `mulligan` | covered | covered | covered | covered | done |
| London mulligan put-back | `mulliganPutBack` | covered | covered | covered | covered | done |
| Priority action | `chooseAction` | covered | covered | covered | covered | done |
| Play/cast option modes | `chooseAction.playableOptions` | covered | covered | partial, action labels only | partial | high |
| Activated abilities | `chooseAction.activatableAbilityIds` | covered | covered | covered for priority actions | covered | done |
| Mana abilities outside cost payment | `chooseAction.manaAbilityOptions` | covered | covered | covered for priority actions | covered | done |
| Interactive mana payment | `payManaCost` | covered | covered | missing | missing | high |
| Specify mana combo | `specifyManaCombo` | covered | covered | missing | missing | medium |
| Declare attackers | `chooseAttackers` | covered | covered | covered | covered | done |
| Choose attack defenders | `chooseAttackers.possibleDefenderIds` | covered | covered | covered | covered | done |
| Exert attackers | `chooseExertAttackers` | covered | covered | missing | missing | medium |
| Enlist attackers | `chooseEnlistAttackers` | covered | covered | missing | missing | medium |
| Pay attack/combat cost | `payCombatCost` | covered | covered | missing | missing | high |
| Declare blockers | `chooseBlockers` | covered | covered | covered | covered | done |
| Blocker damage order | `chooseDamageAssignmentOrder` | covered | covered | covered for attacker damage order | covered | done |
| Exact combat damage assignment | `chooseCombatDamageAssignment` | covered | covered | covered | covered | done |
| Target player | `chooseTargetPlayer` | covered | covered | covered | covered | done |
| Target card | `chooseTargetCard` | covered | covered | covered | covered | done |
| Target card from zone | `chooseTargetCardFromZone` | covered | covered | missing | missing | high |
| Target any | `chooseTargetAny` | covered | covered | covered | covered | done |
| Target stack spell | `chooseTargetSpell` | covered | covered | covered | covered | done |
| Retarget effect | target prompts | partial | partial | partial, via `chooseTargetsFor` | partial | medium |
| Reveal hidden cards | `revealCards` | covered | covered | covered | covered | done |
| Scry | `scry` | covered | covered | covered | covered | done |
| Surveil | `surveil` | covered | covered | covered | covered | done |
| Dig/look at top N | `dig` | covered | covered | covered for card choices | covered | done |
| Reorder library | `reorderLibrary` | covered | covered | covered for library destination | covered | done |
| Ordered top/bottom library split | needs contract extension | missing | missing | missing | missing | high |
| Generic discard | `chooseDiscard` | covered | covered | covered | covered | done |
| Random discard | no UI prompt expected | covered as engine choice | n/a | deterministic | n/a | low |
| Choose cards for effect | `chooseCardsForEffect` | covered | covered | covered for card collections | covered | done |
| Choose single entity for effect | `chooseCardsForEffect`, target prompt, or `chooseMode` fallback | covered | covered | covered for card entities | covered | done |
| Choose multiple entity groups | `chooseCardsForEffect` or `chooseMode` fallback | covered for flat entity sets | covered | missing | missing | medium |
| Choose cards for zone change | `chooseCardsForEffect` or `reorderLibrary` | covered for simple cases | covered | covered | covered | done |
| Choose permanents to sacrifice/destroy | `chooseCardsForEffect` | partial | partial | covered | covered | done |
| Choose mode | `chooseMode` | covered | covered | covered | covered | high |
| Choose spell ability for effect | `chooseMode` | covered | covered | covered via mode choice | covered | done |
| Optional trigger | `chooseOptionalTrigger` | covered | covered | covered | covered | high |
| Confirm action | `chooseOptionalTrigger` with `promptKind=confirm_action` | covered | covered | covered | covered | high |
| Confirm payment | `chooseOptionalTrigger` or cost prompt | partial | partial | covered | covered | done |
| Confirm replacement effect | `chooseOptionalTrigger` | covered | covered | covered | covered | done |
| Choose replacement/static effect | needs contract extension | missing | missing | replacement covered via mode choice; static deterministic | partial | medium |
| Pay cost to prevent effect | `payCostToPreventEffect` | covered | covered | covered | covered | done |
| Choose Phyrexian payment | `choosePhyrexian` | covered | covered | Java auto-pay path | n/a | medium |
| Choose kicker | `chooseKicker` | covered | covered | covered via optional-cost mode choice | covered | done |
| Choose buyback | `chooseBuyback` | covered | covered | covered via optional-cost mode choice | covered | done |
| Choose multikicker count | `chooseMultikicker` | covered | covered | covered via number choice | covered | done |
| Choose replicate count | `chooseReplicate` | covered | covered | covered via number choice | covered | done |
| Choose alternative cost | `chooseAlternativeCost` | covered | covered | covered via optional-cost mode choice | covered | done |
| Choose delve cards | `chooseDelve` | covered | covered | covered | covered | done |
| Choose convoke cards | `chooseConvoke` | covered | covered | covered | covered | done |
| Choose improvise cards | `chooseImprovise` | covered | covered | covered | covered | done |
| Choose color | `chooseColor` | covered | covered | covered | covered | high |
| Choose multiple colors | `chooseMode` fallback | covered | covered | missing | missing | medium |
| Choose type | `chooseType` | covered | covered | covered | covered | high |
| Choose card name | `chooseCardName` | covered | covered | covered for explicit face lists | covered | high |
| Choose number | `chooseNumber` | covered | covered | covered for min/max | covered | high |
| Choose number from explicit list | `chooseMode` fallback | covered | covered | missing | missing | medium |
| Choose X value | `chooseNumber` | covered | covered | deterministic bounds still used | deterministic | medium |
| Explore decision | `exploreDecision` | covered | covered | missing | missing | low |
| Assist payment | `helpPayAssist` | covered | covered | missing | missing | low |
| Choose starting player/hand | no current shared prompt | missing | missing | deterministic | missing | low |
| Choose card face/state | needs contract extension | missing | missing | missing | missing | medium |
| Choose pile | needs contract extension | missing | missing | missing | missing | high |
| Vote | needs contract extension | missing | missing | missing | missing | low |
| Dice choices | `chooseMode`/`chooseNumber` fallback | covered | covered | missing | missing | low |
| Contraption/sector/sprocket choices | needs contract extension | missing | missing | missing | missing | low |
| Protection/keyword/counter type choice | `chooseType` or option-list `chooseMode` | covered for counter/type paths | covered | missing | missing | medium |

## Java Hook Groups To Wire

The Java reference surface is `forge.game.player.PlayerController`. The
interactive Java controller should override hooks in this order.

### Phase 1: Generic Choice And Confirmation

These unlock many cards without adding Java-specific React paths.

- `chooseCardsForEffect`
- `chooseSingleEntityForEffect`
- `chooseEntitiesForEffect`
- `chooseCardsForZoneChange`
- `chooseSingleCardForZoneChange`
- `confirmAction`
- `confirmTrigger`
- `confirmReplacementEffect`
- `confirmPayment`
- `chooseModeForAbility`
- `chooseNumber`
- `chooseColor`
- `chooseSomeType`
- `chooseCardName`

Preferred normalized contracts:

- `chooseCardsForEffect`
- `chooseOptionalTrigger`
- `chooseMode`
- `chooseNumber`
- `chooseColor`
- `chooseType`
- `chooseCardName`
- targeting prompts when the choice is explicitly target selection

### Phase 2: Library And Hidden-Zone Decisions

These are needed for cards such as Lim-Dul's Vault, Ponder-style effects, and
top/bottom ordering decisions.

- `reveal`
- `arrangeForScry`
- `arrangeForSurveil`
- `orderMoveToZoneList`
- `willPutCardOnTop`

Preferred normalized contracts:

- `revealCards`
- `scry`
- `surveil`
- `dig`
- `reorderLibrary`
- a new backend-neutral ordered top/bottom split prompt, if `reorderLibrary`
  cannot express the decision without losing destination information

### Phase 3: Combat And Cost Completeness

These are needed for parity with the Rust combat/cost prompt set.

- `declareBlockers`
- `orderBlockers`
- `orderBlocker`
- `orderAttackers`
- `assignCombatDamage`
- `exertAttackers`
- `enlistAttackers`
- `payCombatCost`
- `payManaCost`
- `chooseCardsToDelve`
- `chooseCardsForConvokeOrImprovise`
- `specifyManaCombo`

Preferred normalized contracts:

- `chooseBlockers`
- `chooseDamageAssignmentOrder`
- `chooseCombatDamageAssignment`
- `chooseExertAttackers`
- `chooseEnlistAttackers`
- `payCombatCost`
- `payManaCost`
- `chooseDelve`
- `chooseConvoke`
- `chooseImprovise`
- `specifyManaCombo`

### Phase 4: Rare Mechanics

These should be explicit but can wait until the common prompt surface is stable.

- `chooseCardsPile`
- `vote`
- dice choice hooks
- `chooseSector`
- `chooseContraptionsToCrank`
- `chooseSprocket`
- `chooseSingleReplacementEffect`
- `chooseSingleStaticAbility`
- `chooseCounterType`
- `chooseKeywordForPump`
- `chooseProtectionType`

## Definition Of Done

A prompt is engine-agnostic only when all of these are true:

- Rust can emit the prompt, or the gap is explicitly not applicable.
- Java can emit an equivalent prompt from the relevant `PlayerController` hook.
- Tauri local Java translates the frontend response back to Java.
- Self-hosted Java translates the frontend response back to Java.
- React has a renderer and response path for the prompt.
- The prompt payload contains enough labels, card IDs, min/max bounds, and zone
  context for the player to make the same choice as in original Forge.
- Pass/cancel behavior is explicit.
- At least one representative card or game situation is named for manual
  verification.

## Representative Cards And Situations

Use these to drive prompt parity work:

- Lim-Dul's Vault: repeated look/reorder/top-bottom/library loop.
- Palantir of Orthanc: opponent choice, reveal, and repeated trigger decisions.
- Ponder, Preordain, Brainstorm: reveal, choose, reorder, top/bottom.
- Fact or Fiction: pile split and pile choice.
- Counterspell: stack target selection.
- Lightning Bolt / Doom Blade: card/player target selection.
- Modal charms: mode choice.
- Propaganda / Ghostly Prison: attack cost payment.
- Delve, convoke, improvise cards: alternate cost card tapping/exiling.
- Multi-block combat with trample/deathtouch: damage order and assignment.
