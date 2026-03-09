# Forge Game Engine — Feature Mapping

> **738 Java files** in `forge/forge-game/src/main/java/forge/game/` mapped against **~130 Rust implementation files** (~160 total including tests/tools) in `forge-engine/`. **~43% coverage.**
>
> Legend: **Implemented** | **Partial** | **Stub** | Not implemented

Parity tooling note (Rust `forge-parity`): **Implemented** low-effort mechanic coverage reporting for unique triggers, effects, and activated abilities (deck-defined unique set vs. unique observed set, with percentage output). Deterministic parity agent now includes activatable abilities in its main-phase decision pool (`MainPhaseAction::ActivateAbility`), not only playable cards. Added `--prefer-actions` flag to bias random main-phase choices toward actions over pass, with matching support in Java harness parity controller.

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
23. [Priority Analysis — What's Missing](#23-priority-analysis--whats-missing)
---

## 1. Core Game (`game/`)

37 files — Core game state, lifecycle, rules, logging, and base abstractions.

| Java File | Feature | forge-engine Status |
|-----------|---------|:-------------------:|
| `Game.java` | Core game state: players, zones, phases, stack, triggers, static effects, lifecycle | **Implemented** (`game.rs`) |
| `GameAction.java` | Common game actions & rule enforcement (move, damage, draw, SBA) | **Implemented** (`action.rs`) |
| `GameActionUtil.java` | Utility: alternative costs, spell mechanics helpers | **Partial** (alternative cost detection/selection in `game_loop.rs`; AB$ line filtering in `game_action_util.rs`) |
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
| `GameStage.java` | Enum: BeforeMulligan, Mulligan, Play, RestartedByKarn, GameOver | **Partial** (game_over bool, mulligan phase in setup) |
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
| `AbilityUtils.java` | Utility: resolve defined cards/players, calculate amounts | **Partial** (`resolve_defined_player` in `effects/mod.rs`: handles `You`, `Player.You`, `Opponent`, `Player.Opponent`, `OpponentCtrl`; Count$ resolution includes `Converge/Sunburst` and `PromisedGift` variants used by Gift cards; core `UnlessCost` gating path wired in effect resolution, including payment parts used by current parity decks such as `DamageYou`/`PayLife`/mana/`Draw`/`Mill`/energy/shards) |
| `ApiType.java` | Enum of all ability API types (~200 types: DealDamage, Destroy, Draw…) | Not implemented |
| `AbilityApiBased.java` | Base class for API-based abilities | Not implemented |
| `SpellAbilityEffect.java` | Abstract base for all spell ability effects | Not implemented |
| `SpellApiBased.java` | Spell with API-based resolution | Not implemented |
| `StaticAbilityApiBased.java` | Static ability with API-based resolution | Not implemented |
| `IllegalAbilityException.java` | Exception for invalid ability definitions | Not implemented |
| `package-info.java` | Package doc | N/A |

---

## 3. Ability Effects (`game/ability/effects/`)

204 files — Individual effect implementations. Each file is a `SpellAbilityEffect` subclass.

| Java File | Feature | forge-engine Status |
|-----------|---------|:-------------------:|
| `AbandonEffect.java` | Abandon a card | Not implemented |
| `ActivateAbilityEffect.java` | Activate an ability on a card | **Partial** — `activate_ability_effect.rs`: resolves common `ManaAbility$ True` path by activating one mana ability on each matching permanent (used by Pygmy Hippo) |
| `AddPhaseEffect.java` | Add extra phase to turn | **Implemented** — `add_phase_effect.rs`: `ExtraPhase$ Combat`, `Amount$`; increments `game.extra_combat_phases`; game loop inserts extra combat cycles |
| `AddTurnEffect.java` | Add extra turn | **Implemented** — `add_turn_effect.rs`: `Defined$` player, `NumTurns$` (default 1), `SkipUntap$`; pushes `ExtraTurn` struct onto queue; `AdvanceTurn` pops and applies skip_untap flag |
| `AmassEffect.java` | Amass N (create/grow army token) | Not implemented |
| `AnimateEffect.java` | Animate a permanent (change type/P&T) | **Implemented** — `animate_effect.rs`: `Defined$`/target card; `Power`, `Toughness`, `Types`, `Keywords`, `Colors`, `OverwriteTypes` params; saves `AnimateState` on card; restores at cleanup (step_cleanup). Covers Mutavault, manlands, vehicles |
| `AnimateAllEffect.java` | Animate all matching permanents | **Implemented** — `animate_all_effect.rs`: `ValidCards$` filter; `Power`, `Toughness`, `Types`, `Keywords` (`&`-separated), `Colors`, `OverwriteColors`, `RemoveCreatureTypes`, `RemoveAllAbilities` params; saves `AnimateState`; restores at cleanup. Covers Natural Affinity, Sylvan Awakening, Start Your Engines, Mass Diminish |
| `AnimateEffectBase.java` | Base class for animate effects | **Implemented** — logic merged into `animate_effect.rs` |
| `AscendEffect.java` | Check/grant City's Blessing | Not implemented |
| `AttachEffect.java` | Attach aura/equipment to permanent | **Implemented** — `attach.rs`: attaches source Equipment/Aura to target creature on battlefield; handles detach from previous host; `CardInstance.attached_to`/`attachments` fields added; `GameState.attach_to`/`detach`/`remove_from_stack` in `action.rs` |
| `BalanceEffect.java` | Balance-type equalization effect | **Implemented** — `balance_effect.rs`: `Valid$` filter, `Zone$` (Battlefield/Hand); counts per player, finds minimum; excess cards sacrificed (Battlefield) or discarded (Hand) via agent choice |
| `BecomeMonarchEffect.java` | Become the Monarch | **Implemented** — `become_monarch_effect.rs`: `Defined$` player; sets `game.monarch`; fires `BecomeMonarch` trigger; monarch draws card at end of turn |
| `BidLifeEffect.java` | Bid life (auction mechanic) | Not implemented |
| `BlockEffect.java` | Force/modify blocking | Not implemented |
| `BranchEffect.java` | Conditional branching in ability chains | Not implemented |
| `ChangeZoneEffect.java` | Move card(s) to another zone | **Implemented** (`game_loop.rs` ChangeZone handler: targeted/defined/self, LibraryPosition, Shuffle, Tapped, ChangesZone trigger) |
| `ChangeZoneAllEffect.java` | Move all matching cards to a zone | **Implemented** (`game_loop.rs`/`change_zone_all_effect.rs`: ValidCards/ChangeType filters, multi-player, triggers, targeted `ChangeType$` clause support incl. `TargetedCard.Self`, `NotDefinedTargeted`, `sharesNameWith Targeted`, `ControlledBy TargetedController`) |
| `CharmEffect.java` | Modal "choose N" charm abilities | **Implemented** — `charm_effect.rs`: `SP$ Charm`, `Choices$ SVar1,...`, `CharmNum$`, `MinCharmNum$`; cast-time mode chaining and target setup mirror Java stack behavior; resolution-time fallback via `TargetKind` dispatch; agent `choose_mode`; TauriAgent `ChooseMode` prompt + `ModeDecision` response; `ChooseModeModal` frontend |
| `ChooseCardEffect.java` | Choose a card from a set | **Implemented** — `choose_card_effect.rs`: `Amount$`, `ChoiceZone$`, `Choices$` filter, `RememberChosen$`; stores chosen on source card's `chosen_cards`; agent `choose_cards_for_effect()` + TauriAgent `ChooseCardsForEffect` prompt + `ChooseCardsModal` frontend |
| `ChooseCardNameEffect.java` | Name a card | **Implemented** — `name_card_effect.rs`: `ChooseFromList$`/`ChooseFromDefinedCards$` modes + open naming; stores in `card.named_cards`; agent `choose_card_name`; TauriAgent `ChooseCardName` prompt + `CardNameDecision` response; `ChooseCardNameModal` frontend |
| `ChooseColorEffect.java` | Choose a color | **Implemented** — `choose_color_effect.rs`: `Defined$` player(s), `Choices$` valid colors (default all 5); stores on source card's `chosen_colors`; agent `choose_color()` + TauriAgent `ChooseColor` prompt + `ChooseColorModal` frontend |
| `ChooseTypeEffect.java` | Choose a type | **Implemented** — `choose_type_effect.rs`: `Type$` category (Creature/Card/Land), builds type list; stores in `card.chosen_type`; agent `choose_type`; TauriAgent `ChooseType` prompt + `TypeDecision` response; `ChooseTypeModal` frontend |
| `ChoosePlayerEffect.java` | Choose a player | **Implemented** — `choose_player_effect.rs`: reuses `choose_target_player` agent method; stores in `card.chosen_player` |
| `CloneEffect.java` | Copy/clone a permanent | **Implemented** — `clone_effect.rs`: copies characteristics (name, types, P/T, keywords, abilities, triggers, svars, statics, replacements) from source to target; `Choices$` + `ChoiceZone$` for player selection; `Defined$` / target resolution; `PumpKeywords$`; re-registers triggers. Distinct from `CopyPermanentEffect` (which creates a token copy) |
| `ConniveEffect.java` | Connive N (draw + discard) | Not implemented |
| `ControlGainEffect.java` | Gain control of permanent | **Implemented** — `control_gain.rs`: changes controller of target battlefield permanent via `GameState.change_controller`; moves card between per-player zone lists |
| `ControlGainVariantEffect.java` | Complex control redistribution | **Implemented** — `control_gain_variant_effect.rs`: `ChangeController$` mode (`CardOwner` for Homeward Path, `Random` for Scrambleverse); `AllValid$` filter for affected permanents |
| `CopyPermanentEffect.java` | Copy a permanent onto battlefield | **Partial** — see `CloneEffect.java` above |
| `CopySpellAbilityEffect.java` | Copy a spell on the stack | **Implemented** — `copy_spell_ability_effect.rs`: clones topmost stack entry with same targets; pushes copy onto stack |
| `CounterEffect.java` | Counter a spell or ability | **Implemented** — `counter.rs`: removes targeted stack entry via `MagicStack.remove_by_id`; moves source card to graveyard (or Destination$); `TargetKind::Spell` + `target_stack_entry: Option<u32>` in targeting system; `ChooseTargetSpell` prompt + clickable stack UI |
| `CountersPutEffect.java` | Put counters on a permanent/player | **Implemented** — `counters_put_effect.rs`: puts `CounterType$` counters on source/defined card or player; fires `CounterAdded`; supports `Monstrosity$ True` with persistent monstrous-state tracking and `BecomeMonstrous` trigger parity |
| `CountersRemoveEffect.java` | Remove counters | **Partial** — `counters_remove_effect.rs`: removes specific `CounterType$` counters from `Defined$ Self` or targeted card; `CounterNum$` supports integer and "All"; fires `CounterRemoved` trigger. Deferred: `CounterType$ Any/All` (interactive selection), `Choices$`, `Optional$`, `UpTo$`, player counter removal |
| `CountersMoveEffect.java` | Move counters between permanents | **Implemented** — `move_counter_effect.rs`: `CounterType$`, `CounterNum$`, `Source$`/`Defined$`; moves counters between permanents; fires CounterAdded/CounterRemoved triggers |
| `CountersMultiplyEffect.java` | Multiply counters | Not implemented |
| `CountersProliferateEffect.java` | Proliferate | **Implemented** — `proliferate_effect.rs`: collects permanents with counters, player chooses which to proliferate via `choose_cards_for_effect`, adds one counter of each existing type |
| `DamageAllEffect.java` | Deal damage to all matching | **Implemented** — `damage_all_effect.rs`: `ValidCards$` + `ValidPlayers$` filters, fixed `NumDmg$`; deals to matching creatures and optionally all players |
| `DamageBaseEffect.java` | Base class for damage effects | **Partial** |
| `DamageDealEffect.java` | Deal damage to target | **Implemented** (`damage_deal_effect.rs`) |
| `DamageEachEffect.java` | Deal damage to each matching | **Implemented** — `each_damage_effect.rs`: each matching creature/player deals damage; `ValidCards$`, `NumDmg$`, `DefinedDamagers$` |
| `DamagePreventEffect.java` | Prevent damage | **Implemented** — `prevent_damage_effect.rs`: `Amount$` shields on target creature or player; `Defined$` (Self/Targeted/You/Opponent); decremented when damage dealt |
| `DamageResolveEffect.java` | Resolve queued damage | Not implemented |
| `DayTimeEffect.java` | Change day/night | Not implemented |
| `DelayedTriggerEffect.java` | Create delayed trigger | **Partial** — `delayed_trigger_effect.rs`: parses/registers delayed triggers (`Mode$ ...`, `Execute$ ...`), supports remembered numeric payload (`RememberNumber$` / `RememberSVarAmount$`) |
| `DestroyEffect.java` | Destroy target permanent | **Implemented** (`destroy_effect.rs`: moves target battlefield permanent to graveyard) |
| `DestroyAllEffect.java` | Destroy all matching permanents | **Implemented** — `destroy_all_effect.rs`: `ValidCards$` filter, respects `Indestructible` keyword and R$-based replacement effects; `NoRegen$ True` noted (regeneration not yet implemented) |
| `DigEffect.java` | Look at top N cards, choose some | **Implemented** — `dig.rs`: `DigNum$`, `ChangeNum$` (All/Any/N), `DestinationZone$`/`DestinationZone2$`, `ChangeValid$`, `LibraryPosition2$`, optional; agent `choose_dig`; TauriAgent `Dig` prompt + `DigDecision` response |
| `DiscardEffect.java` | Force discard | **Implemented** — `discard.rs`: target player or Defined$ player discards N (`NumCards$`) cards; agent `choose_discard`; TauriAgent `ChooseDiscard` prompt (reuses `LibraryPeekModal` in "discard" mode) + `DiscardDecision` response; fires `Discarded` trigger. Frontend discard modal now resolves card IDs from the prompt-local game snapshot (`currentPrompt.gameView.myHand`) to avoid stale hand races after draw-then-discard chains (`Game.tsx`). |
| `DiscoverEffect.java` | Discover N mechanic | Not implemented |
| `DrawEffect.java` | Draw cards | **Partial** (`action.rs` draw_cards) |
| `EffectEffect.java` | Create emblem/effect on battlefield | **Partial** — `effect_effect.rs`: supports `AB$ Effect` with `StaticAbilities$` from SVars by creating command-zone effect cards; remembers targeted/remembered cards/players (`RememberObjects$`) and tracks `Card.EffectSource`; duration support includes default EOT, `Permanent`, `UntilHostLeavesPlay`, `UntilHostLeavesPlayOrEOT`; supports `ForgetOnMoved$` origin wiring for remembered-object cleanup/exile |
| `DrainManaEffect.java` | Drain mana pools | **Implemented** — `drain_mana_effect.rs`: clears defined players' mana pools, `DrainMana$ True` preserves original colors, `RememberDrainedMana$ True`, mana burn life loss |
| `EndTurnEffect.java` | End the turn | **Implemented** — `end_turn_effect.rs`: clears stack, sets `game.end_turn_requested`; game loop skips remaining phases to cleanup |
| `ExploreEffect.java` | Explore mechanic | **Implemented** — `explore_effect.rs`: reveal top card, land→hand, nonland→+1/+1 counter + optional graveyard; reuses `choose_optional_trigger` |
| `FightEffect.java` | Fight between creatures | **Implemented** — `fight.rs`: source creature and target creature deal damage to each other equal to power simultaneously; fires `Fight` trigger; `TriggerType::Fight` + `RunParams.card2` added to event system |
| `FlipCoinEffect.java` | Flip a coin | **Implemented** — `flip_a_coin_effect.rs`: random bool, `NoCall` flag for Heads/TailsSubAbility or WinSubAbility/LoseSubAbility; call-mode now routes through Java-parity `choose_binary(HeadsOrTails)`; resolves sub-ability chains |
| `FogEffect.java` | Prevent all combat damage | **Implemented** — `fog_effect.rs`: sets `prevent_all_combat_damage` flag on `GameState`; combat `resolve_damage_step()` returns empty when flag is set; flag reset at end of turn cleanup |
| `GameDrawEffect.java` | Force game draw | **Implemented** — `game_draw_effect.rs`: sets all players `has_lost = true`, `game_over = true`, `winner = None` |
| `GameLossEffect.java` | Force player to lose | **Implemented** — `game_loss_effect.rs`: `Defined$` player, sets `has_lost = true`; checks remaining alive players for game over |
| `GameWinEffect.java` | Force player to win | **Implemented** — `game_win_effect.rs`: `Defined$` player, sets `has_won = true` on winner, `has_lost = true` on all others, `game_over = true` |
| `GoadEffect.java` | Goad a creature | **Implemented** — `goad_effect.rs`: sets `goaded_by = Some(controller)` on target; goaded creature must attack but can't attack goader |
| `LifeGainEffect.java` | Gain life | **Partial** (`player.rs` gain_life) |
| `LifeLoseEffect.java` | Lose life | **Partial** (`player.rs` lose_life) |
| `LifeSetEffect.java` | Set life total | **Implemented** — `life_set_effect.rs`: `Defined$` player (supports `Each`/`All` multi-player), `LifeAmount$`; uses `PlayerState.set_life()` which computes diff; fires LifeGained or LifeLost trigger based on difference |
| `LifeExchangeEffect.java` | Exchange life totals | **Implemented** — `life_exchange_effect.rs`: swaps life totals between controller and `Defined$`/targeted player; fires LifeGained/LifeLost triggers for each player based on diff |
| `ManaEffect.java` | Add mana to pool | **Implemented** — `mana_effect.rs`: Any/Combo/Chosen/raw Produced$, Amount$ multiplier, ProduceMana replacement effects, Special types (EachColorAmong, DoubleManaInPool, EnchantedManaCost, EachColoredManaSymbol, LastNotedType) |
| `ManaReflectedEffect.java` | Reflected mana (any color matching…) | **Implemented** — `mana_reflected_effect.rs`: ReflectProperty$ Is/Produce/Produced, ColorOrType$ Color/Type, Valid$ filtering |
| `ManifestEffect.java` | Manifest (face-down) | Not implemented |
| `MeldEffect.java` | Meld two cards | Not implemented |
| `MillEffect.java` | Mill N cards | **Implemented** — `mill.rs`: `NumCards$`, targeted or `Defined$` player, moves top N from library to graveyard, emits ChangesZone trigger |
| `MutateEffect.java` | Mutate a creature | Not implemented |
| `PermanentCreatureEffect.java` | Resolve creature permanent spell | Not implemented |
| `PermanentNoncreatureEffect.java` | Resolve non-creature permanent spell | Not implemented |
| `PhasesEffect.java` | Phase in/out | **Implemented** — `phases_effect.rs`: `PhaseInOrOut$ In/Out`; toggles `card.phased_out`; phased-out permanents treated as invisible; phase-in during untap step; fires PhasedOut/PhasedIn triggers |
| `PlayEffect.java` | Play card from zone (exile, GY) | **Implemented** — `play_effect.rs`: casts from exile/graveyard (Flashback, Cascade, etc.) |
| `PoisonEffect.java` | Give poison counters | **Implemented** — `poison_effect.rs`: adds/removes `Num$` poison counters (supports negative values, floors at 0); `Defined$` (Player/Opponent/You) and `ValidTgts$ Player` targeting; `Defined$ Player` applies to all alive players |
| `ProtectEffect.java` | Grant protection | **Implemented** — `protection_effect.rs`: `Gains$` protection keyword, `Choices$` for color choice; adds to `pump_keywords` |
| `PumpEffect.java` | +N/+N (or set P/T) until end of turn | **Implemented** (`pump_effect.rs`: single-target power/toughness modifier until EOT) |
| `PumpAllEffect.java` | Pump all matching creatures | **Implemented** — `pump_all_effect.rs`: `ValidCards$` filter, `NumAtt$`/`NumDef$` (signed, supports negative debuffs), `YouCtrl`/`OppCtrl`; duration = EOT (zeroed by `step_cleanup`) |
| `RegenerateEffect.java` | Regenerate a permanent | **Implemented** — `regenerate_effect.rs`: adds regeneration shields to target creature; shields consumed instead of destroy (tap + remove damage); shields reset at end of turn |
| `RevealEffect.java` | Reveal cards | **Partial** — `reveal.rs`: reveals N cards from target hand, notifies all agents; no full interactive UI reveal |
| `RollDiceEffect.java` | Roll dice | **Implemented** — `roll_dice_effect.rs`: random 1-N (default d20), `ResultSubAbilities$` threshold matching, resolves sub-ability chains |
| `SacrificeEffect.java` | Force sacrifice | **Implemented** (`game_loop.rs` Sacrifice handler: SacValid$Self or matching permanents, agent choose_sacrifice for human choice, ChangesZone trigger) |
| `SacrificeAllEffect.java` | Force sacrifice of all matching | **Implemented** (`game_loop.rs` SacrificeAll handler: ValidCards filter, multi-player, ChangesZone trigger) |
| `ScryEffect.java` | Scry N | **Implemented** — `scry.rs`: `ScryNum$`, `Defined$` player, agent `choose_scry`; TauriAgent `Scry` prompt + `ScryDecision` response; PassAgent keeps all on top |
| `SetStateEffect.java` | Transform / flip / turn face-up/down | **Implemented** — `set_state_effect.rs`: `Mode$ Transform/Flip/TurnFaceUp/TurnFaceDown`; Transform with optional condition gate; Flip toggles `card.flipped`; TurnFaceUp/Down toggles `card.face_down` |
| `ShuffleEffect.java` | Shuffle library | **Implemented** — `shuffle_effect.rs`: `Defined$` player (default You), `Optional` flag; calls `game.shuffle_library()` |
| `SkipPhaseEffect.java` | Skip a phase | **Implemented** — `skip_phase_effect.rs`: `Phase$ Draw/Combat/Untap`, `Defined$` player; sets per-player skip flags; game loop checks before each phase |
| `SurveilEffect.java` | Surveil N | **Implemented** — `surveil.rs`: `Amount$`, `Defined$` player, agent `choose_surveil`; TauriAgent `Surveil` prompt + `SurveilDecision` response; emits ChangesZone trigger for graveyard cards |
| `TapEffect.java` | Tap a permanent | **Implemented** — `tap_effect.rs`: taps targeted or `Defined$ Self` battlefield permanent; `ETB$` (silent tap), `RememberTapped$`/`AlwaysRemember$` (store in remembered_cards); fires `Taps` trigger |
| `TapAllEffect.java` | Tap all matching | **Implemented** — `tap_all_effect.rs`: `ValidCards$` filter with full `YouCtrl`/`OppCtrl`/color qualifier support |
| `TokenEffect.java` | Create token(s) | **Implemented** — `Token` handler in `game_loop.rs`: `TokenScript$`, `TokenAmount$`, `TokenOwner$` (You/Opponent). Token templates loaded from `tokenscripts/` via `get_token_db()` and registered in `GameLoop`. Tokens flagged `is_token` and cease to exist when leaving battlefield (CR 110.5g). |
| `TokenEffectBase.java` | Base class for token creation | **Implemented** — see `TokenEffect.java` above |
| `UntapEffect.java` | Untap a permanent | **Implemented** — `untap_effect.rs`: untaps targeted or `Defined$ Self` / `ParentTarget` / explicit target battlefield permanent; `ETB$` (silent untap); fires `Untaps` trigger |
| `UntapAllEffect.java` | Untap all matching | **Implemented** — `untap_all_effect.rs`: `ValidCards$` filter with full qualifier support |
| `VoteEffect.java` | Council's dilemma / voting mechanic | Not implemented |

> **Note**: 204 effect files total. 67 have full implementation, ~12 partial. Additional implemented effects not listed individually: `RevealHandEffect.java` → `reveal_hand.rs`, `LookAtEffect.java` → `look_at.rs`, `RearrangeTopOfLibraryEffect` → `rearrange_top_of_library.rs`, `PeekAndRevealEffect.java` → `peek_and_reveal_effect.rs`, `CleanupEffect.java` → `cleanup_effect.rs`, `ReverseTurnOrderEffect.java` → `reverse_turn_order_effect.rs` (reverses player_order), `EndCombatPhaseEffect.java` → `end_combat_phase_effect.rs` (sets end_combat_requested flag), `PowerExchangeEffect.java` → `power_exchange_effect.rs` (swaps power between two creatures), `TakeInitiativeEffect.java` → `take_initiative_effect.rs` (sets initiative_holder), `SkipTurnEffect.java` → `skip_turn_effect.rs` (increments player skip_turns counter), `PlayEffect.java` → `play_effect.rs` (casts from exile/graveyard), `RepeatEachEffect.java` → `repeat_each_effect.rs` (loops sub-ability over cards/players).
>
> **Deferred effects** (require major subsystems): `Venture` (dungeon system), `RingTemptsYou` (ring system), `ControlPlayer` (controller redirection).
>
> The remaining ~131 effects are **not implemented**. See [Section 23](#23-priority-analysis--whats-missing) for priority breakdown.

---

## 4. Card System (`game/card/`)

28 files — Core card representation, collections, factories, predicates.

| Java File | Feature | forge-engine Status |
|-----------|---------|:-------------------:|
| `Card.java` | Core card class — full state, abilities, types, counters, damage | **Implemented** (`card.rs` CardInstance) |
| `CardState.java` | Single state of card (front/back) with mutable properties | **Partial** — dual-face support: `CardOtherPart` stores back face; `transform()` swaps all face characteristics, `is_transformed` flag; `CardSplitType::is_dual_faced()` used during card loading |
| `CardFactory.java` | Factory: creates Card instances from templates | **Partial** (create_card in `game.rs`) |
| `CardFactoryUtil.java` | Card creation utilities | **Partial** (ETBReplacement keyword processing in `game_loop.rs` `resolve_stack()`) |
| `CardCollection.java` | Mutable card collection | **Implemented** (Vec<CardId> in zones) |
| `CardCollectionView.java` | Immutable card collection view | Not implemented (no view layer) |
| `CardCopyService.java` | Card copying: tokens, clones, cross-game | Not implemented |
| `CardDamageHistory.java` | Damage history: attacks, blocks, damage per phase | **Implemented** — `card/damage_history.rs`: DamageHistory struct with record_attack/block/damage, end_combat/new_turn; wired in phase_handler.rs |
| `CardDamageMap.java` | Damage source→target mapping with trigger integration | Not implemented |
| `CardFaceView.java` | Card face display record | Not implemented |
| `CardLists.java` | Static filter utilities for card collections | Not implemented |
| `CardPlayOption.java` | Special play permissions from static abilities | Not implemented |
| `CardPredicates.java` | Predicate factories for card filtering | **Partial** (ValidCard matching in trigger.rs) |
| `CardProperty.java` | Evaluates card properties against string specs | **Partial** (`card/card_property.rs`: dot/plus property matching used by targeting filters; includes Java-style inclusive type token checks like `Creature.YouCtrl`) |
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
| `Combat.java` | Combat state: attackers, blockers, damage assignment | **Implemented** (`combat/mod.rs` CombatState with DefenderId multi-defender support, `remove_absent_combatants()`, LKI cache) |
| `CombatUtil.java` | Combat utility methods | **Partial** — attack/block checks, `get_possible_defenders()`, attack costs (`attack_cost.rs`), block costs (`block_cost.rs`), lure/must-block (`compute_must_block_targets`), block validation (`validate_blocks`), and blocker-declaration parity gating (enter blockers only if at least one legal block exists) |
| `CombatView.java` | Combat view for UI | Not implemented |
| `CombatLki.java` | Last-known-information during combat | **Implemented** (`combat/mod.rs` CombatLki struct, save_lki/was_attacking/was_blocking/get_combat_lki) |
| `AttackConstraints.java` | Attack requirement/restriction aggregation | **Partial** — `attack_requirement.rs` + `attack_restriction.rs` handle goad, must-attack, OnlyAlone, NotAlone, NeedGreaterPower, NeedTwoOthers, Never |
| `AttackRequirement.java` | "Must attack" requirements | **Implemented** — `combat/attack_requirement.rs`: computes requirements from statics + goad |
| `AttackRestriction.java` | "Can't attack" restrictions | **Implemented** — `combat/attack_restriction.rs`: validates restrictions after attacker declaration |
| `AttackRestrictionType.java` | Attack restriction type enum | **Implemented** — `combat/attack_restriction.rs`: OnlyAlone, NotAlone, NeedGreaterPower, NeedTwoOthers, Never |
| `AttackingBand.java` | Banding attack groups | Not implemented |
| `GlobalAttackRestrictions.java` | Global attack limits | **Implemented** — `static_ability_attack_restrict.rs` + phase_handler prioritizes must-attackers when limit exceeded |

---

## 8. Costs (`game/cost/`)

60 files — Spell/ability cost definitions and payment logic.

| Java File | Feature | forge-engine Status |
|-----------|---------|:-------------------:|
| `Cost.java` | Cost container: parses cost strings, holds cost parts | **Partial** (`cost/mod.rs` parse_cost + spell cost extraction from SP$ lines) |
| `CostPartMana.java` | Mana portion of costs | **Implemented** (`mana_pool.rs` try_pay) |
| `CostPayment.java` | Cost payment orchestration | **Partial** (`game_loop.rs` pay_ability_cost + pay_additional_costs; Java-parity `confirm_payment` gates specific optional/branching cost parts; `Cost` now parses/carries `Mandatory` and trigger-cost optionality honors non-mandatory, non-zero costs; failed multi-part payments now transactionally roll back via `GameSnapshot` restore) |
| `CostPart.java` | Abstract base for cost components | **Partial** (`cost/mod.rs` CostPart enum now covers Java parse tokens, with some behavioral edge-cases still approximate) |
| `CostPartWithList.java` | Cost part tracking affected cards | Not implemented |
| `CostPartWithTrigger.java` | Cost part that fires triggers | Not implemented |
| `CostTap.java` | Tap as cost | **Implemented** (`CostPart::Tap` in `cost/mod.rs`) |
| `CostUntap.java` | Untap as cost | **Implemented** (`CostPart::Untap` in `cost/mod.rs`, `Q` token) |
| `CostSacrifice.java` | Sacrifice as cost | **Implemented** (`cost/mod.rs` get_sacrifice_targets, `game_loop.rs` pay_sacrifice_cost) |
| `CostPayLife.java` | Pay life as cost | **Implemented** (`CostPart::PayLife`, `PayLife<n>` token) |
| `CostPayEnergy.java` | Pay energy counters | **Implemented** (`CostPart::PayEnergy`, `PayEnergy<n>` token; energy tracked on `PlayerState`) |
| `CostPayShards.java` | Pay shard tokens | **Implemented** (`CostPart::PayShards`; `PlayerState.mana_shards`; paid in `game_action.rs`) |
| `CostDiscard.java` | Discard as cost | **Implemented** (`CostPart::Discard`, `Discard<n/filter>` token) |
| `CostExile.java` | Exile as cost | **Implemented** (`CostPart::Exile`, `Exile<n/filter>` / `ExileFromHand<>` / `ExileFromGrave<>` / `ExileFromTop<>`; includes `CantExile` static-ability legality in can-pay/payment paths) |
| `CostExileFromStack.java` | Exile from stack as cost | **Implemented** (`CostPart::ExileFromStack`; Java-style clause normalization into `matches_valid_cards`, exact stack-entry selection via `choose_target_spell`, and source exile in `game_action.rs`) |
| `CostDamage.java` | Deal damage to self as cost | **Implemented** (`CostPart::DamageYou`, `DamageYou<n>` token) |
| `CostDraw.java` | Draw as cost | **Implemented** (`CostPart::Draw`, `Draw<n>` token) |
| `CostMill.java` | Mill as cost | **Implemented** (`CostPart::Mill`, `Mill<n>` token) |
| `CostReturn.java` | Return to hand as cost | **Implemented** (`CostPart::Return`, `Return<n/filter>` token) |
| `CostReveal.java` | Reveal as cost | **Implemented** (`CostPart::Reveal`, `Reveal<n/filter>` token; no-op state change) |
| `CostPutCounter.java` | Put counter as cost | **Implemented** (`CostPart::AddCounter`, `AddCounter<n/type>` token) |
| `CostRemoveCounter.java` | Remove counter as cost | **Implemented** (`CostPart::SubCounter`, `SubCounter<n/type/CARDNAME>` token) |
| `CostRemoveAnyCounter.java` | Remove any counter as cost | **Implemented** (`CostPart::RemoveAnyCounter`, `RemoveAnyCounter<n/type/filter>` token) |
| `CostTapType.java` | Tap matching permanent as cost | **Implemented** (`CostPart::TapType`, `tapXType<n/filter>` token) |
| `CostUntapType.java` | Untap matching permanent as cost | **Implemented** (`CostPart::UntapType`, `untapYType<n/filter>` token) |
| `CostGainLife.java` | Opponent gains life as cost | **Implemented** (`CostPart::GainLife`, `GainLife<n>` token) |
| `CostGainControl.java` | Give control as cost | **Implemented** (`CostPart::GainControl`, `GainControl<n/filter>` token; calls `change_controller`) |
| `CostFlipCoin.java` | Flip coin as cost | **Implemented** (`CostPart::FlipCoin`; RNG/call handling + `FlippedCoin` trigger fire in `game_action.rs`) |
| `CostRollDice.java` | Roll dice as cost | **Implemented** (`CostPart::RollDice`; cost-time roll + result SVar write + `RolledDie`/`RolledDieOnce` trigger fire) |
| `CostExert.java` | Exert as cost | **Implemented** (`CostPart::Exert`, `Exert<>` token; sets `card.exerted` flag) |
| `CostEnlist.java` | Enlist as cost | **Implemented** (`CostPart::Enlist`; Java-style `Enlist<1/CARDNAME/creature>` parsing, enlist target selection/tap, attacker gets enlisted creature's power until end of turn, Enlisted trigger) |
| `CostForage.java` | Forage as cost | **Implemented** (`CostPart::Forage`; strict graveyard-3 exile vs Food sacrifice payment + Forage trigger) |
| `CostCollectEvidence.java` | Collect evidence as cost | **Implemented** (`CostPart::CollectEvidence`; strict MV-threshold exile payment + CollectEvidence trigger) |
| `CostChooseColor.java` | Choose color as cost | **Implemented** (`CostPart::ChooseColor`; stores chosen colors on source card) |
| `CostChooseCreatureType.java` | Choose creature type as cost | **Implemented** (`CostPart::ChooseCreatureType`; stores chosen type on source card) |
| `CostPutCardToLib.java` | Put card to library as cost | **Implemented** (`CostPart::PutCardToLib`; hand/grave/same-grave/battlefield variants) |
| `CostAddMana.java` | Add mana to pool as cost | **Implemented** — `CostPart::AddMana` in `cost/mod.rs`, parsed as `AddMana<amount/type>`, paid in `game_action.rs` |
| `CostWaterbend.java` | Waterbend cost (tap artifacts/creatures to help pay) | **Implemented** — `CostPart::Waterbend` in `cost/mod.rs`, reuses Convoke agent for creature/artifact tapping |
| `CostUnattach.java` | Unattach as cost | **Implemented** (`CostPart::Unattach`, `Unattach<>` token; calls `game.detach()`) |
| `CostAdjustment.java` | Cost increase/decrease logic | **Implemented** (`ReduceCost`/`RaiseCost`/`SetCost` statics via `static_ability_cost_change.rs`; supports `Color$`, `IgnoreGeneric$`, `IsPresent$`/`PresentZone$`, `EffectZone$`, `MinMana$`, `Activator$`, `ValidCard$`, `CheckSVar$`/`SVarCompare$`, `OnlyFirstSpell$`, `Relative$`, `UpTo$`, `RaiseTo$` (Trinisphere), `ValidTarget$`, `ValidSpell$`, `Condition$` (PlayerTurn/Metalcraft/Delirium)) |
| `CostBlight.java` | Blight as cost | **Implemented** (`CostPart::Blight`; applies -1/-1 counter payment, including `X`/dynamic amount resolution) |
| `CostBehold.java` | Behold as cost | **Implemented** (`CostPart::Behold` with reveal path over hand/battlefield) |
| `CostBeholdExile.java` | Behold exile variant | **Implemented** (`CostPart::Behold { exile: true }`) |
| `CostPromiseGift.java` | Promise a gift as cost | **Implemented** (`CostPart::PromiseGift`; stores `promised_gift` on source card) |
| `CostRevealChosen.java` | Reveal chosen card as cost | **Implemented** (`CostPart::RevealChosen`; chosen value + chooser-controller checked in can-pay, reveal state set on payment) |
| `CostExiledMoveToGrave.java` | Move exiled to graveyard as cost | **Implemented** (`CostPart::ExiledMoveToGrave`, `ExiledMoveToGrave<n/filter>` token) |
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
| `GameEventMulligan.java` | Mulligan event | **Partial** (logged via GameLogEntryType::Mulligan) |
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
| `Keyword.java` | Enum of all MTG keywords (~200+) | **Partial** (string-based keywords in `card.rs`; 46 keywords with runtime logic — see Keyword Runtime Status below) |
| `KeywordInterface.java` | Interface for keyword instances | **Partial** (Vec<String> on CardInstance) |
| `KeywordInstance.java` | Abstract keyword instance with parameters | Not implemented |
| `KeywordCollection.java` | Collection of keyword instances | **Partial** (Vec<String>) |
| `KeywordWithAmount.java` | Keywords with numeric values (Bushido 2) | **Partial** (Toxic:N parsed via `get_toxic_count()`) |
| `KeywordWithCost.java` | Keywords with costs (Equip {3}) | **Partial** (Ward:N, Buyback:N, Spectacle:N, Dash:N, etc. parsed via `get_*_cost()` helpers in `card/mod.rs`) |
| `KeywordWithCostAndType.java` | Keywords with cost + type (Cycling {2}) | **Partial** (Buyback, Spectacle, Evoke, Dash, Blitz, Multikicker, Replicate, Entwine, Escalate, Escape, Overload, Madness, Rebound, Strive, Suspend, Foretell, Emerge, Cycling, Offering, Spree — parsed via `get_X_cost()` helpers in `card/mod.rs`) |
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
| Flashback | **Implemented** | Flashback casting from graveyard in `game_loop.rs`, `card/mod.rs`, `spellability/mod.rs` |
| Kicker | **Implemented** | Single kicker; `kicked` flag on CardInstance, `+kicked` trigger/effect filter, `Condition$ Kicked` gate, `Count$Kicked` SVar, `KW$` keyword grant in Pump/PumpAll |
| Storm | **Implemented** | Basic storm copy logic in `game_loop.rs` |
| Cascade | **Implemented** | Cascade in `game_loop.rs` with player choice (optional cast via `choose_optional_trigger`), UI snapshots during exile/targeting, and proper decline handling |
| Prowess | **Implemented** | `has_prowess()` in `card/mod.rs`; auto-trigger generation added — `SpellCast` triggers are generated for cards with the Prowess keyword via the trigger system |
| Magecraft | **Implemented** | `SpellCopied` TriggerType + TriggerMode added to event/trigger system; auto-trigger generation added — `SpellCast` + `SpellCopied` triggers are generated for cards with the Magecraft keyword |
| Buyback | **Implemented** | `get_buyback_cost()` in `card/mod.rs`; `buyback_paid` flag on SpellAbility; `choose_buyback` agent method; spell returns to hand on resolution instead of graveyard; TauriAgent `ChooseBuyback` prompt + frontend modal |
| Spectacle | **Implemented** | `get_spectacle_cost()` in `card/mod.rs`; `Spectacle` AlternativeCost variant; checks `life_lost_this_turn > 0` on opponents; offered as alternative cost choice in `play_card`; included in `get_playable_cards` when affordable |
| Evoke | **Implemented** | `get_evoke_cost()` in `card/mod.rs`; `Evoke` AlternativeCost variant; evokes now register a one-shot `ChangesZone` sacrifice trigger on ETB (instead of immediate move), so ETB triggers still fire correctly (`game_loop/magic_stack.rs`, `trigger/handler.rs`) |
| Dash | **Implemented** | `get_dash_cost()` in `card/mod.rs`; `Dash` AlternativeCost variant; creature gains haste; EOT delayed trigger returns it to hand |
| Blitz | **Implemented** | `get_blitz_cost()` in `card/mod.rs`; `Blitz` AlternativeCost variant; creature gains haste + "dies → draw a card" trigger; EOT delayed trigger sacrifices it |
| Multikicker | **Implemented** | `get_multikicker_cost()` in `card/mod.rs`; `kick_count` on SpellAbility; `choose_multikicker` agent method; TauriAgent `ChooseMultikicker` prompt + frontend counter modal |
| Replicate | **Implemented** | `get_replicate_cost()` in `card/mod.rs`; `replicate_count` on SpellAbility; copies created like Storm; emits `SpellCopied` trigger; TauriAgent `ChooseReplicate` prompt + frontend counter modal |
| Entwine | **Implemented** | `get_entwine_cost()` in `card/mod.rs`; when entwine is paid (via kicked flag), all modes of a modal spell are chosen automatically in `charm_effect.rs` |
| Escalate | **Implemented** | `get_escalate_cost()` in `card/mod.rs`; allows choosing more modes than minimum in `charm_effect.rs`; per-mode cost scaling wired and functional |
| Escape | **Implemented** | `get_escape_cost()` in `card/mod.rs` returns `(mana_cost, exile_count)`; `Escape` AlternativeCost variant; casts from graveyard, exiles N other graveyard cards as additional cost |
| Overload | **Implemented** | `get_overload_cost()` in `card/mod.rs`; `Overload` AlternativeCost + `overloaded` flag on SpellAbility; per-effect "target→each" dispatch implemented and functional |
| Madness | **Implemented** | `get_madness_cost()` in `card/mod.rs`; `Madness` AlternativeCost variant; cards with madness are exiled instead of going to graveyard on discard (`discard_effect.rs`); can be cast from exile for madness cost; cleanup step discard bug fixed |
| Strive | **Partial** | `get_strive_cost()` in `card/mod.rs`; cost helper exists; needs multi-target system |
| Rebound | **Implemented** | `has_rebound()` in `card/mod.rs`; non-permanent spells cast from hand are exiled instead of going to graveyard; delayed trigger at next upkeep casts for free |
| Suspend | **Implemented** | `get_suspend_cost()` — exile with time counters; upkeep removal; free cast (`game_loop.rs`) |
| Foretell | **Implemented** | `get_foretell_cost()` — pay {2} to exile face-down; cast later for foretell cost (`game_loop.rs`) |
| Emerge | **Implemented** | `get_emerge_cost()` — alt cost; sacrifice creature to reduce cost (`game_loop.rs`) |
| Offering | **Implemented** | `get_offering_type()` — sacrifice permanent of type to reduce cost by CMC; grants instant-speed casting (`game_action_util.rs`) |
| Spree | **Implemented** | `K:Spree` — per-mode `ModeCost$` chosen before payment; modes stored on card for `charm_effect.rs` reuse (`game_action_util.rs`) |
| ETBReplacement | **Implemented** | `K:ETBReplacement:Layer:SVarName:Optional` keyword processing in `game_loop.rs` `resolve_stack()`; intercepts permanent spells before `move_card()` to battlefield; parses keyword, looks up SVar, builds SpellAbility via `build_spell_ability()`, asks player for Optional replacements via `choose_optional_trigger()`, runs targeting + effect chain. Enables Clone, Phantasmal Image, and ~375 other cards with ETBReplacement keywords |

---

## 12. Mana (`game/mana/`)

~10 files — Mana pool, mana objects, mana payment.

| Java File | Feature | forge-engine Status |
|-----------|---------|:-------------------:|
| `Mana.java` | Individual mana object with color, source, restrictions | **Implemented** (`mana/mod.rs` — `Mana` struct with color, source_card, is_snow, is_persistent, is_combat_mana, restriction fields; `RestrictValid$` propagated from abilities) |
| `ManaPool.java` | Player's mana pool with payment logic | **Implemented** (`mana/mod.rs` — refactored from integer counters to `Vec<Mana>` individual mana objects with source tracking and snow support; deterministic auto-pay entrypoint in `mana/auto_pay.rs`; auto-tap source selection in `computer_util_mana.rs`, matching harness `AutoPay`'s legality-first battlefield-order source selection and same-host mana-source exclusion, plus Java-harness parity for self-sacrificing activated abilities that reuse the pending source while paying mana (for example Food + Gilded Goose); dual land support; intrinsic mana ability generation for basic subtypes in `card_db.rs`; `land_mana_atoms()` helper for ability-driven mana production; `clear_pool(phase)` retains persistent/combat mana across phase transitions; interactive mana payment for human players via `pay_mana_cost` agent method) |
| `ManaCostBeingPaid.java` | Tracks partial mana cost payment | **Partial** (`mana/mana_cost_being_paid.rs`: Java-shaped unpaid-shard state, shard-priority payment, 2/C generic fallback; wired into `computer_util_mana.rs` auto-pay flow) |
| `ManaConversionMatrix.java` | Mana color conversion rules | Not implemented |
| `ManaAtom.java` | Mana atom bitmask constants | **Implemented** (ManaAtom in `foundation/mana.rs`) |
| `package-info.java` | Package doc | N/A |

---

## 13. Mulligan (`game/mulligan/`)

7 files — Mulligan rule implementations.

| Java File | Feature | forge-engine Status |
|-----------|---------|:-------------------:|
| `MulliganService.java` | Mulligan orchestration for all players | **Implemented** (`mulligan/mod.rs`) |
| `AbstractMulligan.java` | Base mulligan with draw/keep/tuck logic | **Implemented** (`mulligan/mod.rs`) |
| `LondonMulligan.java` | London mulligan (draw 7, put back N) | **Implemented** (`mulligan/mod.rs`) |
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
| `PhaseHandler.java` | Turn/phase state machine with priority | **Partial** (`phase.rs` TurnState + `game_loop.rs`: APNAP priority rotation, draw/combat/end priority windows, upkeep now mirrors Java's conditional second upkeep priority only after cumulative-upkeep processing/resolution, shared state-mutation gateway (`with_shared_state_mutation`) with implicit state broadcast on mutation (including priority handoff/pass windows); extra turns/phases and full replacement-phase interactions still missing) |
| `Phase.java` | Individual phase instance | **Partial** (advance_phase in `phase.rs`) |
| `ExtraTurn.java` | Extra turn tracking | Not implemented |
| `ExtraPhase.java` | Extra phase tracking | Not implemented |
| `Untap.java` | Untap step logic (phasing, untap restrictions) | **Partial** (`phase_handler.rs` untap step incl. optional `choose_binary(UntapOrLeaveTapped)` for "you may choose not to untap" keyword; phasing + full restriction matrix still partial) |
| `package-info.java` | Package doc | N/A |

---

## 15. Player (`game/player/`)

17 files — Player state, controller interface, properties, statistics.

| Java File | Feature | forge-engine Status |
|-----------|---------|:-------------------:|
| `Player.java` | Core player: life, zones, mana, counters, game actions | **Implemented** (`player.rs` PlayerState) |
| `PlayerController.java` | Abstract: UI/AI decision-making interface | **Implemented** (`agent.rs` PlayerAgent trait; multiplayer human transport unified into single `TauriAgent` implementation for local + relayed seats in `src-tauri/src/tauri_agent.rs`) |
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
| `ActivateAbilityAction.java` | Activate ability action | **Partial** (`ActivateMana` + `ActivateAbility` variants; parity main-action enumeration keeps mana abilities out of `activatable` action lists, matching harness `ActionSpace.java`; activated-ability mana-feasibility checks exclude same-host mana sources, matching Java `ComputerUtilMana.canPayManaCost(...)`) |
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
| `ReplacementType.java` | Enum of all replacement types | **Partial** (`replacement.rs` — DamageDone, Draw, DrawCards, Destroy, Moved, GainLife, AddCounter, GameLoss, GameWin, CreateToken, Counter; 32 others as `Other`) |
| `ReplacementResult.java` | Replacement processing result | **Implemented** (`replacement.rs` — Replaced, NotReplaced, Prevented, Updated, Skipped) |
| `ReplacementLayer.java` | Replacement effect ordering layers | **Implemented** (`replacement.rs` — CantHappen, Control, Copy, Transform, Other) |
| `ReplacementEffectView.java` | Replacement effect UI view | Not implemented |
| `ReplaceDamage.java` | Replace damage events | **Partial** (`replacement_handler.rs` — Prevent$ True zeroes damage, is_combat field threaded; no amount operators, no redirection) |
| `ReplaceDealtDamage.java` | Replace damage-dealt events | Not implemented |
| `ReplaceAssignDealDamage.java` | Replace damage assignment | Not implemented |
| `ReplaceDraw.java` | Replace single draw | **Partial** (`replacement_handler.rs` — Skipped/Replaced result; no draw-into replacement) |
| `ReplaceDrawCards.java` | Replace multiple draws | Not implemented |
| `ReplaceGainLife.java` | Replace life gain | **Implemented** (`handler.rs` — GainLife event with Prevent$/ReplaceWith$ GainDouble; wired in `life_gain_effect.rs`) |
| `ReplaceLifeReduced.java` | Replace life reduction | Not implemented |
| `ReplacePayLife.java` | Replace life payment | Not implemented |
| `ReplaceGameLoss.java` | Replace game loss | **Implemented** (`handler.rs` — GameLoss event; CantHappen prevents loss; wired in `game_loss_effect.rs` + SBA life/poison checks) |
| `ReplaceGameWin.java` | Replace game win | **Implemented** (`handler.rs` — GameWin event; CantHappen prevents win; wired in `game_win_effect.rs`) |
| `ReplaceDestroy.java` | Replace destroy events | **Partial** (`replacement_handler.rs` — Replaced blocks SBA destruction; no regeneration shield) |
| `ReplaceMoved.java` | Replace zone change events | **Partial** (`replacement_handler.rs` — NewDestination$ reroutes zone; Destination$/Origin$/ValidCard$ filters; no LKI/ETB handling) |
| `ReplaceCounter.java` | Replace counter spell | **Implemented** (`handler.rs` — Counter event; CantHappen prevents countering; wired in `counter_effect.rs`) |
| `ReplaceAddCounter.java` | Replace counter addition | **Implemented** (`handler.rs` — AddCounter event with AddOneMoreCounter/DoubleCounters; wired in `counters_put_effect.rs` + `proliferate_effect.rs`) |
| `ReplaceRemoveCounter.java` | Replace counter removal | Not implemented |
| `ReplaceMill.java` | Replace mill events | Not implemented |
| `ReplaceToken.java` | Replace token creation | **Implemented** (`handler.rs` — CreateToken event with DoubleToken; wired in `token_effect.rs`) |
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
| `ReplaceProduceMana.java` | Replace mana production | **Implemented** — `ProduceMana` replacement type with multiplier support (doublers) |
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
| `SpellAbility.java` | Core spell/ability class: cost, targeting, resolution | **Partial** (`stack.rs`/`game_action_util.rs`/`spellability/mod.rs`: stack entry context, target storage/setup, `TargetingPlayer$ ...` chooser support, and Java-style spell announcement ordering where modes/targets are chosen from pre-payment game state before mana payment mutates the battlefield) |
| `SpellAbilityStackInstance.java` | Spell on the stack with full context | **Implemented** (`stack.rs` StackEntry) |
| `SpellPermanent.java` | Permanent spell (creature/non-creature) | Not implemented |
| `Spell.java` | Non-permanent spell base | Not implemented |
| `AbilityActivated.java` | Activated ability | **Implemented** (`ability/activated.rs` parses `AB$` lines; `game_loop.rs` activate_ability resolves mana/non-mana; UI wired via `ActivatableAbilityInfo` in prompt.rs + tauri_agent.rs) |
| `AbilityStatic.java` | Static ability wrapper | Not implemented |
| `AbilitySub.java` | Sub-ability in ability chain | Not implemented |
| `AbilityManaPart.java` | Mana ability component | **Implemented** (`ability/activated.rs` is_mana_ability flag; `game_loop.rs` resolve_mana_ability produces mana from `Produced$` param including `Combo` colors and `Combo ColorIdentity` (commander color identity lookup) via `choose_color`, resolves `SubAbility$` chains for pain lands; creature mana abilities wired to UI via tappable list; multi-ability picker modal for lands with multiple mana abilities; intrinsic mana abilities auto-generated for basic land subtypes — supports shock/dual lands) |
| `LandAbility.java` | Land play ability | **Partial** (intrinsic mana abilities auto-generated for basic land subtypes in `card_db.rs`; `land_mana_atoms()` in `mana/mod.rs`) |
| `OptionalCost.java` | Optional additional costs (Kicker, Buyback) | **Implemented** (`AlternativeCost` enum + kicker/buyback/multikicker/replicate/entwine in `spellability/mod.rs`, `game_loop.rs`; `kicked`/`buyback_paid`/`kick_count`/`replicate_count` flags on SpellAbility; `+kicked` filter, `Condition$ Kicked` gate) |
| `OptionalCostValue.java` | Optional cost value tracking | Not implemented |
| `SpellAbilityCondition.java` | Conditions for ability activation | Not implemented |
| `SpellAbilityPredicates.java` | Spell ability filtering predicates | Not implemented |
| `SpellAbilityRestriction.java` | Activation restrictions | Not implemented |
| `SpellAbilityVariables.java` | Zone, phase, speed, threshold conditions | Not implemented |
| `SpellAbilityView.java` | Trackable view for UI | Not implemented |
| `StackItemView.java` | Stack item view for UI | Not implemented |
| `TargetChoices.java` | Target selection container | **Partial** (TargetChoice enum in `agent.rs`) |
| `TargetRestrictions.java` | Targeting restrictions (type, zone, count) | **Partial** (`spellability/target_restrictions.rs`: `ValidTgts` parsing, `TargetMin/Max`, stack spell targeting, and non-battlefield `Origin$` zone-target parsing for `CardInZone`) |
| `MagicStack.java` | *(in zone/ but logically here)* | **Implemented** (`stack.rs`) |
| `WrappedAbility.java` | *(in trigger/)* Ability wrapper for triggers | Not implemented |
| `package-info.java` | Package doc | N/A |

---

## 19. Static Abilities (`game/staticability/`)

60 files — Continuous effects and restrictions.

| Java File | Feature | forge-engine Status |
|-----------|---------|:-------------------:|
| `StaticAbility.java` | Core static ability class with layer system | **Partial** (`static_ability.rs`: `StaticAbility` struct + parser, expanded `StaticMode` enum (critical/high-priority ticket modes wired), `CardFilter` for `Affected$`/`ValidCards$`; still missing Java dependency graph/timestamp parity) |
| `StaticAbilityLayer.java` | Enum: copy, control, text, type, color, abilities, P/T, rules | **Partial** (`static_ability.rs`: `Layer` enum — Control(2), Type(4), Color(5), Ability(6), SetPT(7b), ModifyPT(7c); missing: copy/text/7a/rules layers) |
| `StaticAbilityMode.java` | Enum: 80+ static ability modes | **Partial** (`static_ability.rs` `StaticMode`: includes baseline 7 plus ticket modes: CantTarget, CantAttach, MustAttack, MustBlock, Panharmonicon, CantGainLosePayLife, CantDraw, CantSacrifice, CantRegenerate, DisableTriggers, CantPutCounter, CastWithFlash, BlockRestrict, AttackRestrict, IgnoreHexproof/Shroud, IgnoreLegendRule, MustTarget, AssignCombatDamageAsUnblocked, AssignNoCombatDamage, CombatDamageToughness, NoCleanupDamage, InfectDamage, WitherDamage, ColorlessDamageSource, CountersRemain, MaxCounter; plus CanAttackDefender and OptionalAttackCost) |
| `StaticAbilityContinuous.java` | Core continuous effect handler | **Partial** (`layer.rs` `apply_continuous_effects()`: Control (2, `GainControl$` incl. aura `Card.EnchantedBy`), Ability/keyword-grant (6), SetPT (7b), ModifyPT (7c) layers applied in CR 613 order; `apply_etb_tapped()` for ETBTapped via static abilities, `R:Event$ Moved | ReplaceWith$ ETBTapped` replacement effects, AND `ReplaceWith$ DBTap` shock land pattern with `UnlessCost$ PayLife<N>` player prompt; missing: type/color layers, dependency resolution) |
| `StaticAbilityCantAttack.java` | Prevents attacking | **Implemented** (`layer.rs`: `Mode$ CantAttack` sets `cant_attack_static` flag; `card.rs` `can_attack()` respects it) |
| `StaticAbilityCantBlock.java` | Prevents blocking | **Implemented** (`layer.rs`: `Mode$ CantBlock` sets `cant_block_static` flag; `card.rs` `can_block()` respects it) |
| `StaticAbilityCantAttackUnless.java` | Attack cost (Propaganda, Ghostly Prison) | **Implemented** — `combat/attack_cost.rs`: `get_attack_cost()` scans `CantAttackUnless` statics; `pay_combat_cost` agent loop in phase_handler with full UI (PayCombatCostModal) |
| `StaticAbilityCantBlockUnless.java` | Block cost (War Cadence) | **Implemented** — `combat/block_cost.rs`: `get_block_cost()` scans `CantBlockUnless` statics; auto-pay in phase_handler |
| `StaticAbilityCantBeSacrificed.java` | Prevents sacrifice | Not implemented |
| `StaticAbilityCantCast.java` | Prevents casting | Not implemented |
| `StaticAbilityCantTarget.java` | Grants hexproof/shroud | **Partial** (`static_ability_cant_target.rs` + `target_restrictions.rs` integration) |
| `StaticAbilityCantDraw.java` | Limits/prevents drawing | **Partial** (`static_ability_cant_draw.rs` + `draw_card()` gate) |
| `StaticAbilityCantDiscard.java` | Prevents discarding | Not implemented |
| `StaticAbilityCantDamage.java` | Prevents damage | Not implemented |
| `StaticAbilityCantExile.java` | Prevents exiling | Not implemented |
| `StaticAbilityCantRegenerate.java` | Prevents regeneration | **Partial** (`static_ability_cant_regenerate.rs` + `regenerate_effect.rs` gate) |
| `StaticAbilityCantTransform.java` | Prevents transformation | Not implemented |
| `StaticAbilityCantPhase.java` | Prevents phasing | Not implemented |
| `StaticAbilityCantPutCounter.java` | Prevents counter placement | **Partial** (`static_ability_cant_put_counter.rs` integrated in PutCounter/PutCounterAll/MoveCounter/Proliferate/Explore and Infect/Wither damage paths) |
| `StaticAbilityCantPreventDamage.java` | Prevents damage prevention | Not implemented |
| `StaticAbilityCantSacrifice.java` | Prevents sacrifice | **Partial** (`static_ability_cant_sacrifice.rs` integrated in sacrifice effects + sacrifice cost payment) |
| `StaticAbilityCantVenture.java` | Prevents venturing | Not implemented |
| `StaticAbilityCantGainLosePayLife.java` | Prevents life gain/loss/payment | **Partial** (`static_ability_cant_gain_lose_pay_life.rs` integrated in life effects, damage-to-player, and PayLife costs) |
| `StaticAbilityCastWithFlash.java` | Grants flash to spells | **Partial** (`static_ability_cast_with_flash.rs` + playable-card instant-speed checks) |
| `StaticAbilityMustAttack.java` | Forces creatures to attack | **Partial** (`static_ability_must_attack.rs` + combat declaration auto-include) |
| `StaticAbilityMustBlock.java` | Forces creatures to block | **Implemented** — `static_ability_must_block.rs` + `combat::compute_must_block_targets()` auto-assigns in phase_handler; Lure/AllMustBlock keyword detection |
| `StaticAbilityMustTarget.java` | Forces targeting restrictions | **Partial** (`static_ability_must_target.rs` scaffold; full target-choice enforcement pending) |
| `StaticAbilityAdapt.java` | Adapt mechanic interactions | Not implemented |
| `StaticAbilityPanharmonicon.java` | Double trigger effects | **Partial** (`static_ability_panharmonicon.rs` + trigger duplication hook in `trigger/handler.rs`) |
| `StaticAbilityManaConvert.java` | Mana color conversion | Not implemented |
| `StaticAbilityIgnoreHexproofShroud.java` | Ignore hexproof/shroud | **Partial** (`static_ability_ignore_hexproof_shroud.rs` + `target_restrictions.rs` integration) |
| `StaticAbilityIgnoreLandwalk.java` | Ignore landwalk | Not implemented |
| `StaticAbilityIgnoreLegendRule.java` | Ignore legend rule | **Partial** (`static_ability_ignore_legend_rule.rs` + legend-rule SBA pass in `action.rs`) |
| `StaticAbilityInfectDamage.java` | Makes damage infect | **Partial** (`static_ability_infect_damage.rs` integrated in combat/spell damage paths) |
| `StaticAbilityWitherDamage.java` | Makes damage wither | **Partial** (`static_ability_wither_damage.rs` integrated in combat/spell damage paths) |
| `StaticAbilityColorlessDamageSource.java` | Makes damage colorless | **Partial** (`static_ability_colorless_damage_source.rs` scaffold; color-identity downstream usage pending) |
| `StaticAbilityCombatDamageToughness.java` | Use toughness for combat damage | **Partial** (`static_ability_combat_damage_toughness.rs` integrated in combat damage power calculation) |
| `StaticAbilityNoCleanupDamage.java` | Prevents damage removal at cleanup | **Partial** (`static_ability_no_cleanup_damage.rs` + cleanup damage reset guard) |
| `StaticAbilityCountersRemain.java` | Prevents counter removal | **Partial** (`static_ability_counters_remain.rs` + zone-change counter clearing exceptions) |
| `StaticAbilityMaxCounter.java` | Sets maximum counters | **Partial** (`static_ability_max_counter.rs` integrated in counter-add paths) |
| `StaticAbilityDevotion.java` | Modifies devotion calculation | Not implemented |
| `StaticAbilityDisableTriggers.java` | Disables triggered abilities | **Partial** (`static_ability_disable_triggers.rs` + pre-dispatch trigger gate in `trigger/handler.rs`) |
| `StaticAbilityFlipCoinMod.java` | Fixes coin flip results | Not implemented |
| `StaticAbilityNumLoyaltyAct.java` | Modifies loyalty activation count | Not implemented |
| `StaticAbilityTurnPhaseReversed.java` | Reverses turn/phase order | Not implemented |
| `StaticAbilityUnspentMana.java` | Mana carry-over rules | **Implemented** (`UnspentMana` static mode; scanned at phase transitions via `compute_unspent_mana_colors()`; `clear_pool_with_keep()` retains matching mana colors — Omnath, Leyline Tyrant, Upwelling, etc.) |
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
| `TriggerType.java` | Enum of all trigger types | **Implemented** (68 types in `event.rs`: 34 original + 34 companion/batch modes from issue #54) |
| `WrappedAbility.java` | Ability wrapper for trigger execution | Not implemented |
| `TriggerWaiting.java` | Waiting trigger queue | **Implemented** (waiting_triggers in handler) |
| **Zone Change Triggers** | | |
| `TriggerChangesZone.java` | Card changes zones (ETB, dies, etc.) | **Implemented** |
| `TriggerChangesZoneAll.java` | All zone changes | **Implemented** (TriggerType + TriggerMode + perform_test + parse_trigger; fires via ChangesZone base event remap) |
| `TriggerChangesController.java` | Control changes | **Implemented** (TriggerType + TriggerMode + perform_test + parse_trigger; fires in control_gain_effect.rs) |
| **Phase/Turn Triggers** | | |
| `TriggerPhase.java` | Phase begin/end | **Implemented** |
| `TriggerTurnBegin.java` | Turn begins | **Implemented** (TriggerType + TriggerMode + fires in phase_handler.rs) |
| `TriggerNewGame.java` | Game starts | Not implemented |
| **Combat Triggers** | | |
| `TriggerAttacks.java` | Creature attacks | **Implemented** (fires in game_loop.rs) |
| `TriggerAttackersDeclared.java` | All attackers declared | **Implemented** (TriggerType + TriggerMode + fires in phase_handler.rs) |
| `TriggerAttackerBlocked.java` | Attacker is blocked | **Implemented** (fires in game_loop.rs) |
| `TriggerAttackerBlockedByCreature.java` | Specific blocker | **Implemented** (TriggerType + TriggerMode + perform_test + parse_trigger; fires via AttackerBlocked base event remap) |
| `TriggerAttackerBlockedOnce.java` | Blocked once per combat | **Implemented** (TriggerType + TriggerMode + perform_test + parse_trigger; fires via AttackerBlocked base event remap) |
| `TriggerAttackerUnblocked.java` | Attacker unblocked | **Implemented** (fires in game_loop.rs) |
| `TriggerAttackerUnblockedOnce.java` | Unblocked once | **Implemented** (TriggerType + TriggerMode + perform_test + parse_trigger; fires via AttackerUnblocked base event remap) |
| `TriggerBlockersDeclared.java` | All blockers declared | **Implemented** (TriggerType + TriggerMode + fires in phase_handler.rs) |
| `TriggerBlocks.java` | Creature blocks | **Implemented** (fires in game_loop.rs) |
| **Spell Triggers** | | |
| `TriggerSpellCast.java` | Spell is cast | **Implemented** |
| `TriggerSpellCastAll.java` | All spell casts | **Implemented** (TriggerType + TriggerMode + perform_test + parse_trigger; fires via SpellCast base event remap) |
| `TriggerSpellCastOnce.java` | First spell cast per turn | **Implemented** (TriggerType + TriggerMode + perform_test + parse_trigger; fires via SpellCast base event remap) |
| `TriggerSpellCastOfType.java` | Specific spell type cast | **Implemented** (TriggerType + TriggerMode + perform_test + parse_trigger; fires via SpellCast base event remap) |
| `TriggerAbilityResolves.java` | Ability resolves | Not implemented |
| `TriggerAbilityTriggered.java` | Ability triggered | Not implemented |
| `TriggerCountered.java` | Spell countered | **Implemented** (fires in counter_effect) |
| **Damage Triggers** | | |
| `TriggerDamageDone.java` | Damage dealt | **Implemented** (fires in combat, damage_deal, damage_all) |
| `TriggerDamageDoneOnce.java` | Damage dealt once | **Implemented** (TriggerType + TriggerMode + perform_test + parse_trigger; fires via DamageDone base event remap) |
| `TriggerDamageDoneOnceByController.java` | Once per controller | Not implemented |
| `TriggerDamageAll.java` | All damage events | **Implemented** (TriggerType + TriggerMode + perform_test + parse_trigger; fires via DamageDone base event remap) |
| `TriggerDamageDealtOnce.java` | Once per damage event | **Partial** (TriggerType + TriggerMode defined, per-turn tracking not yet implemented) |
| `TriggerDamagePreventedOnce.java` | Damage prevented | **Implemented** (TriggerType + TriggerMode + perform_test + parse_trigger; fires via DamageDone base event remap) |
| `TriggerExcessDamage.java` | Excess damage (trample) | **Implemented** (TriggerType + TriggerMode + perform_test + parse_trigger; fires via DamageDone base event remap) |
| `TriggerExcessDamageAll.java` | All excess damage | Not implemented |
| **Life Triggers** | | |
| `TriggerLifeGained.java` | Life gained | **Implemented** (fires in life_gain_effect, combat lifelink) |
| `TriggerLifeLost.java` | Life lost | **Implemented** (fires in life_lose_effect, pay costs) |
| `TriggerLifeLostAll.java` | All life loss | **Implemented** (TriggerType + TriggerMode + perform_test + parse_trigger; fires via LifeLost base event remap) |
| `TriggerPayLife.java` | Life paid as cost | Not implemented |
| `TriggerLosesGame.java` | Player loses | Not implemented |
| **Counter Triggers** | | |
| `TriggerCounterAdded.java` | Counter added | **Implemented** (fires in counters_put_effect) |
| `TriggerCounterAddedAll.java` | All counter additions | Not implemented |
| `TriggerCounterAddedOnce.java` | Counter added once | **Implemented** (TriggerType + TriggerMode + perform_test + parse_trigger; fires via CounterAdded base event remap) |
| `TriggerCounterRemoved.java` | Counter removed | **Implemented** (TriggerType + TriggerMode + perform_test, ready to fire from counter removal effects) |
| `TriggerCounterRemovedOnce.java` | Counter removed once | **Implemented** (TriggerType + TriggerMode + perform_test + parse_trigger; fires via CounterRemoved base event remap) |
| `TriggerCounterPlayerAddedAll.java` | Player counters | Not implemented |
| `TriggerCounterTypeAddedAll.java` | Specific counter type | Not implemented |
| **Card Action Triggers** | | |
| `TriggerDrawn.java` | Card drawn | **Implemented** (fires in draw_effect, step_draw) |
| `TriggerDiscarded.java` | Card discarded | **Implemented** (fires in discard_effect) |
| `TriggerDiscardedAll.java` | All discards | **Implemented** (TriggerType + TriggerMode + perform_test + parse_trigger; fires via Discarded base event remap) |
| `TriggerMilled.java` | Card milled | **Implemented** (fires in mill_effect) |
| `TriggerMilledAll.java` | All mills | Not implemented |
| `TriggerMilledOnce.java` | Milled once | Not implemented |
| `TriggerExiled.java` | Card exiled | **Implemented** (fires in change_zone_effect when destination is Exile) |
| `TriggerSacrificed.java` | Card sacrificed | **Implemented** (fires in sacrifice_effect, sacrifice_all_effect, game_loop) |
| `TriggerSacrificedOnce.java` | Sacrificed once | **Implemented** (TriggerType + TriggerMode + perform_test + parse_trigger; fires via Sacrificed base event remap) |
| `TriggerDestroyed.java` | Card destroyed | **Implemented** (fires in destroy_effect) |
| `TriggerCycled.java` | Card cycled | **Implemented** (TriggerType + TriggerMode + perform_test + parse_trigger; fires in magic_stack.rs after cycling ability resolves) |
| `TriggerLandPlayed.java` | Land played | **Implemented** (fires in game_loop play_card) |
| `TriggerTaps.java` | Permanent tapped | **Implemented** (fires in tap_all_effect, game_loop) |
| `TriggerTapAll.java` | All taps | **Implemented** (TriggerType + TriggerMode + perform_test + parse_trigger; fires via Taps base event remap) |
| `TriggerUntaps.java` | Permanent untapped | **Implemented** (fires in untap_all_effect, game_loop) |
| `TriggerUntapAll.java` | All untaps | **Implemented** (TriggerType + TriggerMode + perform_test + parse_trigger; fires via Untaps base event remap) |
| `TriggerTapsForMana.java` | Tapped for mana | **Implemented** (fires in game_loop ActivateMana + resolve_mana_ability) |
| **Keyword Mechanic Triggers** | | |
| `TriggerBecomesTarget.java` | Becomes target | **Implemented** (fires in game_loop play_card + activate_ability_on_stack) |
| `TriggerBecomesTargetOnce.java` | Targeted once | **Implemented** (TriggerType + TriggerMode + perform_test + parse_trigger; fires via BecomesTarget base event remap) |
| `TriggerEvolved.java` | Creature evolves | Not implemented |
| `TriggerExplores.java` | Creature explores | **Implemented** (TriggerType + TriggerMode defined, `explore_effect.rs` mechanic implemented) |
| `TriggerMutates.java` | Creature mutates | Not implemented |
| `TriggerAdapt.java` | Creature adapts | Not implemented |
| `TriggerBecomeMonstrous.java` | Becomes monstrous | **Implemented** (`trigger.rs`/`handler.rs` + `counters_put_effect.rs`: parses `BecomeMonstrous`, fires when `Monstrosity$ True` resolves for a non-monstrous permanent, passes `MonstrosityAmount`) |
| `TriggerBecomeRenowned.java` | Becomes renowned | Not implemented |
| `TriggerBecomeMonarch.java` | Becomes monarch | **Implemented** — TriggerType::BecomeMonarch fired by `become_monarch_effect.rs`; `game.monarch` tracks current monarch; monarch draws card at end of turn |
| `TriggerBecomesCrewed.java` | Vehicle crewed | Not implemented |
| `TriggerBecomesSaddled.java` | Mount saddled | Not implemented |
| `TriggerBecomesPlotted.java` | Card plotted | Not implemented |
| `TriggerFlippedCoin.java` | Coin flipped | Not implemented |
| `TriggerFight.java` | Creatures fight | **Implemented** (fires in fight_effect) |
| `TriggerFightOnce.java` | Fight once | Not implemented |
| `TriggerExerted.java` | Creature exerted | Not implemented |
| `TriggerExploited.java` | Creature exploited | Not implemented |
| `TriggerInvestigated.java` | Investigated | Not implemented |
| `TriggerForetell.java` | Card foretold | **Implemented** (TriggerType + TriggerMode + perform_test + parse_trigger; fires in game_action_util.rs after foretell exile) |
| `TriggerForage.java` | Foraged | Not implemented |
| `TriggerSurveil.java` | Surveiled | **Implemented** (TriggerType + TriggerMode + perform_test + parse_trigger; fires in surveil_effect.rs) |
| `TriggerScry.java` | Scryed | **Implemented** (TriggerType + TriggerMode + perform_test + parse_trigger; fires in scry_effect.rs) |
| `TriggerProliferate.java` | Proliferated | Not implemented |
| `TriggerCollectEvidence.java` | Evidence collected | Not implemented |
| `TriggerCommitCrime.java` | Crime committed | Not implemented |
| `TriggerDiscover.java` | Discovered | Not implemented |
| `TriggerConnive.java` *(if exists)* | Connived | Not implemented |
| **Misc Triggers** | | |
| `TriggerAlways.java` | Always fires | **Implemented** (TriggerType + TriggerMode + fires alongside Phase in phase_handler.rs) |
| `TriggerImmediate.java` | Immediate trigger | **Implemented** (TriggerType + TriggerMode + perform_test + parse_trigger) |
| `TriggerAttached.java` | Aura/equipment attached | **Implemented** (fires in attach_effect) |
| `TriggerUnattach.java` | Detached | **Implemented** (TriggerType + TriggerMode + perform_test, ready to fire from detach effects) |
| `TriggerPhaseIn.java` | Phased in | **Implemented** (TriggerType + TriggerMode + perform_test + parse_trigger) |
| `TriggerPhaseOut.java` | Phased out | **Implemented** (TriggerType + TriggerMode + perform_test + parse_trigger) |
| `TriggerPhaseOutAll.java` | All phased out | Not implemented |
| `TriggerTransformed.java` | Card transformed | **Implemented** (fires in set_state_effect) |
| `TriggerTurnFaceUp.java` | Turned face up | Not implemented |
| `TriggerSearchedLibrary.java` | Library searched | **Implemented** (TriggerType + TriggerMode + perform_test + parse_trigger; fires in change_zone_effect.rs on library search) |
| `TriggerShuffled.java` | Library shuffled | **Implemented** (TriggerType + TriggerMode + perform_test + parse_trigger; fires in shuffle_effect.rs, rearrange_top_of_library_effect.rs, change_zone_effect.rs) |
| `TriggerManaAdded.java` | Mana added | **Implemented** (TriggerType + TriggerMode + perform_test + parse_trigger; fires in game_action.rs resolve_mana_ability) |
| `TriggerManaExpend.java` | Mana expended | **Implemented** — TriggerType::ManaExpend + TriggerMode::ManaExpend with Amount/Player params; cumulative per-turn tracking via `mana_expended_this_turn`; fires in game_action_util.rs after spell payment |
| `TriggerTokenCreated.java` | Token created | **Implemented** (fires in token_effect) |
| `TriggerTokenCreatedOnce.java` | Token created once | **Implemented** (TriggerType + TriggerMode + perform_test + parse_trigger; fires via TokenCreated base event remap) |
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
| `event.rs` | **Complete** | TriggerType enum (68 types: 34 original + 34 new from issue #54), RunParams (~33 fields) |
| `trigger.rs` | **Complete** | Trigger matching, ValidCard/ValidPlayer filters, parsing |
| `trigger_handler.rs` | **Complete** | Active/waiting/delayed triggers, dispatch, OptionalDecider$ support, APNAP ordering |
| `agent.rs` | **Complete** | PlayerAgent trait (Java-parity decision hooks including `confirm_action`, `confirm_payment`, `choose_binary`, optional-cost callbacks including choose_buyback/multikicker/replicate/alternative_cost, plus combat optional-cost callbacks `exert_attackers`/`enlist_attackers`), MainPhaseAction, TargetChoice, BinaryChoiceKind |
| `game_loop.rs` | **Partial** | Game flow orchestration with APNAP priority handoff, `priority_player` tracking, draw/combat/end priority windows, and illegal-action guardrails; still missing full Java parity for extra turns/phases and advanced phase replacement hooks |
| `spellability/mod.rs` | **Complete** | SpellAbility module structure |
| `spellability/targeting.rs` | **Complete** | Targeting system: parse_valid_targets, choose_targets, CardInZone support for graveyard/exile targeting, dynamic `TargetMin/TargetMax` expression resolution (e.g. `TargetMin$ X`) |

### Crate: `forge-parity` (cross-engine differential testing)
| File | Status | Features |
|------|--------|----------|
| `main.rs` | **Implemented** | CLI entry point: Rust-only, full parity (`--java-jar`), matrix (`--matrix`), fuzz (`--fuzz`), and multi-game single-match runs via `--games` (seeds increment from `--seed`); quiet-by-default batch output with per-step logging behind `--verbose`; failed matchups include first divergence plus full Rust trace; text reports append a run-level **Coverage Report** (covered vs uncovered deck cards), a per-game completion column (`FINISHED TURN X` / `STOPPED AT MAX`), and low-effort ability/effect/trigger signal coverage from notify messages |
| `runner.rs` | **Implemented** | Rust game runner with deterministic agents, snapshot collection, `resolve_deck_spec()` supporting both preset names and `inline:` deck specs; captures Java-parity decision kinds including `confirm_payment` and `choose_binary`, and forwards optional combat-cost callbacks (`exert_attackers`/`enlist_attackers`) |
| `deterministic_agent.rs` | **Implemented** | Hybrid RNG agent: JavaRandom for core decisions (play/attack/block/target/type), plus deterministic overrides for `confirm_action`/`confirm_payment`/`choose_binary` parity callbacks and optional combat-cost callbacks (`exert_attackers`/`enlist_attackers`); main-action ordering matches harness `ParityOrder.actionComparator()` including concatenated sort-key behavior for alternative-cost variants like Flashback |
| `snapshot.rs` | **Implemented** | Extract normalized StateSnapshot from GameState (sorted, name-based) |
| `protocol.rs` | **Implemented** | Shared JSON types: StateSnapshot, DecisionPoint, Decision, GameTrace, ParityReport, MatrixReport, FuzzReport/Result |
| `comparator.rs` | **Implemented** | Snapshot diff engine: field-by-field comparison, Divergence reporting |
| `report.rs` | **Implemented** | Report generation: JSON and human-readable text formats for parity, matrix, and fuzz modes |
| `java_bridge.rs` | **Implemented** | Subprocess bridge: launches Java harness JAR, reads JSONL snapshots |
| `java_random.rs` | **Implemented** | Faithful port of `java.util.Random` LCG and `Collections.shuffle()` for cross-engine determinism |
| `card_pool.rs` | **Implemented** | Dynamic card pool discovery: scans CardDatabase, includes only cards whose triggers/statics/replacements fully parse (~80.9% of 32k cards) |
| `deck_generator.rs` | **Implemented** | Random deck generation from discovered pool using JavaRandom; inline format (`Name*Count\|...`) for serialization |

### Java Module: `forge-harness` (Java side of parity testing)
| File | Status | Features |
|------|--------|----------|
| `Main.java` | **Implemented** | Headless CLI: loads decks, runs game, emits JSONL snapshots |
| `DeterministicController.java` | **Implemented** | Deterministic PlayerController matching Rust DeterministicAgent logic; `canCastSorcery()` gate enforces main-phase-only spell casting; cost/cast entry overrides funnel through shared deterministic payment plumbing (`payWithDeterministicDecision`) and harness `AutoPay` (no `PlayerControllerAi` dependency on mana payment path); explicit parity overrides for `confirmPayment(...)` and both `chooseBinary(...)` overloads |
| `AutoPay.java` | **Implemented** | Harness-owned legality-first deterministic mana payment (`payManaCost`) that avoids AI decision classes, pays floating mana first, selects legal mana abilities deterministically, pays required activation costs through deterministic cost plumbing, and refunds/aborts on failed payment |
| `DeterministicLobbyPlayer.java` | **Implemented** | LobbyPlayer factory that creates DeterministicController instances |
| `SnapshotExtractor.java` | **Implemented** | Extracts JSON snapshots from Java Game state matching Rust format |
| `PresetDecks.java` | **Implemented** | Builds preset decks and inline deck specs (`inline:Name*Count\|...`) for fuzz mode |

### Crate: `forge-cli`
| File | Status | Features |
|------|--------|----------|
| `main.rs` | **Basic** | ANSI-colored card display, CLI test harness |

---

## Summary Statistics

| Category | Java Files | Fully Implemented | Partially Implemented | Not Implemented |
|----------|:----------:|:-----------------:|:---------------------:|:---------------:|
| Core Game | 37 | 4 | 8 | 25 |
| Ability System | 10 | 0 | 3 | 7 |
| Ability Effects | 204 | 80 | 10 | 114 |
| Card System | 28 | 5 | 5 | 18 |
| Perpetual Effects | 9 | 0 | 0 | 9 |
| Tokens | 1 | 1 | 0 | 0 |
| Combat | 10 | 6 | 2 | 2 |
| Costs | 52 | 26 | 3 | 23 |
| Events | 58 | 0 | 5 | 53 |
| Extra Hands | 1 | 0 | 0 | 1 |
| Keywords | 20 | 4 | 8 | 8 |
| Mana | 6 | 3 | 1 | 2 |
| Mulligan | 7 | 3 | 0 | 4 |
| Phases | 7 | 1 | 3 | 3 |
| Player | 17 | 2 | 1 | 14 |
| Player Actions | 10 | 1 | 3 | 6 |
| Replacement Effects | 46 | 11 | 5 | 30 |
| Spell Abilities | 22 | 5 | 3 | 14 |
| Static Abilities | 54 | 6 | 24 | 24 |
| Triggers | 131 | 72 | 1 | 58 |
| Zones | 8 | 3 | 1 | 4 |
| **TOTAL** | **738** | **233** | **86** | **419** |

> **Coverage: ~43.2% implemented or partially implemented** (319 of 738 features have a Rust counterpart).
>
> The Rust engine has **~130 implementation files** (160+ total incl. tests/tools) across 6 crates. **80 effect handlers**, **72 trigger types**, **51 keyword abilities**, **30 static ability modes**, **14 replacement event types**, and **26 cost types** are functional.
>
> **Mana system: 100% complete** — individual mana objects, interactive payment, persistent/combat mana, conversion matrix, snow, restrictions, keywords/counters on mana, uncounterable mana, production doublers/replacers, sunburst/converge, mana burn, refund, waterbend.
>
> **Remaining gaps by priority**:
> - **Effects**: ~114 not implemented (0 critical, 0 high, ~62 medium, ~48 low)
> - **Triggers**: ~58 not implemented (0 critical, 1 high, ~40 medium)
> - **Static Abilities**: ~24 not implemented (edge-case parity on 24 partial modes)
> - **Costs**: ~23 not implemented (medium/low priority)
> - **Replacement Effects**: ~30 not implemented (2 critical, ~10 high)
> - **Combat**: Banding, ninjutsu block replacement
> - **Infrastructure**: Game logging, snapshots, format configuration

---

## 23. Priority Analysis — What's Missing

> This section organizes all unimplemented features by priority tier (Critical → High → Medium → Low) for actionable GitHub issue creation. The per-file status tables in Sections 1–21 above remain the authoritative reference.

### 23.1 Executive Summary

| Metric | Count |
|--------|------:|
| Total Java files (forge-game) | 738 |
| Total Rust files (forge-engine) | ~160 |
| Effect handlers implemented | 80 of 204 (39%) |
| Trigger types implemented | 71 of ~131 (54%) |
| Keywords implemented | 51 of ~200+ (26%) |
| Static ability modes | 30 of 54 (56%) |
| Replacement event types | 14 of 46 (30%) |
| Cost types | 26 of 52 (50%) |
| Mana system | **100%** complete |
| **Estimated overall coverage** | **~43%** |

### 23.2 Subsystem Coverage Overview

| # | Subsystem | Java | Coverage | Key Gaps |
|---|-----------|:----:|:--------:|----------|
| 1 | Core Game | 37 | ~32% | Logging, snapshots, rules config, formats |
| 2 | Ability System | 10 | ~30% | Factory partial, API type dispatch partial |
| 3 | **Ability Effects** | 204 | **~44%** | ~114 effects not implemented (medium/low priority) |
| 4 | Card System | 28 | ~36% | Factory, views, clone states |
| 5 | Perpetual | 9 | 0% | Arena-specific (low priority) |
| 6 | Tokens | 1 | 100% | Complete |
| 7 | **Combat** | 10 | ~80% | Banding, ninjutsu |
| 8 | **Costs** | 52 | **~56%** | ~23 niche cost types missing |
| 9 | Events | 58 | ~9% | Event types (UI logging, not gameplay-blocking) |
| 10 | Extra Hands | 1 | 0% | Niche (Conspiracy format) |
| 11 | **Keywords** | 20 | ~60% | ~8 infra classes; 51 keywords have runtime logic |
| 12 | **Mana** | 6 | **~83%** | Complete (pool, payment, restrictions, production, all mechanics) |
| 13 | Mulligan | 7 | ~43% | Paris, Vancouver, Original, Houston variants |
| 14 | Phases | 7 | ~57% | Extra turn/phase objects |
| 15 | Player | 17 | ~18% | Properties, predicates, statistics |
| 16 | Player Actions | 10 | ~40% | Handled in Tauri layer (prompt.rs) |
| 17 | **Replacement** | 46 | ~35% | ~30 types missing (2 critical) |
| 18 | Spell Abilities | 22 | ~36% | Conditions, restrictions, views |
| 19 | **Static Abilities** | 54 | ~56% | ~24 modes not implemented; 24 partial |
| 20 | **Triggers** | 131 | **~55%** | ~59 types missing (0 critical, 1 high) |
| 21 | Zones | 8 | ~50% | CostPaymentStack |

### 23.3 Effects — Missing by Priority

**Currently Implemented (67):** DealDamage, GainLife, LoseLife, PutCounter, RemoveCounter, Poison, Pump, Destroy, Draw, ChangeZoneAll, ChangeZone, SacrificeAll, Sacrifice, CopyPermanent, Token, Mana, Mill, Scry, Surveil, Dig, DigMultiple, RearrangeTopOfLibrary, Reveal, RevealHand, LookAt, Charm, PeekAndReveal, SetState, Cleanup, Counter, ControlGain, Fight, Discard, Attach, DestroyAll, DamageAll, PumpAll, TapAll, UntapAll, Tap, Untap, LifeSet, LifeExchange, GameWin, GameLoss, GameDraw, AddTurn, Fog, ReverseTurnOrder, EndCombatPhase, EndTurn, PowerExchange, BecomeMonarch, TakeInitiative, SkipTurn, SkipPhase, AddPhase, Phases, Regenerate, Play, **Animate**, **Balance**, **ChooseCard**, **ChooseColor**, **Clone**, **ControlGainVariant**, **RepeatEach**

#### Critical & High Priority — ✅ ALL IMPLEMENTED

All 7 critical effects (Animate, ControlGainVariant, Balance, ChooseCard, ChooseColor, Clone, RepeatEach) and all 25 high-priority effects (issue #53) are fully implemented.

#### Medium Priority (61 effects)

<details>
<summary>Click to expand full list</summary>

Amass, AssignGroup, BidLife, Block, Bond, Branch, Camouflage, ChangeCombatants, ChangeSpeed, ChangeTargets, ChangeText, ChangeX, ChangeZoneResolve, Clash, ClassLevelUp, Cloak, Connive, ControlExchange, ControlExchangeVariant, ControlPlayer, ControlSpell, CountersMultiply, CountersNote, CountersRemoveAll, DamageBase, DamageResolve, DayTime, Debuff, DetachedCard, Discover, EffectEffect, Endure, Haunt, ImmediateTrigger, Incubate, Intensify, Investigate, Learn, LifeExchangeVariant, ManifestBase, Manifest, ManaReflected, Meld, MultiplePiles, Mutate, PermanentCreature, Permanent, PermanentNoncreature, PlayLandVariant, RegenerationEffect, RemoveFromGame, RemoveFromMatch, ReorderZone, RepeatEffect, ReplaceCounter, ReplaceDamage, ReplaceMana, ReplaceSplitDamage, ReplaceToken, StoreSVar

</details>

#### Low Priority (43 effects — niche/format-specific)

<details>
<summary>Click to expand full list</summary>

Abandon, AdvanceCrank, Airbend, AlterAttribute, Ascend, AssembleContraption, BecomesBlocked, BlankLine, Blight, ChaosEnsues, ChooseGeneric, ChooseSector, ClaimThePrize, DraftEffect, Earthbend, FlipOntoBattlefield, Heist, InternalRadiation, LosePerpetual, MakeCard, ManifestDread, OpenAttraction, OwnershipGain, Planeswalk, Radiation, ReplaceEffect, RestartGame, RingTemptsYou, RollPlanarDice, RunChaos, Seek, SetInMotion, SubgameEffect, SwitchBlock, TextBoxExchange, Unattach, UnlockDoor, Venture, VillainousChoice, Vote, ZoneExchange

</details>

### 23.4 Triggers — Missing by Priority

**Currently Implemented (66):** ChangesZone, ChangesZoneAll, ChangesController, Phase, TurnBegin, SpellCast, SpellCastAll, SpellCastOnce, SpellCastOfType, Attacks, AttackersDeclared, Blocks, BlockersDeclared, AttackerBlocked, AttackerBlockedByCreature, AttackerBlockedOnce, AttackerUnblocked, AttackerUnblockedOnce, DamageDone, DamageDoneOnce, DamageAll, DamagePreventedOnce, ExcessDamage, Countered, LifeGained, LifeLost, LifeLostAll, CounterAdded, CounterAddedOnce, CounterRemoved, CounterRemovedOnce, Drawn, Discarded, DiscardedAll, Milled, Exiled, Sacrificed, SacrificedOnce, Destroyed, Cycled, LandPlayed, Taps, TapAll, Untaps, UntapAll, TapsForMana, BecomesTarget, BecomesTargetOnce, BecomeMonarch, Fight, Attached, Unattached, Transformed, TokenCreated, TokenCreatedOnce, SpellCopied, Explored, TakeInitiative, Surveil, Scry, Foretell, SearchedLibrary, Shuffled, ManaAdded, PhaseIn, PhaseOut, Always, Immediate

#### Critical (0 triggers — all previously critical triggers are now implemented)

_None remaining._

#### High Priority (1 trigger)

LifeGainedAll

#### Medium Priority (40+ triggers)

Evolved, Mutates, Adapt, BecomeMonstrous, BecomeRenowned, BecomesCrewed, BecomesSaddled, FlippedCoin, FightOnce, Exerted, Exploited, Investigated, ForetellDone, Forage, Proliferate, CollectEvidence, CommitCrime, Discover, DayTimeChanges, ClassLevelGained, CompletedDungeon, EnteredRoom, Vote, Championed, Clashed, Devoured, Enlisted, Mentored, Trains, CaseSolved, ClaimPrize, GiveGift, RingTemptsYou, Specializes, UnlockDoor, FullyUnlock, ManifestDread, Elementalbend, CrankContraption, PlanarDice, Abandoned, AbilityResolves, AbilityTriggered

#### Low Priority (20+ triggers)

NewGame, LosesGame, PayLife, ExcessDamageAll, DamageDoneOnceByController, CounterPlayerAddedAll, CounterTypeAddedAll, MilledAll, MilledOnce, BecomesPlotted, CrewedSaddled, BecomesSuspected, Waiting, WrappedAbility

### 23.5 Keywords — Missing

**Implemented (51):** Flying, Reach, First Strike, Double Strike, Trample, Deathtouch, Lifelink, Vigilance, Defender, Haste, Flash, Hexproof, Shroud, Hexproof from X, Menace, Fear, Intimidate, Shadow, Skulk, Horsemanship, Indestructible, Infect, Wither, Toxic, Protection, Flashback, Kicker, Storm, Cascade, Buyback, Spectacle, Evoke, Dash, Blitz, Multikicker, Replicate, Entwine, Escalate, Escape, Overload, Madness, Rebound, Suspend, Foretell, Emerge, Prowess, Cycling, Offering, Spree, Strive, ETBReplacement

#### High Priority Missing

| Keyword | Description |
|---------|-------------|
| Equip | Equipment attachment cost |
| Morph / Megamorph | Face-down casting |
| Annihilator | Force sacrifice on attack |
| Undying | Return with +1/+1 counter |
| Persist | Return with -1/-1 counter |
| Bestow | Cast as aura or creature |
| Embalm / Eternalize | Create token copy from graveyard |
| Fabricate | +1/+1 counters or tokens |
| Adapt | +1/+1 counters if none |
| Crew | Tap creatures to crew vehicle |
| Ward | Counter unless pay (partially parsed) |
| Afterlife | Create spirit tokens on death |
| Exploit | Sacrifice creature for ETB |

#### Medium Priority Missing

Ninjutsu, Champion, Devour, Hideaway, Companion, Mutate, Boast, Forage, Landwalk, Banding, Rampage, Flanking, Phasing, Cumulative Upkeep, Echo, Fading, Vanishing, Modular, Dredge, Haunt, Bloodthirst, Graft, Forecast, Retrace, Exalted, Unearth, Living Weapon, Soulbond, Unleash, Extort, Tribute, Outlast, Renown, Myriad, Ascend, Riot

### 23.6 Static Abilities — Missing

**31 modes have Rust implementations** (6 fully implemented, 25 partial with core hooks wired). All 11 critical modes and 15 high-priority modes have at least partial implementations. Remaining work is edge-case parity with Java.

#### Medium Priority Missing (23 modes — no Rust code yet)

Devotion, Exhaust, FlipCoinMod, GainLifeRadiation, IgnoreLandwalk, NumLoyaltyAct, SurveilNum, TapPowerValue, TurnPhaseReversed, UntapOtherPlayer, Adapt, ActivateAbilityAsIfHaste, CantBeCopied, CantBecomeMonarch, CantBeSuspected, CantChangeDayTime, CantCrew, CantDiscard, CantPhase, CantPreventDamage, CantTransform, CantVenture, PlotZone

### 23.7 Replacement Effects — Missing

**Implemented (14 event types):** DamageDone, Draw, DrawCards, Destroy, Moved, GainLife, AddCounter, GameLoss, GameWin, CreateToken, Counter + fire points wired for all

#### Critical Missing (2 types)

| Type | Java File | Description |
|------|-----------|-------------|
| ReplaceDamage (full) | `ReplaceDamage.java` | Full damage replacement (redirect, modify, prevent with shield) |
| ReplaceTap | `ReplaceTap.java` | Replace tap events |

#### High Priority Missing (12 types)

ReplaceDealtDamage, ReplaceLifeReduced, ReplacePayLife, ReplaceMill, ReplaceRemoveCounter, ReplaceCopySpell, ReplaceCascade, ReplaceDeclareBlocker, ReplaceScry, ReplaceTransform, ReplaceTurnFaceUp

#### Low Priority Missing (18 types)

ReplaceAttached, ReplaceBeginPhase, ReplaceBeginTurn, ReplaceExplore, ReplaceLearn, ReplaceLoseMana, ReplacePlanarDiceResult, ReplacePlaneswalk, ReplaceProliferate, ReplaceRollDice, ReplaceRollPlanarDice, ReplaceSetInMotion, ReplaceUntap, ReplaceAssembleContraption, ReplaceAssignDealDamage, ReplaceBehold, ReplaceBeholdExile, ReplaceDrawCards (full)

### 23.8 Cost System

**Implemented:** Mana, Tap, Untap (Q), PayLife, Sacrifice, Discard, Exile (battlefield/hand/graveyard/library), AddCounter (PutCounter), SubCounter (RemoveCounter), Return, TapType (tapXType), UntapType (untapYType), PayEnergy, DamageYou, Draw, Mill, Reveal, Exert, GainLife, GainControl, RemoveAnyCounter, Unattach, ExiledMoveToGrave, AddMana, Waterbend — plus Java-parity additions for Behold/BeholdExile, Blight, ChooseColor, ChooseCreatureType, CollectEvidence, Enlist, ExileFromStack, FlipCoin, Forage, PayShards, PromiseGift, PutCardToLib, RevealChosen, RollDice in `cost/mod.rs` and `game_action.rs`.

### 23.9 Combat System — Gaps

**Implemented:** Basic attack/block declaration, full damage resolution (first/double strike, trample, deathtouch, lifelink, infect, toxic, wither), Fog, blocking legality (flying/reach, fear, intimidate, shadow, horsemanship, skulk, menace, protection), commander damage tracking, attack restrictions (OnlyAlone, NotAlone, NeedGreaterPower, NeedTwoOthers, Never), attack requirements (must-attack statics + goad integration), global attack limit prioritization (must-attackers kept when limit exceeded), damage assignment order (player chooses order for multi-blocked attackers, UI modal), multi-defender support (DefenderId enum for player/planeswalker targets, get_possible_defenders, damage routing to permanents), combat last-known-information (CombatLki struct, pre-populated before damage, was_attacking/was_blocking queries).

| Missing Feature | Java File(s) | Priority |
|----------------|-------------|----------|
| Ninjutsu block replacement | — | Medium |
| Banding | `AttackingBand.java` | Low |

### 23.10 Mana System — ✅ COMPLETE

The mana system is fully implemented. See `mana_system_gaps.md` for the detailed breakdown. All items from the roadmap (Tier 1-3) are checked off including: individual mana objects, interactive payment, persistent/combat mana, conversion matrix, snow, restrictions, keywords/counters on mana, uncounterable mana (Cavern of Souls), production doublers/replacers, sunburst/converge, mana burn, refund on cancel, waterbend, X costs, phyrexian mana, cost reduction/increase statics, and shock land ETB prompts.

### 23.11 Infrastructure Gaps

| System | Status | Java Files | Notes |
|--------|--------|-----------|-------|
| Game Logging | Partial | `GameLog.java`, `GameLogEntry.java`, `GameLogEntryType.java`, `GameLogFormatter.java` | Rust has `game_log.rs`/`game_log_entry*.rs` + formatter + loop call sites; Tauri emits structured log DTOs and UI renders typed entries with timestamps, turn separators, and source/target chips. Added explicit combat logs (`Combat phase begins`, `Attackers: ...`, `Blockers: ...`) and UI name resolution for players/cards in log metadata. Missing Java parity: in-memory observable log store, event-visitor formatter pipeline, and full entry-type coverage. |
| Game Snapshots | Partial | `GameSnapshot.java` | Rust now has engine snapshot primitives: `game_snapshot.rs` + `GameLoop::{make_snapshot,restore_snapshot,stash_game_state,restore_game_state}` (captures `GameState`, `mana_pools`, `combat`, `trigger_handler`; optional stack inclusion). Not yet wired to Karn/subgame effect handlers or UI rewind timeline. |
| Format/Rules Config | 0% | `GameRules.java`, `GameFormat.java`, `GameType.java` | Format enforcement, banned lists |
| Match Management | 0% | `Match.java` | Best-of-3, sideboarding |
| Event Visitor System | 0% | `IGameEventVisitor.java` + 60+ event classes | TriggerType enum works but less extensible |
| Card Property Evaluation | Partial | `CardProperty.java`, `CardPredicates.java`, `CardLists.java` | String matching in trigger.rs; needs full evaluation |
| SpellAbility Conditions | 0% | `SpellAbilityCondition.java`, `SpellAbilityRestriction.java` | ConditionDefined/ConditionPresent partial in effects only |

---

## UI Features (Tauri + React Frontend)

| Feature | Status | Notes |
|---------|--------|-------|
| Set code on `CardInstance` | **Implemented** | `card.set_code: Option<String>` in `forge-engine/src/card/mod.rs` |
| Set code in `CardDto` | **Implemented** | `set_code: String` in `game_view_dto.rs`; serialized as `setCode` via serde |
| Set code in preset decks | **Implemented** | All 19 preset decks in `preset_decks.rs` carry a Scryfall set code per card entry (3-tuple format) |
| Set code in custom decks | **Implemented** | `CardIdentity.set_code` propagated to engine via `build_custom_deck` |
| Scryfall set-specific image fetch | **Implemented** | `getCardByName(name, setCode?)` in `scryfall.ts`; falls back to name-only on miss |
| `useCardImage` set-aware | **Implemented** | Hook passes `setCode` to `getCardByName`; `Card.tsx` passes `card.setCode` |
| Batch collection fetch with set | **Implemented** | `fetchCardCollection` sends `{ name, set }` identifiers to Scryfall `/cards/collection` |
| Print picker modal | **Implemented** | `PrintPickerModal.tsx` — fetches all printings via `prints_search_uri`, updates deck via `updatePrint` |
| Print picker in deck builder (list view) | **Implemented** | Image icon button on each `CardRow`; opens `PrintPickerModal` |
| Print picker in deck builder (visual/stack) | **Implemented** | Image icon overlay on each `CardVisual`; opens `PrintPickerModal` |
| Print picker for commander | **Implemented** | Commander card visual mode also exposes print picker |
| Auto-priority pass | **Implemented** | `Game.tsx` useEffect auto-responds after random 300-800ms delay when `chooseAction` has no playable cards, `chooseAttackers` has no attackers, or `chooseBlockers` has no blockers; toggle in `Settings.tsx` backed by `usePreferencesStore.autoPassEnabled` |
