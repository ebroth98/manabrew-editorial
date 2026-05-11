# AI Usage

`forge-rs` has been developed with substantial AI assistance. This is an
explicit part of the project workflow, not something hidden from contributors.

## Where AI helps

AI tools are useful for:

- comparing a Java Forge file with the Rust port;
- summarizing long parity traces;
- finding likely owners for a divergence;
- generating first-pass mechanical ports;
- producing card-script and SVar inventories;
- drafting documentation;
- checking whether a proposed change matches existing conventions.

The parity workflow is especially AI-friendly because it has an external oracle:
Java Forge. A model can propose a fix, but the harness decides whether behavior
matches.

## Where AI is not enough

AI must not be treated as a rules authority. For engine behavior, the authority
chain is:

1. Java Forge behavior in `forge/forge-game/`;
2. the parity harness;
3. project docs that describe Forge DSL semantics;
4. the contributor's reviewed reasoning.

Generated code that "looks right" is not enough. It needs a source in Forge or
a documented project convention.

## Contributor expectations

If you use AI in a contribution:

- review the generated diff yourself;
- remove invented abstractions and speculative error handling;
- verify names and file placement against the Java/Rust mirror structure;
- include the parity command, lint command, or manual check you ran;
- be ready to explain the change in review.

You do not need to disclose every prompt. You should disclose AI involvement
when it materially shaped the implementation, investigation, or docs.

## AI and parity work

The best AI-assisted parity loop is:

1. Run the failing parity command.
2. Ask the tool to summarize the first divergence.
3. Locate the likely Java owner.
4. Compare Java and Rust control flow.
5. Patch the missing Rust rule.
6. Re-run the same parity command.
7. Broaden to nearby parity cases if the mechanic is shared.

The important constraint is that the final patch must be rooted in Forge's
mechanic, not in the individual card that happened to fail.

## AI and public trust

The project should not imply that AI output is a substitute for maintainership.
AI speeds up mechanical work, but maintainers remain responsible for correctness,
licensing, review, security, and public communication.
