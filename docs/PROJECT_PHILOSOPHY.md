# Project Philosophy

`manabrew` exists to make Forge's rules knowledge available in new runtime
shapes without losing the correctness that makes Forge valuable.

## Why this exists

Forge is the reason this project is possible. It has accumulated rules behavior,
card-script coverage, and community knowledge over many years. Reimplementing
from printed card text alone would throw away that work and would make
correctness much harder to prove.

The project exists to ask a narrower question: what can we build if Forge's
rules model is available in Rust, with a modern UI, web/WASM deployment,
self-hosted multiplayer, deterministic parity traces, and typed internal
representations for the script domains that benefit from them?

That makes the project conservative in one dimension and experimental in
another. It is conservative about game behavior: Java Forge is the oracle, and
Rust should match it. It is experimental about runtime shape, packaging,
debugging tools, AI workflows, protocol boundaries, and deployment.

## Who this is for

The audience we have in mind first is small: groups of friends who want to play
Magic together online, across whatever devices and time zones they happen to
have, without giving up open source or a modern interface. The project should
be useful to that group before it is useful to anyone else.

This is a deliberately narrow lens. It keeps scope decisions honest: a feature
that helps a small distributed group play a real game is in scope; a feature
that only matters at the scale of a hosted platform is not. If the broader MTG
community finds the result useful, that is a welcome second-order effect, not
the goal we are optimizing for.

## Correctness first

The rules engine is judged against Java Forge. Rust code may use Rust data
structures, caching, compiled IR, and WASM-friendly packaging, but behavior that
appears in a game trace should match Forge unless a divergence is explicit,
documented, and intentional.

This means:

- read Forge before changing engine behavior;
- port mechanics, not individual cards;
- keep parity failures reproducible by deck, seed, and first divergence;
- prefer small fixes in the module that mirrors the Java source.

## Public honesty

The project is not complete. Some game flows work; many mechanics still need
porting or parity repair. Public documentation should say that plainly. A
correctness-grade harness with incomplete engine coverage is more useful than a
polished claim that hides the remaining gaps.

## Good-neighbor Forge posture

Forge is not just a dependency. It is the source of truth for card scripts,
mechanics, and expected behavior. `manabrew` should be framed as a GPL port and
companion effort, not as a successor or replacement.

The working engine / crate name `manabrew` is the only naming question we put
in front of Forge maintainers. The courtesy conversation is narrow: make our
existence known, confirm we are GPL-3.0-or-later and build on their work, and
confirm a crate name that does not cause confusion. If the Forge team would
rather we not use `-rs` on Forge, we change the engine name.

The user-facing product name is **ManaBrew**, hosted at `manabrew.app`.

## Self-hostable and non-commercial

The project should stay non-commercial and self-hostable. Users should be able
to run their own client, engine host, and relay. Project-operated infrastructure
should not be required to play, and the project should not operate a public
card-content distribution path.

Card images are not shipped. When images or card metadata are fetched, they are
fetched at runtime by the user's instance from third-party services under those
services' terms.

## Typed where it helps, compatible where it matters

Forge's card-script DSL remains the compatibility layer. Rust-side compiled IR
is valuable where it makes behavior easier to audit, faster to run, or less
stringly typed. It is not a license to change semantics.

The right direction is gradual:

- type engine-critical domains first;
- keep raw compatibility boundaries visible;
- inventory remaining raw DSL buckets;
- preserve late-bound SVar semantics.

## AI as acceleration, not authority

AI tools are useful for large mechanical work: trace inspection, Java/Rust
comparison, documentation passes, and coverage inventory. They do not replace
review. Every AI-assisted change still needs a human-readable reason, a source
in Forge or project docs, and a test or parity command that supports it.
