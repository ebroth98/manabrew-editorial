# Ecosystem

`manabrew` is part of a wider open-source card-game software ecosystem. This
document lists adjacent projects and explains how this repository is positioned
relative to them.

The goal is not to rank projects. Different implementations make different
trade-offs around licensing, rules authority, card coverage, UI, hosting, AI,
and portability.

## Forge

[Forge](https://github.com/Card-Forge/forge) is the upstream project and the
reference implementation for `manabrew`.

`manabrew` is a GPL Rust port of Forge's rules engine. It consumes Forge card
scripts, vendors the Java source as a reference, and uses Java Forge as the
behavioral oracle through the parity harness.

The working name `manabrew` is subject to courtesy discussion with Forge
maintainers before public release. The intent is to be clear that this project
derives from Forge while avoiding confusion about official status or support.

## Long-running community clients

Several long-running community projects provide useful context for how fan-made
card-game software survives and serves players over time.

- [Forge](https://github.com/Card-Forge/forge): desktop client and rules engine
  with a large card-script corpus.
- [XMage](https://github.com/magefree/mage): open-source client/server rules
  implementation.
- [Cockatrice](https://github.com/Cockatrice/Cockatrice): open-source virtual
  tabletop for multiplayer play.

These projects are independent communities. `manabrew` does not speak for them.

## Modern engine projects

Other open-source projects are also exploring modern implementations of Magic:
The Gathering rules engines and clients.

- [phase-rs](https://github.com/phase-rs/phase): Rust/WASM engine and client
  project.
- [Argentum Engine](https://github.com/wingedsheep/argentum-engine): Kotlin/JVM
  engine and client project.

`manabrew` takes a different path from from-scratch engines: it is Forge-derived
and keeps Java Forge in the loop as the behavioral oracle. The differentiator is
not that this repository is the only modern implementation. The differentiator
is the Forge parity workflow: same decks, same seeds, same deterministic
choices, and side-by-side comparison against Java Forge.

We are open to interoperability and shared correctness work where it is useful,
including protocol discussion, reproducible parity findings, and cross-engine
test cases.

## Hosted Forge frontends

[Forge Web](https://forgeweb.app/about) is an existing browser-based frontend
that describes itself as powered by the open-source Card-Forge engine. Its
public about page describes live AI matches, browser play, drafting, puzzles,
deck building, and 20,000+ cards.

We have not found a public source repository for the Forge Web frontend. We are
not relying on any legal theory about whether a hosted web frontend over a GPL
Forge backend must itself be distributed under the GPL. Our approach is simpler:
the `manabrew` client, server, Java backend adapter, and Rust engine live in the
same public GPL repository, with license posture and Forge derivation documented
up front.

The existence of Forge Web is still useful context. It shows that a modern
browser surface over the Forge engine is a real user-facing shape. `manabrew`
pursues a related shape while keeping the full stack open, self-hostable, and
usable with either the Java Forge backend or the Rust parity engine.

## Card data and images

[Scryfall](https://scryfall.com) is an important public resource for card
metadata and card images. `manabrew` does not ship card images. When images or
printing-specific metadata are used, they should be fetched at runtime by the
user's instance and handled under the applicable third-party terms.

Forge card scripts and bundled card data are covered by Forge's GPL licensing
and are documented in [THIRD-PARTY-NOTICES.md](../THIRD-PARTY-NOTICES.md).
