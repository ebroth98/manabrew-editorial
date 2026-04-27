# Forge Card Script DSL — Formal Grammar

> Reference grammar for the Forge card-definition DSL, derived from the corpus
> in `forge/forge-gui/res/cardsfolder/` and the parsers in `forge-card-script`
> and `forge-carddb`.

## Notation

| Symbol   | Meaning                                 |
| -------- | --------------------------------------- |
| `::=`    | Production rule                         |
| `\|`     | Alternative                             |
| `( … )`  | Grouping                                |
| `[ … ]`  | Optional (0 or 1)                       |
| `{ … }`  | Repetition (0 or more)                  |
| `{ … }+` | Repetition (1 or more)                  |
| `'…'`    | Terminal literal                        |
| `/…/`    | Regex terminal                          |
| `ε`      | Empty string                            |
| `NL`     | Newline (`\n` or `\r\n`)                |
| `WS`     | Horizontal whitespace (`' '` or `'\t'`) |

This grammar is **syntactic only**. It defines how to split input into
structural tokens. All semantic interpretation — determining what a value
_means_ — is performed in a separate phase (§3).

---

## 1. Top-Level Structure

A card script is a sequence of lines, optionally containing alternate faces
separated by the `ALTERNATE` sentinel.

```ebnf
CardScript     ::= Face { FaceSeparator Face }

Face           ::= Line { NL Line } [ NL ]

FaceSeparator  ::= { BlankLine NL } 'ALTERNATE' NL { BlankLine NL }
                 |  SpecializeLine NL { BlankLine NL }

SpecializeLine ::= 'SPECIALIZE:' RawValue

Line           ::= BlankLine
                 |  CommentLine
                 |  FieldLine
                 |  AbilityLine
                 |  KeywordLine
                 |  TriggerLine
                 |  StaticAbilityLine
                 |  ReplacementLine
                 |  SVarLine
                 |  AlternateModeLine
                 |  IgnoredLine

BlankLine      ::= /[ \t]*/          (* empty string or whitespace-only *)
CommentLine    ::= '#' RawValue
```

### 1.1 Line Classification

Line classification is determined by the text before the first `:`.

| Prefix                                                        | Production                |
| ------------------------------------------------------------- | ------------------------- |
| `A`                                                           | AbilityLine               |
| `T`                                                           | TriggerLine               |
| `S`                                                           | StaticAbilityLine         |
| `R`                                                           | ReplacementLine           |
| `K`                                                           | KeywordLine               |
| `SVar`                                                        | SVarLine                  |
| `AlternateMode`                                               | AlternateModeLine         |
| `ALTERNATE` (no colon)                                        | FaceSeparator             |
| `/SPECIALIZE.*/`                                              | SpecializeLine            |
| `/SETCOLORID.*/`                                              | IgnoredLine               |
| Known field key                                               | FieldLine                 |
| `AI`, `DeckHints`, `DeckNeeds`, `DeckHas`, `HandLifeModifier` | IgnoredLine               |
| (anything else)                                               | Unknown — emit diagnostic |

The key is matched **before the first colon** in the line. If no colon
exists and the line is not `ALTERNATE` or blank, the line is malformed.

---

## 2. Syntactic Grammar

All productions in this section define structure only. Values are opaque
strings (`RawValue`) at the syntactic level.

### 2.1 Metadata Fields

```ebnf
FieldLine         ::= FieldKey ':' RawValue

FieldKey          ::= 'Name'
                    |  'ManaCost'
                    |  'Types'
                    |  'PT'
                    |  'Colors'
                    |  'Defense'
                    |  'Loyalty'
                    |  'Oracle'
                    |  'Text'
                    |  'FlavorName'
                    |  'Lights'
                    |  'MeldPair'
                    |  'Draft'
                    |  'Variant'

AlternateModeLine ::= 'AlternateMode:' RawValue

IgnoredLine       ::= IgnoredKey ':' RawValue
IgnoredKey        ::= 'AI' | 'DeckHints' | 'DeckNeeds' | 'DeckHas'
                    |  'HandLifeModifier' | /SETCOLORID.*/
```

`FieldKey` values are semantically distinguished but syntactically
identical: each is `Key ':' RawValue`. The value string is not parsed
further at this level.

### 2.2 Param Record (Core Syntax)

The pipe-delimited `Key$ Value` notation is the backbone of abilities,
triggers, static abilities, replacement effects, and SVar values.

```ebnf
ParamRecord    ::= Param { '|' Param }

Param          ::= ParamKey '$' [ WS ] RawValue

ParamKey       ::= /[^$|\n]+/
```

A `ParamRecord` is always non-empty: it contains at least one `Param`.
Constructs that permit an absent param record use `[ ParamRecord ]` at
the call site.

`ParamKey` must be non-empty after trimming leading and trailing
whitespace. A param segment that contains no `$` or whose key is
empty after trimming is malformed and produces a diagnostic.

`ParamKey` is **trimmed** after extraction: leading and trailing
whitespace is stripped before any comparison or lookup. All key matching
(including the key→type mapping in §11) operates on the trimmed form.

`ParamKey` consumes everything up to the first `$` within the pipe-
delimited segment. Leading and trailing whitespace on both key and value
are trimmed after splitting.

Within a `ParamRecord`, the effective `RawValue` for each param is
bounded by the next `|` delimiter or end of line — not by the newline
regex in the top-level `RawValue` production (§2.9). This is a
structural constraint imposed by the pipe-splitting step, not a
modification of the `RawValue` terminal itself.

### 2.3 Abilities

```ebnf
AbilityLine    ::= 'A:' AbilityBody

AbilityBody    ::= AbilityRecord '$' RawValue [ '|' ParamRecord ]

AbilityRecord  ::= 'SP' | 'AB'
```

The first param in an `AbilityBody` is always the ability record key
(`SP$` or `AB$`). Its `RawValue` is the ability API name (e.g.
`DealDamage`, `Draw`, `Mana`). The remaining `ParamRecord`, if present,
contains the ability's parameters.

When the optional `[ '|' ParamRecord ]` is absent, the ability has the
API name and no additional params. This is unambiguous: the `|` is
mandatory to begin additional params. `A:AB$ Draw` parses as
record=`AB`, API=`Draw`, params=∅.

Sub-abilities (used inside SVars) use `DB$` or `ST$` as the record key.
The distinction between `AB`/`SP`/`DB`/`ST` is **not syntactic** — the
parser sees the same `AbilityBody` structure and distinguishes them by
the record key value during Phase 2. There is no separate
`SubAbilityRecord` production in the grammar.

### 2.4 Triggers

```ebnf
TriggerLine    ::= 'T:' ParamRecord
```

### 2.5 Static Abilities

```ebnf
StaticAbilityLine ::= 'S:' ParamRecord
```

### 2.6 Replacement Effects

```ebnf
ReplacementLine ::= 'R:' ParamRecord
```

### 2.7 SVars (Substitution Variables)

```ebnf
SVarLine       ::= 'SVar:' SVarName ':' SVarBody

SVarName       ::= /[A-Za-z_][A-Za-z0-9_]*/

SVarBody       ::= RawValue
```

`SVarName` is delimited by the **second** colon on the line (the first
colon separates `SVar` from the rest). Everything after `SVarName ':'`
to end of line is `SVarBody`, captured as a raw string.

Interpretation of `SVarBody` is **context-dependent** and happens outside
the grammar. During Phase 2 (§3), `SVarBody` is **trimmed** (leading and
trailing whitespace removed) before the following detection rule is
applied:

- If the trimmed `SVarBody` matches the pattern `^(AB|SP|DB|ST)\$` (i.e.
  it begins with one of the four record keys followed immediately by
  `$`), it is re-parsed as an `AbilityBody`.
- Otherwise, if `SVarBody` contains both `$` and `|`, it is re-parsed
  as a `ParamRecord`.
- Otherwise it is treated as a raw expression string. This includes
  single-param bodies like `Defined$ You` (contains `$` but no `|`):
  these are **not** re-parsed as a one-param `ParamRecord`. They are
  raw expression strings whose `$` is part of the expression syntax
  (e.g., `Count$xPaid`, `SVar$Z1/Plus.Z2`).

This classification is applied during Phase 2, not during structural
parsing. The grammar itself does not distinguish these cases.

### 2.8 Keywords

```ebnf
KeywordLine    ::= 'K:' RawValue
```

Keyword values are syntactically opaque. Semantic interpretation splits
the raw value on `:` to distinguish simple keywords, parameterized
keywords, and class level definitions:

| Pattern                       | Interpretation                           |
| ----------------------------- | ---------------------------------------- |
| `Flying`                      | Simple keyword                           |
| `Evoke:2 U`                   | Parameterized keyword (name `:` param)   |
| `Class:2:1 G:AddTrigger$ Foo` | Class level (level `:` cost `:` payload) |

This sub-parsing is **not part of the syntactic grammar**.

### 2.9 Lexical Rules

```ebnf
RawValue       ::= /[^\n]*/

IDENTIFIER     ::= /[A-Za-z_][A-Za-z0-9_]*/

INTEGER        ::= /[+-]?[0-9]+/
```

`RawValue` is the catch-all terminal for any uninterpreted text to end
of line. Within a `ParamRecord`, the effective `RawValue` for each param
is bounded by `|` rather than by newline.

---

## 3. Parsing Model

Parsing proceeds in two strictly separated phases.

### 3.1 Phase 1 — Structural Parse

Input: raw text of one card script file.
Output: a list of `ScriptLine` records, each tagged with a `LineKind`.

1. Split input on newlines.
2. For each line, trim whitespace and classify by prefix (§1.1).
3. For `A:`, `T:`, `S:`, `R:` lines, split the value portion into a
   `ParamRecord`: split on `|`, then split each segment on the first
   `$` to yield `(key, raw_value)` pairs.
4. For `SVar:` lines, extract `SVarName` and `SVarBody` as raw strings.
   Optionally re-parse `SVarBody` as a `ParamRecord` if it matches the
   heuristic (§2.7).
5. For `K:` lines, capture the raw value string.
6. For field lines (`Name:`, `Types:`, etc.), capture `(key, raw_value)`.

Phase 1 is **context-free**. It requires no knowledge of ability APIs,
trigger modes, or param semantics. The output is a flat list of
`(key, raw_value)` pairs (for params) or `(line_kind, raw_value)` pairs
(for lines).

### 3.2 Phase 2 — Semantic Decode

Input: the structural parse from Phase 1.
Output: typed IR nodes.

For each `(key, raw_value)` pair in a `ParamRecord`, determine the
semantic type of `raw_value` using the key→type mapping (§11). Then
parse `raw_value` according to that type's sub-grammar.

This phase is **context-sensitive**: the same raw string may parse as
an `Amount`, a `Selector`, or plain `Text` depending on the key. The
priority-ordered mapping in §11 defines which interpretation wins.

Phase 2 also resolves:

- SVar bodies into their typed forms (ability, param record, or expression).
- Keyword lines into simple, parameterized, or class keywords.
- Field values into their domain types (mana cost, type line, P/T, etc.).

### 3.3 Invariants

- Phase 1 never fails on well-formed lines (every line can be split).
  Malformed lines (missing colon, empty key) produce diagnostics but do
  not halt parsing.
- Phase 2 may produce `Raw` as a fallback type for any value that does
  not match a more specific rule. This is not an error.
- A `ParamRecord` may contain duplicate keys. The last occurrence wins
  for lookup purposes, but all occurrences are preserved in the IR.
  Param ordering within a record is preserved as authored.
- `RawValue` is **never partially consumed** in Phase 1. Each raw value
  is captured in its entirety (bounded by `|` within a `ParamRecord`,
  or by end of line at the top level). Sub-parsing of raw values into
  typed forms occurs exclusively in Phase 2.
- `ParamRecord` parsing is **round-trip safe**: the Phase 1 output must
  preserve param ordering, raw key strings, and raw value strings
  exactly as authored. Reconstructing the original pipe-delimited string
  from the parsed `(key, raw_value)` pairs must produce an equivalent
  result (modulo whitespace trimming).

---

## 4. Semantic Types

The following types exist only in Phase 2. They are **not** syntactic
productions — they describe how raw strings are decoded after structural
parsing.

### 4.1 ManaCost

```ebnf
ManaCostValue  ::= 'no cost'
                |  { ManaSymbol WS }+
                |  ε

ManaSymbol     ::= GenericMana | ColorMana | HybridMana | PhyrexianMana
                |  SnowMana | ColorlessMana | VariableMana

GenericMana    ::= /[0-9]+/
ColorMana      ::= 'W' | 'U' | 'B' | 'R' | 'G'
HybridMana     ::= ColorMana '/' ColorMana
                |  GenericMana '/' ColorMana
                |  ColorMana '/' 'P'
PhyrexianMana  ::= ColorMana 'P'
SnowMana       ::= 'S'
ColorlessMana  ::= 'C'
VariableMana   ::= 'X' | 'Y' | 'Z'
```

Symbols are space-separated (e.g. `2 U U`, `RW RW`, `X R R`).

Applied to the `RawValue` of a `ManaCost:` field line.

### 4.2 Types

```ebnf
TypesValue     ::= { TypeWord WS }+
TypeWord       ::= Supertype | CardType | Subtype
Supertype      ::= 'Legendary' | 'Basic' | 'Snow' | 'World' | 'Ongoing'
CardType       ::= 'Creature' | 'Instant' | 'Sorcery' | 'Enchantment'
                |  'Artifact' | 'Land' | 'Planeswalker' | 'Kindred'
                |  'Battle' | 'Dungeon'
Subtype        ::= IDENTIFIER
```

Applied to the `RawValue` of a `Types:` field line.

### 4.3 Power/Toughness

```ebnf
PTValue        ::= PTComponent '/' PTComponent
PTComponent    ::= INTEGER | '*' | 'X' | '+' INTEGER | '*+' INTEGER
```

Applied to the `RawValue` of a `PT:` field line.

### 4.4 SplitType

```ebnf
SplitType      ::= 'Split' | 'Flip' | 'Transform' | 'DoubleFaced'
                |  'Modal' | 'Meld' | 'Adventure' | 'Specialize'
                |  'MDFC'
```

Applied to the `RawValue` of an `AlternateMode:` line.

### 4.5 Selectors (Filter Expressions)

Selectors filter game objects. They appear as the `RawValue` of params
whose key maps to type `Selector` in §11.

```ebnf
Selector          ::= SelectorAlt { ',' SelectorAlt }

SelectorAlt       ::= SelectorChain { ' & ' SelectorChain }

SelectorChain     ::= SelectorPart { ( '.' | '+' ) SelectorPart }

SelectorPart      ::= NegatedPart | AtomicPart

NegatedPart       ::= '!' AtomicPart

AtomicPart        ::= ComparisonFilter | IDENTIFIER
```

**Operator precedence** (highest to lowest):

| Precedence  | Operator           | Associativity | Meaning                                         |
| ----------- | ------------------ | ------------- | ----------------------------------------------- |
| 1 (highest) | `.` `+`            | left          | Chain: intersect filters within one alternative |
| 2           | `&` (space-padded) | left          | Conjunction of selector chains                  |
| 3 (lowest)  | `,`                | left          | Disjunction: logical OR of alternatives         |

All selector operators (`.`, `+`, `&`, `,`) are **left-associative**.
The `.` and `+` operators are equivalent in precedence. Both conjoin
filters. The distinction is **semantic**: `.` is the traditional
separator; `+` is used when the filter text itself contains dots (e.g.
subtype names).

`ComparisonFilter` is an `AtomicPart` that matches the pattern:

```ebnf
ComparisonFilter  ::= CompProp CompOp INTEGER
CompProp          ::= 'cmc' | 'power' | 'toughness' | 'counters_'
CompOp            ::= 'EQ' | 'NE' | 'LT' | 'LE' | 'GT' | 'GE'
```

**Disambiguation rule**: `ComparisonFilter` is attempted before
`IDENTIFIER` (greedy parse). Since `ComparisonFilter` and `IDENTIFIER`
overlap at the start (e.g. `cmcGE3` begins with letters), the parser
must attempt `ComparisonFilter` first. If that fails (no known
`CompProp` prefix, or no valid `CompOp` follows), the text is treated
as a plain `IDENTIFIER`.

Classification of `IDENTIFIER` parts into base types, ownership filters,
state filters, property filters, etc. is **semantic**, not syntactic.
The grammar recognizes only `IDENTIFIER` at this level.

**Cardinality invariant**: Selectors evaluate to **sets** of game
objects (zero or more). References (§4.6), though parsed with the same
syntax, may resolve to single objects or engine-defined collections
(e.g., `Remembered`, which is a runtime-populated list). The expected
cardinality is determined by the consuming parameter, not by the
reference syntax itself. This distinction is enforced at evaluation
time, not at parse time.

#### Selector Examples

| Input                           | Parse                                                |
| ------------------------------- | ---------------------------------------------------- |
| `Creature.YouCtrl`              | One alt, one chain, two parts: `Creature`, `YouCtrl` |
| `Creature.YouCtrl.Tapped`       | One alt, one chain, three parts                      |
| `Instant.YouOwn,Sorcery.YouOwn` | Two alts (OR), each one chain                        |
| `Creature.OppCtrl+cmcLE3`       | One alt, one chain, two parts: `OppCtrl`, `cmcLE3`   |
| `Card.Self & Creature.Token`    | One alt, two chains (AND)                            |

### 4.6 References

References point at specific game objects. They appear as the `RawValue`
of params whose key maps to type `Reference` in §11.

Syntactically, a reference is parsed identically to a `Selector` (§4.5):
the same `.`/`+`/`,`/`&` splitting applies. The distinction between
`Selector` and `Reference` is determined entirely by the key→type
mapping.

Well-known reference values (non-exhaustive):

`Self`, `You`, `Opponent`, `Player`, `TriggeredCard`, `TriggeredCardLKI`,
`TriggeredAttackerLKICopy`, `TriggeredPlayer`, `TriggeredTarget`,
`TriggeredDefendingPlayer`, `Remembered`, `RememberedLKI`, `Imprinted`,
`ChosenCard`, `ChosenPlayer`, `TargetedController`, `Targeted`,
`EffectSource`, `Source`, `Enchanted`, `Equipped`, `ParentTarget`.

This set is open and extensible.

### 4.7 Zones

```ebnf
ZoneList       ::= Zone { ',' Zone }
Zone           ::= 'Battlefield' | 'Hand' | 'Graveyard' | 'Library'
                |  'Exile' | 'Stack' | 'Command' | 'Any' | 'All'
```

Applied to `RawValue` of params whose key maps to type `ZoneList` in §11.

### 4.8 Amounts

```ebnf
Amount         ::= INTEGER
                |  'X'
                |  'Any'
                |  'All'
                |  RawValue            (* fallback *)
```

Applied to `RawValue` of params whose key maps to type `Amount` in §11.
The parser attempts each alternative in order. The `RawValue` fallback
captures everything else — including SVar names, `Count$` expressions,
and arithmetic formulas. Whether a `RawValue` that looks like an
`IDENTIFIER` is an SVar reference or something else is resolved during
Phase 2 semantic evaluation, not during amount parsing.

The `RawValue` fallback is not guaranteed to be interpretable at
runtime. If the expression evaluator cannot resolve the value to a
numeric result, this is a **runtime error** (typically an authoring
error in the card script). Handling is implementation-defined.

### 4.9 Cost Strings

Cost strings appear as the `RawValue` of params whose key maps to type
`Cost` in §11 (e.g. `Cost$`, `ExtraCost$`).

**Cost strings are not a strict grammar.** They are tokenized
left-to-right by splitting on whitespace, then classifying each token:

```
CostString     ::= { CostToken WS }+

CostToken      ::= 'T'                              (* tap self *)
                |  'Q'                              (* untap self *)
                |  AngleBracketCost                 (* tried before ManaSymbol *)
                |  ManaSymbol                       (* reuse §4.1 *)

AngleBracketCost ::= CostAction '<' CostSpec '>'

CostAction     ::= 'AddCounter' | 'SubCounter'
                |  'Sacrifice' | 'Discard' | 'Exile'
                |  'Mill' | 'PayLife' | 'TapType'
                |  'UntapType' | 'RemoveCounter'
                |  'Return' | 'Reveal'

CostSpec       ::= CostParam { '/' CostParam }
CostParam      ::= /[^/><]+/
```

`CostAction` is an open set. The listed values are commonly occurring
but not exhaustive.

An `AngleBracketCost` requires a matching `>` to close the `<`. Nested
angle brackets are not permitted — `CostParam` excludes `<` and `>`. If
a token contains `<` but no matching `>`, it is not a valid
`AngleBracketCost` and is treated as `Raw` with a diagnostic emitted.

Tokenization is **greedy and left-to-right**: each whitespace-separated
token is classified independently. `AngleBracketCost` is attempted
before `ManaSymbol` for each token (a token beginning with an uppercase
letter followed by `<` is an angle-bracket cost, not a mana symbol).
Mana symbols and angle-bracket costs may be freely intermixed (e.g.
`2 T SubCounter<All/CHARGE>`).

#### Cost String Examples

| Input                        | Tokens                                                    |
| ---------------------------- | --------------------------------------------------------- |
| `T`                          | `[Tap]`                                                   |
| `2 W B`                      | `[Mana(2), Mana(W), Mana(B)]`                             |
| `AddCounter<2/LOYALTY>`      | `[AngleBracket(AddCounter, [2, LOYALTY])]`                |
| `2 T SubCounter<All/CHARGE>` | `[Mana(2), Tap, AngleBracket(SubCounter, [All, CHARGE])]` |

### 4.10 Comparisons

```ebnf
Comparison        ::= SpacedComparison | CompactComparison

SpacedComparison  ::= LHS WS CompOp WS RHS

CompactComparison ::= [ LHS WS ] CompOp RHS

CompOp            ::= 'GTE' | 'LTE' | 'GE' | 'LE' | 'GT' | 'LT' | 'NE' | 'EQ'
                    |  '==' | '!=' | '>=' | '<=' | '>' | '<'

LHS               ::= /[^\s]+/
RHS               ::= /[^\s]+/
```

Three surface forms exist:

- Spaced: `LHS op RHS` (e.g. `X EQ 0`)
- Compact with LHS: `LHS opRHS` (e.g. `X GE2`)
- Compact without LHS: `opRHS` (e.g. `GE2`)

The parser attempts `SpacedComparison` first (three whitespace-separated
tokens). If that fails, it attempts `CompactComparison`. In
`CompactComparison`, **no whitespace** is permitted between `CompOp` and
`RHS` — they are concatenated (e.g., `GE2`, not `GE 2`). `CompOp`
alternatives are ordered by longest prefix (`GTE` before `GT`, `LTE`
before `LT`) to ensure greedy matching in the compact form.

### 4.11 Transforms

```ebnf
Transform      ::= RawValue '->' RawValue
```

The split is performed on the **first** occurrence of `->` in the raw
value. Everything before it (trimmed) is the source; everything after
it (trimmed) is the target. Both sides must be non-empty after trimming.

Presence of `->` in the raw value triggers this interpretation.

### 4.12 Expressions

Expression values are opaque strings with internal structure. Common
patterns (non-exhaustive):

```
CountExpr      ::= 'Count$' CountBody
CountBody      ::= IDENTIFIER
                |  'Valid' WS SelectorText
                |  IDENTIFIER '/' ArithOp '.' IDENTIFIER

ArithOp        ::= 'Plus' | 'Minus' | 'Times' | 'Twice' | 'Half'
                |  'LimitMax' | 'LimitMin'

SVarExpr       ::= 'SVar$' SVarName [ '/' ArithOp '.' SVarExpr ]

RememberedExpr ::= 'RememberedLKI$' IDENTIFIER
                |  'Remembered$' IDENTIFIER

TriggeredExpr  ::= 'Triggered$' IDENTIFIER
```

Expressions form a **recursive, open-ended sub-language**. They are not
fully specified here because new `Count$` functions and arithmetic
operators are added regularly. The parser should treat the portion after
the `Count$` / `SVar$` / etc. prefix as a raw string and interpret it
via a dedicated expression evaluator at runtime.

### 4.13 Boolean

```ebnf
Boolean        ::= 'True' | 'False' | 'true' | 'false' | 'TRUE' | 'FALSE'
```

Case-insensitive matching.

### 4.14 Symbol

A `Symbol` is a single identifier used as an enum discriminant (e.g.
trigger modes, static modes, phases). Syntactically it is just an
`IDENTIFIER`. The set of valid symbols depends on context.

### 4.15 Text

Free-form text (descriptions, prompts). No sub-parsing. `CARDNAME` and
`NICKNAME` are substitution placeholders replaced at display time.

### 4.16 SVarReference

One or more SVar names, comma-separated or `&`-separated:

```ebnf
SVarRefList    ::= SVarName { ( ',' | ' & ' ) SVarName }
```

### 4.17 DelimitedList

Comma-separated identifiers:

```ebnf
DelimitedList  ::= IDENTIFIER { ',' IDENTIFIER }
```

---

## 5. Triggers (Semantic Constraints)

A `TriggerLine` is syntactically a `ParamRecord` (§2.4). At the semantic
level, the following constraints apply:

The `Mode$` param is **required** and must be one of:

```
'ChangesZone'       'Phase'              'Attacks'
'Blocks'            'SpellCast'          'Damage'
'DamageDealtOnce'   'AttackersDeclared'  'BecomesBlocked'
'Upkeep'            'EndOfTurn'          'DiscardedCard'
'SpellAbilityCast'  'CounterAdded'       'CounterRemoved'
'LifeGained'        'LifeLost'           'TurnFaceUp'
'Untaps'            'Sacrificed'         'Taps'
'AbilityResolves'   'BecomesTarget'      'Always'
'PayLife'
```

This set is open and extensible.

Common params by trigger mode:

| Param                    | Type          | Usage                                   |
| ------------------------ | ------------- | --------------------------------------- |
| `ValidCard$`             | Selector      | What card matches the trigger condition |
| `Origin$`                | Zone          | Source zone (ChangesZone)               |
| `Destination$`           | Zone          | Destination zone (ChangesZone)          |
| `Execute$`               | SVarReference | SVar to invoke when triggered           |
| `TriggerZones$`          | ZoneList      | Where the trigger source must be        |
| `TriggerDescription$`    | Text          | Human-readable description              |
| `Phase$`                 | Symbol        | Which phase (Phase mode)                |
| `ValidActivatingPlayer$` | Reference     | Who activated/cast                      |
| `Secondary$`             | Boolean       | Whether this is a secondary trigger     |

---

## 6. Static Abilities (Semantic Constraints)

A `StaticAbilityLine` is syntactically a `ParamRecord` (§2.5). At the
semantic level:

The `Mode$` param is **required** and must be one of:

```
'Continuous'        'CantAttack'         'CantBlock'
'CantCast'          'CantBeCast'         'ReduceCost'
'RaiseCost'         'AlternativeCost'    'SetProperty'
'CantTarget'        'CantDamage'         'CantAttackBlock'
'CantBeCountered'   'Undo'
'CantPlayLand'      'CantBeSacrificed'
```

This set is open and extensible.

Key params: `Affected$` (Selector), `AddKeyword$` / `KW$` (DelimitedList),
`AddPower$` / `AddToughness$` (Amount), `Layer$` (Symbol), `Description$`
(Text).

---

## 7. Replacement Effects (Semantic Constraints)

A `ReplacementLine` is syntactically a `ParamRecord` (§2.6). At the
semantic level:

The `Event$` param is **required** and must be one of:

```
'Moved'          'Damage'         'DamageDealt'
'Drawn'          'Discard'        'Destroy'
'Cast'           'ChangeZone'     'Counter'
'TurnFaceUp'     'SetInMotion'
```

This set is open and extensible.

Key params: `ValidCard$` (Selector), `ReplaceWith$` (SVarReference),
`ReplacementResult$` (Symbol), `Description$` (Text).

---

## 8. Keywords (Semantic Structure)

A `KeywordLine` is syntactically `'K:' RawValue` (§2.8). The raw value
is interpreted by splitting on `:` with the following patterns:

### 8.1 Simple Keywords

```
K:Flying
K:Haste
K:Indestructible
```

A single identifier (possibly multi-word like `First Strike`).

### 8.2 Parameterized Keywords

```
K:Evoke:2 U
K:Kicker:1 G
K:Partner with:Brallin, Skyshark Rider
K:Crew:3
```

`KeywordName ':' RawParam`. The param is a raw string — it may be a
cost string, a card name, or an integer depending on the keyword.

### 8.3 Class Keywords

```
K:Class:2:1 G:AddTrigger$ TriggerAttackersDeclared
K:Class:3:3 G:AddStaticAbility$ SMayLook & SMayPlay
```

`'Class' ':' Level ':' Cost ':' Payload`

The split is performed on the **first three** `:` characters in the raw
value. Everything after the third `:` is the payload verbatim (the
payload itself may not contain unescaped `:`).

- `Level` is an integer.
- `Cost` is a cost string (may contain spaces but not `:`).
- `Payload` is one or more `Key$ SVarName` pairs separated by `&`.

Well-known keyword names (non-exhaustive):

`Flying`, `First Strike`, `Double Strike`, `Haste`, `Lifelink`,
`Deathtouch`, `Vigilance`, `Trample`, `Menace`, `Hexproof`, `Shroud`,
`Indestructible`, `Defender`, `Reach`, `Flash`, `Fear`, `Wither`,
`Infect`, `Undying`, `Persist`, `Retrace`, `Changeling`, `Convoke`,
`Delve`, `Affinity`, `Evoke`, `Kicker`, `Flashback`, `Crew`,
`Mobilize`, `Renown`, `Partner`, `Partner with`.

This set is open and extensible.

---

## 9. Execution Model

### 9.1 SVar Graph

SVars define a **directed graph** of named values. Each SVar maps a
name to a body. Bodies that are ability records (`DB$`, `AB$`, etc.)
may reference other SVars via `Execute$`, `SubAbility$`,
`TrueSubAbility$`, `FalseSubAbility$`, and similar SVar-reference
params.

```
A:SP$ DealDamage | ... | SubAbility$ DmgController
SVar:DmgController:DB$ DealDamage | Defined$ TargetedController | ...
```

Here `SubAbility$ DmgController` creates an edge from the spell ability
to the SVar `DmgController`.

### 9.2 SVar Resolution

When the engine encounters an SVar-reference param (e.g. `Execute$ Foo`),
it looks up `Foo` in the current face's SVar table. Resolution is
**late-bound**: SVars are resolved at execution time, not at parse time.

SVars may reference other SVars, forming chains:

```
SVar:A:DB$ Draw | SubAbility$ B
SVar:B:DB$ GainLife | ...
```

### 9.3 Cycle Semantics

The SVar graph may contain cycles (e.g. `A → B → A`). The grammar does
not forbid this. Cycle detection and handling is the responsibility of
the runtime engine. In practice, cycles are rare and typically indicate
a script authoring error.

### 9.4 Execution Order

Ability resolution follows a depth-first model:

1. Resolve the top-level ability (the `A:` line or the `Execute$` target).
2. After the ability's effect is applied, resolve `SubAbility$` if present.
3. `TrueSubAbility$` / `FalseSubAbility$` are conditional branches:
   one is chosen based on a runtime condition check.
4. `Execute$` on a trigger is the entry point: the trigger fires, then
   the `Execute$` SVar is resolved as a sub-ability.

### 9.5 Expression Evaluation

SVars whose body is a raw expression (e.g. `Count$xPaid`,
`SVar$Z1/Plus.Z2`) are evaluated at runtime to produce a numeric or
boolean value. Expression evaluation may recursively reference other
SVars.

---

## 10. Implicit Defaults

Many ability params have implicit default values that are filled in when
the param is absent from the `ParamRecord`. These defaults are
**ability-specific** and **not part of the grammar**.

Examples (non-exhaustive):

| Ability              | Param               | Default                   |
| -------------------- | ------------------- | ------------------------- |
| `DealDamage`         | `NumDmg$`           | (required — no default)   |
| Any targeted ability | `TargetMin$`        | `1`                       |
| Any targeted ability | `TargetMax$`        | `1`                       |
| `Draw`               | `NumCards$`         | `1`                       |
| `Draw`               | `Defined$`          | `You`                     |
| `GainLife`           | `Defined$`          | `You`                     |
| `ChangeZone`         | `Origin$`           | (required)                |
| `ChangeZone`         | `Destination$`      | (required)                |
| `PutCounter`         | `CounterNum$`       | `1`                       |
| `Pump`               | `Duration$`         | `UntilEndOfTurn`          |
| `Token`              | `TokenOwner$`       | `You`                     |
| Any ability          | `StackDescription$` | `SpellDescription$` value |
| Any trigger          | `Secondary$`        | `False`                   |

Defaults are defined per-ability in the engine's ability factory, not in
the card script. A conforming parser must not inject defaults during
parsing — defaults are applied at ability construction time.

---

## 11. Key → Type Mapping

The semantic type of a param's `RawValue` is determined by its key. The
following rules are applied in **priority order** during Phase 2. The
first matching rule wins.

| Priority | Key Condition                                                                                                                                                                                               | Value Condition                                                                                                              | Semantic Type   |
| -------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------- | --------------- |
| 1        | Key ∈ `{AB, SP, DB, ST}`                                                                                                                                                                                    | —                                                                                                                            | `AbilityRecord` |
| 2        | Key = `Mode` or `Event`, or Key ends with `Mode` or `Logic`                                                                                                                                                 | —                                                                                                                            | `Symbol`        |
| 3        | Key ends with `Description`, `Desc`, `Prompt`, `Message`, `Title`; or Key ∈ explicit text-key set¹                                                                                                          | —                                                                                                                            | `Text`          |
| 4        | Key = `Cost` or `Incorporate`, or Key ends with `Cost` or `CostDesc`                                                                                                                                        | —                                                                                                                            | `Cost`          |
| 5        | —                                                                                                                                                                                                           | Value contains `->`                                                                                                          | `Transform`     |
| 6        | —                                                                                                                                                                                                           | Value ∈ `{True, False}` (case-insensitive)                                                                                   | `Boolean`       |
| 7        | Key ∈ `{Primary, Secondary}`                                                                                                                                                                                | (after boolean check fails)                                                                                                  | `Text`          |
| 8        | Key ∈ explicit symbol-key set²                                                                                                                                                                              | —                                                                                                                            | `Symbol`        |
| 9        | Key contains `Zone` or ends with `Zones`, `Destination`; or Key ∈ explicit zone-key set³                                                                                                                    | —                                                                                                                            | `ZoneList`      |
| 10       | Key = `Execute` or `sVars`, or Key ends with `SubAbility`, `Ability`, `Abilities`, `SVar`, `Pile`, `Subs`; or Key ∈ explicit svar-ref set⁴                                                                  | —                                                                                                                            | `SVarReference` |
| 11       | Key ends with `Amount`, `CMC`, `Power`, `Toughness`, `HandSize`, `Limit`, `Min`, `Max`; or Key starts with `Num` or ends with `Num`; or Key ∈ explicit amount-key set⁵                                      | —                                                                                                                            | `Amount`        |
| 12       | Key = `Defined` or starts with `Defined` or ends with `Defined`, `DefinedPlayer`, `Controller`, `Owner`, `Payer`, `Player`, `Source`, `Decider`, `Defender`, `Damage`; or Key ∈ explicit reference-key set⁶ | —                                                                                                                            | `Reference`     |
| 13       | Key starts with `Valid` or contains `Valid`; or Key ends with `Type`, `Types`, `Cards`, `Choices`, `Players`, `Restrictions`, `Tgts`, `Objects`; or Key ∈ explicit selector-key set⁷                        | —                                                                                                                            | `Selector`      |
| 14       | —                                                                                                                                                                                                           | Value parses as INTEGER                                                                                                      | `Integer`       |
| 15       | —                                                                                                                                                                                                           | Value matches comparison pattern (§4.10)                                                                                     | `Comparison`    |
| 16       | Key ends with `Compare`, `Condition`, `Formula`; or Key contains `ThisTurn`; or Key starts with `CheckOn`; or Key = `Expression` or `LifeTotal` or `Condition`                                              | —                                                                                                                            | `Expression`    |
| 16b      | —                                                                                                                                                                                                           | Value starts with `Count$`, `Remembered$`, `Triggered$`; or Value contains `/Plus.`, `/Minus.`, `/Times.`, `/Twice`, `/Half` | `Expression`    |
| 17       | Key ends with `List`, `Names`, `Colors`, `Color`, `Keyword`, `Keywords`, `KWs`, `Counters`; or Key ∈ explicit list-key set⁸                                                                                 | —                                                                                                                            | `DelimitedList` |
| 17b      | —                                                                                                                                                                                                           | Value contains `,`                                                                                                           | `DelimitedList` |
| 18       | (fallback)                                                                                                                                                                                                  | —                                                                                                                            | `Raw`           |

The explicit key sets referenced above (¹–⁸) are defined in the parser
source (`forge-card-script/src/lib.rs`, functions `is_text_key`,
`is_symbol_key`, `is_zone_key`, `is_svar_reference_key`,
`is_amount_key`, `is_reference_key`, `is_selector_key`,
`is_delimited_list_key`). They are **open sets** that grow as new card
mechanics are added.

### 11.1 Ambiguity Resolution

The table is evaluated strictly top-to-bottom; the first matching rule
wins. Rules 1–4, 7–13, 16, and 17 are **key-dependent** (they match on
key name alone). Rules 5, 6, 14, 15, 16b, and 17b are
**value-dependent** (they inspect the raw value). Because ordering is
strict, a key-dependent rule at priority 16 loses to a value-dependent
rule at priority 14 if both match.

Worked examples:

- `ConditionCompare$ 3`: Value `3` parses as INTEGER (rule 14, priority
  14). Key ends with `Compare` (rule 16, priority 16). Rule 14 wins:
  result is `Integer`. Verified against the parser: `value.parse::<i32>()`
  at line 833 of `parse_semantic_param_value` fires before
  `looks_like_expression_key` at line 839.
- `ConditionCompare$ GE2`: Value `GE2` does not parse as INTEGER (rule
  14 fails). Rule 15 (Comparison) could match, but rule 16 (key ends
  with `Compare`) has higher priority. Result: `Expression`.
- `Secondary$ True`: Key `Secondary` matches rule 7 (Text), but rule 6
  (Boolean, value = `True`) has higher priority. Result: `Boolean`.
- `ValidCards$ Elf,Goblin`: Key matches rule 13 (Selector). Rule 13
  has priority 13, which is before rule 17b (value contains `,`, priority
  17b). Result: `Selector`. The `,` is interpreted as selector
  disjunction, not as a delimited list separator.

---

## 12. Complete Card Example (Annotated)

### 12.1 Simple Creature

```
Name:Grizzly Bears                               # FieldLine: Name
ManaCost:1 G                                      # FieldLine: ManaCost → [Generic(1), Color(G)]
Types:Creature Bear                               # FieldLine: Types → [Creature, Bear]
PT:2/2                                            # FieldLine: PT → (2, 2)
Oracle:                                           # FieldLine: Oracle (empty)
```

### 12.2 Spell with Sub-Abilities

```
Name:Ravaging Blaze                               # FieldLine
ManaCost:X R R                                     # FieldLine
Types:Instant                                      # FieldLine
A:SP$ DealDamage | ValidTgts$ Creature | NumDmg$ X | SubAbility$ DmgController | SpellDescription$ ...
#  ^--- record=SP, API=DealDamage
#       ValidTgts → Selector: [Creature]
#       NumDmg → Amount: X
#       SubAbility → SVarReference: [DmgController]
#       SpellDescription → Text
SVar:DmgController:DB$ DealDamage | Defined$ TargetedController | NumDmg$ X | ConditionPresent$ Instant.YouOwn,Sorcery.YouOwn | ConditionCompare$ GE2 | ...
#  ^--- SVarBody re-parsed as ability (DB$ present)
#       Defined → Reference: TargetedController
#       ConditionPresent → Selector: [Instant.YouOwn, Sorcery.YouOwn]
#       ConditionCompare → Expression: GE2
SVar:X:Count$xPaid
#  ^--- SVarBody is raw expression: Count$xPaid
Oracle:...                                         # FieldLine
```

### 12.3 Planeswalker

```
Name:Jace, the Mind Sculptor                       # FieldLine
ManaCost:2 U U                                      # FieldLine
Types:Legendary Planeswalker Jace                   # FieldLine
Loyalty:3                                           # FieldLine
A:AB$ Dig | Cost$ AddCounter<2/LOYALTY> | ...       # Ability: AB, API=Dig, Cost → [AngleBracket(AddCounter, [2, LOYALTY])]
A:AB$ Draw | Cost$ AddCounter<0/LOYALTY> | NumCards$ 3 | SubAbility$ DBChangeZone | Planeswalker$ True | ...
SVar:DBChangeZone:DB$ ChangeZone | Origin$ Hand | Destination$ Library | ChangeType$ Card | ChangeNum$ 2 | ...
A:AB$ ChangeZone | Cost$ SubCounter<1/LOYALTY> | ...
A:AB$ ChangeZoneAll | Cost$ SubCounter<12/LOYALTY> | ... | Ultimate$ True | ...
SVar:DBChangeZone2:DB$ ChangeZoneAll | ...
Oracle:...                                          # FieldLine
```

### 12.4 Transform Card (Alternate Face)

```
Name:Lambholt Pacifist                              # Face 0: FieldLine
ManaCost:1 G                                         # Face 0: FieldLine
Types:Creature Human Shaman Werewolf                 # Face 0: FieldLine
PT:3/3                                               # Face 0: FieldLine
S:Mode$ CantAttack | ValidCard$ Card.Self            # Face 0: StaticAbilityLine
T:Mode$ Phase | Phase$ Upkeep | Execute$ TrigTransform  # Face 0: TriggerLine
SVar:TrigTransform:DB$ SetState | Defined$ Self | Mode$ Transform  # Face 0: SVarLine
AlternateMode:DoubleFaced                            # AlternateModeLine → Transform
Oracle:...                                           # Face 0: FieldLine

ALTERNATE                                            # FaceSeparator

Name:Lambholt Butcher                               # Face 1: FieldLine
ManaCost:no cost                                     # Face 1: FieldLine
Colors:green                                         # Face 1: FieldLine
Types:Creature Werewolf                              # Face 1: FieldLine
PT:4/4                                               # Face 1: FieldLine
Oracle:...                                           # Face 1: FieldLine
```

### 12.5 Split Card

```
Name:Response                                        # Face 0
ManaCost:RW RW                                        # HybridMana: [RW, RW]
Types:Instant
A:SP$ DealDamage | ValidTgts$ Creature.attacking,Creature.blocking | NumDmg$ 5 | ...
AlternateMode:Split

ALTERNATE

Name:Resurgence                                      # Face 1
ManaCost:3 R W
Types:Sorcery
A:SP$ PumpAll | ValidCards$ Creature.YouCtrl | KW$ First Strike & Vigilance | SubAbility$ DBAddCombat | ...
SVar:DBAddCombat:DB$ AddPhase | ExtraPhase$ Combat | FollowedBy$ Main2 | ...
Oracle:...
```

---

## 13. Design Notes

1. **Line-oriented**: Each construct occupies exactly one line. No
   multi-line constructs exist. `Oracle:` uses literal `\n` in the
   string for display newlines.

2. **Pipe-dollar delimiters**: `|` separates params; `$` separates key
   from value within each param. This pair is the fundamental syntax of
   the DSL.

3. **Key-driven typing**: Value interpretation depends on the key name,
   not on value syntax. The same string `"3"` is an `Amount` for
   `NumDmg$` but an `Integer` for an unknown key. This is by design
   and must be preserved in any parser implementation.

4. **Two-phase architecture**: Structural parsing (Phase 1) is
   context-free and infallible. Semantic decoding (Phase 2) is
   context-sensitive and uses the key→type mapping. This separation
   enables diagnostics, IDE tooling, and incremental re-parsing.

5. **Open enum sets**: Trigger modes, static modes, ability APIs,
   keyword names, selector parts, cost actions, and expression functions
   are all open sets. New values are added regularly as new card
   mechanics are implemented. The grammar does not enumerate them
   exhaustively.

6. **SVar composition**: Complex behaviors are built by chaining SVars.
   `Execute$ TrigDraw` references `SVar:TrigDraw:DB$ Draw | ...`. This
   is the primary composition mechanism and forms a directed graph
   (§9.1).

7. **Cost string looseness**: Cost strings are not a strict grammar but
   a left-to-right token sequence (§4.9). This reflects the original
   Java parser's approach and the variety of cost patterns in the corpus.

8. **Defaults are external**: Ability-specific default values (§10) are
   not part of the grammar or the card script. They live in the engine's
   ability factory and are applied at construction time.
