# Forge Game Engine — Feature Mapping

> **769 Java files** in `forge/forge-game/src/main/java/forge/game/` mapped against **~37 Rust files** in `forge-engine/`.
>
> Legend: **Implemented** | **Partial** | Not implemented

---

## Table of Contents

1. [Core Game (`game/`)](#1-core-game-game)
2. [Ability System (`game/ability/`)](#2-ability-system-gameability)
3. [Ability Effects (`game/ability/effects/`)](#3-ability-effects-gameabilityeffects)
4. [Card System (`game/card/`)](#4-card-system-gamecard)
5. [Card — Perpetual Effects (`game/card/perpetual/`)](#5-card--perpetual-effects-gamecardperpetual)
6. [Card — Tokens (`game/card/token/`)](#6-card--tokens-gamecardtoken)
7. [Combat (`game/combat/`)](#7-combat-gamecombat)
8. [Costs (`game/cost/`)](#8-costs-gamecost)
9. [Events (`game/event/`)](#9-events-gameevent)
10. [Extra Hands (`game/extrahands/`)](#10-extra-hands-gameextrahands)
11. [Keywords (`game/keyword/`)](#11-keywords-gamekeyword)
12. [Mana (`game/mana/`)](#12-mana-gamemana)
13. [Mulligan (`game/mulligan/`)](#13-mulligan-gamemulligan)
14. [Phases (`game/phase/`)](#14-phases-gamephase)
15. [Player (`game/player/`)](#15-player-gameplayer)
16. [Player Actions (`game/player/actions/`)](#16-player-actions-gameplayeractions)
17. [Replacement Effects (`game/replacement/`)](#17-replacement-effects-gamereplacement)
18. [Spell Abilities (`game/spellability/`)](#18-spell-abilities-gamespellability)
19. [Static Abilities (`game/staticability/`)](#19-static-abilities-gamestaticability)
20. [Triggers (`game/trigger/`)](#20-triggers-gametrigger)
21. [Zones (`game/zone/`)](#21-zones-gamezone)
22. [forge-engine Rust Implementation Summary](#22-forge-engine-rust-implementation-summary)

---

## 1. Core Game (`game/`)

37 files — Core game state, lifecycle, rules, logging, and base abstractions.

| Java File | Feature | forge-engine Status |
|-----------|---------|:-------------------:|
| `Game.java` | Core game state: players, zones, phases, stack, triggers, static effects, lifecycle | **Implemented** (`game.rs`) |
| `GameAction.java` | Common game actions & rule enforcement (move, damage, draw, SBA) | **Implemented** (`action.rs`) |
| `GameActionUtil.java` | Utility: alternative costs, spell mechanics helpers | Not implemented |
| `GameEndReason.java` | Enum: AllOpponentsLost, Draw, WinsGameSpellEffect, etc. | **Partial** (game_over + winner in `game.rs`) |
| `GameEntity.java` | Abstract base for entities (players, permanents) — damage, counters, attachments | **Partial** (split across `card.rs` + `player.rs`) |
| `GameEntityCache.java` | Generic ID→Object caching for entities | Not implemented |
| `GameEntityCounterTable.java` | Counter distribution table across entities | Not implemented |
| `GameEntityView.java` | Trackable entity view for UI synchronization | Not implemented |
| `GameEntityViewMap.java` | Entity view ↔ entity mapping | Not implemented |
| `GameFormat.java` | Format definitions (Standard, Modern, Commander, etc.) | Not implemented |
| `GameLog.java` | Game event logging with Observable pattern | Not implemented (CLI prints exist) |
| `GameLogEntry.java` | Immutable log entry (type + message) | Not implemented |
| `GameLogEntryType.java` | Log entry type enum (TURN, DAMAGE, ZONE_CHANGE, COMBAT…) | Not implemented |
| `GameLogFormatter.java` | Visitor pattern: formats game events into log entries | Not implemented |
| `GameObject.java` | Interface: validation & property methods for game objects | Not implemented |
| `GameObjectPredicates.java` | Predicate filters for GameObjects | Not implemented |
| `GameOutcome.java` | Game result data: player stats, ante, turn count, end conditions | **Partial** (winner tracked) |
| `GameRules.java` | Rule configuration: mana burn, poison, ante, AI settings | Not implemented |
| `GameSnapshot.java` | Game state snapshot/restore for copying | Not implemented |
| `GameStage.java` | Enum: BeforeMulligan, Mulligan, Play, RestartedByKarn, GameOver | **Partial** (game_over bool) |
| `GameType.java` | Enum: Sealed, Draft, Commander, Constructed, etc. | Not implemented |
| `GameView.java` | Trackable game view for UI synchronization | Not implemented |
| `CardTraitBase.java` | Base class for triggers, replacements, static abilities | **Partial** (trigger struct in `trigger.rs`) |
| `CardTraitPredicates.java` | Predicate filters for CardTraitBase | Not implemented |
| `Direction.java` | Turn direction enum (Left/Right) | Not implemented |
| `EvenOdd.java` | Even/Odd enum for game mechanics | Not implemented |
| `ForgeScript.java` | Card property evaluator (color, type, special properties) | **Partial** (ValidCard matching in `trigger.rs`) |
| `IEntityMap.java` | Interface: mapping game objects between states | Not implemented |
| `IHasGameType.java` | Interface: GameType accessor | Not implemented |
| `IHasSVars.java` | Interface: script variable (SVar) access | **Implemented** (SVars on CardInstance) |
| `IIdentifiable.java` | Interface: unique integer ID | **Implemented** (CardId, PlayerId in `ids.rs`) |
| `Match.java` | Match management: series of games, players, win conditions, ante | Not implemented |
| `PlanarDice.java` | Planechase: planar dice rolling, replacement/trigger handling | Not implemented |
| `StaticEffect.java` | Static ability effect with affected cards/players/timestamp | Not implemented |
| `StaticEffects.java` | Container managing all active static effects | Not implemented |
| `TriggerReplacementBase.java` | Abstract base for triggers & replacement effects | **Partial** (Trigger struct) |
| `package-info.java` | Package doc | N/A |

---

## 2. Ability System (`game/ability/`)

10 files — Ability factory, API types, keys, and base effect classes.

| Java File | Feature | forge-engine Status |
|-----------|---------|:-------------------:|
| `AbilityFactory.java` | Parses card script text into SpellAbility objects | **Partial** (parser.rs parses abilities/triggers/SVars) |
| `AbilityKey.java` | Enum of all ability parameter keys used in run/replacement params | **Partial** (RunParams in `event.rs`) |
| `AbilityUtils.java` | Utility: resolve defined cards/players, calculate amounts | Not implemented |
| `ApiType.java` | Enum of all ability API types (~200 types: DealDamage, Destroy, Draw…) | Not implemented |
| `AbilityApiBased.java` | Base class for API-based abilities | Not implemented |
| `SpellAbilityEffect.java` | Abstract base for all spell ability effects | Not implemented |
| `SpellApiBased.java` | Spell with API-based resolution | Not implemented |
| `StaticAbilityApiBased.java` | Static ability with API-based resolution | Not implemented |
| `IllegalAbilityException.java` | Exception for invalid ability definitions | Not implemented |
| `package-info.java` | Package doc | N/A |

---

## 3. Ability Effects (`game/ability/effects/`)

197 files — Individual effect implementations. Each file is a `SpellAbilityEffect` subclass.

| Java File | Feature | forge-engine Status |
|-----------|---------|:-------------------:|
| `AbandonEffect.java` | Abandon a card | Not implemented |
| `ActivateAbilityEffect.java` | Activate an ability on a card | Not implemented |
| `AddPhaseEffect.java` | Add extra phase to turn | Not implemented |
| `AddTurnEffect.java` | Add extra turn | Not implemented |
| `AmassEffect.java` | Amass N (create/grow army token) | Not implemented |
| `AnimateEffect.java` | Animate a permanent (change type/P&T) | Not implemented |
| `AnimateAllEffect.java` | Animate all matching permanents | Not implemented |
| `AnimateEffectBase.java` | Base class for animate effects | Not implemented |
| `AscendEffect.java` | Check/grant City's Blessing | Not implemented |
| `AttachEffect.java` | Attach aura/equipment to permanent | **Implemented** — `attach.rs`: attaches source Equipment/Aura to target creature on battlefield; handles detach from previous host; `CardInstance.attached_to`/`attachments` fields added; `GameState.attach_to`/`detach`/`remove_from_stack` in `action.rs` |
| `BalanceEffect.java` | Balance-type equalization effect | Not implemented |
| `BecomeMonarchEffect.java` | Become the Monarch | Not implemented |
| `BidLifeEffect.java` | Bid life (auction mechanic) | Not implemented |
| `BlockEffect.java` | Force/modify blocking | Not implemented |
| `BranchEffect.java` | Conditional branching in ability chains | Not implemented |
| `ChangeZoneEffect.java` | Move card(s) to another zone | **Implemented** (`game_loop.rs` ChangeZone handler: targeted/defined/self, LibraryPosition, Shuffle, Tapped, ChangesZone trigger) |
| `ChangeZoneAllEffect.java` | Move all matching cards to a zone | **Implemented** (`game_loop.rs` ChangeZoneAll handler: ValidCards filter, multi-player, triggers) |
| `CharmEffect.java` | Modal "choose N" charm abilities | **Implemented** — `charm_effect.rs`: `SP$ Charm`, `Choices$ SVar1,...`, `CharmNum$`, `MinCharmNum$`; resolution-time targeting via `TargetKind` dispatch; agent `choose_mode`; TauriAgent `ChooseMode` prompt + `ModeDecision` response; `ChooseModeModal` frontend |
| `ChooseCardEffect.java` | Choose a card from a set | Not implemented |
| `ChooseCardNameEffect.java` | Name a card | Not implemented |
| `ChooseColorEffect.java` | Choose a color | Not implemented |
| `ChooseTypeEffect.java` | Choose a type | Not implemented |
| `ChoosePlayerEffect.java` | Choose a player | Not implemented |
| `CloneEffect.java` | Copy/clone a permanent | **Partial** — `CopyPermanent` handler in `game_loop.rs`: copies targeted battlefield permanent, supports `PumpKeywords$`; copies are flagged `is_token` and cease to exist off battlefield. `AtEOT$` cleanup not yet implemented. |
| `ConniveEffect.java` | Connive N (draw + discard) | Not implemented |
| `ControlGainEffect.java` | Gain control of permanent | **Implemented** — `control_gain.rs`: changes controller of target battlefield permanent via `GameState.change_controller`; moves card between per-player zone lists |
| `CopyPermanentEffect.java` | Copy a permanent onto battlefield | **Partial** — see `CloneEffect.java` above |
| `CopySpellAbilityEffect.java` | Copy a spell on the stack | Not implemented |
| `CounterEffect.java` | Counter a spell or ability | **Implemented** — `counter.rs`: removes targeted stack entry via `MagicStack.remove_by_id`; moves source card to graveyard (or Destination$); `TargetKind::Spell` + `target_stack_entry: Option<u32>` in targeting system; `ChooseTargetSpell` prompt + clickable stack UI |
| `CountersPutEffect.java` | Put counters on a permanent/player | **Implemented** — `counters_put_effect.rs`: puts `CounterType$` counters on source card (`Defined$ Self`); fires `CounterAdded` trigger |
| `CountersRemoveEffect.java` | Remove counters | **Partial** — `counters_remove_effect.rs`: removes specific `CounterType$` counters from `Defined$ Self` or targeted card; `CounterNum$` supports integer and "All"; fires `CounterRemoved` trigger. Deferred: `CounterType$ Any/All` (interactive selection), `Choices$`, `Optional$`, `UpTo$`, player counter removal |
| `CountersMoveEffect.java` | Move counters between permanents | Not implemented |
| `CountersMultiplyEffect.java` | Multiply counters | Not implemented |
| `CountersProliferateEffect.java` | Proliferate | Not implemented |
| `DamageAllEffect.java` | Deal damage to all matching | **Implemented** — `damage_all_effect.rs`: `ValidCards$` + `ValidPlayers$` filters, fixed `NumDmg$`; deals to matching creatures and optionally all players |
| `DamageBaseEffect.java` | Base class for damage effects | **Partial** |
| `DamageDealEffect.java` | Deal damage to target | **Implemented** (`damage_deal_effect.rs`) |
| `DamageEachEffect.java` | Deal damage to each matching | Not implemented |
| `DamagePreventEffect.java` | Prevent damage | Not implemented |
| `DamageResolveEffect.java` | Resolve queued damage | Not implemented |
| `DayTimeEffect.java` | Change day/night | Not implemented |
| `DelayedTriggerEffect.java` | Create delayed trigger | Not implemented |
| `DestroyEffect.java` | Destroy target permanent | **Implemented** (`destroy_effect.rs`: moves target battlefield permanent to graveyard) |
| `DestroyAllEffect.java` | Destroy all matching permanents | **Implemented** — `destroy_all_effect.rs`: `ValidCards$` filter, respects `Indestructible` keyword and R$-based replacement effects; `NoRegen$ True` noted (regeneration not yet implemented) |
| `DigEffect.java` | Look at top N cards, choose some | **Implemented** — `dig.rs`: `DigNum$`, `ChangeNum$` (All/Any/N), `DestinationZone$`/`DestinationZone2$`, `ChangeValid$`, `LibraryPosition2$`, optional; agent `choose_dig`; TauriAgent `Dig` prompt + `DigDecision` response |
| `DiscardEffect.java` | Force discard | **Implemented** — `discard.rs`: target player or Defined$ player discards N (`NumCards$`) cards; agent `choose_discard`; TauriAgent `ChooseDiscard` prompt (reuses `LibraryPeekModal` in "discard" mode) + `DiscardDecision` response; fires `Discarded` trigger |
| `DiscoverEffect.java` | Discover N mechanic | Not implemented |
| `DrawEffect.java` | Draw cards | **Partial** (`action.rs` draw_cards) |
| `EffectEffect.java` | Create emblem/effect on battlefield | Not implemented |
| `EndTurnEffect.java` | End the turn | Not implemented |
| `ExploreEffect.java` | Explore mechanic | Not implemented |
| `FightEffect.java` | Fight between creatures | **Implemented** — `fight.rs`: source creature and target creature deal damage to each other equal to power simultaneously; fires `Fight` trigger; `TriggerType::Fight` + `RunParams.card2` added to event system |
| `FlipCoinEffect.java` | Flip a coin | Not implemented |
| `FogEffect.java` | Prevent all combat damage | Not implemented |
| `GameDrawEffect.java` | Force game draw | Not implemented |
| `GameLossEffect.java` | Force player to lose | Not implemented |
| `GameWinEffect.java` | Force player to win | Not implemented |
| `GoadEffect.java` | Goad a creature | Not implemented |
| `LifeGainEffect.java` | Gain life | **Partial** (`player.rs` gain_life) |
| `LifeLoseEffect.java` | Lose life | **Partial** (`player.rs` lose_life) |
| `LifeSetEffect.java` | Set life total | Not implemented |
| `LifeExchangeEffect.java` | Exchange life totals | Not implemented |
| `ManaEffect.java` | Add mana to pool | **Partial** (`mana_pool.rs`) |
| `ManaReflectedEffect.java` | Reflected mana (any color matching…) | Not implemented |
| `ManifestEffect.java` | Manifest (face-down) | Not implemented |
| `MeldEffect.java` | Meld two cards | Not implemented |
| `MillEffect.java` | Mill N cards | **Implemented** — `mill.rs`: `NumCards$`, targeted or `Defined$` player, moves top N from library to graveyard, emits ChangesZone trigger |
| `MutateEffect.java` | Mutate a creature | Not implemented |
| `PermanentCreatureEffect.java` | Resolve creature permanent spell | Not implemented |
| `PermanentNoncreatureEffect.java` | Resolve non-creature permanent spell | Not implemented |
| `PhasesEffect.java` | Phase in/out | Not implemented |
| `PlayEffect.java` | Play card from zone (exile, GY) | Not implemented |
| `PoisonEffect.java` | Give poison counters | **Implemented** — `poison_effect.rs`: adds `Num$` poison counters to players; supports `Defined$` (Player/Opponent/You) and `ValidTgts$ Player` targeting; `Defined$ Player` adds to all alive players (Ichor Rats pattern) |
| `ProtectEffect.java` | Grant protection | Not implemented |
| `PumpEffect.java` | +N/+N (or set P/T) until end of turn | **Implemented** (`pump_effect.rs`: single-target power/toughness modifier until EOT) |
| `PumpAllEffect.java` | Pump all matching creatures | **Implemented** — `pump_all_effect.rs`: `ValidCards$` filter, `NumAtt$`/`NumDef$` (signed, supports negative debuffs), `YouCtrl`/`OppCtrl`; duration = EOT (zeroed by `step_cleanup`) |
| `RegenerateEffect.java` | Regenerate a permanent | Not implemented |
| `RevealEffect.java` | Reveal cards | **Partial** — `reveal.rs`: reveals N cards from target hand, notifies all agents; no full interactive UI reveal |
| `RollDiceEffect.java` | Roll dice | Not implemented |
| `SacrificeEffect.java` | Force sacrifice | **Implemented** (`game_loop.rs` Sacrifice handler: SacValid$Self or matching permanents, agent choose_sacrifice for human choice, ChangesZone trigger) |
| `SacrificeAllEffect.java` | Force sacrifice of all matching | **Implemented** (`game_loop.rs` SacrificeAll handler: ValidCards filter, multi-player, ChangesZone trigger) |
| `ScryEffect.java` | Scry N | **Implemented** — `scry.rs`: `ScryNum$`, `Defined$` player, agent `choose_scry`; TauriAgent `Scry` prompt + `ScryDecision` response; PassAgent keeps all on top |
| `SetStateEffect.java` | Transform / flip / turn face-up | **Implemented** — `set_state_effect.rs`: `Mode$ Transform` with optional `ConditionDefined$ Remembered | ConditionPresent$ | ConditionCompare$` gate; calls `card.transform()` and resets active triggers |
| `ShuffleEffect.java` | Shuffle library | **Partial** (`action.rs` shuffle_library) |
| `SkipPhaseEffect.java` | Skip a phase | Not implemented |
| `SurveilEffect.java` | Surveil N | **Implemented** — `surveil.rs`: `Amount$`, `Defined$` player, agent `choose_surveil`; TauriAgent `Surveil` prompt + `SurveilDecision` response; emits ChangesZone trigger for graveyard cards |
| `TapEffect.java` | Tap a permanent | **Partial** (`action.rs` tap) |
| `TapAllEffect.java` | Tap all matching | **Implemented** — `tap_all_effect.rs`: `ValidCards$` filter with full `YouCtrl`/`OppCtrl`/color qualifier support |
| `TokenEffect.java` | Create token(s) | **Implemented** — `Token` handler in `game_loop.rs`: `TokenScript$`, `TokenAmount$`, `TokenOwner$` (You/Opponent). Token templates loaded from `tokenscripts/` via `get_token_db()` and registered in `GameLoop`. Tokens flagged `is_token` and cease to exist when leaving battlefield (CR 110.5g). |
| `TokenEffectBase.java` | Base class for token creation | **Implemented** — see `TokenEffect.java` above |
| `UntapEffect.java` | Untap a permanent | **Partial** (`action.rs` untap) |
| `UntapAllEffect.java` | Untap all matching | **Implemented** — `untap_all_effect.rs`: `ValidCards$` filter with full qualifier support |
| `VoteEffect.java` | Council's dilemma / voting mechanic | Not implemented |

> **Note**: 197 effect files total. ~41 have full or partial implementation. Additional implemented effects not listed individually: `RevealHandEffect.java` → `reveal_hand.rs` (inform all agents of a player's hand), `LookAtEffect.java` → `look_at.rs` (activating player peeks at top N cards of a zone), `RearrangeTopOfLibraryEffect` → `rearrange_top_of_library.rs` (used by Ponder: look at top N, reorder, optional shuffle via `choose_reorder_library` / `choose_may_shuffle`), `PeekAndRevealEffect.java` → `peek_and_reveal_effect.rs` (peek top N cards, optionally store in `remembered_cards`), `CleanupEffect.java` → `cleanup_effect.rs` (clear remembered cards/CMC). The remaining ~100+ effects (AdvanceCrank, Airbend, AlterAttribute, AssembleContraption, Behold, Blight, Bond, Camouflage, ChaosEnsues, Cloak, Endure, Forage, Heist, Incubate, Intensify, Investigate, Learn, MakeCard, ManifestDread, MultiplePiles, OpenAttraction, OwnershipGain, Planeswalk, PlayLandVariant, PowerExchange, Radiation, RemoveFromCombat, RemoveFromGame, RepeatEach, Replace*, RestartGame, ReverseTurnOrder, Ring, RollPlanarDice, Seek, SetInMotion, Subgame, TextBoxExchange, TimeTravel, Venture, VillainousChoice, ZoneExchange, etc.) are **not implemented**.

---

## 4. Card System (`game/card/`)

28 files — Core card representation, collections, factories, predicates.

| Java File | Feature | forge-engine Status |
|-----------|---------|:-------------------:|
| `Card.java` | Core card class — full state, abilities, types, counters, damage | **Implemented** (`card.rs` CardInstance) |
| `CardState.java` | Single state of card (front/back) with mutable properties | **Partial** — dual-face support: `CardOtherPart` stores back face; `transform()` swaps all face characteristics, `is_transformed` flag; `CardSplitType::is_dual_faced()` used during card loading |
| `CardFactory.java` | Factory: creates Card instances from templates | **Partial** (create_card in `game.rs`) |
| `CardFactoryUtil.java` | Card creation utilities | Not implemented |
| `CardCollection.java` | Mutable card collection | **Implemented** (Vec<CardId> in zones) |
| `CardCollectionView.java` | Immutable card collection view | Not implemented (no view layer) |
| `CardCopyService.java` | Card copying: tokens, clones, cross-game | Not implemented |
| `CardDamageHistory.java` | Damage history: attacks, blocks, damage per phase | Not implemented |
| `CardDamageMap.java` | Damage source→target mapping with trigger integration | Not implemented |
| `CardFaceView.java` | Card face display record | Not implemented |
| `CardLists.java` | Static filter utilities for card collections | Not implemented |
| `CardPlayOption.java` | Special play permissions from static abilities | Not implemented |
| `CardPredicates.java` | Predicate factories for card filtering | **Partial** (ValidCard matching in trigger.rs) |
| `CardProperty.java` | Evaluates card properties against string specs | **Partial** (trigger matching) |
| `CardTraitChanges.java` | Record: trait modifications (abilities, triggers, statics) | Not implemented |
| `CardChangedWords.java` | Word replacement tracking in card text | Not implemented |
| `CardCloneStates.java` | Multi-state management for clone/copy | Not implemented |
| `CardUtil.java` | Card operation utilities | Not implemented |
| `CardView.java` | Trackable card view for UI | Not implemented |
| `CardZoneTable.java` | Tracks card zone transitions | Not implemented |
| `CounterEnumType.java` | Enum: standard counter types with display/color | **Implemented** (CounterType enum in `card.rs`) |
| `CounterKeywordType.java` | Keyword-based counters (Flying counter, etc.) | Not implemented |
| `CounterType.java` | Counter type interface | **Implemented** |
| `ActivationTable.java` | Spell ability activation tracking | Not implemented |
| `ICardTraitChanges.java` | Interface for trait modifications | Not implemented |
| `IHasCardView.java` | Interface: getCardView() | Not implemented |
| `TokenCreateTable.java` | Token creation tracking table | Not implemented |
| `package-info.java` | Package doc | N/A |

---

## 5. Card — Perpetual Effects (`game/card/perpetual/`)

8 files — MTG Arena "perpetual" (game-lasting) card modifications.

| Java File | Feature | forge-engine Status |
|-----------|---------|:-------------------:|
| `PerpetualInterface.java` | Interface: perpetual modification contract | Not implemented |
| `PerpetualAbilities.java` | Permanent ability/trigger/replacement additions | Not implemented |
| `PerpetualColors.java` | Permanent color changes | Not implemented |
| `PerpetualIncorporate.java` | Permanent mana cost/color changes | Not implemented |
| `PerpetualKeywords.java` | Permanent keyword add/remove | Not implemented |
| `PerpetualManaCost.java` | Permanent mana cost modifications | Not implemented |
| `PerpetualNewPT.java` | Permanent P/T set values | Not implemented |
| `PerpetualPTBoost.java` | Permanent P/T boost | Not implemented |
| `PerpetualTypes.java` | Permanent card type changes | Not implemented |

---

## 6. Card — Tokens (`game/card/token/`)

1 file — Token creation and representation.

| Java File | Feature | forge-engine Status |
|-----------|---------|:-------------------:|
| `TokenInfo.java` | Token definition: name, image, types, keywords, P/T, colors | **Implemented** — token scripts loaded from `forge/forge-gui/res/tokenscripts/` via `CardDatabase`; keyed by filename stem (e.g. `r_1_1_goblin`). |

---

## 7. Combat (`game/combat/`)

10 files — Attack/block declaration, constraints, resolution.

| Java File | Feature | forge-engine Status |
|-----------|---------|:-------------------:|
| `Combat.java` | Combat state: attackers, blockers, damage assignment | **Implemented** (`combat.rs` CombatState) |
| `CombatUtil.java` | Combat utility methods | **Partial** (basic attack/block checks in `card.rs`) |
| `CombatView.java` | Combat view for UI | Not implemented |
| `CombatLki.java` | Last-known-information during combat | Not implemented |
| `AttackConstraints.java` | Attack requirement/restriction aggregation | Not implemented |
| `AttackRequirement.java` | "Must attack" requirements | Not implemented |
| `AttackRestriction.java` | "Can't attack" restrictions | Not implemented |
| `AttackRestrictionType.java` | Attack restriction type enum | Not implemented |
| `AttackingBand.java` | Banding attack groups | Not implemented |
| `GlobalAttackRestrictions.java` | Global attack limits | Not implemented |

---

## 8. Costs (`game/cost/`)

60 files — Spell/ability cost definitions and payment logic.

| Java File | Feature | forge-engine Status |
|-----------|---------|:-------------------:|
| `Cost.java` | Cost container: parses cost strings, holds cost parts | **Partial** (`cost/mod.rs` parse_cost + spell cost extraction from SP$ lines) |
| `CostPartMana.java` | Mana portion of costs | **Implemented** (`mana_pool.rs` try_pay) |
| `CostPayment.java` | Cost payment orchestration | **Partial** (`game_loop.rs` pay_ability_cost + pay_additional_costs) |
| `CostPart.java` | Abstract base for cost components | **Partial** (`cost/mod.rs` CostPart enum: Tap, Mana, PayLife, Sacrifice) |
| `CostPartWithList.java` | Cost part tracking affected cards | Not implemented |
| `CostPartWithTrigger.java` | Cost part that fires triggers | Not implemented |
| `CostTap.java` | Tap as cost | **Partial** (tap in `action.rs`) |
| `CostUntap.java` | Untap as cost | Not implemented |
| `CostSacrifice.java` | Sacrifice as cost | **Implemented** (`cost/mod.rs` get_sacrifice_targets, `game_loop.rs` pay_sacrifice_cost) |
| `CostPayLife.java` | Pay life as cost | Not implemented |
| `CostPayEnergy.java` | Pay energy counters | Not implemented |
| `CostPayShards.java` | Pay shard tokens | Not implemented |
| `CostDiscard.java` | Discard as cost | Not implemented |
| `CostExile.java` | Exile as cost | Not implemented |
| `CostExileFromStack.java` | Exile from stack as cost | Not implemented |
| `CostDamage.java` | Deal damage to self as cost | Not implemented |
| `CostDraw.java` | Draw as cost | Not implemented |
| `CostMill.java` | Mill as cost | Not implemented |
| `CostReturn.java` | Return to hand as cost | Not implemented |
| `CostReveal.java` | Reveal as cost | Not implemented |
| `CostPutCounter.java` | Put counter as cost | Not implemented |
| `CostRemoveCounter.java` | Remove counter as cost | Not implemented |
| `CostRemoveAnyCounter.java` | Remove any counter as cost | Not implemented |
| `CostTapType.java` | Tap matching permanent as cost | Not implemented |
| `CostUntapType.java` | Untap matching permanent as cost | Not implemented |
| `CostGainLife.java` | Opponent gains life as cost | Not implemented |
| `CostGainControl.java` | Give control as cost | Not implemented |
| `CostFlipCoin.java` | Flip coin as cost | Not implemented |
| `CostRollDice.java` | Roll dice as cost | Not implemented |
| `CostExert.java` | Exert as cost | Not implemented |
| `CostEnlist.java` | Enlist as cost | Not implemented |
| `CostForage.java` | Forage as cost | Not implemented |
| `CostCollectEvidence.java` | Collect evidence as cost | Not implemented |
| `CostChooseColor.java` | Choose color as cost | Not implemented |
| `CostChooseCreatureType.java` | Choose creature type as cost | Not implemented |
| `CostPutCardToLib.java` | Put card to library as cost | Not implemented |
| `CostAddMana.java` | Add mana to pool as cost | Not implemented |
| `CostUnattach.java` | Unattach as cost | Not implemented |
| `CostAdjustment.java` | Cost increase/decrease logic | Not implemented |
| `CostBlight.java` | Blight as cost | Not implemented |
| `CostBehold.java` | Behold as cost | Not implemented |
| `CostBeholdExile.java` | Behold exile variant | Not implemented |
| `CostPromiseGift.java` | Promise a gift as cost | Not implemented |
| `CostRevealChosen.java` | Reveal chosen card as cost | Not implemented |
| `CostExiledMoveToGrave.java` | Move exiled to graveyard as cost | Not implemented |
| `CostWaterbend.java` | Waterbend as cost | Not implemented |
| `CostDecisionMakerBase.java` | Base for AI cost decisions | Not implemented |
| `ICostVisitor.java` | Visitor pattern for costs | Not implemented |
| `IndividualCostPaymentInstance.java` | Per-cost-part payment instance | Not implemented |
| `PaymentDecision.java` | Cost payment decision record | Not implemented |
| `package-info.java` | Package doc | N/A |

---

## 9. Events (`game/event/`)

60 files — Game event types for UI and logging (Visitor pattern).

| Java File | Feature | forge-engine Status |
|-----------|---------|:-------------------:|
| `GameEvent.java` | Base event interface | **Partial** (TriggerType enum in `event.rs`) |
| `IGameEventVisitor.java` | Visitor interface for all event types | Not implemented |
| `GameEventTurnBegan.java` | Turn start event | **Partial** (notify_turn_changed in agent) |
| `GameEventTurnEnded.java` | Turn end event | Not implemented |
| `GameEventTurnPhase.java` | Phase change event | Not implemented |
| `GameEventSpellAbilityCast.java` | Spell/ability cast event | **Partial** (SpellCast trigger) |
| `GameEventSpellResolved.java` | Spell resolved event | Not implemented |
| `GameEventSpellRemovedFromStack.java` | Spell left stack event | Not implemented |
| `GameEventAttackersDeclared.java` | Attackers declared event | Not implemented |
| `GameEventBlockersDeclared.java` | Blockers declared event | Not implemented |
| `GameEventCombatChanged.java` | Combat state changed | Not implemented |
| `GameEventCombatEnded.java` | Combat ended | Not implemented |
| `GameEventCombatUpdate.java` | Combat update | Not implemented |
| `GameEventCardDamaged.java` | Card took damage | Not implemented |
| `GameEventCardDestroyed.java` | Card destroyed | Not implemented |
| `GameEventCardSacrificed.java` | Card sacrificed | Not implemented |
| `GameEventCardChangeZone.java` | Card changed zones | **Partial** (ChangesZone trigger) |
| `GameEventCardTapped.java` | Card tapped/untapped | Not implemented |
| `GameEventCardPhased.java` | Card phased in/out | Not implemented |
| `GameEventCardCounters.java` | Counter changed on card | Not implemented |
| `GameEventCardStatsChanged.java` | Card stats changed | Not implemented |
| `GameEventCardAttachment.java` | Attach/unattach event | Not implemented |
| `GameEventCardRegenerated.java` | Card regenerated | Not implemented |
| `GameEventCardModeChosen.java` | Mode chosen for modal spell | Not implemented |
| `GameEventCardForetold.java` | Card foretold | Not implemented |
| `GameEventCardPlotted.java` | Card plotted | Not implemented |
| `GameEventPlayerDamaged.java` | Player took damage | Not implemented |
| `GameEventPlayerLivesChanged.java` | Life total changed | Not implemented |
| `GameEventPlayerPoisoned.java` | Player poisoned | Not implemented |
| `GameEventPlayerRadiation.java` | Player radiation changed | Not implemented |
| `GameEventPlayerCounters.java` | Player counter changed | Not implemented |
| `GameEventPlayerPriority.java` | Priority passed | Not implemented |
| `GameEventPlayerControl.java` | Control changed | Not implemented |
| `GameEventPlayerShardsChanged.java` | Shard count changed | Not implemented |
| `GameEventPlayerStatsChanged.java` | Player stats changed | Not implemented |
| `GameEventManaPool.java` | Mana pool changed | Not implemented |
| `GameEventManaBurn.java` | Mana burn event | Not implemented |
| `GameEventMulligan.java` | Mulligan event | Not implemented |
| `GameEventLandPlayed.java` | Land played event | Not implemented |
| `GameEventTokenCreated.java` | Token created | Not implemented |
| `GameEventShuffle.java` | Library shuffled | Not implemented |
| `GameEventScry.java` | Scry event | Not implemented |
| `GameEventSurveil.java` | Surveil event | Not implemented |
| `GameEventFlipCoin.java` | Coin flipped | Not implemented |
| `GameEventRollDie.java` | Die rolled | Not implemented |
| `GameEventAnteCardsSelected.java` | Ante cards selected | Not implemented |
| `GameEventGameStarted.java` | Game started | Not implemented |
| `GameEventGameFinished.java` | Game finished | Not implemented |
| `GameEventGameOutcome.java` | Game outcome determined | Not implemented |
| `GameEventGameRestarted.java` | Game restarted (Karn) | Not implemented |
| `GameEventDayTimeChanged.java` | Day/night changed | Not implemented |
| `GameEventDoorChanged.java` | Room door changed | Not implemented |
| `GameEventRandomLog.java` | Random log event | Not implemented |
| `GameEventSpeedChanged.java` | Animation speed changed | Not implemented |
| `GameEventSprocketUpdate.java` | Contraption sprocket update | Not implemented |
| `GameEventSubgameStart.java` | Subgame started | Not implemented |
| `GameEventSubgameEnd.java` | Subgame ended | Not implemented |
| `GameEventSnapshotRestored.java` | Snapshot restored | Not implemented |

---

## 10. Extra Hands (`game/extrahands/`)

1 file — Conspiracy "Backup Plan" mechanic.

| Java File | Feature | forge-engine Status |
|-----------|---------|:-------------------:|
| `BackupPlanService.java` | Extra hands for Conspiracy draft | Not implemented |

---

## 11. Keywords (`game/keyword/`)

~20 files — Keyword definition, parsing, and instance management.

| Java File | Feature | forge-engine Status |
|-----------|---------|:-------------------:|
| `Keyword.java` | Enum of all MTG keywords (~200+) | **Partial** (string-based keywords in `card.rs`; ~20 keywords with runtime logic) |
| `KeywordInterface.java` | Interface for keyword instances | **Partial** (Vec<String> on CardInstance) |
| `KeywordInstance.java` | Abstract keyword instance with parameters | Not implemented |
| `KeywordCollection.java` | Collection of keyword instances | **Partial** (Vec<String>) |
| `KeywordWithAmount.java` | Keywords with numeric values (Bushido 2) | **Partial** (Toxic:N parsed via `get_toxic_count()`) |
| `KeywordWithCost.java` | Keywords with costs (Equip {3}) | **Partial** (Ward:N parsed via `get_ward_cost()`) |
| `KeywordWithCostAndType.java` | Keywords with cost + type (Cycling {2}) | Not implemented |
| `KeywordWithType.java` | Keywords with type (Protection from Red) | **Implemented** (`has_protection_from()`, `is_protected_from()`, `get_protections()` in `card/mod.rs`) |
| `KeywordsChange.java` | Keyword modification tracking | Not implemented |
| Various Keyword*.java | Specific keyword implementations | **Partial** — see keyword status below |
| `package-info.java` | Package doc | N/A |

### Keyword Runtime Status

| Keyword | Status | Implementation |
|---------|:------:|----------------|
| Flying | **Implemented** | `has_flying()` — blocking restriction in `combat/mod.rs` |
| Reach | **Implemented** | `has_reach()` — can block fliers |
| First Strike | **Implemented** | `has_first_strike()` — first-strike damage step |
| Double Strike | **Implemented** | `has_double_strike()` — both damage steps |
| Trample | **Implemented** | `has_trample()` — excess damage to player |
| Deathtouch | **Implemented** | `has_deathtouch()` — lethal with 1 damage |
| Lifelink | **Implemented** | `has_lifelink()` — gain life from damage |
| Vigilance | **Implemented** | `has_vigilance()` — no tap on attack |
| Defender | **Implemented** | `has_defender()` — can't attack |
| Haste | **Implemented** | `has_haste()` — ignores summoning sickness |
| Flash | **Implemented** | `has_keyword("Flash")` — cast at instant speed |
| Hexproof | **Implemented** | `has_hexproof()` — can't be targeted by opponents (`target_restrictions.rs`) |
| Shroud | **Implemented** | `has_shroud()` — can't be targeted by anyone (`target_restrictions.rs`) |
| Hexproof from X | **Implemented** | `has_hexproof_from(color)` — color-specific hexproof |
| Menace | **Implemented** | `has_menace()` — must be blocked by 2+ creatures (`game_loop.rs`) |
| Fear | **Implemented** | `has_fear()` — only blocked by artifact or black (`combat/mod.rs`) |
| Intimidate | **Implemented** | `has_intimidate()` — only blocked by artifact or shared color |
| Shadow | **Implemented** | `has_shadow()` — shadow vs non-shadow blocking |
| Skulk | **Implemented** | `has_skulk()` — can't be blocked by greater power |
| Horsemanship | **Implemented** | `has_horsemanship()` — only blocked by horsemanship |
| Indestructible | **Implemented** | `has_indestructible()` — prevents destroy (`destroy_effect.rs`, `destroy_all_effect.rs`) |
| Infect | **Implemented** | `has_infect()` — damage as poison counters / -1/-1 counters |
| Wither | **Implemented** | `has_wither()` — damage to creatures as -1/-1 counters |
| Toxic | **Implemented** | `get_toxic_count()` — adds poison counters on combat damage |
| Protection from X | **Implemented** | `is_protected_from()` — prevents targeting, blocking, damage, attaching |
| Ward | **Partial** | `get_ward_cost()` — keyword parsed and recognized; counter-unless-pay not yet wired |

---

## 12. Mana (`game/mana/`)

~10 files — Mana pool, mana objects, mana payment.

| Java File | Feature | forge-engine Status |
|-----------|---------|:-------------------:|
| `Mana.java` | Individual mana object with color, source, restrictions | **Partial** (`mana_pool.rs` tracks by color) |
| `ManaPool.java` | Player's mana pool with payment logic | **Implemented** (`mana_pool.rs`) |
| `ManaCostBeingPaid.java` | Tracks partial mana cost payment | **Partial** (try_pay handles full payment) |
| `ManaConversionMatrix.java` | Mana color conversion rules | Not implemented |
| `ManaAtom.java` | Mana atom bitmask constants | **Implemented** (ManaAtom in `foundation/mana.rs`) |
| `package-info.java` | Package doc | N/A |

---

## 13. Mulligan (`game/mulligan/`)

7 files — Mulligan rule implementations.

| Java File | Feature | forge-engine Status |
|-----------|---------|:-------------------:|
| `MulliganService.java` | Mulligan orchestration for all players | Not implemented |
| `AbstractMulligan.java` | Base mulligan with draw/keep/tuck logic | Not implemented |
| `LondonMulligan.java` | London mulligan (draw 7, put back N) | Not implemented |
| `VancouverMulligan.java` | Vancouver mulligan (Paris + scry 1) | Not implemented |
| `ParisMulligan.java` | Paris mulligan (draw N-1) | Not implemented |
| `OriginalMulligan.java` | Original mulligan (all-land/no-land only) | Not implemented |
| `HoustonMulligan.java` | Houston mulligan (draw 10, tuck 3) | Not implemented |

---

## 14. Phases (`game/phase/`)

7 files — Turn structure, phase handling, extra turns/phases.

| Java File | Feature | forge-engine Status |
|-----------|---------|:-------------------:|
| `PhaseType.java` | Enum of all phases/steps | **Implemented** (`foundation/phase.rs`) |
| `PhaseHandler.java` | Turn/phase state machine with priority | **Partial** (`phase.rs` TurnState + `game_loop.rs`: APNAP priority rotation, draw/combat/end priority windows, shared state-mutation gateway (`with_shared_state_mutation`) with implicit state broadcast on mutation (including priority handoff/pass windows); extra turns/phases and full replacement-phase interactions still missing) |
| `Phase.java` | Individual phase instance | **Partial** (advance_phase in `phase.rs`) |
| `ExtraTurn.java` | Extra turn tracking | Not implemented |
| `ExtraPhase.java` | Extra phase tracking | Not implemented |
| `Untap.java` | Untap step logic (phasing, untap restrictions) | **Partial** (`action.rs` untap_all) |
| `package-info.java` | Package doc | N/A |

---

## 15. Player (`game/player/`)

17 files — Player state, controller interface, properties, statistics.

| Java File | Feature | forge-engine Status |
|-----------|---------|:-------------------:|
| `Player.java` | Core player: life, zones, mana, counters, game actions | **Implemented** (`player.rs` PlayerState) |
| `PlayerController.java` | Abstract: UI/AI decision-making interface | **Implemented** (`agent.rs` PlayerAgent trait) |
| `PlayerView.java` | Trackable player view for UI | Not implemented |
| `PlayerCollection.java` | Player list with filtering | Not implemented |
| `PlayerPredicates.java` | Player filtering predicates | Not implemented |
| `PlayerProperty.java` | Evaluates player properties against string specs | Not implemented |
| `PlayerStatistics.java` | Player game statistics tracking | Not implemented |
| `PlayerOutcome.java` | Player game result | **Partial** (has_lost, has_won in PlayerState) |
| `PlayerActionConfirmMode.java` | Confirmation mode enum for AI | Not implemented |
| `PlayerFactoryUtil.java` | Player creation utilities | Not implemented |
| `RegisteredPlayer.java` | Pre-game player registration | Not implemented |
| `AchievementTracker.java` | Achievement tracking | Not implemented |
| `DelayedReveal.java` | Deferred card reveal to player | Not implemented |
| `GameLossReason.java` | Loss reason enum | Not implemented |
| `IGameEntitiesFactory.java` | Interface: entity factory | Not implemented |
| `IHasIcon.java` | Interface: icon accessor | Not implemented |
| `package-info.java` | Package doc | N/A |

---

## 16. Player Actions (`game/player/actions/`)

10 files — Discrete player action types (for macro recording/replay).

| Java File | Feature | forge-engine Status |
|-----------|---------|:-------------------:|
| `PlayerAction.java` | Abstract base action | **Partial** (MainPhaseAction enum in `agent.rs`) |
| `CastSpellAction.java` | Cast spell action | **Partial** (Play(CardId) variant) |
| `ActivateAbilityAction.java` | Activate ability action | **Partial** (ActivateMana variant) |
| `PassPriorityAction.java` | Pass priority action | **Implemented** (Pass variant) |
| `PayCostAction.java` | Pay cost action | Not implemented |
| `PayManaFromPoolAction.java` | Pay mana from pool | Not implemented |
| `SelectCardAction.java` | Select card action | Not implemented |
| `SelectPlayerAction.java` | Select player action | Not implemented |
| `TargetEntityAction.java` | Target entity action | Not implemented |
| `FinishTargetingAction.java` | Finish targeting | Not implemented |

---

## 17. Replacement Effects (`game/replacement/`)

46 files — "Instead of" effect handlers.

| Java File | Feature | forge-engine Status |
|-----------|---------|:-------------------:|
| `ReplacementHandler.java` | Central replacement effect dispatcher | **Implemented** (`replacement_handler.rs` — `apply_replacements`, CR 616 layer loop, `ReplacementEvent` enum) |
| `ReplacementEffect.java` | Base replacement effect class | **Implemented** (`replacement.rs` — `ReplacementEffect` struct, `can_replace_*`, `active_in_zone`, R$ parser) |
| `ReplacementType.java` | Enum of all replacement types | **Partial** (`replacement.rs` — DamageDone, Draw, DrawCards, Destroy, Moved, GainLife, AddCounter, GameLoss; 35 others as `Other`) |
| `ReplacementResult.java` | Replacement processing result | **Implemented** (`replacement.rs` — Replaced, NotReplaced, Prevented, Updated, Skipped) |
| `ReplacementLayer.java` | Replacement effect ordering layers | **Implemented** (`replacement.rs` — CantHappen, Control, Copy, Transform, Other) |
| `ReplacementEffectView.java` | Replacement effect UI view | Not implemented |
| `ReplaceDamage.java` | Replace damage events | **Partial** (`replacement_handler.rs` — Prevent$ True zeroes damage; no amount operators, no redirection) |
| `ReplaceDealtDamage.java` | Replace damage-dealt events | Not implemented |
| `ReplaceAssignDealDamage.java` | Replace damage assignment | Not implemented |
| `ReplaceDraw.java` | Replace single draw | **Partial** (`replacement_handler.rs` — Skipped/Replaced result; no draw-into replacement) |
| `ReplaceDrawCards.java` | Replace multiple draws | Not implemented |
| `ReplaceGainLife.java` | Replace life gain | Not implemented |
| `ReplaceLifeReduced.java` | Replace life reduction | Not implemented |
| `ReplacePayLife.java` | Replace life payment | Not implemented |
| `ReplaceGameLoss.java` | Replace game loss | Not implemented |
| `ReplaceGameWin.java` | Replace game win | Not implemented |
| `ReplaceDestroy.java` | Replace destroy events | **Partial** (`replacement_handler.rs` — Replaced blocks SBA destruction; no regeneration shield) |
| `ReplaceMoved.java` | Replace zone change events | **Partial** (`replacement_handler.rs` — NewDestination$ reroutes zone; Destination$/Origin$/ValidCard$ filters; no LKI/ETB handling) |
| `ReplaceCounter.java` | Replace counter spell | Not implemented |
| `ReplaceAddCounter.java` | Replace counter addition | Not implemented |
| `ReplaceRemoveCounter.java` | Replace counter removal | Not implemented |
| `ReplaceMill.java` | Replace mill events | Not implemented |
| `ReplaceToken.java` | Replace token creation | Not implemented |
| `ReplaceTap.java` | Replace tap events | Not implemented |
| `ReplaceUntap.java` | Replace untap events | Not implemented |
| `ReplaceTransform.java` | Replace transformation | Not implemented |
| `ReplaceTurnFaceUp.java` | Replace turning face up | Not implemented |
| `ReplaceAttached.java` | Replace attachment | Not implemented |
| `ReplaceDeclareBlocker.java` | Replace blocker declaration | Not implemented |
| `ReplaceBeginPhase.java` | Replace phase start | Not implemented |
| `ReplaceBeginTurn.java` | Replace turn start | Not implemented |
| `ReplaceCopySpell.java` | Replace spell copy | Not implemented |
| `ReplaceCascade.java` | Replace cascade | Not implemented |
| `ReplacePlaneswalk.java` | Replace planeswalk | Not implemented |
| `ReplaceProduceMana.java` | Replace mana production | Not implemented |
| `ReplaceLoseMana.java` | Replace mana loss | Not implemented |
| `ReplaceProliferate.java` | Replace proliferate | Not implemented |
| `ReplaceScry.java` | Replace scry | Not implemented |
| `ReplaceExplore.java` | Replace explore | Not implemented |
| `ReplaceLearn.java` | Replace learn | Not implemented |
| `ReplaceRollDice.java` | Replace dice roll | Not implemented |
| `ReplaceRollPlanarDice.java` | Replace planar dice roll | Not implemented |
| `ReplacePlanarDiceResult.java` | Replace planar dice result | Not implemented |
| `ReplaceSetInMotion.java` | Replace set in motion (schemes) | Not implemented |
| `ReplaceAssembleContraption.java` | Replace contraption assembly | Not implemented |
| `package-info.java` | Package doc | N/A |

---

## 18. Spell Abilities (`game/spellability/`)

~25 files — Spell/ability representation, stack, targeting.

| Java File | Feature | forge-engine Status |
|-----------|---------|:-------------------:|
| `SpellAbility.java` | Core spell/ability class: cost, targeting, resolution | **Partial** (StackEntry in `stack.rs`) |
| `SpellAbilityStackInstance.java` | Spell on the stack with full context | **Implemented** (`stack.rs` StackEntry) |
| `SpellPermanent.java` | Permanent spell (creature/non-creature) | Not implemented |
| `Spell.java` | Non-permanent spell base | Not implemented |
| `AbilityActivated.java` | Activated ability | Not implemented |
| `AbilityStatic.java` | Static ability wrapper | Not implemented |
| `AbilitySub.java` | Sub-ability in ability chain | Not implemented |
| `AbilityManaPart.java` | Mana ability component | Not implemented |
| `LandAbility.java` | Land play ability | Not implemented |
| `OptionalCost.java` | Optional additional costs (Kicker, Buyback) | Not implemented |
| `OptionalCostValue.java` | Optional cost value tracking | Not implemented |
| `SpellAbilityCondition.java` | Conditions for ability activation | Not implemented |
| `SpellAbilityPredicates.java` | Spell ability filtering predicates | Not implemented |
| `SpellAbilityRestriction.java` | Activation restrictions | Not implemented |
| `SpellAbilityVariables.java` | Zone, phase, speed, threshold conditions | Not implemented |
| `SpellAbilityView.java` | Trackable view for UI | Not implemented |
| `StackItemView.java` | Stack item view for UI | Not implemented |
| `TargetChoices.java` | Target selection container | **Partial** (TargetChoice enum in `agent.rs`) |
| `TargetRestrictions.java` | Targeting restrictions (type, zone, count) | Not implemented |
| `MagicStack.java` | *(in zone/ but logically here)* | **Implemented** (`stack.rs`) |
| `WrappedAbility.java` | *(in trigger/)* Ability wrapper for triggers | Not implemented |
| `package-info.java` | Package doc | N/A |

---

## 19. Static Abilities (`game/staticability/`)

60 files — Continuous effects and restrictions.

| Java File | Feature | forge-engine Status |
|-----------|---------|:-------------------:|
| `StaticAbility.java` | Core static ability class with layer system | **Partial** (`static_ability.rs`: `StaticAbility` struct + parser, `StaticMode` enum (6 modes), `CardFilter` for `Affected$`/`ValidCards$`; missing: 74+ modes, dependency graph, timestamp tracking) |
| `StaticAbilityLayer.java` | Enum: copy, control, text, type, color, abilities, P/T, rules | **Partial** (`static_ability.rs`: `Layer` enum — Control(2), Type(4), Color(5), Ability(6), SetPT(7b), ModifyPT(7c); missing: copy/text/7a/rules layers) |
| `StaticAbilityMode.java` | Enum: 80+ static ability modes | **Partial** (`static_ability.rs` `StaticMode`: Continuous, CantAttack, CantBlock, ETBTapped, CantBeCast, ReduceCost, IncreaseCost; 73+ modes not yet handled) |
| `StaticAbilityContinuous.java` | Core continuous effect handler | **Partial** (`layer.rs` `apply_continuous_effects()`: Control (2, `GainControl$` incl. aura `Card.EnchantedBy`), Ability/keyword-grant (6), SetPT (7b), ModifyPT (7c) layers applied in CR 613 order; `apply_etb_tapped()` for ETBTapped; missing: type/color layers, dependency resolution) |
| `StaticAbilityCantAttack.java` | Prevents attacking | **Implemented** (`layer.rs`: `Mode$ CantAttack` sets `cant_attack_static` flag; `card.rs` `can_attack()` respects it) |
| `StaticAbilityCantBlock.java` | Prevents blocking | **Implemented** (`layer.rs`: `Mode$ CantBlock` sets `cant_block_static` flag; `card.rs` `can_block()` respects it) |
| `StaticAbilityCantBeSacrificed.java` | Prevents sacrifice | Not implemented |
| `StaticAbilityCantCast.java` | Prevents casting | Not implemented |
| `StaticAbilityCantTarget.java` | Grants hexproof/shroud | Not implemented |
| `StaticAbilityCantDraw.java` | Limits/prevents drawing | Not implemented |
| `StaticAbilityCantDiscard.java` | Prevents discarding | Not implemented |
| `StaticAbilityCantDamage.java` | Prevents damage | Not implemented |
| `StaticAbilityCantExile.java` | Prevents exiling | Not implemented |
| `StaticAbilityCantRegenerate.java` | Prevents regeneration | Not implemented |
| `StaticAbilityCantTransform.java` | Prevents transformation | Not implemented |
| `StaticAbilityCantPhase.java` | Prevents phasing | Not implemented |
| `StaticAbilityCantPutCounter.java` | Prevents counter placement | Not implemented |
| `StaticAbilityCantPreventDamage.java` | Prevents damage prevention | Not implemented |
| `StaticAbilityCantSacrifice.java` | Prevents sacrifice | Not implemented |
| `StaticAbilityCantVenture.java` | Prevents venturing | Not implemented |
| `StaticAbilityCantGainLosePayLife.java` | Prevents life gain/loss/payment | Not implemented |
| `StaticAbilityCastWithFlash.java` | Grants flash to spells | Not implemented |
| `StaticAbilityMustAttack.java` | Forces creatures to attack | Not implemented |
| `StaticAbilityMustBlock.java` | Forces creatures to block | Not implemented |
| `StaticAbilityMustTarget.java` | Forces targeting restrictions | Not implemented |
| `StaticAbilityAdapt.java` | Adapt mechanic interactions | Not implemented |
| `StaticAbilityPanharmonicon.java` | Double trigger effects | Not implemented |
| `StaticAbilityManaConvert.java` | Mana color conversion | Not implemented |
| `StaticAbilityIgnoreHexproofShroud.java` | Ignore hexproof/shroud | Not implemented |
| `StaticAbilityIgnoreLandwalk.java` | Ignore landwalk | Not implemented |
| `StaticAbilityIgnoreLegendRule.java` | Ignore legend rule | Not implemented |
| `StaticAbilityInfectDamage.java` | Makes damage infect | Not implemented |
| `StaticAbilityWitherDamage.java` | Makes damage wither | Not implemented |
| `StaticAbilityColorlessDamageSource.java` | Makes damage colorless | Not implemented |
| `StaticAbilityCombatDamageToughness.java` | Use toughness for combat damage | Not implemented |
| `StaticAbilityNoCleanupDamage.java` | Prevents damage removal at cleanup | Not implemented |
| `StaticAbilityCountersRemain.java` | Prevents counter removal | Not implemented |
| `StaticAbilityMaxCounter.java` | Sets maximum counters | Not implemented |
| `StaticAbilityDevotion.java` | Modifies devotion calculation | Not implemented |
| `StaticAbilityDisableTriggers.java` | Disables triggered abilities | Not implemented |
| `StaticAbilityFlipCoinMod.java` | Fixes coin flip results | Not implemented |
| `StaticAbilityNumLoyaltyAct.java` | Modifies loyalty activation count | Not implemented |
| `StaticAbilityTurnPhaseReversed.java` | Reverses turn/phase order | Not implemented |
| `StaticAbilityUnspentMana.java` | Mana carry-over rules | Not implemented |
| `StaticAbilityUntapOtherPlayer.java` | Untap opponent's permanents | Not implemented |
| `StaticAbilityExhaust.java` | Exhaust mechanic | Not implemented |
| `StaticAbilityGainLifeRadiation.java` | Radiation life gain | Not implemented |
| `StaticAbilityPlotZone.java` | Plot zone mechanics | Not implemented |
| `StaticAbilitySurveilNum.java` | Modifies surveil number | Not implemented |
| `StaticAbilityTapPowerValue.java` | Uses toughness for tap power | Not implemented |
| `StaticAbilityView.java` | UI view for static abilities | Not implemented |
| `package-info.java` | Package doc | N/A |

---

## 20. Triggers (`game/trigger/`)

~140 files — Triggered ability types and handler.

| Java File | Feature | forge-engine Status |
|-----------|---------|:-------------------:|
| `TriggerHandler.java` | Central trigger management & dispatch | **Implemented** (`trigger_handler.rs`) |
| `Trigger.java` | Abstract base trigger class | **Implemented** (`trigger.rs` Trigger struct) |
| `TriggerType.java` | Enum of all trigger types | **Implemented** (33 types in `event.rs`: 8 original + 25 new) |
| `WrappedAbility.java` | Ability wrapper for trigger execution | Not implemented |
| `TriggerWaiting.java` | Waiting trigger queue | **Implemented** (waiting_triggers in handler) |
| **Zone Change Triggers** | | |
| `TriggerChangesZone.java` | Card changes zones (ETB, dies, etc.) | **Implemented** |
| `TriggerChangesZoneAll.java` | All zone changes | Not implemented |
| `TriggerChangesController.java` | Control changes | Not implemented |
| **Phase/Turn Triggers** | | |
| `TriggerPhase.java` | Phase begin/end | **Implemented** |
| `TriggerTurnBegin.java` | Turn begins | Not implemented |
| `TriggerNewGame.java` | Game starts | Not implemented |
| **Combat Triggers** | | |
| `TriggerAttacks.java` | Creature attacks | **Implemented** (fires in game_loop.rs) |
| `TriggerAttackersDeclared.java` | All attackers declared | Not implemented |
| `TriggerAttackerBlocked.java` | Attacker is blocked | **Implemented** (fires in game_loop.rs) |
| `TriggerAttackerBlockedByCreature.java` | Specific blocker | Not implemented |
| `TriggerAttackerBlockedOnce.java` | Blocked once per combat | Not implemented |
| `TriggerAttackerUnblocked.java` | Attacker unblocked | **Implemented** (fires in game_loop.rs) |
| `TriggerAttackerUnblockedOnce.java` | Unblocked once | Not implemented |
| `TriggerBlockersDeclared.java` | All blockers declared | Not implemented |
| `TriggerBlocks.java` | Creature blocks | **Implemented** (fires in game_loop.rs) |
| **Spell Triggers** | | |
| `TriggerSpellCast.java` | Spell is cast | **Implemented** |
| `TriggerSpellCastAll.java` | All spell casts | Not implemented |
| `TriggerSpellCastOnce.java` | First spell cast per turn | Not implemented |
| `TriggerSpellCastOfType.java` | Specific spell type cast | Not implemented |
| `TriggerAbilityResolves.java` | Ability resolves | Not implemented |
| `TriggerAbilityTriggered.java` | Ability triggered | Not implemented |
| `TriggerCountered.java` | Spell countered | **Implemented** (fires in counter_effect) |
| **Damage Triggers** | | |
| `TriggerDamageDone.java` | Damage dealt | **Implemented** (fires in combat, damage_deal, damage_all) |
| `TriggerDamageDoneOnce.java` | Damage dealt once | Not implemented |
| `TriggerDamageDoneOnceByController.java` | Once per controller | Not implemented |
| `TriggerDamageAll.java` | All damage events | Not implemented |
| `TriggerDamageDealtOnce.java` | Once per damage event | **Partial** (TriggerType + TriggerMode defined, per-turn tracking not yet implemented) |
| `TriggerDamagePreventedOnce.java` | Damage prevented | Not implemented |
| `TriggerExcessDamage.java` | Excess damage (trample) | Not implemented |
| `TriggerExcessDamageAll.java` | All excess damage | Not implemented |
| **Life Triggers** | | |
| `TriggerLifeGained.java` | Life gained | **Implemented** (fires in life_gain_effect, combat lifelink) |
| `TriggerLifeLost.java` | Life lost | **Implemented** (fires in life_lose_effect, pay costs) |
| `TriggerLifeLostAll.java` | All life loss | Not implemented |
| `TriggerPayLife.java` | Life paid as cost | Not implemented |
| `TriggerLosesGame.java` | Player loses | Not implemented |
| **Counter Triggers** | | |
| `TriggerCounterAdded.java` | Counter added | **Implemented** (fires in counters_put_effect) |
| `TriggerCounterAddedAll.java` | All counter additions | Not implemented |
| `TriggerCounterAddedOnce.java` | Counter added once | Not implemented |
| `TriggerCounterRemoved.java` | Counter removed | **Implemented** (TriggerType + TriggerMode + perform_test, ready to fire from counter removal effects) |
| `TriggerCounterRemovedOnce.java` | Counter removed once | Not implemented |
| `TriggerCounterPlayerAddedAll.java` | Player counters | Not implemented |
| `TriggerCounterTypeAddedAll.java` | Specific counter type | Not implemented |
| **Card Action Triggers** | | |
| `TriggerDrawn.java` | Card drawn | **Implemented** (fires in draw_effect, step_draw) |
| `TriggerDiscarded.java` | Card discarded | **Implemented** (fires in discard_effect) |
| `TriggerDiscardedAll.java` | All discards | Not implemented |
| `TriggerMilled.java` | Card milled | **Implemented** (fires in mill_effect) |
| `TriggerMilledAll.java` | All mills | Not implemented |
| `TriggerMilledOnce.java` | Milled once | Not implemented |
| `TriggerExiled.java` | Card exiled | **Implemented** (fires in change_zone_effect when destination is Exile) |
| `TriggerSacrificed.java` | Card sacrificed | **Implemented** (fires in sacrifice_effect, sacrifice_all_effect, game_loop) |
| `TriggerSacrificedOnce.java` | Sacrificed once | Not implemented |
| `TriggerDestroyed.java` | Card destroyed | **Implemented** (fires in destroy_effect) |
| `TriggerCycled.java` | Card cycled | Not implemented |
| `TriggerLandPlayed.java` | Land played | **Implemented** (fires in game_loop play_card) |
| `TriggerTaps.java` | Permanent tapped | **Implemented** (fires in tap_all_effect, game_loop) |
| `TriggerTapAll.java` | All taps | Not implemented |
| `TriggerUntaps.java` | Permanent untapped | **Implemented** (fires in untap_all_effect, game_loop) |
| `TriggerUntapAll.java` | All untaps | Not implemented |
| `TriggerTapsForMana.java` | Tapped for mana | **Implemented** (fires in game_loop ActivateMana + resolve_mana_ability) |
| **Keyword Mechanic Triggers** | | |
| `TriggerBecomesTarget.java` | Becomes target | **Implemented** (fires in game_loop play_card + activate_ability_on_stack) |
| `TriggerBecomesTargetOnce.java` | Targeted once | Not implemented |
| `TriggerEvolved.java` | Creature evolves | Not implemented |
| `TriggerExplores.java` | Creature explores | **Partial** (TriggerType + TriggerMode defined, explore mechanic not yet implemented) |
| `TriggerMutates.java` | Creature mutates | Not implemented |
| `TriggerAdapt.java` | Creature adapts | Not implemented |
| `TriggerBecomeMonstrous.java` | Becomes monstrous | Not implemented |
| `TriggerBecomeRenowned.java` | Becomes renowned | Not implemented |
| `TriggerBecomeMonarch.java` | Becomes monarch | **Partial** (TriggerType + TriggerMode defined, monarch mechanic not yet implemented) |
| `TriggerBecomesCrewed.java` | Vehicle crewed | Not implemented |
| `TriggerBecomesSaddled.java` | Mount saddled | Not implemented |
| `TriggerBecomesPlotted.java` | Card plotted | Not implemented |
| `TriggerFlippedCoin.java` | Coin flipped | Not implemented |
| `TriggerFight.java` | Creatures fight | **Implemented** (fires in fight_effect) |
| `TriggerFightOnce.java` | Fight once | Not implemented |
| `TriggerExerted.java` | Creature exerted | Not implemented |
| `TriggerExploited.java` | Creature exploited | Not implemented |
| `TriggerInvestigated.java` | Investigated | Not implemented |
| `TriggerForetell.java` | Card foretold | Not implemented |
| `TriggerForage.java` | Foraged | Not implemented |
| `TriggerSurveil.java` | Surveiled | Not implemented |
| `TriggerScry.java` | Scryed | Not implemented |
| `TriggerProliferate.java` | Proliferated | Not implemented |
| `TriggerCollectEvidence.java` | Evidence collected | Not implemented |
| `TriggerCommitCrime.java` | Crime committed | Not implemented |
| `TriggerDiscover.java` | Discovered | Not implemented |
| `TriggerConnive.java` *(if exists)* | Connived | Not implemented |
| **Misc Triggers** | | |
| `TriggerAlways.java` | Always fires | Not implemented |
| `TriggerImmediate.java` | Immediate trigger | Not implemented |
| `TriggerAttached.java` | Aura/equipment attached | **Implemented** (fires in attach_effect) |
| `TriggerUnattach.java` | Detached | **Implemented** (TriggerType + TriggerMode + perform_test, ready to fire from detach effects) |
| `TriggerPhaseIn.java` | Phased in | Not implemented |
| `TriggerPhaseOut.java` | Phased out | Not implemented |
| `TriggerPhaseOutAll.java` | All phased out | Not implemented |
| `TriggerTransformed.java` | Card transformed | **Implemented** (fires in set_state_effect) |
| `TriggerTurnFaceUp.java` | Turned face up | Not implemented |
| `TriggerSearchedLibrary.java` | Library searched | Not implemented |
| `TriggerShuffled.java` | Library shuffled | Not implemented |
| `TriggerManaAdded.java` | Mana added | Not implemented |
| `TriggerManaExpend.java` | Mana expended | Not implemented |
| `TriggerTokenCreated.java` | Token created | **Implemented** (fires in token_effect) |
| `TriggerTokenCreatedOnce.java` | Token created once | Not implemented |
| `TriggerClassLevelGained.java` | Class leveled up | Not implemented |
| `TriggerCompletedDungeon.java` | Dungeon completed | Not implemented |
| `TriggerEnteredRoom.java` | Entered dungeon room | Not implemented |
| `TriggerVote.java` | Voting occurred | Not implemented |
| `TriggerChampioned.java` | Creature championed | Not implemented |
| `TriggerClashed.java` | Clashed | Not implemented |
| `TriggerDevoured.java` | Creature devoured | Not implemented |
| `TriggerEnlisted.java` | Creature enlisted | Not implemented |
| `TriggerMentored.java` | Creature mentored | Not implemented |
| `TriggerTrains.java` | Creature trained | Not implemented |
| `TriggerCaseSolved.java` | Case solved | Not implemented |
| `TriggerClaimPrize.java` | Prize claimed | Not implemented |
| `TriggerGiveGift.java` | Gift given | Not implemented |
| `TriggerRingTemptsYou.java` | Ring tempts you | Not implemented |
| `TriggerDayTimeChanges.java` | Day/night changed | Not implemented |
| `TriggerSpecializes.java` | Card specializes | Not implemented |
| `TriggerUnlockDoor.java` | Door unlocked | Not implemented |
| `TriggerFullyUnlock.java` | Fully unlocked | Not implemented |
| `TriggerManifestDread.java` | Manifest dread | Not implemented |
| `TriggerElementalbend.java` | Elementalbend | Not implemented |
| `TriggerCrankContraption.java` | Contraption cranked | Not implemented |
| `TriggerCrewedSaddled.java` | Crewed/saddled | Not implemented |
| `TriggerPlanarDice.java` | Planar dice rolled | Not implemented |
| `TriggerAbandoned.java` | Card abandoned | Not implemented |
| `TriggerWinnsTheGame.java` | Player wins | Not implemented |
| Various others… | ~20 more specialized triggers | Not implemented |

---

## 21. Zones (`game/zone/`)

8 files — Game zone implementations.

| Java File | Feature | forge-engine Status |
|-----------|---------|:-------------------:|
| `ZoneType.java` | Enum: all 18+ zone types | **Implemented** (`foundation/zone.rs`) |
| `Zone.java` | Base zone: card collection with type tracking | **Implemented** (`zone.rs` Zone struct) |
| `PlayerZone.java` | Player-specific zone with permissions | **Partial** (zones keyed by ZoneKey) |
| `PlayerZoneBattlefield.java` | Battlefield with meld tracking | Not implemented |
| `MagicStack.java` | Stack: spell/ability resolution LIFO | **Implemented** (`stack.rs`) |
| `CostPaymentStack.java` | Cost payment tracking stack | Not implemented |
| `ZoneView.java` | Zone snapshot for events | Not implemented |
| `package-info.java` | Package doc | N/A |

---

## 22. forge-engine Rust Implementation Summary

### Crate: `forge-foundation` (shared types)
| File | Status | Features |
|------|--------|----------|
| `color.rs` | **Complete** | Color enum, ColorSet bitmask, all 32 combinations, predicates |
| `mana.rs` | **Complete** | ManaAtom, 41 ManaCostShard variants, ManaCost parsing, CMC, hybrid/phyrexian/snow |
| `card_type.rs` | **Complete** | 15 CoreTypes, 7 Supertypes, CardTypeLine parsing, type queries |
| `card_split.rs` | **Complete** | 9 split types, 14 card state names, face selection |
| `phase.rs` | **Complete** | 13 phases, turn order, grouping, queries |
| `zone.rs` | **Complete** | 19 zone types, characteristics, parsing |

### Crate: `forge-carddb` (card database)
| File | Status | Features |
|------|--------|----------|
| `card_face.rs` | **Complete** | Card face: name, cost, types, oracle, P/T, keywords, abilities, triggers |
| `card_rules.rs` | **Complete** | Multi-face cards, color identity, aggregation |
| `parser.rs` | **Complete** | Full Forge script parser (25+ keywords), multi-face, SVars |
| `database.rs` | **Complete** | Card database: load, query, iterate |

### Crate: `forge-engine` (game engine)
| File | Status | Features |
|------|--------|----------|
| `ids.rs` | **Complete** | CardId, PlayerId typed wrappers |
| `game.rs` | **Complete** | GameState arena, card/player/zone management |
| `player.rs` | **Complete** | Life, poison, land plays, commander damage, win/loss |
| `card.rs` | **Complete** | CardInstance: P/T, counters, keywords, combat checks, triggers, SVars |
| `zone.rs` | **Complete** | Zone CRUD, top/bottom, peek |
| `phase.rs` | **Complete** | TurnState, phase advancement, multiplayer turns |
| `stack.rs` | **Complete** | StackEntry, MagicStack LIFO |
| `mana_pool.rs` | **Complete** | Full mana payment: hybrid, phyrexian, colorless, generic |
| `combat.rs` | **Complete** | Attack/block declaration, queries |
| `action.rs` | **Complete** | move_card, damage, SBAs (lethal, poison, commander), draw, shuffle, tap/untap |
| `event.rs` | **Complete** | TriggerType enum (33 types: 8 original + 25 new), RunParams (~30 fields) |
| `trigger.rs` | **Complete** | Trigger matching, ValidCard/ValidPlayer filters, parsing |
| `trigger_handler.rs` | **Complete** | Active/waiting/delayed triggers, dispatch, OptionalDecider$ support, APNAP ordering |
| `agent.rs` | **Complete** | PlayerAgent trait (15 callbacks incl. choose_optional_trigger), MainPhaseAction, TargetChoice |
| `game_loop.rs` | **Partial** | Game flow orchestration with APNAP priority handoff, `priority_player` tracking, draw/combat/end priority windows, and illegal-action guardrails; still missing full Java parity for extra turns/phases and advanced phase replacement hooks |
| `spellability/mod.rs` | **Complete** | SpellAbility module structure |
| `spellability/targeting.rs` | **Complete** | Targeting system: parse_valid_targets, choose_targets, CardInZone support for graveyard/exile targeting |

### Crate: `forge-cli`
| File | Status | Features |
|------|--------|----------|
| `main.rs` | **Basic** | ANSI-colored card display, CLI test harness |

---

## Summary Statistics

| Category | Java Files | Fully Implemented | Partially Implemented | Not Implemented |
|----------|:----------:|:-----------------:|:---------------------:|:---------------:|
| Core Game | 37 | 3 | 8 | 26 |
| Ability System | 10 | 0 | 2 | 8 |
| Ability Effects | 197 | 30 | 14 | 153 |
| Card System | 28 | 4 | 4 | 20 |
| Perpetual Effects | 8 | 0 | 0 | 8 |
| Tokens | 1 | 0 | 0 | 1 |
| Combat | 10 | 1 | 1 | 8 |
| Costs | 60 | 1 | 4 | 55 |
| Events | 60 | 0 | 3 | 57 |
| Extra Hands | 1 | 0 | 0 | 1 |
| Keywords | 20 | 1 | 5 | 14 |
| Mana | 10 | 1 | 2 | 7 |
| Mulligan | 7 | 0 | 0 | 7 |
| Phases | 7 | 1 | 3 | 3 |
| Player | 17 | 1 | 2 | 14 |
| Player Actions | 10 | 1 | 3 | 6 |
| Replacement Effects | 46 | 4 | 4 | 38 |
| Spell Abilities | 25 | 3 | 2 | 20 |
| Static Abilities | 60 | 2 | 4 | 54 |
| Triggers | 140 | 26 | 5 | 109 |
| Zones | 8 | 3 | 1 | 4 |
| **TOTAL** | **769** | **75** | **66** | **628** |

> **Coverage: ~18.3% implemented or partially implemented** (141 of 769 features have some Rust counterpart)
>
> The Rust engine has a solid **architectural foundation** (types, state, zones, stack, mana, combat, triggers, actions, agent). The trigger system now supports **33 trigger types** (8 original + 25 new) with OptionalDecider$ prompting, delayed trigger infrastructure, and comprehensive ValidCard$/ValidPlayer$ filtering. The major gaps are: **ability effects** (197 files), **static abilities** (60 files), **replacement effects** (38 still not implemented), **trigger types** (109 still not implemented), and **costs** (58 files).

---

## UI Features (Tauri + React Frontend)

| Feature | Status | Notes |
|---------|--------|-------|
| Set code on `CardInstance` | **Implemented** | `card.set_code: Option<String>` in `forge-engine/src/card/mod.rs` |
| Set code in `CardDto` | **Implemented** | `set_code: String` in `game_view_dto.rs`; serialized as `setCode` via serde |
| Set code in preset decks | **Implemented** | All 16 preset decks in `preset_decks.rs` carry a Scryfall set code per card entry (3-tuple format) |
| Set code in custom decks | **Implemented** | `CardIdentity.set_code` propagated to engine via `build_custom_deck` |
| Scryfall set-specific image fetch | **Implemented** | `getCardByName(name, setCode?)` in `scryfall.ts`; falls back to name-only on miss |
| `useCardImage` set-aware | **Implemented** | Hook passes `setCode` to `getCardByName`; `Card.tsx` passes `card.setCode` |
| Batch collection fetch with set | **Implemented** | `fetchCardCollection` sends `{ name, set }` identifiers to Scryfall `/cards/collection` |
| Print picker modal | **Implemented** | `PrintPickerModal.tsx` — fetches all printings via `prints_search_uri`, updates deck via `updatePrint` |
| Print picker in deck builder (list view) | **Implemented** | Image icon button on each `CardRow`; opens `PrintPickerModal` |
| Print picker in deck builder (visual/stack) | **Implemented** | Image icon overlay on each `CardVisual`; opens `PrintPickerModal` |
| Print picker for commander | **Implemented** | Commander card visual mode also exposes print picker |
| Auto-priority pass | **Implemented** | `Game.tsx` useEffect auto-responds after random 300-800ms delay when `chooseAction` has no playable cards, `chooseAttackers` has no attackers, or `chooseBlockers` has no blockers; toggle in `Settings.tsx` backed by `usePreferencesStore.autoPassEnabled` |
